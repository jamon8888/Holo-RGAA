//! Text normalisation, word coverage and subtitle synchronisation (spec §3.4,
//! §3.5). Every value here is computed from supplied text: no model, no
//! estimation, deterministic and reproducible (R-P2/R-P4).

use crate::mesures::Passage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A timed span of text: a transcription segment or a subtitle cue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TimedText {
    pub start_s: f64,
    pub end_s: f64,
    pub text: String,
}

/// Lowercase, remove diacritics and punctuation, fold number words to digits.
/// Both sides of every comparison go through the same function.
pub fn normalize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|ch| match ch {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'œ' => 'o',
            _ => ch,
        })
        .flat_map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                Some(ch)
            } else {
                None
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(|word| {
            NUMBER_WORDS
                .iter()
                .find(|(spelled, _)| *spelled == word)
                .map_or(word, |(_, digit)| *digit)
                .to_owned()
        })
        .collect()
}

const NUMBER_WORDS: &[(&str, &str)] = &[
    ("zero", "0"),
    ("un", "1"),
    ("une", "1"),
    ("deux", "2"),
    ("trois", "3"),
    ("quatre", "4"),
    ("cinq", "5"),
    ("six", "6"),
    ("sept", "7"),
    ("huit", "8"),
    ("neuf", "9"),
    ("dix", "10"),
    ("onze", "11"),
    ("douze", "12"),
    ("treize", "13"),
    ("quatorze", "14"),
    ("quinze", "15"),
    ("seize", "16"),
    ("vingt", "20"),
    ("trente", "30"),
    ("quarante", "40"),
    ("cinquante", "50"),
    ("soixante", "60"),
    ("cent", "100"),
    ("mille", "1000"),
];

/// Fraction of `reference` words found in `candidate`, aligned with a
/// word-level LCS that tolerates a one-character typo in words of 4+ letters
/// (spec: "alignement par distance d'édition sur les mots").
#[must_use]
pub fn coverage(reference: &[String], candidate: &[String]) -> f64 {
    if reference.is_empty() {
        return 1.0;
    }
    let (n, m) = (reference.len(), candidate.len());
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            dp[i][j] = if similar(&reference[i - 1], &candidate[j - 1]) {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    dp[n][m] as f64 / n as f64
}

fn similar(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let longest = left.chars().count().max(right.chars().count());
    longest >= 4 && edit_distance(left, right) <= 1
}

fn edit_distance(left: &str, right: &str) -> usize {
    let a: Vec<char> = left.chars().collect();
    let b: Vec<char> = right.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = if ca == cb {
                previous
            } else {
                1 + previous.min(row[j]).min(old)
            };
            previous = old;
        }
    }
    row[b.len()]
}

/// Uncovered windows of `reference` longer than `min_window_s`, with the
/// reference excerpt. Consecutive uncovered segments closer than
/// `min_window_s` are merged into one window (spec §3.4, 15 s).
#[must_use]
pub fn missing_passages(
    reference: &[TimedText],
    covered: &[TimedText],
    min_window_s: f64,
) -> Vec<Passage> {
    let covered_words: Vec<String> = covered
        .iter()
        .flat_map(|cue| normalize(&cue.text))
        .collect();
    let mut groups: Vec<(f64, f64, String)> = Vec::new();
    for segment in reference {
        let words = normalize(&segment.text);
        if coverage(&words, &covered_words) >= 0.5 {
            continue;
        }
        if let Some(last) = groups.last_mut() {
            if segment.start_s - last.1 <= min_window_s {
                last.1 = last.1.max(segment.end_s);
                last.2.push(' ');
                last.2.push_str(&segment.text);
                continue;
            }
        }
        groups.push((segment.start_s, segment.end_s, segment.text.clone()));
    }
    groups
        .into_iter()
        .filter(|(start, end, _)| end - start > min_window_s)
        .map(|(debut_s, fin_s, extrait)| Passage {
            debut_s,
            fin_s,
            extrait,
        })
        .collect()
}

/// Median and p95 of `cue.start - segment.start` in milliseconds, matching
/// each cue to the reference segment with the highest word overlap (spec §3.5).
#[must_use]
pub fn sync_offsets(cues: &[TimedText], reference: &[TimedText]) -> Option<(f64, f64)> {
    let mut offsets = Vec::with_capacity(cues.len());
    for cue in cues {
        let cue_words = normalize(&cue.text);
        let mut best: Option<(f64, f64)> = None; // (overlap, offset_ms)
        for segment in reference {
            let segment_words = normalize(&segment.text);
            if cue_words.is_empty() || segment_words.is_empty() {
                continue;
            }
            let overlap =
                coverage(&cue_words, &segment_words).min(coverage(&segment_words, &cue_words));
            if overlap >= 0.5 {
                let offset = (cue.start_s - segment.start_s) * 1000.0;
                if best.is_none_or(|(score, _)| overlap > score) {
                    best = Some((overlap, offset));
                }
            }
        }
        if let Some((_, offset)) = best {
            offsets.push(offset);
        }
    }
    if offsets.is_empty() {
        return None;
    }
    offsets.sort_by(f64::total_cmp);
    Some((percentile(&offsets, 0.5), percentile(&offsets, 0.95)))
}

fn percentile(sorted: &[f64], quantile: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = quantile * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] + (sorted[upper] - sorted[lower]) * (position - lower as f64)
    }
}

/// Reference speech intervals not covered by any cue, longer than `min_gap_s`
/// (spec §3.5, 3 s).
#[must_use]
pub fn speech_without_cue(
    reference: &[TimedText],
    cues: &[TimedText],
    min_gap_s: f64,
) -> Vec<Passage> {
    let mut result = Vec::new();
    for segment in reference {
        let mut covered: Vec<(f64, f64)> = cues
            .iter()
            .filter(|cue| cue.end_s > segment.start_s && cue.start_s < segment.end_s)
            .map(|cue| {
                (
                    cue.start_s.max(segment.start_s),
                    cue.end_s.min(segment.end_s),
                )
            })
            .collect();
        covered.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut cursor = segment.start_s;
        for (start, end) in covered {
            if start - cursor > min_gap_s {
                result.push(Passage {
                    debut_s: cursor,
                    fin_s: start,
                    extrait: segment.text.clone(),
                });
            }
            cursor = cursor.max(end);
        }
        if segment.end_s - cursor > min_gap_s {
            result.push(Passage {
                debut_s: cursor,
                fin_s: segment.end_s,
                extrait: segment.text.clone(),
            });
        }
    }
    result
}

/// Share of the media duration occupied by speech segments (spec §3.2).
#[must_use]
pub fn speech_ratio(segments: &[TimedText], duration_s: f64) -> f64 {
    if duration_s <= 0.0 {
        return 0.0;
    }
    let spoken: f64 = segments
        .iter()
        .map(|segment| {
            let end = segment.end_s.min(duration_s);
            (end - segment.start_s.max(0.0)).max(0.0)
        })
        .sum();
    (spoken / duration_s).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> Vec<TimedText> {
        vec![
            TimedText {
                start_s: 1.0,
                end_s: 12.0,
                text: "Bonjour le monde et bienvenue".into(),
            },
            TimedText {
                start_s: 20.0,
                end_s: 40.0,
                text: "Voici la deuxième phrase importante".into(),
            },
        ]
    }

    #[test]
    fn normalize_folds_case_diacritics_punctuation_and_number_words() {
        let words = normalize("Bonjour, le monde... Deux chiens !");
        assert_eq!(words, vec!["bonjour", "le", "monde", "2", "chiens"]);
    }

    #[test]
    fn a_complete_transcription_has_full_coverage() {
        let reference_words: Vec<String> = reference()
            .iter()
            .flat_map(|segment| normalize(&segment.text))
            .collect();
        assert!((coverage(&reference_words, &reference_words) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_missing_second_sentence_yields_one_passage_at_its_timestamps() {
        let mut partial = reference();
        partial.remove(1);
        let passages = missing_passages(&reference(), &partial, 15.0);
        assert_eq!(passages.len(), 1, "{passages:?}");
        assert!((passages[0].debut_s - 20.0).abs() < 0.001);
        assert!((passages[0].fin_s - 40.0).abs() < 0.001);
        assert!(passages[0].extrait.contains("deuxième"));
    }

    #[test]
    fn a_vtt_shifted_by_two_seconds_has_a_2000ms_median_offset() {
        let shifted: Vec<TimedText> = reference()
            .iter()
            .map(|segment| TimedText {
                start_s: segment.start_s + 2.0,
                end_s: segment.end_s + 2.0,
                text: segment.text.clone(),
            })
            .collect();
        let (median, p95) = sync_offsets(&shifted, &reference()).expect("offsets");
        assert!((median - 2000.0).abs() < 300.0, "median {median}");
        assert!((p95 - 2000.0).abs() < 300.0, "p95 {p95}");
    }

    #[test]
    fn speech_without_any_cue_is_reported() {
        let gaps = speech_without_cue(&reference(), &[], 3.0);
        assert_eq!(gaps.len(), 2, "{gaps:?}");
        assert!((gaps[0].debut_s - 1.0).abs() < 0.001);
        assert!((gaps[1].fin_s - 40.0).abs() < 0.001);
    }

    #[test]
    fn music_only_has_no_speech() {
        assert_eq!(speech_ratio(&[], 30.0), 0.0);
    }
}
