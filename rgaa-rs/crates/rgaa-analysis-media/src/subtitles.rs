//! Subtitle readers: a small hand-written SRT parser and `webvtt-parser` for
//! VTT (spec §2). `subparse` is abandoned, so SRT stays in-house (~40 lines).

use crate::align::TimedText;
use rgaa_analysis::{analysis_error, Result};
use webvtt_parser::OwnedVtt;

/// Reads VTT with the maintained parser. Markup tags are stripped so the text
/// can be normalised and compared by [`crate::align`].
///
/// # Errors
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when the VTT is malformed.
pub fn parse_vtt(input: &str) -> Result<Vec<TimedText>> {
    let vtt = OwnedVtt::parse(input)
        .map_err(|error| analysis_error("media", format!("VTT illisible : {error}")))?;
    Ok(vtt
        .cues
        .into_iter()
        .map(|cue| TimedText {
            start_s: cue.start.as_milliseconds() as f64 / 1000.0,
            end_s: cue.end.as_milliseconds() as f64 / 1000.0,
            text: strip_markup(&cue.text),
        })
        .collect())
}

/// Reads SRT. Accepts `HH:MM:SS,mmm` and `MM:SS,mmm` timestamps.
///
/// # Errors
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when a timing line is
/// malformed.
pub fn parse_srt(input: &str) -> Result<Vec<TimedText>> {
    let normalised = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = Vec::new();
    for block in normalised.split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        let timing_index = lines
            .iter()
            .position(|line| line.contains("-->"))
            .ok_or_else(|| {
                analysis_error("media", format!("bloc SRT sans horodatage : {block:?}"))
            })?;
        if timing_index == 0 && lines[0].parse::<usize>().is_err() {
            return Err(analysis_error(
                "media",
                format!("numéro de cue SRT invalide : {}", lines[0]),
            ));
        }
        let timing = lines[timing_index];
        let (start_text, end_text) = timing.split_once("-->").ok_or_else(|| {
            analysis_error("media", format!("horodatage SRT invalide : {timing}"))
        })?;
        let start_s = parse_srt_time(start_text.trim()).ok_or_else(|| {
            analysis_error(
                "media",
                format!("début SRT invalide : {}", start_text.trim()),
            )
        })?;
        let end_s = parse_srt_time(end_text.trim()).ok_or_else(|| {
            analysis_error("media", format!("fin SRT invalide : {}", end_text.trim()))
        })?;
        let text = lines[timing_index + 1..].join(" ");
        cues.push(TimedText {
            start_s,
            end_s,
            text: strip_markup(&text),
        });
    }
    Ok(cues)
}

/// Picks the reader from the file name when available, else from the content.
///
/// # Errors
/// Propagates a parse error from the selected reader.
pub fn parse_subtitles(input: &str, name_hint: Option<&str>) -> Result<Vec<TimedText>> {
    let looks_vtt = name_hint
        .map(|name| name.to_ascii_lowercase().ends_with(".vtt"))
        .unwrap_or(false)
        || input.trim_start().starts_with("WEBVTT");
    if looks_vtt {
        parse_vtt(input)
    } else {
        parse_srt(input)
    }
}

/// Removes `<v …>`, `<i>`, `</c>` and similar inline markup.
fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    for ch in text.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_srt_time(value: &str) -> Option<f64> {
    let (clock, millis) = value.rsplit_once(',')?;
    let fields: Vec<&str> = clock.split(':').collect();
    let (hours, minutes, seconds) = match fields.len() {
        3 => (
            fields[0].parse::<f64>().ok()?,
            fields[1].parse::<f64>().ok()?,
            fields[2].parse::<f64>().ok()?,
        ),
        2 => (
            0.0,
            fields[0].parse::<f64>().ok()?,
            fields[1].parse::<f64>().ok()?,
        ),
        _ => return None,
    };
    Some(hours * 3600.0 + minutes * 60.0 + seconds + millis.parse::<f64>().ok()? / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRT: &str = "1\n00:00:01,000 --> 00:00:04,500\nBonjour le monde\n\n\
                       2\n00:00:20,250 --> 00:00:40,000\nDeuxième phrase.\n";
    const VTT: &str = "WEBVTT\n\n00:00:01.000 --> 00:00:04.500\nBonjour <v Jean>le monde\n\n\
                       00:00:20.250 --> 00:00:40.000\nDeuxième phrase.\n";

    fn expected() -> Vec<TimedText> {
        vec![
            TimedText {
                start_s: 1.0,
                end_s: 4.5,
                text: "Bonjour le monde".into(),
            },
            TimedText {
                start_s: 20.25,
                end_s: 40.0,
                text: "Deuxième phrase.".into(),
            },
        ]
    }

    #[test]
    fn srt_parser_reads_indices_timestamps_and_text() {
        assert_eq!(parse_srt(SRT).expect("SRT"), expected());
    }

    #[test]
    fn vtt_parser_reads_the_same_cues_and_strips_markup() {
        assert_eq!(parse_vtt(VTT).expect("VTT"), expected());
    }

    #[test]
    fn subtitle_reader_sniffs_the_format() {
        assert_eq!(parse_subtitles(SRT, Some("fr.srt")).expect("SRT").len(), 2);
        assert_eq!(parse_subtitles(VTT, Some("fr.vtt")).expect("VTT").len(), 2);
        assert_eq!(parse_subtitles(VTT, None).expect("sniffed VTT").len(), 2);
    }

    #[test]
    fn malformed_srt_is_an_error_not_an_empty_result() {
        assert!(parse_srt("1\nnot a timing\nhello").is_err());
    }
}
