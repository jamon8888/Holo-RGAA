//! P-03 — native `images` analysis plugin.
//!
//! It inventories images in a captured DOM, applies deterministic rules on
//! textual alternatives, and attaches OCR / CLIP estimates when the local
//! models declared in [`models`] have been installed with `rgaa models pull`.
//! Nothing is downloaded during an audit (R-P3), no LLM is called, and a
//! CAPTCHA is never OCRised.
//!
//! The sidecars `ocr-tesseract` and `oar-ocr` are out of scope: they will
//! implement the [`ocr::OcrBackend`] extension trait in a later ticket.

pub mod cache;
pub mod clip;
pub mod inventory;
pub mod mesures;
pub mod models;
pub mod ocr;
pub mod phash;
pub mod regles;
pub mod schema;
pub mod svg;

use async_trait::async_trait;
use image::DynamicImage;
use inventory::{est_captcha, ImageNode, Snapshot};
use mesures::{MesureCaptcha, MesureClip, MesureOcr, Mesures};
use rgaa_analysis::{
    analysis_error, AnalysisPlugin, Indice, PageCapture, PluginOptions, Portee, Preuve, Result,
    Subject,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// RGAA tests served by this plugin (catalogue `rgaa-core`).
pub const IMAGES_TESTS: &[&str] = &["1.2.1", "1.3.1", "1.4.1", "1.7.1", "1.8.1"];
pub const IMAGES_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Maximum dimension of an image passed to a model (spec §7.2).
pub const VIGNETTE_MAX: u32 = 768;

/// The P-03 plugin, stateless except for its per-audit decision cache.
pub struct ImagesPlugin {
    ocr: Option<Arc<dyn ocr::OcrBackend>>,
    clip: Option<Arc<dyn clip::ClipBackend>>,
    models_dir: PathBuf,
    cache: Arc<Mutex<cache::DecisionCache>>,
}

impl Default for ImagesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ImagesPlugin {
    /// Uses the real native backends, whose models are read from the standard
    /// cache and never downloaded.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ocr: None,
            clip: None,
            models_dir: rgaa_analysis::models::models_dir(),
            cache: Arc::new(Mutex::new(cache::DecisionCache::new())),
        }
    }

    /// Injects backends (tests, future sidecars). `None` keeps the native one.
    #[must_use]
    pub fn with_backends(
        ocr: Option<Arc<dyn ocr::OcrBackend>>,
        clip: Option<Arc<dyn clip::ClipBackend>>,
    ) -> Self {
        Self {
            ocr,
            clip,
            ..Self::new()
        }
    }

    /// Overrides the model cache directory (tests, air-gapped installs).
    #[must_use]
    pub fn with_models_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.models_dir = dir.into();
        self
    }

    /// The per-audit decision cache, shared across calls to `analyze`.
    #[must_use]
    pub fn cache(&self) -> Arc<Mutex<cache::DecisionCache>> {
        self.cache.clone()
    }
}

/// Builds the P-03 plugin for an explicit registry.
#[must_use]
pub fn images_plugin() -> Box<dyn AnalysisPlugin> {
    Box::new(ImagesPlugin::new())
}

#[async_trait]
impl AnalysisPlugin for ImagesPlugin {
    fn id(&self) -> &'static str {
        "images"
    }

    fn version(&self) -> &'static str {
        IMAGES_VERSION
    }

    fn tests(&self) -> &'static [&'static str] {
        IMAGES_TESTS
    }

    fn models(&self) -> &'static [rgaa_analysis::ModelSpec] {
        models::MODELS
    }

    fn models_optional(&self) -> bool {
        // Déterministes d'abord (critère d'acceptation P-03 §7.1) : l'absence
        // d'un modèle donne une « preuve manquante » ciblée, pas un échec.
        true
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        _options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let snapshot = Snapshot::parse(&page.dom_snapshot.json)?;
        let ocr = initialiser_ocr(self);
        let clip = initialiser_clip(self);

        // Alternatives des autres images de la page, pour l'ancre de hasard.
        let alts_page: Vec<String> = snapshot
            .images
            .iter()
            .filter_map(|noeud| noeud.alt.as_deref())
            .filter(|alt| !alt.trim().is_empty())
            .map(str::to_owned)
            .collect();

        let mut preuves = Vec::with_capacity(snapshot.images.len() * 3);
        for noeud in &snapshot.images {
            // Contexte = texte voisin et légende, PAS l'URL : c'est ce qui
            // permet de mutualiser une image répétée sur 41 pages (spec §5).
            let contexte = format!(
                "{}\0{}",
                noeud.texte_adjacent.as_deref().unwrap_or_default(),
                noeud.legende.as_deref().unwrap_or_default()
            );
            let image = charger_image(page, noeud).await?;
            let p_hash = image
                .as_ref()
                .map(phash::calculer)
                .transpose()?
                .unwrap_or_default();
            let deja_analyse = self
                .cache
                .lock()
                .map_err(|_| analysis_error("images", "verrou du cache de décision empoisonné"))?
                .enregistrer(p_hash, alt_de(noeud), &contexte, page.url);
            let occurrences = self
                .cache
                .lock()
                .map_err(|_| analysis_error("images", "verrou du cache de décision empoisonné"))?
                .occurrences(p_hash);
            let empreinte = empreinte_entree(page.url, noeud, p_hash);

            preuves.push(preuve_deterministe(noeud, occurrences, &empreinte)?);

            if est_captcha(noeud) {
                preuves.push(preuve_captcha(noeud, &empreinte)?);
                continue;
            }

            if deja_analyse {
                // Une même image avec le même alt n'est analysée qu'une fois
                // par audit (spec §5). La preuve garde la clé de réutilisation.
                let mesures = Mesures {
                    reutilisation_cache: Some("déjà analysée dans cet audit".to_owned()),
                    ..Mesures::default()
                };
                preuves.push(preuve(
                    mesures,
                    &["1.3.1"],
                    Portee::Estimee,
                    None,
                    &empreinte,
                    None,
                )?);
                continue;
            }

            let Some(image) = image else {
                preuves.push(preuve_manquante("ocr", "image indisponible", &empreinte)?);
                preuves.push(preuve_manquante("clip", "image indisponible", &empreinte)?);
                continue;
            };
            let image = reduire(image);

            match &ocr {
                Ok(backend) => {
                    preuves.push(preuve_ocr(backend.as_ref(), &image, noeud, &empreinte)?)
                }
                Err(raison) => preuves.push(preuve_manquante("ocr", raison, &empreinte)?),
            }
            match &clip {
                Ok(backend) => preuves.push(preuve_clip(
                    backend.as_ref(),
                    &image,
                    noeud,
                    &alts_page,
                    &empreinte,
                )?),
                Err(raison) => preuves.push(preuve_manquante("clip", raison, &empreinte)?),
            }
        }
        Ok(preuves)
    }
}

type BackendResult<B> = std::result::Result<Arc<B>, String>;

fn initialiser_ocr(plugin: &ImagesPlugin) -> BackendResult<dyn ocr::OcrBackend> {
    if let Some(backend) = &plugin.ocr {
        return Ok(backend.clone());
    }
    ocr::OcrsBackend::from_models_dir(&plugin.models_dir)
        .map(|backend| Arc::new(backend) as Arc<dyn ocr::OcrBackend>)
        .map_err(|error| error.to_string())
}

fn initialiser_clip(plugin: &ImagesPlugin) -> BackendResult<dyn clip::ClipBackend> {
    if let Some(backend) = &plugin.clip {
        return Ok(backend.clone());
    }
    clip::ClipEmbeddings::from_models_dir(&plugin.models_dir)
        .map(|backend| Arc::new(backend) as Arc<dyn clip::ClipBackend>)
        .map_err(|error| error.to_string())
}

fn alt_de(noeud: &ImageNode) -> &str {
    noeud
        .alt
        .as_deref()
        .unwrap_or_else(|| noeud.nom_accessible.as_deref().unwrap_or(""))
}

/// Décode une ressource image. Un SVG inline ou `.svg` est rasterisé avec
/// `resvg` avant OCR / CLIP (spec §6).
async fn charger_image(page: &PageCapture<'_>, noeud: &ImageNode) -> Result<Option<DynamicImage>> {
    let Some(src) = noeud.src.as_deref() else {
        return Ok(None);
    };
    if src.trim().is_empty() {
        return Ok(None);
    }
    if src.trim_start().starts_with("<svg") {
        return svg::rasteriser(src.as_bytes(), VIGNETTE_MAX).map(Some);
    }
    let octets = match page.resources.fetch(src).await {
        Ok(octets) => octets,
        Err(error) => {
            tracing::warn!(src, %error, "image indisponible : preuve manquante");
            return Ok(None);
        }
    };
    if octets.windows(4).any(|f| f == b"<svg") {
        return svg::rasteriser(&octets, VIGNETTE_MAX).map(Some);
    }
    match image::load_from_memory(&octets) {
        Ok(image) => Ok(Some(image)),
        Err(error) => {
            tracing::warn!(src, %error, "image illisible : preuve manquante");
            Ok(None)
        }
    }
}

fn reduire(image: DynamicImage) -> DynamicImage {
    let (largeur, hauteur) = (image.width(), image.height());
    let plus_grand = largeur.max(hauteur);
    if plus_grand <= VIGNETTE_MAX {
        return image;
    }
    let echelle = VIGNETTE_MAX as f32 / plus_grand as f32;
    image.resize(
        (largeur as f32 * echelle).round().max(1.0) as u32,
        (hauteur as f32 * echelle).round().max(1.0) as u32,
        image::imageops::FilterType::Lanczos3,
    )
}

fn empreinte_entree(url: &str, noeud: &ImageNode, p_hash: u64) -> String {
    let selecteur = &noeud.selecteur;
    let alt = alt_de(noeud);
    rgaa_analysis::sha256_hex(
        format!("images\0{url}\0{selecteur}\0{alt}\0{p_hash:016x}").as_bytes(),
    )
}

#[allow(clippy::too_many_arguments)]
fn preuve(
    mesures: Mesures,
    tests: &[&str],
    portee: Portee,
    indice: Option<Indice>,
    empreinte: &str,
    version_modele: Option<String>,
) -> Result<Preuve> {
    let mesures = serde_json::to_value(mesures)
        .map_err(|error| analysis_error("images", format!("mesures sérialisables : {error}")))?;
    Ok(Preuve {
        plugin: "images".to_owned(),
        version_plugin: IMAGES_VERSION.to_owned(),
        version_modele,
        tests: tests.iter().map(|test| (*test).to_owned()).collect(),
        sujet: Subject::Page,
        portee,
        mesures,
        indice,
        artefacts: Vec::new(),
        empreinte_entree: empreinte.to_owned(),
    })
}

fn preuve_deterministe(noeud: &ImageNode, occurrences: usize, empreinte: &str) -> Result<Preuve> {
    let regles = regles::appliquer(noeud);
    let indice = regles.first().map(|raison| Indice::Suspect {
        raison: (*raison).to_owned(),
    });
    preuve(
        Mesures {
            type_noeud: noeud.type_noeud.clone(),
            nom_accessible: noeud.nom_accessible.clone(),
            dans_lien: noeud.dans_lien,
            occurrences_site: occurrences,
            regles: regles.into_iter().map(str::to_owned).collect(),
            ..Mesures::default()
        },
        &["1.2.1", "1.3.1", "1.7.1"],
        Portee::Mesuree,
        indice,
        empreinte,
        None,
    )
}

fn preuve_captcha(noeud: &ImageNode, empreinte: &str) -> Result<Preuve> {
    preuve(
        Mesures {
            type_noeud: noeud.type_noeud.clone(),
            captcha: Some(MesureCaptcha {
                ocr: false,
                raison: "CAPTCHA : aucune OCR, aucun appel LLM (1.4)".to_owned(),
            }),
            ..Mesures::default()
        },
        &["1.4.1"],
        Portee::Mesuree,
        None,
        empreinte,
        None,
    )
}

fn preuve_manquante(champ: &str, raison: &str, empreinte: &str) -> Result<Preuve> {
    let message = format!("preuve manquante ({champ}) : {raison}");
    let mesure = if champ == "ocr" {
        Mesures {
            ocr: Some(MesureOcr {
                moteur: "absent".to_owned(),
                preuve_manquante: Some(message.clone()),
                ..MesureOcr::default()
            }),
            ..Mesures::default()
        }
    } else {
        Mesures {
            clip: Some(MesureClip {
                moteur: "absent".to_owned(),
                preuve_manquante: Some(message.clone()),
                ..MesureClip::default()
            }),
            ..Mesures::default()
        }
    };
    preuve(
        mesure,
        if champ == "ocr" {
            &["1.3.1", "1.8.1"]
        } else {
            &["1.2.1", "1.3.1"]
        },
        Portee::Estimee,
        None,
        empreinte,
        None,
    )
}

fn preuve_ocr(
    backend: &dyn ocr::OcrBackend,
    image: &DynamicImage,
    noeud: &ImageNode,
    empreinte: &str,
) -> Result<Preuve> {
    let mesure = backend.analyze(image)?;
    let alt = alt_de(noeud).to_lowercase();
    let mots_ocr: Vec<&str> = mesure
        .texte
        .split_whitespace()
        .filter(|mot| mot.chars().count() > 1)
        .collect();
    let recouvre = mots_ocr.iter().all(|mot| alt.contains(&mot.to_lowercase()));
    let image_texte = mesure.couverture > 0.5 && mesure.texte.split_whitespace().count() > 3;
    let indice = if image_texte && (alt.is_empty() || !recouvre) {
        Some(Indice::Suspect {
            raison: "image texte (1.8) : l'OCR ne figure pas dans l'alt".to_owned(),
        })
    } else if !mots_ocr.is_empty() && !alt.is_empty() && !recouvre {
        Some(Indice::Suspect {
            raison: "texte OCR absent de l'alt (1.3)".to_owned(),
        })
    } else {
        None
    };
    preuve(
        Mesures {
            type_noeud: noeud.type_noeud.clone(),
            ocr: Some(MesureOcr {
                texte: Some(mesure.texte),
                couverture: Some(mesure.couverture),
                confiance: mesure.confiance,
                moteur: mesure.moteur.clone(),
                preuve_manquante: None,
            }),
            ..Mesures::default()
        },
        &["1.3.1", "1.8.1"],
        Portee::Estimee,
        indice,
        empreinte,
        Some(mesure.moteur),
    )
}

fn preuve_clip(
    backend: &dyn clip::ClipBackend,
    image: &DynamicImage,
    noeud: &ImageNode,
    alts_page: &[String],
    empreinte: &str,
) -> Result<Preuve> {
    let alt = alt_de(noeud);
    let melanges: Vec<String> = alts_page
        .iter()
        .filter(|autre| *autre != alt)
        .take(5)
        .cloned()
        .collect();
    let mesure = backend.analyze(image, alt, &melanges)?;
    let sans_rapport = mesure.similarite_alt < mesure.ancre_hasard;
    let decoratif_avec_alt = mesure.classe.contains("décoratif") && !alt.is_empty();
    let indice = if sans_rapport {
        Some(Indice::Suspect {
            raison: "alt sans rapport avec l'image (1.3)".to_owned(),
        })
    } else if decoratif_avec_alt {
        Some(Indice::Suspect {
            raison: "image décorative probable avec un alt non vide (1.2)".to_owned(),
        })
    } else if mesure.classe.contains("texte") && alt.is_empty() {
        Some(Indice::Suspect {
            raison: "capture de texte sans alternative (1.8)".to_owned(),
        })
    } else {
        Some(Indice::Rassurant {
            raison: "alt cohérente avec l'image (1.3)".to_owned(),
        })
    };
    preuve(
        Mesures {
            type_noeud: noeud.type_noeud.clone(),
            clip: Some(MesureClip {
                classe: Some(mesure.classe),
                score: Some(mesure.score),
                similarite_alt: Some(mesure.similarite_alt),
                ancre_hasard: Some(mesure.ancre_hasard),
                moteur: backend.id().to_owned(),
                preuve_manquante: None,
            }),
            ..Mesures::default()
        },
        &["1.2.1", "1.3.1"],
        Portee::Estimee,
        indice,
        empreinte,
        Some(backend.id().to_owned()),
    )
}
