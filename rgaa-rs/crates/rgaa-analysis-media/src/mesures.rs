//! Typed `mesures` payloads for the P-04 plugin (spec §4).
//!
//! These are data only: no status, no verdict (R-P1). They are serialised into
//! `Preuve::mesures` and their schema can be exported with the other analysis
//! schemas in `types/`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Everything measured for one media, except the 4.10 loudness proof.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mesures {
    pub source: String,
    pub duree_s: f64,
    /// Share of the media carrying speech. From the transcription stand-in in
    /// PR 1, from Silero VAD in PR 2.
    pub part_parole: Option<f64>,
    pub langue_detectee: Option<String>,
    pub decodage: DecodageMesure,
    pub transcription_fournie: Option<TranscriptionMesure>,
    pub sous_titres: Option<SousTitresMesure>,
    pub audiodescription: AudiodescriptionMesure,
    pub son_auto: Option<SonAutoMesure>,
    pub moteur_transcription: Option<String>,
    /// Human-readable reason attached to a `Neutre` review hint.
    pub note: Option<String>,
}

/// How the audio was obtained (native decode or ffmpeg sidecar fallback).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DecodageMesure {
    pub codec: String,
    pub frequence_hz: u32,
    pub canaux: u16,
    pub repli_ffmpeg: bool,
    pub duree_max_s: f64,
    pub ignore_pour_duree: bool,
}

/// A time-bounded passage of the media (missing text or uncovered speech).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Passage {
    pub debut_s: f64,
    pub fin_s: f64,
    pub extrait: String,
}

/// Coverage of the provided transcription against what was said (4.2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TranscriptionMesure {
    pub url: String,
    pub couverture: f64,
    pub passages_manquants: Vec<Passage>,
}

/// Textual coverage and synchronisation of a subtitle track (4.3, 4.4).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SousTitresMesure {
    pub piste: String,
    pub couverture: f64,
    pub decalage_median_ms: f64,
    pub decalage_p95_ms: f64,
    pub parole_sans_cue: Vec<Passage>,
}

/// Presence of an audio-description track and coverage of long silences
/// (4.5, 4.6). Relevance stays human.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AudiodescriptionMesure {
    pub piste_descriptions: bool,
    pub silences_couverts: f64,
}

/// Autoplay sound measurement for 4.10.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SonAutoMesure {
    pub autoplay: bool,
    pub muted: bool,
    pub duree_au_dessus_seuil_s: f64,
    pub controles_accessibles: bool,
}
