//! Sources d'images : GIF animé (natif) et vidéo (sidecar `ffmpeg`).
//!
//! Pour la vidéo, `ffmpeg-sidecar` est lié **sans** téléchargement de binaire :
//! il utilise le `ffmpeg` présent sur la machine (ou `RGAA_FFMPEG_BIN`). Sans
//! binaire, l'analyse renvoie `preuve manquante` et l'audit continue.

use crate::Result;
use ffmpeg_sidecar::command::FfmpegCommand;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use rgaa_analysis::{analysis_error, missing_proof};
use schemars::JsonSchema;
use serde::Serialize;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::PLUGIN_ID;

/// Frames RGBA horodatées, dans l'ordre d'affichage.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Frame {
    pub largeur: u32,
    pub hauteur: u32,
    pub rgba: Vec<u8>,
    pub t_s: f64,
}

impl Frame {
    /// Fabrique une frame uniforme, utile aux fixtures et aux tests.
    #[must_use]
    pub fn de_couleur(largeur: u32, hauteur: u32, couleur: [u8; 4], t_s: f64) -> Self {
        Self {
            largeur,
            hauteur,
            rgba: couleur.repeat((largeur * hauteur) as usize),
            t_s,
        }
    }
}

/// Décode un GIF animé et conserve le délai de chaque frame (centièmes de
/// seconde dans le format, convertis en secondes ici). Le décodage s'arrête à
/// la limite d'observation afin de ne pas conserver toutes les frames d'un
/// GIF long.
///
/// # Errors
/// Renvoie `RgaaError::Analysis` si le GIF est illisible.
pub fn decode_gif(octets: &[u8], duree_s: u32) -> Result<Vec<Frame>> {
    let decodeur = GifDecoder::new(Cursor::new(octets))
        .map_err(|erreur| analysis_error(PLUGIN_ID, format!("GIF illisible : {erreur}")))?;
    let mut frames = Vec::new();
    let mut temps_ms = 0u64;
    let limite_ms = u64::from(duree_s) * 1000;
    for resultat in decodeur.into_frames() {
        if temps_ms >= limite_ms {
            break;
        }
        let image_frame = resultat.map_err(|erreur| {
            analysis_error(PLUGIN_ID, format!("frame GIF illisible : {erreur}"))
        })?;
        let (numerateur, denominateur) = image_frame.delay().numer_denom_ms();
        let delai_ms = (numerateur / denominateur.max(1)).max(10);
        let image = image_frame.into_buffer();
        let (largeur, hauteur) = image.dimensions();
        frames.push(Frame {
            largeur,
            hauteur,
            rgba: image.into_raw(),
            t_s: temps_ms as f64 / 1000.0,
        });
        temps_ms += u64::from(delai_ms);
    }
    Ok(frames)
}

/// Chemin du binaire ffmpeg : variable d'environnement, puis `ffmpeg` sur le
/// `PATH`. Le binaire n'est jamais téléchargé pendant un audit (R-P3).
#[must_use]
pub fn chemin_ffmpeg() -> PathBuf {
    std::env::var_os("RGAA_FFMPEG_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

/// Extrait les images d'une vidéo avec `ffmpeg -vf fps=30,scale=1024:-1`.
///
/// La limite d'observation est transmise à ffmpeg pour que le processus
/// s'arrête sans décoder toute la vidéo.
///
/// Le chemin du binaire est injectable pour tester le cas « ffmpeg absent »
/// sans modifier le `PATH` du processus.
///
/// # Errors
/// Renvoie « preuve manquante » si le binaire manque ou si ffmpeg échoue.
pub fn decode_video_avec(ffmpeg: &Path, source: &str, duree_s: u32) -> Result<Vec<Frame>> {
    let mut commande = FfmpegCommand::new_with_path(ffmpeg);
    commande
        .input(source)
        .args(["-vf", "fps=30,scale=1024:-1", "-t", &duree_s.to_string()])
        .rawvideo();
    let mut enfant = commande.spawn().map_err(|erreur| {
        missing_proof(
            PLUGIN_ID,
            &format!("ffmpeg ({}) indisponible : {erreur}", ffmpeg.display()),
        )
    })?;
    let iterateur = enfant
        .iter()
        .map_err(|erreur| missing_proof(PLUGIN_ID, &format!("flux ffmpeg illisible : {erreur}")))?;
    let mut frames = Vec::new();
    for image in iterateur.filter_frames() {
        frames.push(Frame {
            largeur: image.width,
            hauteur: image.height,
            rgba: image.data,
            t_s: frames.len() as f64 / 30.0,
        });
    }
    let sortie = enfant
        .wait()
        .map_err(|erreur| missing_proof(PLUGIN_ID, &format!("ffmpeg interrompu : {erreur}")))?;
    if !sortie.success() {
        return Err(missing_proof(
            PLUGIN_ID,
            &format!("ffmpeg a échoué sur {source} (code {sortie})"),
        ));
    }
    Ok(frames)
}

/// Extrait les images d'une vidéo avec le binaire configuré sur la machine.
///
/// # Errors
/// Renvoie « preuve manquante » si ffmpeg est absent ou échoue.
pub fn decode_video(source: &str, duree_s: u32) -> Result<Vec<Frame>> {
    let chemin = chemin_ffmpeg();
    decode_video_avec(&chemin, source, duree_s)
}
