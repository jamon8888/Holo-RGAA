//! P-04 — analysis plugin for temporal media (audio and video).
//!
//! The plugin produces *evidence* only (R-P1): a DOM inventory of media and
//! tracks, decoded/resampled audio measurements, text coverage and subtitle
//! synchronisation offsets, and EBU R128 loudness for 4.10. It never renders a
//! status and never sends media or transcriptions to the LLM (R-P3).
//!
//! PR 1 needs no model: the "what was said" side is a hand-written timed
//! transcription supplied through `PluginOptions::transcription_reference_url`.
//! PR 2 replaces it with sherpa-onnx VAD + Whisper and declares its models via
//! `AnalysisPlugin::models`, downloaded only by `rgaa models pull`.

pub mod align;
pub mod audio;
pub mod inventory;
pub mod mesures;
pub mod subtitles;

use async_trait::async_trait;
use inventory::{MediaItem, ThirdParty, TrackItem};
use mesures::{
    AudiodescriptionMesure, DecodageMesure, Mesures, SonAutoMesure, SousTitresMesure,
    TranscriptionMesure,
};
use rgaa_analysis::{
    analysis_error, sha256_hex, AnalysisPlugin, Indice, PageCapture, PluginKind, PluginOptions,
    Portee, Preuve, Result, Subject,
};
use serde_json::Value;

pub use inventory::{
    Inventory as DomInventory, MediaItem as DomMediaItem, ThirdParty as DomThirdParty,
};
pub use mesures::Mesures as MediaMesures;

/// Stable identifier of the plugin.
pub const PLUGIN_ID: &str = "media";
/// Version reported in every proof.
pub const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
/// RGAA criteria served by this plugin (catalogue keys of `rgaa-core`).
pub const TESTS: &[&str] = &["4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.9", "4.10"];
/// Default `opts.duree_max_s`: media longer than 60 minutes are not analysed.
pub const DUREE_MAX_PAR_DEFAUT_S: u64 = 3600;

const FENETTE_MANQUANTE_S: f64 = 15.0;
const PAROLE_SANS_CUE_S: f64 = 3.0;
const SEUIL_PAROLE: f64 = 0.02;
const SEUIL_SON_AUTO_S: f64 = 3.0;

/// Builds the plugin for the `rgaa-plugins` registry.
#[must_use]
pub fn media_plugin() -> Box<dyn AnalysisPlugin> {
    Box::new(MediaPlugin)
}

/// P-04 native plugin. Stateless, in keeping with the P-00 unit-struct pattern.
pub struct MediaPlugin;

#[async_trait]
impl AnalysisPlugin for MediaPlugin {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn version(&self) -> &'static str {
        PLUGIN_VERSION
    }

    fn tests(&self) -> &'static [&'static str] {
        TESTS
    }

    // PR 1 declares no model: nothing is downloaded during an audit (R-P3).
    // PR 2 declares Silero VAD and Whisper here via ModelSpec.

    fn kind(&self) -> PluginKind {
        PluginKind::Natif
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let inventaire = inventory::parse_dom(&page.dom_snapshot.json, page.ax_tree)?;
        let duree_max = f64::from(
            options
                .duree_max_s
                .unwrap_or(DUREE_MAX_PAR_DEFAUT_S)
                .min(u64::from(u32::MAX)) as u32,
        );
        let mut preuves = Vec::new();

        for tiers in &inventaire.third_party {
            preuves.push(preuve_tiers(tiers));
        }
        for media in &inventaire.media {
            preuves.extend(analyse_media(media, page, options, duree_max).await?);
        }
        Ok(preuves)
    }
}

fn preuve_tiers(tiers: &ThirdParty) -> Preuve {
    Preuve {
        plugin: PLUGIN_ID.to_owned(),
        version_plugin: PLUGIN_VERSION.to_owned(),
        version_modele: None,
        tests: test_ids(),
        sujet: Subject::Media {
            url: tiers.url.clone(),
        },
        portee: Portee::Mesuree,
        mesures: serde_json::json!({
            "source": tiers.url,
            "media_tiers": true,
            "plateforme": tiers.plateforme,
            "raison": "média tiers, analyse non effectuée",
        }),
        indice: Some(Indice::Neutre),
        artefacts: Vec::new(),
        empreinte_entree: sha256_hex(tiers.url.as_bytes()),
    }
}

async fn analyse_media(
    media: &MediaItem,
    page: &PageCapture<'_>,
    options: &PluginOptions,
    duree_max: f64,
) -> Result<Vec<Preuve>> {
    let url = media
        .src
        .as_deref()
        .ok_or_else(|| analysis_error(PLUGIN_ID, "élément média sans attribut src"))?;
    let bytes = page.resources.fetch(url).await.map_err(|error| {
        analysis_error(
            PLUGIN_ID,
            format!("preuve manquante (ressource {url}) : {error}"),
        )
    })?;
    let empreinte = sha256_hex(&bytes);
    let decoded = audio::decode(&bytes, extension_from_url(url))?;
    let duree = decoded.duree_s();
    let mut decodage = DecodageMesure {
        codec: decoded.codec.clone(),
        frequence_hz: decoded.sample_rate,
        canaux: decoded.channels,
        repli_ffmpeg: decoded.via_ffmpeg,
        duree_max_s: duree_max,
        ignore_pour_duree: false,
    };

    if duree > duree_max {
        decodage.ignore_pour_duree = true;
        let mesures = Mesures {
            source: url.to_owned(),
            duree_s: duree,
            decodage,
            note: Some("non analysé : durée".to_owned()),
            ..Mesures::default()
        };
        return Ok(vec![new_preuve(
            url,
            &TESTS[..TESTS.len() - 1],
            to_value(&mesures)?,
            Some(Indice::Neutre),
            &empreinte,
        )]);
    }

    let mono = audio::to_mono_16k(&decoded)?;
    let (cues, piste) = read_subtitles(media, page).await?;
    let reference = match options.transcription_reference_url.as_deref() {
        Some(url) => read_text_segments(url, page).await?,
        None => Vec::new(),
    };
    let fournie = match media.transcript_url.as_deref() {
        Some(url) => read_transcription(url, duree, page).await?,
        None => Vec::new(),
    };

    let reference_words: Vec<String> = reference
        .iter()
        .flat_map(|segment| align::normalize(&segment.text))
        .collect();
    let fournie_words: Vec<String> = fournie
        .iter()
        .flat_map(|segment| align::normalize(&segment.text))
        .collect();
    let cue_words: Vec<String> = cues
        .iter()
        .flat_map(|cue| align::normalize(&cue.text))
        .collect();
    let part_parole = align::speech_ratio(&reference, duree);
    let (median_ms, p95_ms) = align::sync_offsets(&cues, &reference).unwrap_or((0.0, 0.0));

    let mut mesures = Mesures {
        source: url.to_owned(),
        duree_s: duree,
        part_parole,
        langue_detectee: None,
        decodage,
        transcription_fournie: Some(TranscriptionMesure {
            url: media.transcript_url.clone().unwrap_or_default(),
            couverture: align::coverage(&reference_words, &fournie_words),
            passages_manquants: align::missing_passages(&reference, &fournie, FENETTE_MANQUANTE_S),
        }),
        sous_titres: Some(SousTitresMesure {
            piste,
            couverture: align::coverage(&reference_words, &cue_words),
            decalage_median_ms: median_ms,
            decalage_p95_ms: p95_ms,
            parole_sans_cue: align::speech_without_cue(&reference, &cues, PAROLE_SANS_CUE_S),
        }),
        audiodescription: mesures_audiodescription(media),
        son_auto: None,
        moteur_transcription: None,
        note: None,
    };
    let indice = if part_parole < SEUIL_PAROLE {
        mesures.note = Some(
            "média sans parole détectée ; transcription peut-être non nécessaire (4.1)".to_owned(),
        );
        Some(Indice::Neutre)
    } else {
        None
    };
    let mut preuves = vec![new_preuve(
        url,
        &TESTS[..TESTS.len() - 1],
        to_value(&mesures)?,
        indice,
        &empreinte,
    )];

    let duree_au_dessus = if media.autoplay && !media.muted {
        audio::loudness_above_threshold(&mono, audio::CIBLE_HZ, audio::SEUIL_LUFS, 100)?
    } else {
        0.0
    };
    let son_auto = SonAutoMesure {
        autoplay: media.autoplay,
        muted: media.muted,
        duree_au_dessus_seuil_s: duree_au_dessus,
        controles_accessibles: media.controls,
    };
    let suspect =
        media.autoplay && !media.muted && duree_au_dessus > SEUIL_SON_AUTO_S && !media.controls;
    let indice_son = if suspect {
        Some(Indice::Suspect {
            raison: format!(
                "son automatique au-dessus de -50 LUFS pendant {duree_au_dessus:.1} s sans contrôle accessible"
            ),
        })
    } else {
        None
    };
    preuves.push(new_preuve(
        url,
        &TESTS[TESTS.len() - 1..],
        to_value(&son_auto)?,
        indice_son,
        &empreinte,
    ));
    Ok(preuves)
}

async fn read_subtitles(
    media: &MediaItem,
    page: &PageCapture<'_>,
) -> Result<(Vec<align::TimedText>, String)> {
    let mut cues = Vec::new();
    let mut piste = String::new();
    for track in media
        .tracks
        .iter()
        .filter(|track| track.kind == "captions" || track.kind == "subtitles")
    {
        cues.extend(read_text_segments(&track.src, page).await?);
        if piste.is_empty() {
            piste = track.src.clone();
        }
    }
    Ok((cues, piste))
}

async fn read_text_segments(url: &str, page: &PageCapture<'_>) -> Result<Vec<align::TimedText>> {
    let bytes = page.resources.fetch(url).await.map_err(|error| {
        analysis_error(
            PLUGIN_ID,
            format!("preuve manquante (ressource {url}) : {error}"),
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);
    subtitles::parse_subtitles(&text, Some(url))
}

async fn read_transcription(
    url: &str,
    duree_s: f64,
    page: &PageCapture<'_>,
) -> Result<Vec<align::TimedText>> {
    let bytes = page.resources.fetch(url).await.map_err(|error| {
        analysis_error(
            PLUGIN_ID,
            format!("preuve manquante (ressource {url}) : {error}"),
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);
    match subtitles::parse_subtitles(&text, Some(url)) {
        Ok(segments) if !segments.is_empty() => Ok(segments),
        _ => Ok(vec![align::TimedText {
            start_s: 0.0,
            end_s: duree_s,
            text: text.trim().to_owned(),
        }]),
    }
}

fn mesures_audiodescription(media: &MediaItem) -> AudiodescriptionMesure {
    AudiodescriptionMesure {
        piste_descriptions: media.tracks.iter().any(|track: &TrackItem| {
            track.kind == "descriptions" || track.kind == "descriptions_metadata"
        }),
        // PR 1 has no AD transcript to compare with the silences; PR 2 fills
        // this from the transcribed description track.
        silences_couverts: 0.0,
    }
}

fn new_preuve(
    url: &str,
    tests: &[&str],
    mesures: Value,
    indice: Option<Indice>,
    empreinte: &str,
) -> Preuve {
    Preuve {
        plugin: PLUGIN_ID.to_owned(),
        version_plugin: PLUGIN_VERSION.to_owned(),
        version_modele: None,
        tests: tests.iter().map(|test| (*test).to_owned()).collect(),
        sujet: Subject::Media {
            url: url.to_owned(),
        },
        portee: Portee::Mesuree,
        mesures,
        indice,
        artefacts: Vec::new(),
        empreinte_entree: empreinte.to_owned(),
    }
}

fn to_value<T: serde::Serialize>(value: &T) -> Result<Value> {
    serde_json::to_value(value)
        .map_err(|error| analysis_error(PLUGIN_ID, format!("mesures non sérialisables : {error}")))
}

fn test_ids() -> Vec<String> {
    TESTS.iter().map(|test| (*test).to_owned()).collect()
}

fn extension_from_url(url: &str) -> Option<&str> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit_once('.').map(|(_, extension)| extension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_analysis::{DomSnapshot, ResourceFetcher};
    use std::collections::HashMap;

    const MEDIA_URL: &str = "https://example.test/media/audio.wav";
    const REF_URL: &str = "https://example.test/ref.vtt";
    const SUB_URL: &str = "https://example.test/fr.vtt";
    const TRANS_URL: &str = "https://example.test/transcription.txt";

    struct FakeFetcher {
        files: HashMap<String, Vec<u8>>,
    }

    #[async_trait]
    impl ResourceFetcher for FakeFetcher {
        async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
            self.files
                .get(url)
                .cloned()
                .ok_or_else(|| analysis_error("media", format!("ressource absente : {url}")))
        }
    }

    fn sine(rate: u32, seconds: f64, amplitude: f32) -> Vec<f32> {
        let frames = (f64::from(rate) * seconds) as usize;
        (0..frames)
            .map(|index| {
                amplitude
                    * (2.0 * std::f64::consts::PI * 220.0 * index as f64 / f64::from(rate)).sin()
                        as f32
            })
            .collect()
    }

    fn media_bytes(seconds: f64) -> Vec<u8> {
        audio::wav_bytes(8_000, 1, &sine(8_000, seconds, 0.4))
    }

    fn reference_vtt() -> &'static str {
        "WEBVTT\n\n00:00:01.000 --> 00:00:12.000\nBonjour le monde et bienvenue\n\n\
         00:00:20.000 --> 00:00:40.000\nVoici la deuxième phrase importante\n"
    }

    fn subtitle_vtt(offset_s: f64) -> String {
        format!(
            "WEBVTT\n\n00:00:{:06.3} --> 00:00:{:06.3}\nBonjour le monde et bienvenue\n\n\
             00:00:{:06.3} --> 00:00:{:06.3}\nVoici la deuxième phrase importante\n",
            1.0 + offset_s,
            12.0 + offset_s,
            20.0 + offset_s,
            40.0 + offset_s
        )
    }

    fn dom(video_attrs: &str, with_tracks: bool, with_iframe: bool) -> String {
        let mut nodes = vec![format!(
            "{{\"tag\":\"video\",\"attrs\":{{\"src\":\"{MEDIA_URL}\"{video_attrs}}},\"text\":\"\"}}"
        )];
        if with_tracks {
            nodes.push(format!(
                "{{\"tag\":\"track\",\"attrs\":{{\"kind\":\"captions\",\"src\":\"{SUB_URL}\"}}}}"
            ));
            nodes.push(format!(
                "{{\"tag\":\"a\",\"attrs\":{{\"href\":\"{TRANS_URL}\"}},\"text\":\"Transcription\"}}"
            ));
        }
        if with_iframe {
            nodes.push(
                "{\"tag\":\"iframe\",\"attrs\":{\"src\":\"https://www.youtube.com/embed/x\"}}"
                    .to_owned(),
            );
        }
        format!("{{\"nodes\":[{}]}}", nodes.join(","))
    }

    async fn run(
        dom_json: &str,
        files: HashMap<String, Vec<u8>>,
        options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let snapshot = DomSnapshot::new(dom_json);
        let fetcher = FakeFetcher { files };
        let page = PageCapture {
            url: "https://example.test/page",
            dom_snapshot: &snapshot,
            ax_tree: None,
            screenshot: None,
            dpr: 1.0,
            resources: &fetcher,
            lang_declared: Some("fr"),
        };
        MediaPlugin.analyze(&page, options).await
    }

    fn base_files(transcription: &str, subtitle: String) -> HashMap<String, Vec<u8>> {
        let mut files = HashMap::new();
        files.insert(MEDIA_URL.to_owned(), media_bytes(45.0));
        files.insert(REF_URL.to_owned(), reference_vtt().as_bytes().to_vec());
        files.insert(SUB_URL.to_owned(), subtitle.into_bytes());
        files.insert(TRANS_URL.to_owned(), transcription.as_bytes().to_vec());
        files
    }

    fn options() -> PluginOptions {
        PluginOptions {
            message: None,
            duree_max_s: None,
            transcription_reference_url: Some(REF_URL.to_owned()),
            duree_s: None,
        }
    }

    fn mesures_of(proofs: &[Preuve], plugin: &str) -> Mesures {
        let proof = proofs
            .iter()
            .find(|proof| proof.mesures.get(plugin).is_some())
            .unwrap_or_else(|| panic!("no proof with {plugin}: {proofs:?}"));
        serde_json::from_value(proof.mesures.clone()).expect("typed mesures")
    }

    #[tokio::test]
    async fn inventory_never_downloads_a_third_party_player() {
        let iframe_only = r#"{"nodes":[
            {"tag":"iframe","attrs":{"src":"https://www.youtube.com/embed/x"}}
        ]}"#;
        let proofs = run(iframe_only, HashMap::new(), &PluginOptions::default())
            .await
            .expect("third-party proof");
        assert_eq!(proofs.len(), 1);
        assert_eq!(
            proofs[0].sujet,
            Subject::Media {
                url: "https://www.youtube.com/embed/x".into()
            }
        );
        assert_eq!(
            proofs[0].mesures["raison"],
            "média tiers, analyse non effectuée"
        );
        assert!(proofs[0].mesures["media_tiers"].as_bool().expect("bool"));
    }

    #[tokio::test]
    async fn a_complete_transcription_covers_at_least_ninety_percent() {
        let transcription = "Bonjour le monde et bienvenue. Voici la deuxième phrase importante.";
        let proofs = run(
            &dom("", true, false),
            base_files(transcription, subtitle_vtt(0.0)),
            &options(),
        )
        .await
        .expect("analysis");
        let mesures = mesures_of(&proofs, "transcription_fournie");
        let transcription = mesures
            .transcription_fournie
            .as_ref()
            .expect("transcription");
        assert!(
            transcription.couverture >= 0.9,
            "{}",
            transcription.couverture
        );
        assert!(transcription.passages_manquants.is_empty());
    }

    #[tokio::test]
    async fn a_missing_second_sentence_is_reported_at_its_timestamps() {
        let proofs = run(
            &dom("", true, false),
            base_files("Bonjour le monde et bienvenue.", subtitle_vtt(0.0)),
            &options(),
        )
        .await
        .expect("analysis");
        let mesures = mesures_of(&proofs, "transcription_fournie");
        let passages = mesures
            .transcription_fournie
            .expect("transcription")
            .passages_manquants;
        assert_eq!(passages.len(), 1, "{passages:?}");
        assert!(
            (passages[0].debut_s - 20.0).abs() < 1.0,
            "{:?}",
            passages[0]
        );
        assert!((passages[0].fin_s - 40.0).abs() < 1.0, "{:?}", passages[0]);
    }

    #[tokio::test]
    async fn a_subtitle_track_shifted_by_two_seconds_has_a_2000ms_offset() {
        let proofs = run(
            &dom("", true, false),
            base_files(
                "Bonjour le monde et bienvenue. Voici la deuxième phrase importante.",
                subtitle_vtt(2.0),
            ),
            &options(),
        )
        .await
        .expect("analysis");
        let mesures = mesures_of(&proofs, "sous_titres");
        let subtitles = mesures.sous_titres.expect("subtitles");
        assert!(
            (subtitles.decalage_median_ms - 2000.0).abs() < 300.0,
            "{subtitles:?}"
        );
        assert!(
            (subtitles.decalage_p95_ms - 2000.0).abs() < 300.0,
            "{subtitles:?}"
        );
    }

    #[tokio::test]
    async fn music_only_is_neutral() {
        let mut files = base_files("", subtitle_vtt(0.0));
        files.insert(REF_URL.to_owned(), b"WEBVTT\n".to_vec());
        let proofs = run(&dom("", true, false), files, &options())
            .await
            .expect("analysis");
        let mesures = mesures_of(&proofs, "part_parole");
        assert!(mesures.part_parole < 0.02, "{}", mesures.part_parole);
        assert!(matches!(proofs[0].indice, Some(Indice::Neutre)));
    }

    #[tokio::test]
    async fn autoplay_sound_without_controls_is_suspect() {
        let mut files = HashMap::new();
        files.insert(MEDIA_URL.to_owned(), media_bytes(6.0));
        let proofs = run(
            &dom(", \"autoplay\":\"\"", false, false),
            files,
            &PluginOptions::default(),
        )
        .await
        .expect("analysis");
        let proof = proofs
            .iter()
            .find(|proof| proof.tests == vec!["4.10".to_owned()])
            .expect("4.10 proof");
        let son_auto: SonAutoMesure =
            serde_json::from_value(proof.mesures.clone()).expect("son auto");
        assert!(son_auto.autoplay);
        assert!(!son_auto.muted);
        assert!(son_auto.duree_au_dessus_seuil_s > 3.0, "{son_auto:?}");
        assert!(matches!(proof.indice, Some(Indice::Suspect { .. })));
    }

    #[tokio::test]
    async fn a_media_longer_than_the_limit_is_not_analysed() {
        let mut opts = options();
        opts.duree_max_s = Some(1);
        let proofs = run(
            &dom("", true, false),
            base_files("texte", subtitle_vtt(0.0)),
            &opts,
        )
        .await
        .expect("analysis");
        let mesures = mesures_of(&proofs, "decodage");
        assert!(mesures.decodage.ignore_pour_duree);
        assert!(matches!(proofs[0].indice, Some(Indice::Neutre)));
    }
}
