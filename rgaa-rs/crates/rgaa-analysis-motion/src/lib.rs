//! P-05 — plugin d'analyse `motion` : flashs (13.7), mouvement (13.8) et
//! limites de temps (13.1).
//!
//! Le plugin rend des **preuves**, jamais de statut (R-P1). Le rapport porte
//! l'avertissement « analyse indicative » : la surface réelle dépend de la
//! distance de vision et c'est le relecteur qui conclut. L'observation
//! dynamique de la page (screencast CDP, `prefers-reduced-motion`) fait l'objet
//! d'une seconde PR.

pub mod flash;
pub mod frames;
pub mod iris;
pub mod limites;
pub mod luminance;
pub mod mesures;
pub mod mouvement;

pub use rgaa_analysis::{analysis_error, missing_proof, AnalysisPlugin, Result, RgaaError};

use async_trait::async_trait;
use flash::detect_flashs;
use frames::{decode_gif, decode_video, Frame};
use indexmap::IndexMap;
use iris::{IrisOracle, OracleFlash};
use limites::{detecte_limites, LimiteTemps};
use mesures::Mesures;
use mouvement::{analyse_mouvement, Mouvement};
use rgaa_analysis::{
    sha256_hex, Indice, PageCapture, PluginKind, PluginOptions, Portee, Preuve, Subject,
};
use serde_json::{json, Value};

/// Identifiant stable du plugin.
pub const PLUGIN_ID: &str = "motion";
/// Version reportée dans chaque preuve.
pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Tests RGAA servis (clés du catalogue `rgaa-core`).
pub const TESTS: &[&str] = &["13.1", "13.7", "13.8"];
/// Avertissement obligatoire dans le rapport (spec §8.3).
pub const AVERTISSEMENT: &str =
    "analyse indicative, la surface réelle dépend de la distance de vision";
/// Note sur la luminance calculée localement, faute de module `color` dans P-00.
pub const NOTE_LUMINANCE: &str =
    "luminance relative sRGB calculée localement (formule WCAG 2.1), rgaa-analysis n'expose pas de module color";
/// Durée d'observation par défaut (spec §8.2).
pub const DUREE_OBSERVATION_DEFAUT_S: u64 = 10;
/// Taille de référence et géométrie de la fenêtre de 10° (spec §3).
pub const LARGEUR_REFERENCE: u32 = 1024;
pub const HAUTEUR_REFERENCE: u32 = 768;
pub const FENETRE_LARGEUR: u32 = 341;
pub const FENETRE_HAUTEUR: u32 = 256;
pub const PAS_FENETRE: u32 = 64;
pub const SURFACE_MIN: f64 = 0.25;
pub const DELTA_LUMINANCE: f32 = 0.10;
pub const LUMINANCE_SOMBRE_MAX: f32 = 0.80;
pub const DELTA_ROUGE: f32 = 20.0;

/// Construit le plugin pour le registre `rgaa-plugins`.
#[must_use]
pub fn motion_plugin() -> Box<dyn AnalysisPlugin> {
    Box::new(MotionPlugin)
}

/// Plugin P-05, stateless (motif des unit structs du projet).
pub struct MotionPlugin;

#[async_trait]
impl AnalysisPlugin for MotionPlugin {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn version(&self) -> &'static str {
        PLUGIN_VERSION
    }

    fn tests(&self) -> &'static [&'static str] {
        TESTS
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Natif
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let mut preuves = Vec::new();
        for limite in detecte_limites(&page.dom_snapshot.json)? {
            preuves.push(preuve_limite(page.url, &limite));
        }

        let duree = options
            .duree_s
            .unwrap_or(DUREE_OBSERVATION_DEFAUT_S)
            .min(u64::from(u32::MAX)) as u32;
        let oracle = IrisOracle::depuis_environnement();
        for (url, type_source) in sources_media(&page.dom_snapshot.json)? {
            let octets = page.resources.fetch(&url).await.map_err(|erreur| {
                missing_proof(
                    PLUGIN_ID,
                    &format!("ressource {url} inaccessible : {erreur}"),
                )
            })?;
            let empreinte = sha256_hex(&octets);
            let frames = match type_source {
                TypeSource::Gif => decode_gif(&octets, duree)?,
                TypeSource::Video => decode_video_avec_donnees(&url, &octets, &empreinte, duree)?,
            };
            let frames: Vec<Frame> = frames
                .into_iter()
                .filter(|frame| frame.t_s < f64::from(duree))
                .collect();
            if frames.is_empty() {
                continue;
            }
            let duree_observee =
                frames.last().map_or(0.0, |frame| frame.t_s - frames[0].t_s) + 1.0 / 30.0;
            let (general, rouge) = detect_flashs(&frames);
            let mouvement = analyse_mouvement(&frames, page.ax_tree, &url);
            let second_avis = if oracle.disponible() {
                oracle.compte_flashs(&frames)?
            } else {
                None
            };
            let mesures = Mesures {
                source: format!("{}:{url}", type_source.prefixe()),
                fps_analyse: frames.len() as f64 / duree_observee.max(f64::EPSILON),
                avertissement: AVERTISSEMENT.to_owned(),
                note_luminance: NOTE_LUMINANCE.to_owned(),
                flash_general: general,
                flash_rouge: rouge,
                mouvement: mouvement.clone(),
                limites_temps: Vec::new(),
                oracle_iris: second_avis,
            };
            let indice = indice(&general, &rouge, &mouvement);
            preuves.push(nouvelle_preuve(
                &["13.7", "13.8"],
                Subject::Media { url: url.clone() },
                Portee::Estimee,
                to_value(&mesures)?,
                indice,
                &empreinte,
            ));
        }
        Ok(preuves)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeSource {
    Gif,
    Video,
}

impl TypeSource {
    fn prefixe(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Video => "video",
        }
    }

    fn depuis_url(url: &str) -> Option<Self> {
        let chemin = url.split(['?', '#']).next().unwrap_or(url).to_lowercase();
        if chemin.ends_with(".gif") {
            Some(Self::Gif)
        } else if [".mp4", ".webm", ".ogv", ".mov", ".m4v", ".mkv"]
            .iter()
            .any(|extension| chemin.ends_with(extension))
        {
            Some(Self::Video)
        } else {
            None
        }
    }
}

/// Inventaire ordonné des médias animés du document, sans doublon.
fn sources_media(dom_json: &str) -> Result<IndexMap<String, TypeSource>> {
    let document: Value = serde_json::from_str(dom_json).map_err(|erreur| {
        analysis_error(PLUGIN_ID, format!("DOM snapshot illisible : {erreur}"))
    })?;
    let mut sources = IndexMap::new();
    let Some(nodes) = document.get("nodes").and_then(Value::as_array) else {
        return Ok(sources);
    };
    for noeud in nodes {
        let Some(attrs) = noeud.get("attrs").and_then(Value::as_object) else {
            continue;
        };
        for cle in ["src", "data-src"] {
            if let Some(url) = attrs.get(cle).and_then(Value::as_str) {
                if let Some(type_source) = TypeSource::depuis_url(url) {
                    sources.entry(url.to_owned()).or_insert(type_source);
                }
            }
        }
    }
    Ok(sources)
}

/// Écrit les octets dans un fichier temporaire : `ffmpeg` lit un chemin, jamais
/// un flux réseau (R-P3). Le fichier est supprimé même en cas d'erreur.
fn decode_video_avec_donnees(
    url: &str,
    octets: &[u8],
    empreinte: &str,
    duree_s: u32,
) -> Result<Vec<Frame>> {
    let extension = TypeSource::depuis_url(url).map_or("mp4", TypeSource::prefixe);
    let chemin = std::env::temp_dir().join(format!("rgaa-motion-{empreinte}.{extension}"));
    std::fs::write(&chemin, octets).map_err(|erreur| {
        missing_proof(PLUGIN_ID, &format!("fichier vidéo temporaire : {erreur}"))
    })?;
    let resultat = decode_video(chemin.to_string_lossy().as_ref(), duree_s);
    let _ = std::fs::remove_file(&chemin);
    resultat
}

fn indice(
    general: &flash::FlashResult,
    rouge: &flash::FlashResult,
    mouvement: &[Mouvement],
) -> Option<Indice> {
    let flash_au_dessus = [general, rouge]
        .iter()
        .any(|resultat| resultat.max_par_s > 3 && resultat.surface > SURFACE_MIN);
    let mouvement_sans_controle = mouvement
        .iter()
        .any(|item| !item.controle_pause && !item.reduced_motion_respecte);
    if flash_au_dessus || mouvement_sans_controle {
        let raison = if mouvement_sans_controle {
            "mouvement de plus de 5 s sans contrôle pause ni respect de reduced-motion"
        } else {
            "plus de 3 flashs par seconde sur plus de 25 % d'une fenêtre de 341×256 px"
        };
        Some(Indice::Suspect {
            raison: raison.to_owned(),
        })
    } else if mouvement
        .iter()
        .any(|item| item.controle_pause || item.reduced_motion_respecte)
    {
        Some(Indice::Rassurant {
            raison: "un contrôle ou reduced-motion est présent ; ne remplace pas le contrôle exigé"
                .to_owned(),
        })
    } else {
        None
    }
}

fn preuve_limite(url: &str, limite: &LimiteTemps) -> Preuve {
    nouvelle_preuve(
        &["13.1"],
        Subject::Page,
        Portee::Mesuree,
        json!({
            "source": url,
            "avertissement": AVERTISSEMENT,
            "limites_temps": [limite],
        }),
        None,
        &sha256_hex(format!("limite\0{url}\0{:?}", limite).as_bytes()),
    )
}

fn nouvelle_preuve(
    tests: &[&str],
    sujet: Subject,
    portee: Portee,
    mesures: Value,
    indice: Option<Indice>,
    empreinte: &str,
) -> Preuve {
    Preuve {
        plugin: PLUGIN_ID.to_owned(),
        version_plugin: PLUGIN_VERSION.to_owned(),
        version_modele: None,
        tests: tests.iter().map(|test| (*test).to_owned()).collect(),
        sujet,
        portee,
        mesures,
        indice,
        artefacts: Vec::new(),
        empreinte_entree: empreinte.to_owned(),
    }
}

fn to_value<T: serde::Serialize>(valeur: &T) -> Result<Value> {
    serde_json::to_value(valeur).map_err(|erreur| {
        analysis_error(PLUGIN_ID, format!("mesures non sérialisables : {erreur}"))
    })
}
