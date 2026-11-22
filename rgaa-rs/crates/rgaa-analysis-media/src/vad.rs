//! Silero VAD v5 through the workspace's single ONNX Runtime (`ort`).
//!
//! The model is read from the local cache only; a missing model is an error
//! naming `rgaa models pull media`, never a download (R-P3). The decoder
//! upstream already delivers 16 kHz mono, which is what the model expects.

use crate::align::TimedText;
use crate::models;
use ndarray::{concatenate, Array2, Array3, ArrayD, Axis, Ix3};
use ort::session::Session;
use ort::value::Value;
use rgaa_analysis::{analysis_error, Result};

/// 512 samples at 16 kHz — the window of Silero VAD v5.
pub const FENETRE: usize = 512;
/// Silero VAD v5 prepends 64 samples of previous audio to every 512-sample window.
pub const CONTEXTE: usize = 64;
/// Conservative speech probability threshold with hysteresis.
pub const SEUIL_PAROLE: f32 = 0.5;
const HYSTERESIS: f32 = 0.15;
const MIN_PAROLE_S: f64 = 0.25;
const MIN_SILENCE_S: f64 = 0.50;
const PADDING_S: f64 = 0.30;

/// Speech segments and the resulting speech share.
#[derive(Debug, Clone, PartialEq)]
pub struct VadOutput {
    pub segments: Vec<TimedText>,
    pub part_parole: f64,
}

/// Runs Silero VAD on 16 kHz mono samples.
///
/// # Errors
/// Returns an `Analysis` error when the model is absent/corrupted, the ONNX
/// session cannot be created, or inference fails.
pub fn detect_speech(mono: &[f32], sample_rate: u32) -> Result<VadOutput> {
    if sample_rate != 16_000 {
        return Err(analysis_error(
            "media",
            format!("VAD Silero : 16 kHz attendu, reçu {sample_rate} Hz"),
        ));
    }
    let path = models::require(&models::VAD_MODEL)?;
    // ort rc.15 requires an explicit environment before any session builder.
    let environment = ort::init()
        .build()
        .map_err(|error| analysis_error("media", format!("environnement ort : {error}")))?;
    let mut session = Session::builder(&environment)
        .and_then(|mut builder| builder.commit_from_file(&path))
        .map_err(|error| analysis_error("media", format!("session Silero VAD : {error}")))?;

    let window_count = mono.len() / FENETRE;
    let mut probabilities = Vec::with_capacity(window_count);
    let mut state = Array3::<f32>::zeros((2, 1, 128));

    let (windows, _tail) = mono.as_chunks::<FENETRE>();
    let mut context = [0.0_f32; CONTEXTE];
    for window in windows {
        let mut samples = Vec::with_capacity(CONTEXTE + FENETRE);
        samples.extend_from_slice(&context);
        samples.extend_from_slice(window);
        let input = Array2::from_shape_vec((1, CONTEXTE + FENETRE), samples.clone())
            .map_err(|error| analysis_error("media", format!("entrée audio VAD : {error}")))?;
        let input_value = Value::from_array(input)
            .map_err(|error| analysis_error("media", format!("entrée audio VAD : {error}")))?;
        let state_value = Value::from_array(state.clone())
            .map_err(|error| analysis_error("media", format!("état VAD : {error}")))?;
        let sr_value = Value::from_array(ndarray::arr0(16_000_i64))
            .map_err(|error| analysis_error("media", format!("entrée sr : {error}")))?;
        let inputs = ort::inputs![
            "input" => input_value,
            "state" => state_value,
            "sr" => sr_value,
        ];
        let output = session
            .run(inputs)
            .map_err(|error| analysis_error("media", format!("inférence Silero VAD : {error}")))?;

        let probability = output["output"]
            .try_extract_array::<f32>()
            .map_err(|error| analysis_error("media", format!("sortie VAD : {error}")))?
            .to_owned();
        probabilities.push(probability.iter().next().copied().unwrap_or(0.0));

        // Upstream silero_vad.onnx returns the whole recurrent state as
        // `stateN`; older exports split it into `hn` and `cn`. Support both.
        let next_state: ArrayD<f32> = if let Some(state_next) = output.get("stateN") {
            state_next
                .try_extract_array::<f32>()
                .map_err(|error| analysis_error("media", format!("état stateN : {error}")))?
                .to_owned()
        } else {
            let hn: ArrayD<f32> = output["hn"]
                .try_extract_array::<f32>()
                .map_err(|error| analysis_error("media", format!("état hn : {error}")))?
                .to_owned();
            let cn: ArrayD<f32> = output["cn"]
                .try_extract_array::<f32>()
                .map_err(|error| analysis_error("media", format!("état cn : {error}")))?
                .to_owned();
            concatenate(Axis(0), &[hn.view(), cn.view()])
                .map_err(|error| analysis_error("media", format!("état VAD : {error}")))?
        };
        state = next_state
            .into_dimensionality::<Ix3>()
            .map_err(|error| analysis_error("media", format!("état VAD invalide : {error}")))?;
        context.copy_from_slice(&samples[samples.len() - CONTEXTE..]);
    }

    let segments = windows_to_segments(&probabilities, mono.len(), f64::from(sample_rate));
    let spoken: f64 = segments
        .iter()
        .map(|segment| {
            (segment.end_s - segment.start_s).min(mono.len() as f64 / f64::from(sample_rate))
        })
        .sum();
    let part_parole = if mono.is_empty() {
        0.0
    } else {
        (spoken / (mono.len() as f64 / f64::from(sample_rate))).min(1.0)
    };
    Ok(VadOutput {
        segments,
        part_parole,
    })
}

/// Turns per-window probabilities into padded, merged speech segments.
fn windows_to_segments(
    probabilities: &[f32],
    sample_count: usize,
    sample_rate: f64,
) -> Vec<TimedText> {
    let window_s = FENETRE as f64 / sample_rate;
    let mut raw: Vec<(f64, f64)> = Vec::new();
    let mut start: Option<f64> = None;
    let mut last_voice: f64 = 0.0;

    for (index, &probability) in probabilities.iter().enumerate() {
        let time = index as f64 * window_s;
        if probability >= SEUIL_PAROLE {
            last_voice = time + window_s;
            if start.is_none() {
                start = Some(time);
            }
        } else if let Some(begin) = start {
            if probability < SEUIL_PAROLE - HYSTERESIS && last_voice - begin >= MIN_PAROLE_S {
                raw.push((begin, last_voice));
                start = None;
            }
        }
    }
    if let Some(begin) = start {
        if last_voice - begin >= MIN_PAROLE_S {
            raw.push((begin, last_voice));
        }
    }

    let duration = sample_count as f64 / sample_rate;
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (begin, end) in raw {
        if let Some(last) = merged.last_mut() {
            if begin - last.1 < MIN_SILENCE_S {
                last.1 = end;
                continue;
            }
        }
        merged.push((begin, end));
    }

    merged
        .into_iter()
        .map(|(begin, end)| TimedText {
            start_s: (begin - PADDING_S).max(0.0),
            end_s: (end + PADDING_S).min(duration),
            text: String::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio;
    use crate::testing::model_test_enabled;

    #[test]
    fn windows_to_segments_merges_short_silences_and_pads() {
        // 20 windows of 512 samples at 16 kHz = 0.64 s.
        let mut probabilities = vec![0.0_f32; 20];
        for probability in probabilities.iter_mut().take(5) {
            *probability = 0.9;
        }
        for probability in probabilities.iter_mut().skip(7).take(5) {
            *probability = 0.9;
        }
        let segments = windows_to_segments(&probabilities, 20 * FENETRE, 16_000.0);
        assert_eq!(segments.len(), 1, "{segments:?}");
        assert!(segments[0].start_s < 0.01);
        assert!(segments[0].end_s > 0.4);
    }

    #[test]
    fn vad_rejects_a_wrong_sample_rate_without_touching_the_model() {
        assert!(detect_speech(&[0.0; FENETRE], 8_000).is_err());
    }

    #[tokio::test]
    async fn silero_vad_detects_speech_when_the_model_is_present() {
        if !model_test_enabled("silero_vad_detects_speech", &models::VAD_MODEL) {
            return;
        }
        // Public whisper.cpp speech sample: real speech, not a synthetic tone.
        let wav = include_bytes!("../tests/data/jfk.wav");
        let decoded = audio::decode_symphonia(wav, Some("wav")).expect("decode");
        let result = detect_speech(&decoded.samples, decoded.sample_rate).expect("VAD");
        assert!(result.part_parole > 0.1, "{result:?}");
        assert!(!result.segments.is_empty(), "{result:?}");
    }
}
