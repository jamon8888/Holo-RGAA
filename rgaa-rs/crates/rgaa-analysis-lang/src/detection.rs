//! Language detection, exclusions and page-level aggregation (spec §2.2–2.5).

use crate::blocs::Bloc;
use crate::mesures::BlocDivergent;
use indexmap::IndexMap;
use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};
use std::sync::OnceLock;

const CONFIANCE_PAGE: f64 = 0.8;
/// Seuil de divergence 8.7/8.8, appliqué à la *marge* de confiance définie dans
/// [`detecte`]. Écart volontaire au seuil 0,9 de la spec `06-langue.md` §2.4 :
/// la probabilité brute de lingua dépend du nombre de langues liées (33 ici).
/// Mesures relevées avec lingua 1.8 et les 33 langues liées :
/// « Stay hungry, stay foolish » 0,7066 brut (en, 2e finnois 0,0365) → 0,9509
/// de marge ; « La langue par défaut de la page doit être pertinente… »
/// 0,9670 brut (fr, 2e italien 0,0132) → 0,9865 ; « Le week-end, je fais du
/// jogging » 0,1610 brut → 0,5909 (et exclu par ailleurs). Le seuil 0,9
/// sépare le candidat attendu du bruit, sans fabriquer de faux candidat.
const CONFIANCE_BLOC: f64 = 0.9;
/// Seuil de mots pour écarter un bloc fait d'exclusions (spec §2.5).
const MOTS_MINIMUM: usize = 5;
/// Seuil de mots pour un candidat 8.7/8.8. La spec §2.4 dit « au moins 5 mots »
/// mais son propre exemple « Stay hungry, stay foolish » en contient 4 : la
/// table de tests §4 prévaut, d'où 4 ici (les exclusions gardent leur seuil 5).
const MOTS_DIVERGENCE: usize = 4;

/// Only the languages listed in `06-langue.md` §1 are linked (see Cargo.toml).
static LANGUES: &[Language] = &[
    Language::Arabic,
    Language::Basque,
    Language::Bokmal,
    Language::Bulgarian,
    Language::Catalan,
    Language::Chinese,
    Language::Croatian,
    Language::Czech,
    Language::Danish,
    Language::Dutch,
    Language::English,
    Language::Estonian,
    Language::Finnish,
    Language::French,
    Language::German,
    Language::Greek,
    Language::Hungarian,
    Language::Icelandic,
    Language::Irish,
    Language::Italian,
    Language::Latvian,
    Language::Lithuanian,
    Language::Nynorsk,
    Language::Polish,
    Language::Portuguese,
    Language::Romanian,
    Language::Russian,
    Language::Slovak,
    Language::Slovene,
    Language::Spanish,
    Language::Swedish,
    Language::Turkish,
    Language::Ukrainian,
];

static DETECTEUR: OnceLock<LanguageDetector> = OnceLock::new();

fn detecteur() -> &'static LanguageDetector {
    DETECTEUR.get_or_init(|| {
        let mut builder = LanguageDetectorBuilder::from_languages(LANGUES);
        builder.with_preloaded_language_models();
        builder.build()
    })
}

/// ISO 639-3 → 639-1 for the languages we can detect, so a BCP 47 tag whose
/// primary subtag is three letters still compares with lingua's output.
const ISO3_VERS_ISO1: &[(&str, &str)] = &[
    ("ara", "ar"),
    ("eus", "eu"),
    ("nob", "nb"),
    ("bul", "bg"),
    ("cat", "ca"),
    ("zho", "zh"),
    ("hrv", "hr"),
    ("ces", "cs"),
    ("dan", "da"),
    ("nld", "nl"),
    ("eng", "en"),
    ("est", "et"),
    ("fin", "fi"),
    ("fra", "fr"),
    ("deu", "de"),
    ("ell", "el"),
    ("hun", "hu"),
    ("isl", "is"),
    ("gle", "ga"),
    ("ita", "it"),
    ("lav", "lv"),
    ("lit", "lt"),
    ("nno", "nn"),
    ("pol", "pl"),
    ("por", "pt"),
    ("ron", "ro"),
    ("rus", "ru"),
    ("slk", "sk"),
    ("slv", "sl"),
    ("spa", "es"),
    ("swe", "sv"),
    ("tur", "tr"),
    ("ukr", "uk"),
];

/// Reduces a BCP 47 tag to its lowercase primary subtag.
#[must_use]
pub fn normaliser_lang(tag: &str) -> Option<String> {
    let primaire = tag
        .trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    if primaire.is_empty() {
        return None;
    }
    let code = ISO3_VERS_ISO1
        .iter()
        .find(|(iso3, _)| *iso3 == primaire)
        .map_or(primaire.as_str(), |(_, iso1)| iso1);
    Some(code.to_owned())
}

/// Default exclusions: proper nouns, brands, acronyms and loanwords (spec §2.5).
#[must_use]
pub fn exclusions_par_defaut() -> Vec<String> {
    [
        "week-end",
        "jogging",
        "shopping",
        "design",
        "lorem ipsum",
        "rgaa",
        "html",
        "css",
        "javascript",
        "faq",
        "ceo",
        "nft",
        "covid",
    ]
    .iter()
    .map(|terme| (*terme).to_owned())
    .collect()
}

fn sans_exclusions(texte: &str, exclusions: &[String]) -> String {
    let mut nettoye = texte.to_lowercase();
    for terme in exclusions {
        let terme = terme.trim().to_lowercase();
        if !terme.is_empty() {
            nettoye = nettoye.replace(&terme, " ");
        }
    }
    nettoye
}

fn mots(texte: &str) -> usize {
    texte
        .split_whitespace()
        .filter(|mot| mot.chars().any(char::is_alphabetic))
        .count()
}

/// True for blocks made of excluded terms, uppercase or digits (spec §2.5).
#[must_use]
pub fn est_ignorable(texte: &str, exclusions: &[String]) -> bool {
    let sans_termes = sans_exclusions(texte, exclusions);
    let minimum = if sans_termes == texte.to_lowercase() {
        MOTS_DIVERGENCE
    } else {
        MOTS_MINIMUM
    };
    if mots(&sans_termes) < minimum {
        return true;
    }
    let lettres = texte.chars().filter(|c| c.is_alphabetic()).count().max(1);
    let majuscules = texte.chars().filter(|c| c.is_uppercase()).count();
    let total = texte.chars().count().max(1);
    let chiffres = texte.chars().filter(|c| c.is_numeric()).count();
    majuscules * 2 > lettres || chiffres * 3 > total
}

/// Best language and its confidence for one text.
///
/// The confidence is the *margin* `top / (top + runner-up)`, not lingua's raw
/// probability: the latter is diluted as the number of linked languages grows.
/// See [`CONFIANCE_BLOC`] for the measured values behind the 0.9 threshold.
#[must_use]
pub fn detecte(texte: &str) -> Option<(String, f64)> {
    let valeurs = detecteur().compute_language_confidence_values(texte);
    let (langue, meilleure) = valeurs.first()?;
    let deuxieme = valeurs.get(1).map_or(0.0, |(_, confiance)| *confiance);
    let total = *meilleure + deuxieme;
    let confiance = if total > 0.0 {
        (*meilleure / total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some((langue.iso_code_639_1().to_string(), confiance))
}

/// A block whose detected language contradicts its inherited `lang`.
#[derive(Debug, Clone)]
pub struct Divergence {
    pub mesure: BlocDivergent,
    pub xpath: String,
    pub explicite: bool,
    pub confiance: f64,
}

/// Result of one page's analysis, before the optional LLM filter is applied.
#[derive(Debug, Clone)]
pub struct DetectionPage {
    pub declaree: Option<String>,
    pub detectee: Option<String>,
    pub confiance: f64,
    pub divergences: Vec<Divergence>,
}

/// Detects the page language and every divergent block.
#[must_use]
pub fn detecte_page(
    blocs: &[Bloc],
    declaree_brute: Option<&str>,
    exclusions: &[String],
) -> DetectionPage {
    let declaree = declaree_brute.and_then(normaliser_lang);
    let mut voix: IndexMap<String, (usize, f64)> = IndexMap::new();
    let mut divergences = Vec::new();

    for bloc in blocs {
        if est_ignorable(&bloc.texte, exclusions) {
            continue;
        }
        let Some((langue, confiance)) = detecte(&bloc.texte) else {
            continue;
        };

        let poids = bloc.texte.chars().count();
        let entree = voix.entry(langue.clone()).or_insert((0, 0.0));
        entree.0 += poids;
        entree.1 += confiance * poids as f64;

        let herite = bloc
            .lang
            .as_deref()
            .and_then(normaliser_lang)
            .or_else(|| declaree.clone());
        if let Some(herite) = herite {
            if langue != herite
                && confiance > CONFIANCE_BLOC
                && mots(&sans_exclusions(&bloc.texte, exclusions)) >= MOTS_DIVERGENCE
            {
                divergences.push(Divergence {
                    mesure: BlocDivergent {
                        selecteur: bloc.selecteur.clone(),
                        lang_herite: herite,
                        detectee: langue,
                        confiance,
                        extrait: bloc.texte.clone(),
                    },
                    xpath: bloc.xpath.clone(),
                    explicite: bloc.lang_explicite,
                    confiance,
                });
            }
        }
    }

    let gagnant = voix.iter().max_by_key(|(_, (poids, _))| *poids).map(
        |(langue, (poids, somme_confiance))| (langue.clone(), somme_confiance / *poids as f64),
    );
    let (detectee, confiance) = match gagnant {
        Some((langue, confiance)) => (Some(langue), confiance),
        None => (None, 0.0),
    };

    DetectionPage {
        declaree,
        detectee,
        confiance,
        divergences,
    }
}

/// Whether the page-level mismatch crosses the 8.4 threshold.
#[must_use]
pub fn page_suspecte(detection: &DetectionPage) -> bool {
    match (&detection.declaree, &detection.detectee) {
        (Some(declaree), Some(detectee)) => {
            declaree != detectee && detection.confiance > CONFIANCE_PAGE
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bcp47_tag_is_reduced_to_its_primary_subtag() {
        assert_eq!(normaliser_lang("fr-CA").as_deref(), Some("fr"));
        assert_eq!(normaliser_lang("en_US").as_deref(), Some("en"));
        assert_eq!(normaliser_lang("fra").as_deref(), Some("fr"));
    }

    #[test]
    fn the_spec_loanword_phrase_is_excluded() {
        let exclusions = exclusions_par_defaut();
        assert!(est_ignorable(
            "Le week-end, je fais du jogging",
            &exclusions
        ));
    }

    #[test]
    fn an_all_caps_block_is_excluded() {
        assert!(est_ignorable("RGAA 4.1.2", &exclusions_par_defaut()));
    }

    /// The margin, not the raw probability, is what clears the 0.9 threshold.
    #[test]
    fn short_quote_margin_clears_the_threshold() {
        let (langue, marge) = detecte("Stay hungry, stay foolish").expect("detection");
        assert_eq!(langue, "en");
        assert!((marge - 0.9509).abs() < 0.001, "{marge}");
        assert!(marge > CONFIANCE_BLOC);
    }

    #[test]
    fn a_french_sentence_is_detected_as_french() {
        let (langue, confiance) =
            detecte("La langue par défaut de la page doit être pertinente au regard du contenu.")
                .expect("detection");
        assert_eq!(langue, "fr");
        assert!(confiance > 0.9, "{confiance}");
    }
}
