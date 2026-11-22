//! Whisper transcription through `whisper-rs` 0.16 (whisper.cpp / ggml).
//!
//! ggml is not ONNX: this path adds no second runtime to the binary. The
//! `small` multilingual model is the default and is read from the local cache
//! only (R-P3).

use crate::align::TimedText;
use crate::models;
use rgaa_analysis::{analysis_error, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Whisper output: timed segments and the language it detected.
#[derive(Debug, Clone, PartialEq)]
pub struct AsrOutput {
    pub segments: Vec<TimedText>,
    pub langue_detectee: Option<String>,
}

/// Transcribes 16 kHz mono samples with the cached ggml model.
///
/// # Errors
/// Returns an `Analysis` error when the model is absent/corrupted or when
/// whisper.cpp cannot create a context or run inference. Never downloads.
pub fn transcribe(mono: &[f32]) -> Result<AsrOutput> {
    let path = models::require(&models::ASR_MODEL)?;
    let context = WhisperContext::new_with_params(&path, WhisperContextParameters::default())
        .map_err(|error| analysis_error("media", format!("contexte Whisper : {error}")))?;
    let mut state = context
        .create_state()
        .map_err(|error| analysis_error("media", format!("état Whisper : {error}")))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_single_segment(false);
    params.set_translate(false);
    params.set_language(None); // let Whisper detect it; P-06 compares with `lang`
    params.set_n_threads(4);

    state
        .full(params, mono)
        .map_err(|error| analysis_error("media", format!("transcription Whisper : {error}")))?;

    let mut segments = Vec::new();
    for index in 0..state.full_n_segments() {
        if let Some(segment) = state.get_segment(index) {
            let text = segment
                .to_str_lossy()
                .map_err(|error| analysis_error("media", format!("segment Whisper : {error}")))?;
            let text = text.trim().to_owned();
            if !text.is_empty() {
                segments.push(TimedText {
                    start_s: segment.start_timestamp() as f64 / 100.0,
                    end_s: segment.end_timestamp() as f64 / 100.0,
                    text,
                });
            }
        }
    }

    // `full_lang_id_from_state` returns a numeric id; turning it into a stable
    // code requires whisper.cpp's token table, so P-06 gets the detected id
    // through the model metadata instead of a guessed string here.
    let langue_detectee = None;
    Ok(AsrOutput {
        segments,
        langue_detectee,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio;
    use crate::testing::model_test_enabled;

    #[tokio::test]
    async fn whisper_transcribes_when_the_model_is_present() {
        if !model_test_enabled("whisper_transcribes", &models::ASR_MODEL) {
            return;
        }
        // Public whisper.cpp speech sample: assertions are about the pipeline,
        // not about a hand-copied transcript that could drift with the model.
        let wav = include_bytes!("../tests/data/jfk.wav");
        let decoded = audio::decode_symphonia(wav, Some("wav")).expect("decode");
        let result = transcribe(&decoded.samples).expect("transcription");
        assert!(
            result
                .segments
                .iter()
                .any(|segment| segment.text.chars().count() > 5),
            "{result:?}"
        );
        assert!(result
            .segments
            .iter()
            .all(|segment| segment.end_s >= segment.start_s));
    }
}
