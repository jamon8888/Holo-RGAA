//! The P-02 native plugin: proofs, never a status (R-P1).

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{ImageEncoder, RgbaImage};
use rgaa_analysis::capture::{PageCapture, RgbaImage as CaptureImage};
use rgaa_analysis::color::{delta_e_improved_2000, Color};
use rgaa_analysis::error::{analysis_error, Result};
use rgaa_analysis::types::{Artefact, Indice, PluginOptions, Portee, Preuve, Rect, Subject};
use rgaa_analysis::{sha256_hex, AnalysisPlugin};
use schemars::JsonSchema;
use serde::Serialize;

use crate::candidates::{detect_candidates, CandidateKind, DomCandidate};
use crate::matrices::{simulate_rgb, CvdKind, ALL_KINDS};
use crate::simulate::simulate_image;

/// RGAA 4.1.2 tests served by this plugin, from the catalogue's `3.1.x` list.
pub const TESTS: &[&str] = &["3.1.1", "3.1.2", "3.1.3", "3.1.4"];

/// Per-call settings. There is no local model to download (R-P3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorVisionOptions {
    /// Machado severity, 0.0–1.0 (1.0 = dichromacy, the published matrices).
    pub severite: f64,
    /// ΔE2000 below which two colours are "indiscernable" (default 5).
    pub seuil_delta_e: f64,
    /// dHamming distance at which two chart series are confounded (default 4).
    pub hamming_max: u32,
    /// Whether a model opinion is requested for collapsed subjects. The plugin
    /// never calls the network itself: the orchestrator attaches `avis_modele`.
    pub llm: bool,
}

impl ColorVisionOptions {
    /// Published defaults from P-02 §3–4.
    pub const fn new() -> Self {
        Self {
            severite: 1.0,
            seuil_delta_e: 5.0,
            hamming_max: 4,
            llm: true,
        }
    }
}

impl Default for ColorVisionOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// One entry of `mesures.simulations` (P-02 §5).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct SimulationMesure {
    /// One of `protanopia`, `deuteranopia`, `tritanopia`, `achromatopsia`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub delta_e: f64,
    pub indiscernable: bool,
}

/// Optional model opinion. It is never a verdict (R2/R3).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct AvisModele {
    pub autre_moyen: String,
    pub explication: String,
}

/// Plugin-specific `mesures` schema, exported in `types/analysis-schemas.json`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Mesures {
    pub candidat: &'static str,
    pub simulations: Vec<SimulationMesure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratio_lien_texte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_confondues: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// The P-02 analysis plugin. Stateless apart from its configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct ColorVisionPlugin {
    options: ColorVisionOptions,
}

impl ColorVisionPlugin {
    /// Builds a plugin with explicit options.
    pub const fn new(options: ColorVisionOptions) -> Self {
        Self { options }
    }

    /// The settings in use, for diagnostics.
    pub const fn options(&self) -> &ColorVisionOptions {
        &self.options
    }
}

/// Ready-to-use default instance for the P-06 `rgaa-plugins` aggregator.
///
/// Registration is intentionally not done here: `rgaa-analysis` must not
/// depend on plugin crates (Cargo cycle), and the aggregator owns the single
/// `default_registry()` entry point.
pub static COLORVISION_PLUGIN: ColorVisionPlugin =
    ColorVisionPlugin::new(ColorVisionOptions::new());

#[async_trait]
impl AnalysisPlugin for ColorVisionPlugin {
    fn id(&self) -> &'static str {
        "colorvision"
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn tests(&self) -> &'static [&'static str] {
        TESTS
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        _options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let candidates = detect_candidates(&page.dom_snapshot.json)?;
        let mut proofs = Vec::with_capacity(candidates.len() * 2);
        for candidate in candidates {
            let fingerprint = sha256_hex(
                format!(
                    "{}\0{}\0{}",
                    page.url, page.dom_snapshot.json, candidate.xpath
                )
                .as_bytes(),
            );
            proofs.push(measured_proof(&candidate, &fingerprint));
            proofs.push(self.estimated_proof(page, &candidate, &fingerprint)?);
        }
        Ok(proofs)
    }
}

fn measured_proof(candidate: &DomCandidate, fingerprint: &str) -> Preuve {
    let indice = if candidate.error_text.is_empty() {
        None
    } else {
        Some(Indice::Rassurant {
            raison: "texte d'erreur présent".to_owned(),
        })
    };
    let mut mesures = serde_json::json!({
        "candidat": candidate.kind.as_str(),
        "element": { "selecteur": candidate.selecteur, "xpath": candidate.xpath },
    });
    if candidate.kind == CandidateKind::LienSansSoulignement {
        mesures["ratio_lien_texte"] = serde_json::json!(candidate.ratio);
    }
    if !candidate.error_text.is_empty() {
        mesures["message_erreur"] = serde_json::json!(candidate.error_text);
    }
    Preuve {
        plugin: "colorvision".to_owned(),
        version_plugin: env!("CARGO_PKG_VERSION").to_owned(),
        version_modele: None,
        tests: vec![candidate.test.to_owned()],
        sujet: Subject::Element {
            selecteur: candidate.selecteur.clone(),
            xpath: candidate.xpath.clone(),
            bbox: candidate.bbox,
        },
        portee: Portee::Mesuree,
        mesures,
        indice,
        artefacts: Vec::new(),
        empreinte_entree: fingerprint.to_owned(),
    }
}

impl ColorVisionPlugin {
    fn estimated_proof(
        &self,
        page: &PageCapture<'_>,
        candidate: &DomCandidate,
        fingerprint: &str,
    ) -> Result<Preuve> {
        let options = &self.options;
        let mut collapsed: Vec<CvdKind> = Vec::new();
        let mut simulations = Vec::with_capacity(ALL_KINDS.len());
        for kind in ALL_KINDS {
            let simulated_foreground = simulate_rgb(candidate.foreground, kind, options.severite);
            let simulated_neighbor = simulate_rgb(candidate.neighbor, kind, options.severite);
            let delta_e = delta_e_improved_2000(
                as_colour(simulated_foreground),
                as_colour(simulated_neighbor),
            );
            let indiscernable = delta_e < options.seuil_delta_e;
            if indiscernable {
                collapsed.push(kind);
            }
            simulations.push(SimulationMesure {
                kind: kind.as_str(),
                delta_e,
                indiscernable,
            });
        }
        let series_confondues = confounded_series(candidate, options.seuil_delta_e);
        let artefacts = vignettes(page, candidate, &collapsed, options.severite)?;
        let indice = collapsed.first().map(|kind| Indice::Suspect {
            raison: format!("information indiscernable sous {}", kind.label()),
        });
        let mut mesures = serde_json::to_value(Mesures {
            candidat: candidate.kind.as_str(),
            simulations,
            ratio_lien_texte: (candidate.kind == CandidateKind::LienSansSoulignement)
                .then_some(candidate.ratio),
            series_confondues,
            avis_modele: None,
        })
        .map_err(|error| analysis_error("colorvision", format!("mesures invalides : {error}")))?;
        if !options.llm {
            // The option is recorded but never turns into a verdict: no plugin
            // is allowed to call the network (R-P3).
            mesures["llm_demande"] = serde_json::json!(false);
        }
        Ok(Preuve {
            plugin: "colorvision".to_owned(),
            version_plugin: env!("CARGO_PKG_VERSION").to_owned(),
            version_modele: None,
            tests: vec![candidate.test.to_owned()],
            sujet: Subject::Element {
                selecteur: candidate.selecteur.clone(),
                xpath: candidate.xpath.clone(),
                bbox: candidate.bbox,
            },
            portee: Portee::Estimee,
            mesures,
            indice,
            artefacts,
            empreinte_entree: fingerprint.to_owned(),
        })
    }
}

fn confounded_series(candidate: &DomCandidate, threshold: f64) -> Option<Vec<String>> {
    if candidate.series.len() < 2 {
        return None;
    }
    let mut found = Vec::new();
    for (index, series) in candidate.series.iter().enumerate().skip(1) {
        for (previous_index, previous) in candidate.series[..index].iter().enumerate() {
            if delta_e_improved_2000(as_colour(series.color), as_colour(previous.color)) < threshold
            {
                found.push(format!(
                    "{previous_index}<->{index}: {} / {}",
                    as_colour(previous.color).to_hex(),
                    as_colour(series.color).to_hex()
                ));
            }
        }
    }
    Some(found)
}

/// Bridges the plugin's `[u8; 3]` colours to the canonical `Color` type.
fn as_colour(rgb: crate::Rgb8) -> Color {
    Color::rgb(rgb[0], rgb[1], rgb[2])
}

/// Crops the original vignette and at most two simulated ones (P-02 §4:
/// three PNG per subject maximum). The PNGs are declared as bundle artefacts;
/// writing the bundle is the storage layer's job (P-00 §5).
fn vignettes(
    page: &PageCapture<'_>,
    candidate: &DomCandidate,
    collapsed: &[CvdKind],
    severity: f64,
) -> Result<Vec<Artefact>> {
    let Some(screenshot) = page.screenshot else {
        return Ok(Vec::new());
    };
    let original = crop(screenshot, &candidate.bbox, page.dpr);
    let mut images = vec![("original", original.clone())];
    for kind in collapsed.iter().take(2) {
        let simulated = simulate_image(&original, *kind, severity);
        images.push((kind.as_str(), simulated));
    }
    let mut artefacts = Vec::with_capacity(images.len());
    for (name, image) in images {
        let png = encode_png(&image)?;
        let hash = sha256_hex(&png);
        artefacts.push(Artefact {
            nom: format!("{}-{}.png", candidate.kind.as_str(), name),
            media_type: "image/png".to_owned(),
            chemin_bundle: format!("analysis/colorvision/{hash}.png"),
            sha256: hash,
            contenu_base64: Some(STANDARD.encode(png)),
        });
    }
    Ok(artefacts)
}

fn crop(screenshot: &CaptureImage, bbox: &Rect, dpr: f32) -> RgbaImage {
    let scale = if dpr.is_finite() && dpr > 0.0 {
        dpr
    } else {
        1.0
    };
    let x = (bbox.x * f64::from(scale)).round().max(0.0) as u32;
    let y = (bbox.y * f64::from(scale)).round().max(0.0) as u32;
    let width = if bbox.width <= 0.0 {
        screenshot.width
    } else {
        (bbox.width * f64::from(scale)).round().max(1.0) as u32
    };
    let height = if bbox.height <= 0.0 {
        screenshot.height
    } else {
        (bbox.height * f64::from(scale)).round().max(1.0) as u32
    };
    let width = width.min(screenshot.width).clamp(1, 512);
    let height = height.min(screenshot.height).clamp(1, 512);
    let x = x.min(screenshot.width.saturating_sub(width));
    let y = y.min(screenshot.height.saturating_sub(height));
    RgbaImage::from_fn(width, height, |px, py| {
        let source_x = x.saturating_add(px) as usize;
        let source_y = y.saturating_add(py) as usize;
        let index = (source_y * screenshot.width as usize + source_x) * 4;
        let channels: [u8; 4] = screenshot
            .pixels
            .get(index..index + 4)
            .and_then(|slice| slice.try_into().ok())
            .unwrap_or([0, 0, 0, 255]);
        image::Rgba(channels)
    })
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(image.as_raw().len());
    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| analysis_error("colorvision", format!("PNG invalide : {error}")))?;
    Ok(bytes)
}

/// Number of images the orchestrator should forward to the model: only the
/// artefacts of estimated proofs for subjects where information collapsed.
pub fn images_for_model(proofs: &[Preuve]) -> usize {
    proofs
        .iter()
        .filter(|proof| {
            proof
                .mesures
                .get("simulations")
                .and_then(|value| value.as_array())
                .is_some_and(|sims| sims.iter().any(|sim| sim["indiscernable"] == true))
        })
        .flat_map(|proof| proof.artefacts.iter())
        .filter(|artefact| artefact.media_type == "image/png")
        .count()
}
