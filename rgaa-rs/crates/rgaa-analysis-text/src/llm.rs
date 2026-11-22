//! Préparation des lots LLM, cache et point d'extension (spec §4).
//!
//! Aucun appel réseau n'est fait ici. [`ModelClient`] est le point où un futur
//! client Mistral Large 4 se branchera ; [`NoopModelClient`] ne répond jamais,
//! si bien que `avis_modele` reste `null` dans les preuves. Les requêtes et les
//! réponses sont versionnées, et l'empreinte du prompt figure dans
//! `version_modele` (critère d'acceptation 3).

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::mesures::{AvisModele, Mesure, Pertinence};
use crate::similarite::{normaliser, tronquer};
use indexmap::IndexMap;
use rgaa_analysis::{analysis_error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Version du contrat de requête/réponse.
pub const SCHEMA_VERSION: u32 = 1;
/// Identifiant stable du prompt, incrémenté à chaque modification.
pub const PROMPT_ID: &str = "pertinence-textuelle-v1";
/// Modèle prévu par la spec, non appelé dans ce ticket.
pub const MODELE: &str = "mistral-large-4";
/// La sortie représentant 83 % du coût (PRD §8.2).
pub const REASONING_EFFORT: &str = "low";
/// Nombre maximal d'éléments par requête (spec §4).
pub const MAX_PAR_LOT: usize = 40;
/// Longueur maximale du contexte réduit (spec §4).
pub const MAX_EXTRAIT: usize = 300;
/// Critères dont l'alternative absente est mesurée, sinon regroupée (spec §3).
pub const CRITERES_ALTERNATIVES: &[&str] = &["1.5", "7.2", "13.6"];

/// Prompt versionné. Toute modification doit incrémenter [`PROMPT_ID`] :
/// l'empreinte ci-dessous entre dans `version_modele` des preuves.
pub const PROMPT: &str = "pertinence-textuelle-v1\n\
    Tu reçois des éléments de texte et leur contexte réduit (300 caractères maximum). \
    Pour chacun, dis si son intitulé est pertinent au regard du critère RGAA indiqué. \
    Réponds uniquement avec un objet JSON {\"schema_version\":1,\"prompt_sha256\":\"...\",\
    \"reponses\":[{\"id\":\"...\",\"pertinent\":\"oui|non|incertain\",\"explication\":\"...\"}]}. \
    Ne rends aucun statut de conformité : l'avis ne sert qu'à ordonner la revue humaine.";

/// Empreinte SHA-256 du prompt, calculée une seule fois.
#[must_use]
pub fn prompt_sha256() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| format!("{:x}", Sha256::digest(PROMPT.as_bytes())))
}

/// Valeur de `version_modele` portée par les preuves P-08.
#[must_use]
pub fn version_modele() -> String {
    format!(
        "{MODELE}:{REASONING_EFFORT}/{PROMPT_ID}@{}",
        prompt_sha256()
    )
}

/// Contexte envoyé au modèle, borné et sans page entière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContexteReduit {
    pub titre_section: String,
    pub extrait: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cible: Option<String>,
}

impl ContexteReduit {
    /// Construit un contexte dont l'extrait tient en 300 caractères.
    #[must_use]
    pub fn new(titre_section: impl Into<String>, extrait: &str, cible: Option<String>) -> Self {
        Self {
            titre_section: titre_section.into(),
            extrait: tronquer(extrait, MAX_EXTRAIT),
            cible,
        }
    }

    /// Empreinte de la clé de cache : le titre de section et l'extrait, **pas**
    /// la cible du lien. Sept « En savoir plus » du pied de page vers sept URL
    /// partagent donc une seule clé (spec §6) tout en gardant leur cible dans
    /// l'élément envoyé.
    #[must_use]
    pub fn contexte_hash(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(format!("{}\0{}", self.titre_section, self.extrait).as_bytes())
        )
    }
}

/// Un élément d'un lot LLM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ElementLlm {
    pub id: String,
    pub intitule: String,
    pub contexte: ContexteReduit,
    pub signaux: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

impl ElementLlm {
    /// Construit un élément à partir des signaux déjà calculés.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        intitule: impl Into<String>,
        contexte: ContexteReduit,
        signaux: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            intitule: intitule.into(),
            contexte,
            signaux,
            avis_modele: None,
        }
    }
}

/// Clé de cache `(critère, intitulé normalisé, contexte_hash)` de la spec §4.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub critere: String,
    pub intitule_normalise: String,
    pub contexte_hash: String,
}

impl CacheKey {
    /// Dérive la clé d'un élément et du test qu'il sert.
    #[must_use]
    pub fn new(critere: &str, element: &ElementLlm) -> Self {
        Self {
            critere: critere.to_owned(),
            intitule_normalise: normaliser(&element.intitule),
            contexte_hash: element.contexte.contexte_hash(),
        }
    }

    /// Forme lisible pour les journaux et les tests.
    #[must_use]
    pub fn as_texte(&self) -> String {
        format!(
            "{}|{}|{}",
            self.critere, self.intitule_normalise, self.contexte_hash
        )
    }
}

/// Cache des avis déjà obtenus, pour ne renvoyer un groupe qu'une fois par audit.
#[derive(Debug, Clone, Default)]
pub struct LlmCache {
    reponses: IndexMap<CacheKey, Vec<AvisModele>>,
    /// Groupes déjà envoyés, même quand le modèle n'a pas répondu : un lot
    /// sans avis ne doit pas être renvoyé à chaque analyse du même audit.
    envoyes: IndexMap<CacheKey, usize>,
}

impl LlmCache {
    /// Vrai si le groupe a déjà été envoyé au modèle.
    #[must_use]
    pub fn contient(&self, cle: &CacheKey) -> bool {
        self.envoyes.contains_key(cle)
    }

    /// Marque le groupe comme envoyé, avec la position de l'élément.
    pub fn marquer(&mut self, cle: CacheKey, index: usize) {
        self.envoyes.entry(cle).or_insert(index);
    }

    /// Avis déjà reçu pour un élément d'un groupe, le cas échéant.
    #[must_use]
    pub fn get(&self, cle: &CacheKey, index: usize) -> Option<&AvisModele> {
        self.reponses
            .get(cle)
            .and_then(|avis| avis.get(index).or_else(|| avis.first()))
    }

    /// Enregistre l'avis d'un élément du groupe.
    pub fn insert(&mut self, cle: CacheKey, index: usize, avis: AvisModele) {
        let entre = self.reponses.entry(cle).or_default();
        while entre.len() <= index {
            let remplissage = entre.first().cloned().unwrap_or_else(|| avis.clone());
            entre.push(remplissage);
        }
        entre[index] = avis;
    }

    /// Nombre de groupes déjà envoyés.
    #[must_use]
    pub fn len(&self) -> usize {
        self.envoyes.len()
    }

    /// Vrai si aucun groupe n'a été envoyé.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.envoyes.is_empty()
    }
}

/// Configuration du modèle, exposée dans la requête versionnée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModeleConfig {
    pub nom: String,
    pub reasoning_effort: String,
}

/// Requête vers le modèle : un critère, au plus [`MAX_PAR_LOT`] éléments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LlmRequest {
    pub schema_version: u32,
    pub prompt_id: String,
    pub prompt_sha256: String,
    pub critere: String,
    pub modele: ModeleConfig,
    pub elements: Vec<ElementLlm>,
}

/// Réponse structurée attendue du modèle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LlmResponse {
    pub schema_version: u32,
    pub prompt_sha256: String,
    pub reponses: Vec<ReponseElement>,
}

/// Avis d'un élément, tel que spécifié en §4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReponseElement {
    pub id: String,
    pub pertinent: Pertinence,
    pub explication: String,
}

/// Point d'extension : aucun appel réel n'est effectué dans ce ticket.
pub trait ModelClient: Send + Sync {
    /// Renvoie `None` quand aucun avis n'est disponible.
    ///
    /// # Errors
    ///
    /// Une implémentation peut renvoyer [`rgaa_analysis::RgaaError::Analysis`]
    /// si la réponse est illisible ; l'audit continue avec `avis_modele` nul.
    fn complete(&self, requete: &LlmRequest) -> Result<Option<LlmResponse>>;
}

/// Client par défaut : ne répond jamais, ne contacte personne.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopModelClient;

impl ModelClient for NoopModelClient {
    fn complete(&self, _requete: &LlmRequest) -> Result<Option<LlmResponse>> {
        Ok(None)
    }
}

/// Résultat de la préparation des lots.
#[derive(Debug, Clone)]
pub struct LotsPrepares {
    pub requetes: Vec<LlmRequest>,
    /// Pour chaque identifiant d'élément : sa clé de cache et sa position.
    pub correspondances: IndexMap<String, (CacheKey, usize)>,
}

/// Regroupe les éléments restants par critère, les déduplique par clé de cache
/// et les répartit en lots de [`MAX_PAR_LOT`] au plus.
///
/// Les éléments déjà présents dans `cache` reçoivent directement leur avis et
/// ne partent pas au modèle.
pub fn preparer_lots(mesures: &mut [Mesure], cache: &mut LlmCache) -> LotsPrepares {
    let mut groupes: IndexMap<CacheKey, Vec<ElementLlm>> = IndexMap::new();
    let mut correspondances = IndexMap::new();
    for mesure in mesures.iter_mut() {
        let Some(element) = &mut mesure.element else {
            continue;
        };
        let cle = CacheKey::new(critere_du_test(mesure.test), element);
        let index = groupes.get(&cle).map_or(0, Vec::len);
        // Lit l'état du cache avant de marquer ce groupe : les autres
        // éléments du même lot doivent rester dans la même requête.
        let deja_vu = cache.contient(&cle) && !groupes.contains_key(&cle);
        if deja_vu {
            if let Some(avis) = cache.get(&cle, index) {
                let avis = avis.clone();
                mesure.set_avis(avis);
            }
            continue;
        }
        cache.marquer(cle.clone(), index);
        correspondances.insert(element.id.clone(), (cle.clone(), index));
        groupes.entry(cle).or_default().push(element.clone());
    }

    let mut par_critere: IndexMap<String, Vec<ElementLlm>> = IndexMap::new();
    for (cle, elements) in groupes {
        par_critere.entry(cle.critere).or_default().extend(elements);
    }

    let mut requetes = Vec::new();
    for (critere, elements) in par_critere {
        for lot in elements.chunks(MAX_PAR_LOT) {
            requetes.push(LlmRequest {
                schema_version: SCHEMA_VERSION,
                prompt_id: PROMPT_ID.to_owned(),
                prompt_sha256: prompt_sha256().to_owned(),
                critere: critere.clone(),
                modele: ModeleConfig {
                    nom: MODELE.to_owned(),
                    reasoning_effort: REASONING_EFFORT.to_owned(),
                },
                elements: lot.to_vec(),
            });
        }
    }
    LotsPrepares {
        requetes,
        correspondances,
    }
}

/// Applique une réponse du modèle aux mesures et la met en cache.
///
/// # Errors
///
/// Refuse une réponse dont la version ou l'empreinte du prompt ne correspond
/// pas à la requête : une réponse d'un autre prompt ne doit jamais contaminer
/// une preuve.
pub fn appliquer_reponse(
    mesures: &mut [Mesure],
    cache: &mut LlmCache,
    requete: &LlmRequest,
    reponse: &LlmResponse,
    correspondances: &IndexMap<String, (CacheKey, usize)>,
) -> Result<()> {
    if reponse.schema_version != SCHEMA_VERSION || reponse.prompt_sha256 != prompt_sha256() {
        return Err(analysis_error(
            "text",
            "réponse LLM d'une version ou d'un prompt différent",
        ));
    }
    let connus: HashMap<&str, &(CacheKey, usize)> = correspondances
        .iter()
        .map(|(id, cle)| (id.as_str(), cle))
        .collect();
    for item in &reponse.reponses {
        let Some((cle, index)) = connus.get(item.id.as_str()) else {
            continue;
        };
        let avis = AvisModele {
            pertinent: item.pertinent.clone(),
            explication: item.explication.clone(),
        };
        if let Some(mesure) = mesures
            .iter_mut()
            .find(|mesure| mesure.element.as_ref().is_some_and(|e| e.id == item.id))
        {
            mesure.set_avis(avis.clone());
        }
        cache.insert((*cle).clone(), *index, avis);
    }
    let _ = requete;
    Ok(())
}

/// Critère RGAA d'un identifiant de test (`6.1.1` → `6.1`).
#[must_use]
pub fn critere_du_test(test: &str) -> &str {
    test.rsplit_once('.').map_or(test, |(critere, _)| critere)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empreinte_du_prompt_dans_la_version_du_modele() {
        assert_eq!(prompt_sha256().len(), 64);
        assert!(version_modele().contains(prompt_sha256()));
        assert!(version_modele().contains(PROMPT_ID));
    }

    #[test]
    fn contexte_hash_ignore_la_cible_du_lien() {
        let a = ContexteReduit::new("Pied", "À propos", Some("/a".to_owned()));
        let b = ContexteReduit::new("Pied", "À propos", Some("/b".to_owned()));
        assert_eq!(a.contexte_hash(), b.contexte_hash());
    }

    #[test]
    fn extraits_sont_bornes_a_300_caracteres() {
        let contexte = ContexteReduit::new("Section", &"x".repeat(900), None);
        assert_eq!(contexte.extrait.chars().count(), MAX_EXTRAIT);
    }
}
