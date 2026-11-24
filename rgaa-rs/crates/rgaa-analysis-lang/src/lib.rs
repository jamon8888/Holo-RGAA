//! P-06 — native language detection plugin (spec `06-langue.md`).
//!
//! It detects the page language (8.4, with support for 8.3) and blocks whose
//! language differs from their inherited `lang` (8.7 when unmarked, 8.8 when a
//! `lang` is declared). It returns proofs only, never a status (R-P1): the
//! `Indice` is a review-queue hint, not a verdict.

mod blocs;
mod detection;
pub mod mesures;

use async_trait::async_trait;
use mesures::{BlocDivergent, LangMesures};
use rgaa_analysis::{
    analysis_error, sha256_hex, Indice, PageCapture, PluginOptions, Portee, Preuve, Rect, Result,
    Subject,
};

pub use detection::{detecte, exclusions_par_defaut, normaliser_lang, page_suspecte};

const VERSION_MODELE: &str = "lingua@1.8.0";
const TEST_NON_SIGNALE: &str = "8.7.1";
const TEST_LANG_NON_PERTINENT: &str = "8.8.1";

/// Extension point for the optional LLM filter (spec §2.6).
///
/// Not implemented in P-06: every 8.7 candidate is kept. A future
/// implementation can plug a model here without changing the contract or the
/// shape of the proofs.
pub trait CandidateFilter: Send + Sync {
    /// Called with every 8.7 candidate, just before proofs are built.
    fn retain(&self, candidats: &mut Vec<BlocDivergent>);
}

/// Default filter: keeps all candidates; `opts.llm` is deliberately inert.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoCandidateFilter;

impl CandidateFilter for NoCandidateFilter {
    fn retain(&self, _candidats: &mut Vec<BlocDivergent>) {}
}

/// The native language plugin.
pub struct LangPlugin {
    filtre: Box<dyn CandidateFilter>,
}

impl LangPlugin {
    /// Builds a plugin without the optional LLM filter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            filtre: Box::new(NoCandidateFilter),
        }
    }

    /// Builds a plugin with an explicit candidate filter (extension point).
    #[must_use]
    pub fn with_filter(filtre: Box<dyn CandidateFilter>) -> Self {
        Self { filtre }
    }
}

impl Default for LangPlugin {
    fn default() -> Self {
        Self::new()
    }
}

fn preuve_page(mesures: &LangMesures, indice: Option<Indice>, hash: &str) -> Result<Preuve> {
    Ok(Preuve {
        plugin: "lang".to_owned(),
        version_plugin: env!("CARGO_PKG_VERSION").to_owned(),
        version_modele: Some(VERSION_MODELE.to_owned()),
        tests: vec!["8.4.1".to_owned()],
        sujet: Subject::Page,
        portee: Portee::Estimee,
        mesures: serde_json::to_value(mesures)
            .map_err(|error| analysis_error("lang", format!("mesures sérialisables : {error}")))?,
        indice,
        artefacts: Vec::new(),
        empreinte_entree: hash.to_owned(),
    })
}

fn preuve_bloc(
    test: &str,
    bloc: &BlocDivergent,
    xpath: &str,
    raison: String,
    hash: &str,
) -> Result<Preuve> {
    Ok(Preuve {
        plugin: "lang".to_owned(),
        version_plugin: env!("CARGO_PKG_VERSION").to_owned(),
        version_modele: Some(VERSION_MODELE.to_owned()),
        tests: vec![test.to_owned()],
        sujet: Subject::Element {
            selecteur: bloc.selecteur.clone(),
            xpath: xpath.to_owned(),
            bbox: Rect::default(),
        },
        portee: Portee::Estimee,
        mesures: serde_json::to_value(bloc)
            .map_err(|error| analysis_error("lang", format!("mesures sérialisables : {error}")))?,
        indice: Some(Indice::Suspect { raison }),
        artefacts: Vec::new(),
        empreinte_entree: hash.to_owned(),
    })
}

#[async_trait]
impl rgaa_analysis::AnalysisPlugin for LangPlugin {
    fn id(&self) -> &'static str {
        "lang"
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn tests(&self) -> &'static [&'static str] {
        &["8.3.1", "8.4.1", "8.7.1", "8.8.1"]
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        // The P-00 convenience runner has no DOM capture yet: never invent a
        // language from an empty snapshot.
        let blocs = if page.dom_snapshot.json.trim().is_empty() {
            Vec::new()
        } else {
            blocs::segmenter(&page.dom_snapshot.json, page.lang_declared)?
        };
        let exclusions = options
            .exclusions
            .as_deref()
            .map_or_else(detection::exclusions_par_defaut, <[String]>::to_vec);
        let analyse = detection::detecte_page(&blocs, page.lang_declared, &exclusions);

        // Only the 8.7 candidates are offered to the optional LLM extension.
        let mut candidats: Vec<BlocDivergent> = analyse
            .divergences
            .iter()
            .filter(|divergence| !divergence.explicite)
            .map(|divergence| divergence.mesure.clone())
            .collect();
        self.filtre.retain(&mut candidats);

        let entre = format!(
            "lang\0{}\0{}",
            page.lang_declared.unwrap_or_default(),
            page.dom_snapshot.json
        );
        let hash_page = sha256_hex(entre.as_bytes());
        let mesures_page = LangMesures {
            declaree: analyse.declaree.clone(),
            detectee: analyse.detectee.clone(),
            confiance: analyse.confiance,
            blocs_divergents: candidats.clone(),
        };
        let mut preuves = Vec::new();
        if analyse.detectee.is_some() {
            let indice = detection::page_suspecte(&analyse).then(|| {
                match (&analyse.declaree, &analyse.detectee) {
                    (Some(declaree), Some(detectee)) => Indice::Suspect {
                        raison: format!("page détectée en {detectee}, déclarée {declaree}"),
                    },
                    _ => Indice::Neutre,
                }
            });
            preuves.push(preuve_page(&mesures_page, indice, &hash_page)?);
        }

        for candidat in &candidats {
            let source = analyse.divergences.iter().find(|divergence| {
                !divergence.explicite
                    && divergence.mesure.selecteur == candidat.selecteur
                    && divergence.mesure.detectee == candidat.detectee
            });
            let xpath = source.map_or("", |divergence| divergence.xpath.as_str());
            let raison = format!(
                "langue détectée {} alors que le lang hérité est {}",
                candidat.detectee, candidat.lang_herite
            );
            preuves.push(preuve_bloc(
                TEST_NON_SIGNALE,
                candidat,
                xpath,
                raison,
                &hash_page,
            )?);
        }

        for divergence in analyse.divergences.iter().filter(|d| d.explicite) {
            let candidat = &divergence.mesure;
            let raison = format!(
                "lang explicite {} contredit par la détection {}",
                candidat.lang_herite, candidat.detectee
            );
            preuves.push(preuve_bloc(
                TEST_LANG_NON_PERTINENT,
                candidat,
                &divergence.xpath,
                raison,
                &hash_page,
            )?);
        }

        Ok(preuves)
    }
}
