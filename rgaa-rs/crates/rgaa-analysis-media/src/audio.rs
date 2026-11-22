//! Audio decode, resample and loudness (spec §2, §3.7).
//!
//! Symphonia is the default decoder; `ffmpeg` is a sidecar fallback, never a
//! linked library. When neither can decode, the caller reports « preuve
//! manquante (ffmpeg absent) » rather than inventing a measurement (R-P2).

use ebur128::{EbuR128, Mode};
use rgaa_analysis::{analysis_error, Result};
use rubato::audioadapter_buffers::owned::InterleavedOwned;
use rubato::{Fft, FixedSync, Resampler};
use std::io::Cursor;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Target for ASR/VAD and the loudness meter: 16 kHz mono.
pub const CIBLE_HZ: u32 = 16_000;
/// EBU R128 short-term threshold for 4.10 (spec §3.7).
pub const SEUIL_LUFS: f64 = -50.0;

/// Decoded interleaved audio, before resampling.
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved `f32` samples.
    pub samples: Vec<f32>,
    pub codec: String,
    pub via_ffmpeg: bool,
}

impl DecodedAudio {
    /// Duration in seconds, derived from sample count and format.
    #[must_use]
    pub fn duree_s(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
    }
}

/// Decodes with symphonia. The format hint is the URL extension when known.
///
/// # Errors
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when demuxing or decoding
/// fails, or when no audio track is present.
pub fn decode_symphonia(bytes: &[u8], hint_ext: Option<&str>) -> Result<DecodedAudio> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = hint_ext {
        hint.with_extension(extension);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| analysis_error("media", format!("démultiplexage impossible : {error}")))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| analysis_error("media", "aucune piste audio dans le média"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| analysis_error("media", "paramètres audio absents"))?;
    let sample_rate = params
        .sample_rate
        .ok_or_else(|| analysis_error("media", "fréquence d'échantillonnage absente"))?;
    let channels = params.channels.as_ref().map_or(1, |layout| layout.count()) as u16;
    let codec = format!("{:?}", params.codec);
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|error| analysis_error("media", format!("décodeur indisponible : {error}")))?;

    let mut samples = Vec::new();
    while let Ok(Some(packet)) = format.next_packet() {
        if packet.track_id != track_id {
            continue;
        }
        if let Ok(decoded) = decoder.decode(&packet) {
            let mut block = vec![0.0_f32; decoded.samples_interleaved()];
            decoded.copy_to_slice_interleaved(&mut block);
            samples.extend_from_slice(&block);
        }
    }
    if samples.is_empty() {
        return Err(analysis_error("media", "aucun échantillon audio décodé"));
    }
    Ok(DecodedAudio {
        sample_rate,
        channels,
        samples,
        codec,
        via_ffmpeg: false,
    })
}

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Decodes through the `ffmpeg` binary (sidecar, never linked). Requests 16 kHz
/// mono `f32le` directly, as specified by the sidecar fallback in §2.
///
/// # Errors
/// Missing binary, a non-zero exit status or unreadable stdout are all
/// returned as an `Analysis` error naming ffmpeg.
pub fn decode_ffmpeg(bytes: &[u8], hint_ext: Option<&str>) -> Result<DecodedAudio> {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("rgaa-media-{}-{}", std::process::id(), sequence));
    std::fs::create_dir_all(&directory).map_err(|error| {
        analysis_error(
            "media",
            format!("répertoire temporaire indisponible : {error}"),
        )
    })?;
    let extension = hint_ext
        .filter(|ext| ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("media");
    let input = directory.join(format!("input.{extension}"));
    let write_result = std::fs::write(&input, bytes).map_err(|error| {
        analysis_error(
            "media",
            format!("écriture du média temporaire impossible : {error}"),
        )
    });
    if let Err(error) = write_result {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&input)
        .args([
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-ar",
            &CIBLE_HZ.to_string(),
            "-ac",
            "1",
            "pipe:1",
        ])
        .output()
        .map_err(|error| {
            let _ = std::fs::remove_dir_all(&directory);
            analysis_error(
                "media",
                format!("preuve manquante (ffmpeg absent) : {error}"),
            )
        })?;
    let _ = std::fs::remove_dir_all(&directory);
    if !output.status.success() {
        return Err(analysis_error(
            "media",
            format!(
                "ffmpeg a échoué : {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    let (frames, _tail): (&[[u8; 4]], &[u8]) = output.stdout.as_chunks::<4>();
    let samples: Vec<f32> = frames
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect();
    if samples.is_empty() {
        return Err(analysis_error(
            "media",
            "ffmpeg n'a produit aucun échantillon",
        ));
    }
    Ok(DecodedAudio {
        sample_rate: CIBLE_HZ,
        channels: 1,
        samples,
        codec: "ffmpeg-pcm-f32le".to_owned(),
        via_ffmpeg: true,
    })
}

/// Native decode first, ffmpeg fallback second. Set `allow_ffmpeg` to `false`
/// to exercise the "ffmpeg absent" acceptance criterion without uninstalling
/// the binary.
///
/// # Errors
/// Returns a « preuve manquante » error when no decoder can read the bytes.
pub fn decode_with_ffmpeg(
    bytes: &[u8],
    hint_ext: Option<&str>,
    allow_ffmpeg: bool,
) -> Result<DecodedAudio> {
    match decode_symphonia(bytes, hint_ext) {
        Ok(audio) => Ok(audio),
        Err(native_error) => {
            tracing::warn!(
                error = %native_error,
                "symphonia could not decode media; trying the ffmpeg sidecar"
            );
            if allow_ffmpeg {
                decode_ffmpeg(bytes, hint_ext)
            } else {
                Err(analysis_error(
                    "media",
                    format!("preuve manquante (ffmpeg absent) : {native_error}"),
                ))
            }
        }
    }
}

/// Convenience wrapper with the fallback enabled.
///
/// # Errors
/// Propagates [`decode_with_ffmpeg`] errors.
pub fn decode(bytes: &[u8], hint_ext: Option<&str>) -> Result<DecodedAudio> {
    decode_with_ffmpeg(bytes, hint_ext, true)
}

/// Downmixes to mono and resamples to 16 kHz with rubato. Nothing else is
/// assumed about the source layout (spec §2, §3).
///
/// # Errors
/// Returns an `Analysis` error if the resampler cannot be built or run.
pub fn to_mono_16k(audio: &DecodedAudio) -> Result<Vec<f32>> {
    if audio.via_ffmpeg && audio.sample_rate == CIBLE_HZ && audio.channels == 1 {
        return Ok(audio.samples.clone());
    }
    let channels = audio.channels as usize;
    if channels == 0 {
        return Err(analysis_error("media", "média sans canal audio"));
    }
    let frames = audio.samples.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let mut sum = 0.0_f32;
        for channel in 0..channels {
            sum += audio.samples[frame * channels + channel];
        }
        mono.push(sum / channels as f32);
    }
    if audio.sample_rate == CIBLE_HZ {
        return Ok(mono);
    }
    let mut resampler = Fft::<f32>::new(
        usize::try_from(audio.sample_rate)
            .map_err(|_| analysis_error("media", "fréquence invalide"))?,
        CIBLE_HZ as usize,
        1024,
        1,
        FixedSync::Both,
    )
    .map_err(|error| {
        analysis_error("media", format!("rééchantillonneur indisponible : {error}"))
    })?;
    let input = InterleavedOwned::new_from(mono.clone(), 1, frames)
        .map_err(|error| analysis_error("media", format!("buffer audio invalide : {error}")))?;
    let output = resampler
        .process_all(&input, frames, None)
        .map_err(|error| {
            analysis_error("media", format!("rééchantillonnage impossible : {error}"))
        })?;
    Ok(output.take_data())
}

/// Seconds during which EBU R128 short-term loudness exceeds `threshold_lufs`.
/// Blocks of `hop_ms` are accumulated once the 400 ms short-term window is
/// warm (spec §3.7: 3 s above −50 LUFS without an accessible control → `Suspect`).
///
/// # Errors
/// Returns an `Analysis` error when the meter cannot be created or fed.
pub fn loudness_above_threshold(
    mono: &[f32],
    sample_rate: u32,
    threshold_lufs: f64,
    hop_ms: u32,
) -> Result<f64> {
    let mut meter = EbuR128::new(1, sample_rate, Mode::S)
        .map_err(|error| analysis_error("media", format!("ebur128 indisponible : {error}")))?;
    let hop = usize::try_from(u64::from(sample_rate) * u64::from(hop_ms) / 1000)
        .map_err(|_| analysis_error("media", "pas de mesure audio invalide"))?;
    if hop == 0 {
        return Err(analysis_error("media", "pas de mesure audio nul"));
    }
    let mut above = 0.0;
    for block in mono.chunks(hop) {
        meter
            .add_frames_f32(block)
            .map_err(|error| analysis_error("media", format!("ebur128 : {error}")))?;
        if meter
            .loudness_shortterm()
            .is_ok_and(|loudness| loudness > threshold_lufs)
        {
            above += block.len() as f64 / f64::from(sample_rate);
        }
    }
    Ok(above)
}

/// Writes a minimal 16-bit PCM WAV in test code — no fixture file, no model.
#[cfg(test)]
pub(crate) fn wav_bytes(sample_rate: u32, channels: u16, frames: &[f32]) -> Vec<u8> {
    let bit_depth = 16u16;
    let data_len = (frames.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    wav.extend_from_slice(&(channels * 2).to_le_bytes());
    wav.extend_from_slice(&bit_depth.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in frames {
        let clipped = sample.clamp(-1.0, 1.0);
        wav.extend_from_slice(&((clipped * 32_767.0) as i16).to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, seconds: f64, amplitude: f32) -> Vec<f32> {
        let frames = (rate as f64 * seconds) as usize;
        (0..frames)
            .map(|index| {
                amplitude
                    * (2.0 * std::f64::consts::PI * 440.0 * index as f64 / rate as f64).sin() as f32
            })
            .collect()
    }

    #[test]
    fn symphonia_decodes_a_generated_wav() {
        let bytes = wav_bytes(8_000, 1, &sine(8_000, 1.0, 0.5));
        let decoded = decode_symphonia(&bytes, Some("wav")).expect("decode");
        assert_eq!(decoded.sample_rate, 8_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.via_ffmpeg);
        assert!(
            (decoded.duree_s() - 1.0).abs() < 0.01,
            "{}",
            decoded.duree_s()
        );
    }

    #[test]
    fn resampling_upsamples_by_the_rate_ratio() {
        let bytes = wav_bytes(8_000, 1, &sine(8_000, 1.0, 0.5));
        let decoded = decode_symphonia(&bytes, Some("wav")).expect("decode");
        let mono = to_mono_16k(&decoded).expect("resample");
        assert!(
            mono.len() > 15_000 && mono.len() < 17_000,
            "unexpected output length {}",
            mono.len()
        );
    }

    #[test]
    fn stereo_is_downmixed_to_mono() {
        let left = vec![1.0_f32; 100];
        let right = vec![-1.0_f32; 100];
        let mut interleaved = Vec::with_capacity(200);
        for index in 0..100 {
            interleaved.push(left[index]);
            interleaved.push(right[index]);
        }
        let audio = DecodedAudio {
            sample_rate: CIBLE_HZ,
            channels: 2,
            samples: interleaved,
            codec: "pcm".into(),
            via_ffmpeg: false,
        };
        let mono = to_mono_16k(&audio).expect("downmix");
        assert_eq!(mono.len(), 100);
        assert!(mono.iter().all(|sample| sample.abs() < 1e-6));
    }

    #[test]
    fn a_loud_signal_exceeds_minus_50_lufs_for_more_than_three_seconds() {
        let bytes = wav_bytes(CIBLE_HZ, 1, &sine(CIBLE_HZ, 5.0, 0.8));
        let decoded = decode_symphonia(&bytes, Some("wav")).expect("decode");
        let mono = to_mono_16k(&decoded).expect("mono");
        let above = loudness_above_threshold(&mono, CIBLE_HZ, SEUIL_LUFS, 100).expect("loudness");
        assert!(above > 3.0, "above threshold for {above}s");
    }

    #[test]
    fn silence_never_exceeds_the_threshold() {
        let bytes = wav_bytes(CIBLE_HZ, 1, &vec![0.0; (CIBLE_HZ * 5) as usize]);
        let decoded = decode_symphonia(&bytes, Some("wav")).expect("decode");
        let mono = to_mono_16k(&decoded).expect("mono");
        let above = loudness_above_threshold(&mono, CIBLE_HZ, SEUIL_LUFS, 100).expect("loudness");
        assert!((above - 0.0).abs() < 1e-9, "silence measured {above}s");
    }

    #[test]
    fn unsupported_input_without_ffmpeg_is_missing_proof() {
        let error =
            decode_with_ffmpeg(b"not media at all", None, false).expect_err("missing proof");
        assert!(
            error
                .to_string()
                .contains("preuve manquante (ffmpeg absent)"),
            "{error}"
        );
    }
}
