//! Similarités lexicales hors ligne (signaux estimés, jamais un verdict).
//!
//! `strsim` fournit Jaro-Winkler et Levenshtein ; le cosinus de jetons apporte
//! la couverture du contenu. Ces mesures sont reproductibles et ne téléchargent
//! aucun modèle (R-P3). Un moteur de plongements (fastembed 7.7,
//! `multilingual-e5-small`) peut être branché plus tard derrière la même
//! interface de signaux sans changer la forme des preuves.

use crate::snapshot::Champ;

/// Intitulés génériques FR/EN de la spec §3 (6.1).
pub const INTITULE_GENERIQUE: &[&str] = &[
    "cliquez ici",
    "clique ici",
    "en savoir plus",
    "lire la suite",
    "ici",
    "more",
    "click here",
    "learn more",
    "read more",
];

/// Titres génériques de cadre de la spec §3 (2.2).
pub const TITRE_CADRE_GENERIQUE: &[&str] = &["iframe", "frame", "widget"];

/// Messages d'erreur génériques de la spec §3 (11.11).
pub const MESSAGE_GENERIQUE: &[&str] = &[
    "champ invalide",
    "champ obligatoire",
    "invalid field",
    "erreur de saisie",
    "input error",
];

/// Normalise pour la comparaison : minuscules, accents retirés, ponctuation
/// remplacée par des espaces. Utilise la même table que la liste d'actions
/// partagée, sans en dépendre (le contrat ne connaît pas `strsim`).
#[must_use]
pub fn normaliser(texte: &str) -> String {
    let sans_accents: String = texte
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'œ' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            '’' | 'ʼ' => '\'',
            _ => c,
        })
        .collect();
    sans_accents
        .split(|c: char| !c.is_alphanumeric())
        .filter(|mot| !mot.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mots d'un texte normalisé.
#[must_use]
pub fn jetons(texte: &str) -> Vec<String> {
    normaliser(texte)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Similarité combinée Jaro-Winkler / cosinus de jetons, dans `[0, 1]`.
#[must_use]
pub fn similarite(gauche: &str, droite: &str) -> f64 {
    let a = normaliser(gauche);
    let b = normaliser(droite);
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let jaro = strsim::jaro_winkler(&a, &b);
    let cosinus = cosinus_jetons(&jetons(&a), &jetons(&b));
    ((jaro + cosinus) / 2.0).clamp(0.0, 1.0)
}

/// Jaro-Winkler seul, pour `similarite_h1` de 8.6.
#[must_use]
pub fn jaro_winkler(gauche: &str, droite: &str) -> f64 {
    strsim::jaro_winkler(&normaliser(gauche), &normaliser(droite)).clamp(0.0, 1.0)
}

/// Distance de Levenshtein normalisée, utilisée pour les libellés proches.
#[must_use]
pub fn distance_levenshtein(gauche: &str, droite: &str) -> f64 {
    let a = normaliser(gauche);
    let b = normaliser(droite);
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let max = a.chars().count().max(b.chars().count());
    if max == 0 {
        return 0.0;
    }
    let distance = strsim::levenshtein(&a, &b);
    1.0 - (distance as f64 / max as f64)
}

/// Part des jetons de `original` retrouvés dans `candidate` (13.4, P-04 §3.4).
#[must_use]
pub fn couverture(original: &str, candidate: &str) -> f64 {
    let a = jetons(original);
    let b = jetons(candidate);
    if a.is_empty() {
        return 0.0;
    }
    let presents = a
        .iter()
        .filter(|jeton| b.iter().any(|autre| autre == *jeton))
        .count();
    presents as f64 / a.len() as f64
}

fn cosinus_jetons(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut communs = 0usize;
    for jeton in a {
        if b.iter().any(|autre| autre == jeton) {
            communs += 1;
        }
    }
    // Cosinus multiplicatif sur des vecteurs de présence : une mesure bornée,
    // sans dépendance de plus (les jetons répétés n'apportent rien à la
    // couverture sémantique ici).
    communs as f64 / ((a.len() * b.len()) as f64).sqrt()
}

/// Vrai si `libelle` est exactement un intitulé générique connu.
#[must_use]
pub fn est_generique(libelle: &str, liste: &[&str]) -> bool {
    let normalise = normaliser(libelle);
    liste.iter().any(|mot| normalise == normaliser(mot))
}

/// Vrai si le texte ne contient aucun caractère alphanumérique (« * », icône).
#[must_use]
pub fn est_symbole_seul(texte: &str) -> bool {
    !texte.is_empty() && !texte.chars().any(char::is_alphanumeric)
}

/// Vrai si le titre ressemble à un nom de fichier plutôt qu'à un titre de page.
#[must_use]
pub fn est_nom_de_fichier(titre: &str) -> bool {
    let bas = titre.trim().to_lowercase();
    // L'extension se lit sur le texte brut : `normaliser` remplace le point
    // par un espace et détruirait précisément le signal recherché.
    if let Some((racine, extension)) = bas.rsplit_once('.') {
        if !racine.is_empty()
            && (2..=5).contains(&extension.len())
            && extension
                .chars()
                .all(|caractere| caractere.is_ascii_alphanumeric())
        {
            return true;
        }
    }
    let normalise = normaliser(&bas);
    [
        "untitled",
        "sans titre",
        "index",
        "document",
        "img",
        "image",
    ]
    .iter()
    .any(|mot| normalise == *mot)
}

/// Vrai si le message d'erreur est générique ou ne propose pas de correction.
#[must_use]
pub fn est_message_generique(message: &str) -> bool {
    let normalise = normaliser(message);
    if normalise.is_empty() {
        return false;
    }
    if MESSAGE_GENERIQUE
        .iter()
        .any(|modele| normalise == normaliser(modele))
    {
        return true;
    }
    // « sans suggestion » : aucun verbe d'invitation, ni format attendu.
    ![
        "veuillez",
        "doit",
        "format",
        "exemple",
        "please",
        "expected",
        "doit etre",
    ]
    .iter()
    .any(|indice| normalise.contains(indice))
}

/// Classe d'`autocomplete` attendue, déduite du libellé par un lexique FR/EN.
#[must_use]
pub fn finalite_attendue(champ: &Champ) -> Option<&'static str> {
    libelle_vers_finalite(&champ.etiquette)
}

/// Variante publique pour tester le lexique sans construire un champ.
#[must_use]
pub fn libelle_vers_finalite(libelle: &str) -> Option<&'static str> {
    let tokens = jetons(libelle);
    let contient = |mots: &[&str]| {
        mots.iter().all(|mot| {
            let attendu = normaliser(mot);
            tokens.iter().any(|jeton| jeton.as_str() == attendu)
        })
    };
    if contient(&["courriel"]) || contient(&["e mail"]) || contient(&["email"]) {
        Some("email")
    } else if contient(&["telephone"]) || contient(&["tel"]) || contient(&["phone"]) {
        Some("tel")
    } else if contient(&["code", "postal"]) || contient(&["zip", "code"]) {
        Some("postal-code")
    } else if contient(&["mot", "passe"]) || contient(&["password"]) {
        Some("new-password")
    } else if contient(&["prenom"]) || contient(&["first", "name"]) {
        Some("given-name")
    } else if contient(&["adresse"]) || contient(&["address"]) {
        Some("street-address")
    } else if contient(&["nom"]) || contient(&["name"]) {
        Some("name")
    } else {
        None
    }
}

/// Compare un `autocomplete` présent à la classe attendue.
#[must_use]
pub fn autocomplete_ecart(champ: &Champ) -> Option<&'static str> {
    let attendu = finalite_attendue(champ)?;
    let present = champ.autocomplete.as_deref()?;
    let present_normalise = normaliser(present);
    if present_normalise == normaliser(attendu) {
        None
    } else {
        Some(attendu)
    }
}

/// Extrait tronqué à `max` caractères, sans couper un caractère UTF-8.
#[must_use]
pub fn tronquer(texte: &str, max: usize) -> String {
    texte.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_valeurs_oracle_ne_sont_pas_recalculees() {
        assert!(est_generique("En savoir plus", INTITULE_GENERIQUE));
        assert!(!est_generique(
            "En savoir plus sur nos offres",
            INTITULE_GENERIQUE
        ));
        assert!((similarite("Accueil", "Accueil") - 1.0).abs() < 1e-9);
        assert!((couverture("un deux trois", "un deux") - 2.0 / 3.0).abs() < 1e-9);
        assert!(est_nom_de_fichier("Accueil.pdf"));
        assert!(!est_nom_de_fichier("Notre organisation"));
        assert!(est_symbole_seul("*"));
        assert!(!est_symbole_seul("Courriel"));
        assert!(est_message_generique("Champ invalide"));
        assert!(!est_message_generique(
            "Veuillez saisir une date au format AAAA-MM-JJ"
        ));
    }

    #[test]
    fn le_lexique_de_finalite_reconnait_courriel() {
        let champ = Champ {
            etiquette: "Courriel".to_owned(),
            autocomplete: Some("tel".to_owned()),
            ..Champ::default()
        };
        assert_eq!(finalite_attendue(&champ), Some("email"));
        assert_eq!(autocomplete_ecart(&champ), Some("email"));
    }
}
