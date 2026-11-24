//! 10.9 / 10.10 — textual references to shape, size, position or colour.
//!
//! Detection is a deterministic regex scan over the captured text (P-06
//! provides the language through `lang_declared`; no model is called here).
//! `avis_modele` is deliberately left `None`: judging whether a textual
//! alternative exists is the extension point for a future LLM second opinion.

use crate::mesures::{CandidatSensoriel, Measurement, Mesures};
use crate::snapshot::Snapshot;
use regex_lite::Regex;
use rgaa_analysis::Portee;

/// FR/EN phrases, grouped by sensory category (spec §3, 10.9/10.10).
const PATTERNS: &[(&str, &str)] = &[
    (
        "forme",
        r"(?i)\b(rond|ronde|carre|carree|cercle|rectangulaire|circle|square|round)\b",
    ),
    (
        "position",
        r"(?i)\b(ci-dessous|ci-dessus|ci-contre|a droite|a gauche|en haut|en bas|dessous|dessus|below|above|on the right|on the left)\b",
    ),
    (
        "taille",
        r"(?i)\b(grand|grande|petit|petite|gros|grosse|big|small|large)\b",
    ),
    (
        "couleur",
        r"(?i)\b(vert|verte|rouge|bleu|bleue|jaune|green|red|blue|yellow)\b",
    ),
];

/// Lowercases and folds the French accents so ASCII regexes catch both
/// "à droite" and "a droite" without a Unicode dependency.
#[must_use]
pub fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| match character {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// 10.9 / 10.10 measurement.
#[must_use]
pub fn analyze(snapshot: &Snapshot, lang: &str) -> Measurement {
    let compiled: Vec<(&str, &str, Regex)> = PATTERNS
        .iter()
        .filter_map(|(category, pattern)| {
            Regex::new(pattern)
                .ok()
                .map(|regex| (*category, *pattern, regex))
        })
        .collect();

    let mut candidats = Vec::new();
    for node in &snapshot.nodes {
        if !node.is_visible() || !node.has_text() {
            continue;
        }
        let normalized = normalize(&node.text);
        for (category, pattern, regex) in &compiled {
            if regex.is_match(&normalized) {
                candidats.push(CandidatSensoriel {
                    selecteur: node.selecteur.clone(),
                    texte: node.text.clone(),
                    motif: (*pattern).to_owned(),
                    categorie: (*category).to_owned(),
                    langue: lang.to_owned(),
                });
            }
        }
    }

    let mesure = Measurement::new(
        Mesures::Sensory {
            candidats: candidats.clone(),
            langue: lang.to_owned(),
            avis_modele: None,
        },
        Portee::Mesuree,
        &["10.9.1", "10.10.1"],
    );
    if candidats.is_empty() {
        mesure
    } else {
        mesure.suspect("référence à la forme, la taille, la position ou la couleur")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_french_and_english_sensory_phrases() {
        assert_eq!(normalize("À droite"), "a droite");
        let cases = [
            ("Cliquez sur le bouton vert à droite.", "couleur"),
            ("Cliquez sur le bouton vert à droite.", "position"),
            ("Select the green icon on the right.", "couleur"),
            ("Voir le cercle ci-dessous.", "forme"),
        ];
        for (text, category) in cases {
            let normalized = normalize(text);
            assert!(
                compiled_has(&normalized, category),
                "{text} should match {category}"
            );
        }
    }

    fn compiled_has(normalized: &str, category: &str) -> bool {
        PATTERNS
            .iter()
            .find(|(name, _)| *name == category)
            .and_then(|(_, pattern)| Regex::new(pattern).ok())
            .is_some_and(|regex| regex.is_match(normalized))
    }
}
