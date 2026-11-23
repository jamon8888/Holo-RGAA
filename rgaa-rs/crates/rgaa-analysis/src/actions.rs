//! Liste partagée des actions qu'un plugin ne doit jamais effectuer (F-SEC,
//! commentaire du coordinateur sur la PR #470).
//!
//! P-07 (actions simulées sur les widgets) et P-08 (soumissions de
//! formulaires de 11.10/11.11/11.12) lisent **la même** liste : elle vit ici,
//! dans le contrat, et non dans un crate de plugin. Une action reconnue est
//! observée, jamais exécutée : aucun plugin n'a le droit de payer, commander,
//! confirmer ou envoyer quoi que ce soit à la place d'un être humain (R1–R7).

/// Actions interdites en français (formes canoniques du commentaire PR #470).
pub const INTERDITES_FR: &[&str] = &[
    "soumettre",
    "payer",
    "supprimer",
    "commander",
    "acheter",
    "valider",
    "envoyer",
    "confirmer",
    "s'abonner",
    "se désabonner",
    "réserver",
    "transférer",
    "virement",
    "résilier",
];

/// Actions interdites en anglais (mêmes intentions).
pub const INTERDITES_EN: &[&str] = &[
    "submit",
    "pay",
    "delete",
    "order",
    "buy",
    "checkout",
    "confirm",
    "send",
    "subscribe",
    "unsubscribe",
    "book",
    "transfer",
    "cancel",
];

/// Liste complète, dans un ordre stable et déterministe.
///
/// Les deux vues [`INTERDITES_FR`] et [`INTERDITES_EN`] sont redondantes avec
/// cette liste ; un test vérifie qu'aucune entrée ne dérive.
pub const FORBIDDEN_ACTION_WORDS: &[&str] = &[
    "soumettre",
    "payer",
    "supprimer",
    "commander",
    "acheter",
    "valider",
    "envoyer",
    "confirmer",
    "s'abonner",
    "se désabonner",
    "réserver",
    "transférer",
    "virement",
    "résilier",
    "submit",
    "pay",
    "delete",
    "order",
    "buy",
    "checkout",
    "confirm",
    "send",
    "subscribe",
    "unsubscribe",
    "book",
    "transfer",
    "cancel",
];

/// Retourne l'action interdite reconnue dans `texte`, ou `None`.
///
/// La comparaison porte sur des suites de mots normalisés (casse, accents et
/// apostrophes ignorés), afin que « S’abonner » et `s'abonner` donnent le même
/// résultat sans confondre « payer » avec « paiement » ou « commander » avec
/// « commande ».
#[must_use]
pub fn action_interdite(texte: &str) -> Option<&'static str> {
    let tokens = tokeniser(texte);
    FORBIDDEN_ACTION_WORDS.iter().copied().find(|mot| {
        let cibles = tokeniser(mot);
        !cibles.is_empty()
            && cibles.len() <= tokens.len()
            && tokens
                .windows(cibles.len())
                .any(|fenetre| fenetre == cibles.as_slice())
    })
}

/// Vrai si `texte` contient une action interdite.
#[must_use]
pub fn est_action_interdite(texte: &str) -> bool {
    action_interdite(texte).is_some()
}

/// Découpe en mots minuscules sans accents, séparés aux caractères non
/// alphanumériques (les apostrophes comprise : « s'abonner » → `s`, `abonner`).
fn tokeniser(texte: &str) -> Vec<String> {
    texte
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            _ => c,
        })
        .flat_map(|c| c.to_lowercase())
        .collect::<String>()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|mot| !mot.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partage_les_deux_listes_sans_derive() {
        assert_eq!(
            FORBIDDEN_ACTION_WORDS.len(),
            INTERDITES_FR.len() + INTERDITES_EN.len()
        );
        for mot in INTERDITES_FR.iter().chain(INTERDITES_EN) {
            assert!(
                FORBIDDEN_ACTION_WORDS.contains(mot),
                "entrée en dérive : {mot}"
            );
        }
    }

    #[test]
    fn retourne_le_mot_reconnu() {
        assert_eq!(action_interdite("Valider le panier"), Some("valider"));
        assert_eq!(action_interdite("Acheter ce billet"), Some("acheter"));
    }
}
