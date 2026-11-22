//! Moteur du sidecar : configuration, suivi de journal, corrélation et
//! réponses JSON-RPC (P-00 §7 bis). Les erreurs sont typées : aucun `panic`.

use std::cell::Cell;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use rgaa_analysis::Preuve;
use serde_json::{json, Value};

use crate::correlation::{self, Enonce, InstantAction};
use crate::journal;
use crate::Horloge;

/// Identifiant du manifeste et de la preuve.
pub const ID: &str = "screenreader-nvda";
pub const VERSION: &str = "0.1.0";
pub const TESTS: &[&str] = &["7.1.1", "7.1.2", "7.1.3", "7.5.1", "7.5.2", "7.5.3"];

/// Tout échec devient une « preuve manquante » with reason, jamais une panique.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("fichier de parole absent : {0}")]
    ParoleAbsente(String),
    #[error("fichier de parole illisible : {0}")]
    ParoleIllisible(String),
    #[error("fichier d'actions illisible : {0}")]
    ActionsIllisibles(String),
    #[error("actions invalides : {0}")]
    ActionsInvalides(String),
    #[error("NVDA non détecté")]
    NvdaNonDetecte,
    #[error("artefact non écrit : {0}")]
    ArtefactNonEcrit(String),
    #[error("paramètre manquant : {0}")]
    ParametreManquant(String),
    #[error("aucun énoncé corrélé à une activation")]
    AucuneCorrelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub separateur: String,
    pub environnement: Option<String>,
    pub nvda_detecte: Option<bool>,
    pub source: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            separateur: "  ".into(),
            environnement: None,
            nvda_detecte: None,
            source: "speech-logger".into(),
        }
    }
}

fn texte_optionnel(options: &Value, cle: &str) -> Option<String> {
    options
        .get(cle)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|texte| !texte.is_empty())
        .map(str::to_owned)
}

impl Configuration {
    /// Lit les options de `initialiser`, puis éventuellement celles de
    /// `analyser` (qui les surchargent).
    #[must_use]
    pub fn depuis_options(options: &Value) -> Self {
        let mut configuration = Self::default();
        configuration.appliquer(options);
        configuration
    }

    /// Applique une surcharge d'options sans écraser une valeur absente.
    pub fn appliquer(&mut self, options: &Value) {
        if let Some(separateur) = texte_optionnel(options, "separateur") {
            self.separateur = separateur;
        }
        if let Some(environnement) = texte_optionnel(options, "environnement") {
            self.environnement = Some(environnement);
        }
        if let Some(source) = texte_optionnel(options, "source") {
            self.source = source;
        }
        if let Some(detecte) = options.get("nvda_detecte").and_then(Value::as_bool) {
            self.nvda_detecte = Some(detecte);
        }
    }
}

/// Horloge de rejeu, activée uniquement en build de debug : elle rend les
/// tests du binaire déterministes sans `sleep` et sans dépendre de l'heure
/// réelle. En release, l'option `horodatage_simule` est ignorée.
#[derive(Debug)]
pub struct HorlogeSimulee {
    instants: Vec<DateTime<Utc>>,
    suivant: Cell<usize>,
}

impl HorlogeSimulee {
    #[must_use]
    pub fn new(instants: Vec<DateTime<Utc>>) -> Self {
        Self {
            instants,
            suivant: Cell::new(0),
        }
    }
}

impl Horloge for HorlogeSimulee {
    fn maintenant(&self) -> DateTime<Utc> {
        if self.instants.is_empty() {
            return Utc::now();
        }
        let index = self.suivant.get().min(self.instants.len() - 1);
        self.suivant.set(index + 1);
        self.instants[index]
    }
}

/// Lecture incrémentale du journal : à chaque appel, seuls les octets
/// nouvellement écrits sont parsés et horodatés (recherche §1.2, tail).
pub struct SuiveurJournal {
    separateur: String,
    octets_lus: u64,
    environnement: Option<String>,
    enonce_partiel: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LectureJournal {
    pub enonces: Vec<Enonce>,
    pub environnement: Option<String>,
    pub octets: Vec<u8>,
    pub complet: String,
}

impl SuiveurJournal {
    #[must_use]
    pub fn new(separateur: impl Into<String>) -> Self {
        Self {
            separateur: separateur.into(),
            octets_lus: 0,
            environnement: None,
            enonce_partiel: String::new(),
        }
    }

    /// Lit le journal depuis la position courante. Un fichier absent est une
    /// erreur typée ; un UTF-8 tronqué est décodé avec perte, jamais paniqué.
    ///
    /// # Errors
    ///
    /// Retourne [`Erreur::ParoleAbsente`] ou [`Erreur::ParoleIllisible`].
    pub fn lire<H: Horloge + ?Sized>(
        &mut self,
        chemin: &Path,
        horloge: &H,
    ) -> Result<LectureJournal, Erreur> {
        let octets = fs::read(chemin).map_err(|erreur| {
            if erreur.kind() == std::io::ErrorKind::NotFound {
                Erreur::ParoleAbsente(chemin.display().to_string())
            } else {
                Erreur::ParoleIllisible(format!("{} : {erreur}", chemin.display()))
            }
        })?;
        let debut = usize::try_from(self.octets_lus.min(octets.len() as u64)).unwrap_or(0);
        let nouveaux = &octets[debut..];
        let complet = format!(
            "{}{}",
            self.enonce_partiel,
            String::from_utf8_lossy(nouveaux)
        );

        // On garde la dernière ligne incomplète pour le prochain passage :
        // un journal encore ouvert peut s'arrêter au milieu d'une écriture.
        let (a_parser, partiel) = match complet.rfind('\n') {
            Some(index) => (&complet[..index], complet[index + 1..].to_owned()),
            None => ("", complet.clone()),
        };

        let journal = journal::analyser(a_parser, &self.separateur);
        if self.environnement.is_none() {
            self.environnement = journal.environnement.clone();
        }
        let environnement = journal.environnement.or_else(|| self.environnement.clone());
        let enonces = journal
            .enonces
            .into_iter()
            .map(|texte| Enonce {
                texte,
                horodatage: horloge.maintenant(),
            })
            .collect();

        self.enonce_partiel = partiel;
        self.octets_lus = octets.len() as u64;
        Ok(LectureJournal {
            enonces,
            environnement,
            octets: nouveaux.to_vec(),
            complet,
        })
    }

    /// Dernier énoncé sans newline : on le traite à la fin d'une analyse.
    pub fn dernier_enonce<H: Horloge + ?Sized>(&mut self, horloge: &H) -> Option<Enonce> {
        let partiel = std::mem::take(&mut self.enonce_partiel);
        let journal = journal::analyser(&partiel, &self.separateur);
        journal.enonces.into_iter().next().map(|texte| Enonce {
            texte,
            horodatage: horloge.maintenant(),
        })
    }
}

fn instant_depuis(valeur: &Value) -> Option<DateTime<Utc>> {
    valeur
        .get("t")
        .and_then(Value::as_str)
        .and_then(|texte| DateTime::parse_from_rfc3339(texte).ok())
        .map(|instant| instant.with_timezone(&Utc))
}

/// Construit les actions depuis la demande et les instants du JSONL.
///
/// # Errors
///
/// Retourne [`Erreur::ActionsInvalides`] sur une ligne JSONL mal formée.
pub fn analyser_actions(
    definitions: &Value,
    instants_jsonl: Option<&str>,
) -> Result<Vec<InstantAction>, Erreur> {
    let mut instants: IndexMap<String, DateTime<Utc>> = IndexMap::new();
    if let Some(texte) = instants_jsonl {
        for ligne in texte.lines().filter(|ligne| !ligne.trim().is_empty()) {
            let valeur: Value = serde_json::from_str(ligne).map_err(|erreur| {
                Erreur::ActionsInvalides(format!("JSONL d'actions : {erreur}"))
            })?;
            let Some(id) = valeur.get("id").and_then(Value::as_str) else {
                tracing::warn!(%ligne, "instant d'action sans id : ignoré");
                continue;
            };
            if let Some(instant) = instant_depuis(&valeur) {
                instants.insert(id.to_owned(), instant);
            }
        }
    }

    let Some(definitions) = definitions.as_array() else {
        return Err(Erreur::ParametreManquant("actions".into()));
    };
    let mut actions = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let Some(id) = definition.get("id").and_then(Value::as_str) else {
            tracing::warn!("action sans id : ignorée");
            continue;
        };
        let Some(type_action) = definition.get("type").and_then(Value::as_str) else {
            tracing::warn!(action = id, "action sans type : ignorée");
            continue;
        };
        let fenetre_ms = definition
            .get("fenetre_ms")
            .and_then(Value::as_u64)
            .unwrap_or(1500);
        actions.push(InstantAction {
            id: id.to_owned(),
            type_action: type_action.to_owned(),
            selecteur: texte_optionnel(definition, "selecteur"),
            declencheur: texte_optionnel(definition, "declencheur"),
            nom_accessible_attendu: texte_optionnel(definition, "nom_accessible_attendu"),
            fenetre_ms,
            t: instants
                .get(id)
                .copied()
                .or_else(|| instant_depuis(definition)),
            xpath: texte_optionnel(definition, "xpath"),
            bbox: definition
                .get("bbox")
                .cloned()
                .and_then(|valeur| serde_json::from_value(valeur).ok()),
        });
    }
    Ok(actions)
}

/// Agrégateur : configuration + journal suivi + corrélation.
pub struct Analyseur<H: Horloge> {
    configuration: Configuration,
    suiveur: SuiveurJournal,
    horloge: H,
}

impl<H: Horloge> Analyseur<H> {
    #[must_use]
    pub fn new(configuration: Configuration, horloge: H) -> Self {
        let separateur = configuration.separateur.clone();
        Self {
            configuration,
            suiveur: SuiveurJournal::new(separateur),
            horloge,
        }
    }

    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    /// Traite une requête `analyser`. Les fichiers voyagent par chemins
    /// (P-00 §7 bis) ; jamais de réseau.
    ///
    /// # Errors
    ///
    /// Chaque échec est une raison de « preuve manquante », jamais une panique.
    pub fn analyser(&mut self, params: &Value) -> Result<Vec<Preuve>, Erreur> {
        if let Some(options) = params.get("options") {
            self.configuration.appliquer(options);
        }
        if self.configuration.nvda_detecte == Some(false) {
            return Err(Erreur::NvdaNonDetecte);
        }
        let fichiers = params.get("fichiers");
        let parole = fichiers
            .and_then(|fichiers| fichiers.get("parole"))
            .and_then(Value::as_str)
            .ok_or_else(|| Erreur::ParametreManquant("fichiers.parole".into()))?;
        let chemin_parole = Path::new(parole);

        let instants_jsonl = match fichiers
            .and_then(|fichiers| fichiers.get("actions"))
            .and_then(Value::as_str)
        {
            Some(chemin) => Some(
                fs::read_to_string(chemin)
                    .map_err(|erreur| Erreur::ActionsIllisibles(format!("{chemin} : {erreur}")))?,
            ),
            None => None,
        };
        let actions = analyser_actions(
            params.get("actions").unwrap_or(&Value::Null),
            instants_jsonl.as_deref(),
        )?;

        let lecture = self.suiveur.lire(chemin_parole, &self.horloge)?;
        let mut enonces = lecture.enonces;
        if let Some(dernier) = self.suiveur.dernier_enonce(&self.horloge) {
            enonces.push(dernier);
        }
        let environnement = self
            .configuration
            .environnement
            .clone()
            .or(lecture.environnement);
        if environnement.is_none() {
            tracing::warn!("preuve screenreader sans environnement déclaré");
        }

        let articulation = enonces
            .iter()
            .map(|enonce| {
                json!({
                    "texte": enonce.texte,
                    "horodatage": correlation::horodatage(&enonce.horodatage),
                    "horodatage_type": "reception",
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let articulation = format!("{articulation}\n");
        let empreinte = format!(
            "sha256:{}",
            rgaa_analysis::sha256_hex(
                &[
                    lecture.octets.as_slice(),
                    instants_jsonl.as_deref().unwrap_or_default().as_bytes(),
                ]
                .concat()
            )
        );
        self.ecrire_artefact(&articulation)?;

        let sujets = params
            .get("sujets")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let preuves = correlation::preuves(&correlation::Correlation {
            actions: &actions,
            enonces: &enonces,
            environnement: environnement.as_deref(),
            sujets,
            parole_jsonl: &articulation,
            empreinte_entree: &empreinte,
        });
        if preuves.is_empty() {
            return Err(Erreur::AucuneCorrelation);
        }
        Ok(preuves)
    }

    fn ecrire_artefact(&self, articulation: &str) -> Result<(), Erreur> {
        let sha256 = rgaa_analysis::sha256_hex(articulation.as_bytes());
        let dossier = std::env::temp_dir().join("rgaa-screenreader");
        fs::create_dir_all(&dossier)
            .map_err(|erreur| Erreur::ArtefactNonEcrit(erreur.to_string()))?;
        let chemin = dossier.join(format!("{sha256}.jsonl"));
        fs::write(&chemin, articulation)
            .map_err(|erreur| Erreur::ArtefactNonEcrit(format!("{} : {erreur}", chemin.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HorlogeFixe;

    fn instant(valeur: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(valeur)
            .expect("instant RFC 3339")
            .with_timezone(&Utc)
    }

    #[test]
    fn les_options_par_defaut_sont_celles_de_la_recherche() {
        let configuration = Configuration::default();
        assert_eq!(configuration.separateur, "  ");
        assert_eq!(configuration.source, "speech-logger");
        assert_eq!(configuration.nvda_detecte, None);
    }

    #[test]
    fn un_separateur_personnalise_est_lu_depuis_les_options() {
        let options = json!({"separateur": "|", "environnement": "NVDA 2026.3.1 + Firefox 141.0", "nvda_detecte": false});
        let configuration = Configuration::depuis_options(&options);
        assert_eq!(configuration.separateur, "|");
        assert_eq!(
            configuration.environnement.as_deref(),
            Some("NVDA 2026.3.1 + Firefox 141.0")
        );
        assert_eq!(configuration.nvda_detecte, Some(false));
    }

    #[test]
    fn le_suiveur_ne_relit_pas_les_enerces_deja_lus() {
        let dossier = tempfile::tempdir().expect("dossier temporaire");
        let chemin = dossier.path().join("speech.log");
        fs::write(&chemin, "menu, bouton\n").expect("journal");
        let horloge = HorlogeFixe(instant("2026-11-22T09:14:00.204Z"));
        let mut suiveur = SuiveurJournal::new("  ");
        let premier = suiveur.lire(&chemin, &horloge).expect("lecture");
        assert_eq!(premier.enonces.len(), 1);
        let second = suiveur.lire(&chemin, &horloge).expect("lecture");
        assert!(second.enonces.is_empty(), "tail : rien de nouveau");
    }

    #[test]
    fn le_suiveur_voit_les_enerces_ajoutees_et_les_horodate_a_la_lecture() {
        let dossier = tempfile::tempdir().expect("dossier temporaire");
        let chemin = dossier.path().join("speech.log");
        fs::write(&chemin, "menu, bouton\n").expect("journal");
        let premier = HorlogeFixe(instant("2026-11-22T09:14:00.204Z"));
        let second = HorlogeFixe(instant("2026-11-22T09:14:02.481Z"));
        let mut suiveur = SuiveurJournal::new("  ");
        assert_eq!(
            suiveur
                .lire(&chemin, &premier)
                .expect("lecture")
                .enonces
                .len(),
            1
        );
        fs::write(&chemin, "menu, bouton\nmenu, bouton, coché\n").expect("ajout");
        let lecture = suiveur.lire(&chemin, &second).expect("lecture");
        assert_eq!(lecture.enonces.len(), 1);
        assert_eq!(lecture.enonces[0].texte, "menu, bouton, coché");
        assert_eq!(
            lecture.enonces[0].horodatage,
            instant("2026-11-22T09:14:02.481Z")
        );
        assert_eq!(lecture.enonces[0].horodatage, second.maintenant());
    }

    #[test]
    fn un_fichier_absent_donne_une_erreur_typee() {
        let dossier = tempfile::tempdir().expect("dossier temporaire");
        let mut suiveur = SuiveurJournal::new("  ");
        let erreur = suiveur
            .lire(&dossier.path().join("absent.log"), &HorlogeFixe(Utc::now()))
            .expect_err("fichier absent");
        assert!(matches!(erreur, Erreur::ParoleAbsente(_)));
        assert!(erreur.to_string().contains("absent.log"));
    }

    #[test]
    fn les_instants_sont_joints_aux_actions_par_identifiant() {
        let definitions = json!([
            {"id": "a1", "type": "focus", "selecteur": "#menu-button", "fenetre_ms": 1500},
            {"id": "a2", "type": "activation", "selecteur": "#menu-button", "declencheur": "Enter", "fenetre_ms": 2500}
        ]);
        let instants = "{\"id\":\"a1\",\"t\":\"2026-11-22T09:14:00.204Z\"}\n{\"id\":\"a2\",\"t\":\"2026-11-22T09:14:02.120Z\"}\n";
        let actions = analyser_actions(&definitions, Some(instants)).expect("actions");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[1].id, "a2");
        assert_eq!(actions[1].t, Some(instant("2026-11-22T09:14:02.120Z")));
        assert_eq!(actions[1].fenetre_ms, 2500);
        assert_eq!(actions[1].declencheur.as_deref(), Some("Enter"));
    }

    #[test]
    fn une_ligne_dactions_invalide_est_une_erreur_pas_une_panique() {
        let definitions = json!([]);
        let erreur =
            analyser_actions(&definitions, Some("{pas du json\n")).expect_err("JSONL invalide");
        assert!(matches!(erreur, Erreur::ActionsInvalides(_)));
    }
}
