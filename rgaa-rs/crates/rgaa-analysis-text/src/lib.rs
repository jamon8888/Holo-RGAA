//! P-08 — plugin natif de pertinence textuelle (spec `08-pertinence-textuelle.md`).
//!
//! Le plugin mesure d'abord (listes génériques, titres répétés, étiquettes
//! vides, finalités, plans, versions alternatives…), regroupe les cas restants
//! par critère, puis prépare des lots LLM versionnés. **Il ne contacte aucun
//! modèle dans ce ticket** : [`llm::ModelClient`] est le point d'extension et
//! [`llm::NoopModelClient`] ne rend jamais d'avis. Les preuves ne contiennent
//! jamais de statut ni de verdict (R-P1).

pub mod llm;
pub mod mesures;
pub mod regles;
pub mod schema;
pub mod similarite;
pub mod snapshot;

pub use llm::{
    CacheKey, ContexteReduit, ElementLlm, LlmCache, LlmRequest, LlmResponse, ModelClient,
    NoopModelClient, ReponseElement,
};
pub use mesures::{AvisModele, Mesure, Mesures, Pertinence};
pub use regles::formulaires::{FormObservation, FormProbe, NoopFormProbe};
pub use snapshot::TextSnapshot;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use llm::{appliquer_reponse, preparer_lots};
use rgaa_analysis::{
    analysis_error, sha256_hex, AnalysisPlugin, PageCapture, PluginOptions, Portee, Preuve, Result,
};

/// Identifiant stable du plugin.
pub const TEXT_ID: &str = "text";
/// Tests RGAA servis par P-08 (clés du catalogue `rgaa-core`).
pub const TEXT_TESTS: &[&str] = &[
    "1.5.1", "2.2.1", "5.2.1", "5.5.1", "6.1.1", "7.2.1", "8.6.1", "11.2.1", "11.7.1", "11.9.1",
    "11.10.1", "11.11.1", "11.12.1", "11.13.1", "12.3.1", "13.4.1", "13.6.1",
];

/// Plugin P-08. Le cache est propre à une instance, donc à un audit.
pub struct TextPlugin {
    client: Arc<dyn ModelClient>,
    sonde: Arc<dyn FormProbe>,
    cache: Mutex<LlmCache>,
}

impl Default for TextPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TextPlugin {
    /// Plugin sans client LLM et sans sonde de formulaire.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Arc::new(NoopModelClient),
            sonde: Arc::new(NoopFormProbe),
            cache: Mutex::new(LlmCache::default()),
        }
    }

    /// Branche un client LLM (point d'extension, aucun appel dans ce ticket).
    #[must_use]
    pub fn with_client(client: Arc<dyn ModelClient>) -> Self {
        Self {
            client,
            ..Self::new()
        }
    }

    /// Branche une sonde de formulaire sans effet de bord (point d'extension).
    #[must_use]
    pub fn with_form_probe(sonde: Arc<dyn FormProbe>) -> Self {
        Self {
            sonde,
            ..Self::new()
        }
    }
}

fn convertir(mesure: Mesure, entre: &str, page_url: &str) -> Result<Preuve> {
    let mesures = serde_json::to_value(&mesure.donnees)
        .map_err(|error| analysis_error(TEXT_ID, format!("mesures sérialisables : {error}")))?;
    let empreinte = sha256_hex(format!("text\0{page_url}\0{entre}\0{}", mesure.test).as_bytes());
    Ok(Preuve {
        plugin: TEXT_ID.to_owned(),
        version_plugin: env!("CARGO_PKG_VERSION").to_owned(),
        version_modele: Some(llm::version_modele()),
        tests: vec![mesure.test.to_owned()],
        sujet: mesure.sujet,
        portee: mesure.portee,
        mesures,
        indice: mesure.indice,
        artefacts: Vec::new(),
        empreinte_entree: empreinte,
    })
}

#[async_trait]
impl AnalysisPlugin for TextPlugin {
    fn id(&self) -> &'static str {
        TEXT_ID
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn tests(&self) -> &'static [&'static str] {
        TEXT_TESTS
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        _options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let json = page.dom_snapshot.json.trim();
        if json.is_empty() {
            return Ok(Vec::new());
        }
        let mut snapshot: TextSnapshot = serde_json::from_str(json).map_err(|error| {
            analysis_error(TEXT_ID, format!("snapshot textuel illisible : {error}"))
        })?;

        // La sonde n'observe que : aucune soumission, aucun effet de bord.
        for formulaire in &mut snapshot.page.formulaires {
            let observation = self.sonde.observe(formulaire)?;
            if !observation.messages_erreur.is_empty() {
                formulaire
                    .messages_erreur
                    .extend(observation.messages_erreur);
            }
            if observation.recapitulatif_detecte.is_some() {
                formulaire.recapitulatif_detecte = observation.recapitulatif_detecte;
            }
        }

        let mut mesures = regles::mesurer(&snapshot);
        let lots = {
            let mut cache = self.cache.lock().map_err(|error| {
                analysis_error(TEXT_ID, format!("cache LLM inaccessible : {error}"))
            })?;
            preparer_lots(&mut mesures, &mut cache)
        };
        for requete in &lots.requetes {
            if let Ok(Some(reponse)) = self.client.complete(requete) {
                let mut cache = self.cache.lock().map_err(|error| {
                    analysis_error(TEXT_ID, format!("cache LLM inaccessible : {error}"))
                })?;
                let _ = appliquer_reponse(
                    &mut mesures,
                    &mut cache,
                    requete,
                    &reponse,
                    &lots.correspondances,
                );
            }
        }

        let entre = format!("text\0{}\0{}", page.url, json);
        mesures
            .into_iter()
            .map(|mesure| convertir(mesure, &entre, page.url))
            .collect()
    }
}

/// Vrai si une mesure est purement déterministe (aucun signal de modèle).
#[must_use]
pub fn est_determininiste(preuve: &Preuve) -> bool {
    preuve.portee == Portee::Mesuree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiant_et_tests_sont_stables() {
        let plugin = TextPlugin::new();
        assert_eq!(plugin.id(), "text");
        assert_eq!(plugin.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(plugin.tests(), TEXT_TESTS);
        assert!(
            plugin.models().is_empty(),
            "aucun modèle téléchargé en audit"
        );
    }

    #[test]
    fn tous_les_tests_sont_des_cles_du_catalogue() {
        let cles: Vec<String> = rgaa_core::RgaaCatalog::all_test_keys()
            .into_iter()
            .map(|(critere, test)| format!("{critere}.{test}"))
            .collect();
        for test in TEXT_TESTS {
            assert!(
                cles.contains(&(*test).to_owned()),
                "test absent du catalogue : {test}"
            );
        }
    }
}
