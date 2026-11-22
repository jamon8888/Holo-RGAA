//! Corrélation action ↔ parole et construction des preuves P-00 §5.
//!
//! Chaque énoncé est associé à la fenêtre `[t, t + fenetre_ms]` de son action.
//! Pour une activation on joint l'état annoncé avant et après (§4.2–4.3 de la
//! recherche), sans jamais poser de statut ni de verdict (R-P1).

use chrono::{DateTime, SecondsFormat, Utc};
use rgaa_analysis::{Artefact, Indice, Portee, Preuve, Rect, Subject};
use serde_json::{json, Value};

/// Action de parcours, avec son instant mesuré par le pilote.
#[derive(Debug, Clone, PartialEq)]
pub struct InstantAction {
    pub id: String,
    pub type_action: String,
    pub selecteur: Option<String>,
    pub declencheur: Option<String>,
    pub nom_accessible_attendu: Option<String>,
    pub fenetre_ms: u64,
    pub t: Option<DateTime<Utc>>,
    pub xpath: Option<String>,
    pub bbox: Option<Rect>,
}

/// Énoncé lu dans le journal et horodaté à la réception.
#[derive(Debug, Clone, PartialEq)]
pub struct Enonce {
    pub texte: String,
    pub horodatage: DateTime<Utc>,
}

/// Entrée pure de la corrélation : aucune IO, aucune horloge.
pub struct Correlation<'a> {
    pub actions: &'a [InstantAction],
    pub enonces: &'a [Enonce],
    pub environnement: Option<&'a str>,
    pub sujets: &'a [Value],
    /// Contenu NDJSON de l'artefact `parole.jsonl`.
    pub parole_jsonl: &'a str,
    /// Empreinte de l'entrée analysée (journal + instants d'actions).
    pub empreinte_entree: &'a str,
}

pub(crate) fn horodatage(instant: &DateTime<Utc>) -> String {
    instant.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Construit les preuves de restitution. Une activation sans énoncé dans sa
/// fenêtre ne produit aucune preuve : le test restera en « preuve manquante ».
#[must_use]
pub fn preuves(correlation: &Correlation<'_>) -> Vec<Preuve> {
    let sha256 = rgaa_analysis::sha256_hex(correlation.parole_jsonl.as_bytes());
    let artefact = Artefact {
        nom: "parole.jsonl".into(),
        media_type: "application/x-ndjson".into(),
        chemin_bundle: format!("analysis/screenreader-nvda/{sha256}.jsonl"),
        sha256,
    };
    let mut resultats = Vec::new();

    for action in correlation
        .actions
        .iter()
        .filter(|action| action.type_action == "activation")
    {
        let Some(debut) = action.t else {
            tracing::warn!(action = %action.id, "action sans instant : ignorée");
            continue;
        };
        let millisecondes = i64::try_from(action.fenetre_ms).unwrap_or(i64::MAX);
        let Some(fin) = debut.checked_add_signed(chrono::Duration::milliseconds(millisecondes))
        else {
            tracing::warn!(action = %action.id, "fenêtre d'action débordante : ignorée");
            continue;
        };
        let Some(apres) = correlation
            .enonces
            .iter()
            .find(|enonce| enonce.horodatage >= debut && enonce.horodatage <= fin)
        else {
            tracing::warn!(action = %action.id, "aucun énoncé dans la fenêtre");
            continue;
        };
        let avant = correlation
            .enonces
            .iter()
            .filter(|enonce| enonce.horodatage < debut)
            .max_by_key(|enonce| enonce.horodatage.timestamp_millis());

        let mesures = json!({
            "action": {
                "id": action.id,
                "type": action.type_action,
                "declencheur": action.declencheur,
            },
            "element": {
                "selecteur": action.selecteur,
                "nom_accessible_attendu": action.nom_accessible_attendu,
            },
            "texte_prononce": apres.texte,
            "horodatage": horodatage(&apres.horodatage),
            "horodatage_type": "reception",
            "avant": avant.map(|enonce| json!({
                "texte_prononce": enonce.texte,
                "horodatage": horodatage(&enonce.horodatage),
            })).unwrap_or(Value::Null),
            "apres": {
                "texte_prononce": apres.texte,
                "horodatage": horodatage(&apres.horodatage),
            },
            "source": "nvda-speech-logger",
            "environnement": correlation.environnement,
        });

        resultats.push(Preuve {
            plugin: "screenreader-nvda".into(),
            version_plugin: "0.1.0".into(),
            version_modele: None,
            tests: vec!["7.1.1".into(), "7.5.1".into()],
            sujet: sujet_pour(action, correlation.sujets),
            portee: Portee::Mesuree,
            mesures,
            indice: Some(Indice::Suspect {
                raison: "état d'expansion non exposé dans l'AX tree (P-07)".into(),
            }),
            artefacts: vec![artefact.clone()],
            empreinte_entree: correlation.empreinte_entree.to_owned(),
        });
    }
    resultats
}

/// Sujet Élément repris de la demande `analyser`, ou `Page` à défaut.
fn sujet_pour(action: &InstantAction, sujets: &[Value]) -> Subject {
    let Some(selecteur) = action.selecteur.as_deref() else {
        return Subject::Page;
    };
    let Some(sujet) = sujets
        .iter()
        .find(|sujet| sujet.get("selecteur").and_then(Value::as_str) == Some(selecteur))
    else {
        return Subject::Page;
    };
    let xpath = sujet
        .get("xpath")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let bbox = sujet
        .get("bbox")
        .cloned()
        .and_then(|valeur| serde_json::from_value::<Rect>(valeur).ok())
        .unwrap_or_default();
    Subject::Element {
        selecteur: selecteur.to_owned(),
        xpath,
        bbox,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(valeur: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(valeur)
            .expect("instant RFC 3339 valide")
            .with_timezone(&Utc)
    }

    // Valeurs attendues : recherche §4.2 (exemple de résultat) et §1.2 pour la
    // source `nvda-speech-logger`. Elles ne sont pas recalculées depuis le code.
    fn correlation_avec_enonces(enonces: Vec<Enonce>) -> Correlation<'static> {
        // `Box::leak` est réservé aux fixtures de test : les durées de vie
        // statiques simplifient la construction des valeurs attendues.
        let actions: &'static [InstantAction] = Box::leak(
            vec![
                InstantAction {
                    id: "a1".into(),
                    type_action: "focus".into(),
                    selecteur: Some("#menu-button".into()),
                    declencheur: None,
                    nom_accessible_attendu: None,
                    fenetre_ms: 1500,
                    t: Some(instant("2026-11-22T09:14:00.204Z")),
                    xpath: None,
                    bbox: None,
                },
                InstantAction {
                    id: "a2".into(),
                    type_action: "activation".into(),
                    selecteur: Some("#menu-button".into()),
                    declencheur: Some("Enter".into()),
                    nom_accessible_attendu: Some("menu".into()),
                    fenetre_ms: 2500,
                    t: Some(instant("2026-11-22T09:14:02.120Z")),
                    xpath: None,
                    bbox: None,
                },
            ]
            .into_boxed_slice(),
        );
        let enonces: &'static [Enonce] = Box::leak(enonces.into_boxed_slice());
        let sujets: &'static [Value] = Box::leak(
            vec![json!({
                "selecteur": "#menu-button",
                "xpath": "/html/body/button[1]",
                "bbox": {"x": 120.0, "y": 210.0, "width": 90.0, "height": 32.0}
            })]
            .into_boxed_slice(),
        );
        Correlation {
            actions,
            enonces,
            environnement: Some("NVDA 2026.3.1 + Firefox 141.0"),
            sujets,
            parole_jsonl: "{\"texte\":\"menu, bouton, coché\"}\n",
            empreinte_entree: "sha256:abc",
        }
    }

    fn enonces_acceptation() -> Vec<Enonce> {
        vec![
            Enonce {
                texte: "menu, bouton, pas coché".into(),
                horodatage: instant("2026-11-22T09:14:00.500Z"),
            },
            Enonce {
                texte: "menu, bouton, coché".into(),
                horodatage: instant("2026-11-22T09:14:02.481Z"),
            },
        ]
    }

    #[test]
    fn lactivation_joint_leteat_avant_et_apres() {
        let resultats = preuves(&correlation_avec_enonces(enonces_acceptation()));
        assert_eq!(resultats.len(), 1, "une activation = une preuve 7.1");
        let preuve = &resultats[0];
        assert_eq!(preuve.plugin, "screenreader-nvda");
        assert_eq!(preuve.version_plugin, "0.1.0");
        assert_eq!(preuve.tests, vec!["7.1.1", "7.5.1"]);
        assert_eq!(preuve.portee, Portee::Mesuree);
        assert!(matches!(
            preuve.sujet,
            Subject::Element { ref selecteur, .. } if selecteur == "#menu-button"
        ));
        assert_eq!(preuve.mesures["action"]["id"], "a2");
        assert_eq!(preuve.mesures["action"]["type"], "activation");
        assert_eq!(preuve.mesures["action"]["declencheur"], "Enter");
        assert_eq!(preuve.mesures["element"]["selecteur"], "#menu-button");
        assert_eq!(preuve.mesures["element"]["nom_accessible_attendu"], "menu");
        assert_eq!(preuve.mesures["texte_prononce"], "menu, bouton, coché");
        assert_eq!(preuve.mesures["horodatage"], "2026-11-22T09:14:02.481Z");
        assert_eq!(preuve.mesures["horodatage_type"], "reception");
        assert_eq!(
            preuve.mesures["avant"]["texte_prononce"],
            "menu, bouton, pas coché"
        );
        assert_eq!(
            preuve.mesures["avant"]["horodatage"],
            "2026-11-22T09:14:00.500Z"
        );
        assert_eq!(
            preuve.mesures["apres"]["texte_prononce"],
            "menu, bouton, coché"
        );
        assert_eq!(
            preuve.mesures["apres"]["horodatage"],
            "2026-11-22T09:14:02.481Z"
        );
        assert_eq!(preuve.mesures["source"], "nvda-speech-logger");
        assert_eq!(
            preuve.mesures["environnement"],
            "NVDA 2026.3.1 + Firefox 141.0"
        );
    }

    #[test]
    fn la_preuve_ne_porte_jamais_de_statut_ni_de_verdict() {
        let resultats = preuves(&correlation_avec_enonces(enonces_acceptation()));
        let valeur = serde_json::to_value(&resultats[0]).expect("preuve JSON");
        let texte = valeur.to_string();
        assert!(!texte.contains("\"status\""), "R-P1");
        assert!(!texte.contains("\"verdict\""), "R-P1");
    }

    #[test]
    fn la_preuve_joint_lartefact_parole_avec_son_sha256() {
        let resultats = preuves(&correlation_avec_enonces(enonces_acceptation()));
        let preuve = &resultats[0];
        assert_eq!(preuve.empreinte_entree, "sha256:abc");
        assert_eq!(preuve.artefacts.len(), 1);
        let artefact = &preuve.artefacts[0];
        assert_eq!(artefact.nom, "parole.jsonl");
        assert_eq!(artefact.media_type, "application/x-ndjson");
        assert!(artefact
            .chemin_bundle
            .starts_with("analysis/screenreader-nvda/"));
        assert!(artefact.chemin_bundle.ends_with(".jsonl"));
        assert_eq!(
            artefact.sha256,
            rgaa_analysis::sha256_hex(preuve_artefact_attendu().as_bytes())
        );
    }

    fn preuve_artefact_attendu() -> &'static str {
        "{\"texte\":\"menu, bouton, coché\"}\n"
    }

    #[test]
    fn un_enonce_hors_fenetre_ne_produit_pas_de_preuve() {
        let hors_fenetre = vec![Enonce {
            texte: "menu, bouton, coché".into(),
            // 09:14:10 > 09:14:02.120 + 2500 ms : en dehors de la fenêtre.
            horodatage: instant("2026-11-22T09:14:10.000Z"),
        }];
        assert!(preuves(&correlation_avec_enonces(hors_fenetre)).is_empty());
    }

    #[test]
    fn un_indice_de_revue_peut_etre_joint_sans_devenir_un_verdict() {
        let resultats = preuves(&correlation_avec_enonces(enonces_acceptation()));
        assert!(matches!(resultats[0].indice, Some(Indice::Suspect { .. })));
    }

    #[test]
    fn un_artefact_est_toujours_cite_meme_sans_sujet_correspondant() {
        let mut correlation = correlation_avec_enonces(enonces_acceptation());
        correlation.sujets = &[];
        let resultats = preuves(&correlation);
        assert_eq!(resultats.len(), 1);
        assert!(matches!(resultats[0].sujet, Subject::Page));
        assert_eq!(resultats[0].artefacts.len(), 1);
    }
}
