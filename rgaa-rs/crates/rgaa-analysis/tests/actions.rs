//! Règle F-SEC / commentaire du coordinateur sur la PR #470 : la liste des
//! actions interdites est partagée entre P-07 (actions simulées) et P-08
//! (soumissions de formulaires), et ne vit dans aucun plugin.

use rgaa_analysis::actions::{
    est_action_interdite, FORBIDDEN_ACTION_WORDS, INTERDITES_EN, INTERDITES_FR,
};

#[test]
fn la_liste_partagee_contient_les_actions_du_commentaire() {
    for mot in [
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
    ] {
        assert!(
            FORBIDDEN_ACTION_WORDS.contains(&mot),
            "action interdite absente de la liste partagée : {mot}"
        );
    }
    assert!(INTERDITES_FR.contains(&"commander"));
    assert!(INTERDITES_EN.contains(&"subscribe"));
}

#[test]
fn détecte_les_actions_interdites_quelque_soit_la_casse_et_la_typographie() {
    for texte in [
        "Payer maintenant",
        "COMMANDER",
        "Supprimer mon compte",
        "S’abonner à la newsletter",
        "Je veux valider ma commande",
        "Checkout",
        "Unsubscribe",
        "Réserver une table",
        "Effectuer un virement",
    ] {
        assert!(est_action_interdite(texte), "action non détectée : {texte}");
    }
}

#[test]
fn ne_detecte_pas_de_faux_positifs_sur_des_mots_proches() {
    for texte in [
        " afficher les détails ",
        "paiement",
        "commande",
        "annuler la recherche",
        "les conditions générales",
    ] {
        assert!(!est_action_interdite(texte), "faux positif sur {texte}");
    }
}

#[test]
fn la_détection_ne_depend_pas_des_accents() {
    assert!(est_action_interdite("Réserver"));
    assert!(est_action_interdite("reserver"));
    assert!(est_action_interdite("résilier"));
    assert!(est_action_interdite("resilier"));
}
