//! Cas oracle de la spec P-08 §6. Les valeurs attendues viennent du texte de
//! la spec et des listes RGAA, jamais d'un recalcul par le code testé.

use std::sync::{Arc, Mutex};

use rgaa_analysis::{
    AnalysisPlugin, DomSnapshot, NoopResourceFetcher, PageCapture, PluginOptions, Preuve, Result,
};
use rgaa_analysis_text::{
    llm::{LlmRequest, LlmResponse, ModelClient},
    mesures::{Mesures, Pertinence},
    snapshot::{
        Alternative, Bouton, Cadre, Champ, Formulaire, Lien, PageText, PlanSite, Tableau,
        TextSnapshot, VersionAlternative,
    },
    TextPlugin,
};

#[derive(Default)]
struct RecordingClient {
    appels: Mutex<Vec<LlmRequest>>,
    repondre: bool,
    mauvaise_version: bool,
}

impl ModelClient for RecordingClient {
    fn complete(&self, requete: &LlmRequest) -> Result<Option<LlmResponse>> {
        self.appels
            .lock()
            .expect("mutex test")
            .push(requete.clone());
        if !self.repondre {
            return Ok(None);
        }
        let mut reponses = Vec::new();
        for element in &requete.elements {
            reponses.push(rgaa_analysis_text::llm::ReponseElement {
                id: element.id.clone(),
                pertinent: Pertinence::Non,
                explication: "intitulé générique sans contexte de liste".to_owned(),
            });
        }
        Ok(Some(LlmResponse {
            schema_version: if self.mauvaise_version {
                99
            } else {
                requete.schema_version
            },
            prompt_sha256: requete.prompt_sha256.clone(),
            reponses,
        }))
    }
}

#[derive(Default)]
struct CountingProbe {
    observations: Mutex<usize>,
}

impl rgaa_analysis_text::FormProbe for CountingProbe {
    fn observe(&self, formulaire: &Formulaire) -> Result<rgaa_analysis_text::FormObservation> {
        *self.observations.lock().expect("mutex sonde") += 1;
        Ok(rgaa_analysis_text::FormObservation {
            messages_erreur: vec![format!("Erreur sur {}", formulaire.id)],
            recapitulatif_detecte: Some(false),
        })
    }
}

fn page<'a>(dom: &'a DomSnapshot, resources: &'a NoopResourceFetcher) -> PageCapture<'a> {
    PageCapture {
        url: "https://example.test/",
        dom_snapshot: dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources,
        lang_declared: Some("fr"),
    }
}

fn analyser_avec(snapshot: &TextSnapshot, plugin: &TextPlugin) -> Vec<Preuve> {
    let dom = DomSnapshot::new(serde_json::to_string(snapshot).expect("snapshot sérialisable"));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, &resources);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime de test");
    runtime
        .block_on(plugin.analyze(&capture, &PluginOptions::default()))
        .expect("analyse")
}

fn analyser(snapshot: &TextSnapshot, client: Arc<RecordingClient>) -> Vec<Preuve> {
    analyser_avec(snapshot, &TextPlugin::with_client(client))
}

fn mesures(prouve: &Preuve) -> Mesures {
    serde_json::from_value(prouve.mesures.clone()).expect("mesures typées")
}

fn liens(snapshot: &TextSnapshot) -> Vec<&Lien> {
    snapshot.page.liens.iter().collect()
}

#[test]
fn oracle_6_1_sept_liens_identiques_vers_sept_cibles_un_seul_appel_llm() {
    let page = PageText {
        url: "https://example.test/".to_owned(),
        titre: "Offres".to_owned(),
        liens: (1..=7)
            .map(|n| Lien {
                id: format!("l{n}"),
                intitule: "En savoir plus".to_owned(),
                cible: format!("https://example.test/offres/{n}"),
                titre_section: "Pied de page".to_owned(),
                ..Lien::default()
            })
            .collect(),
        ..PageText::default()
    };
    let snapshot = TextSnapshot {
        page,
        ..TextSnapshot::default()
    };
    let client = Arc::new(RecordingClient::default());
    let preuves = analyser(&snapshot, client.clone());

    let generiques: Vec<_> = preuves
        .iter()
        .filter(|preuve| preuve.tests.iter().any(|test| test == "6.1.1"))
        .collect();
    assert_eq!(generiques.len(), 7, "une preuve par lien");
    for preuve in &generiques {
        assert_eq!(preuve.portee, rgaa_analysis::Portee::Mesuree);
        match mesures(preuve) {
            Mesures::Lien(mesure) => {
                assert!(mesure.generique, "intitulé générique attendu");
                assert_eq!(mesure.meme_intitule_cibles_differentes, 7);
            }
            autre => panic!("mesure inattendue : {autre:?}"),
        }
    }
    let lots_6_1: Vec<_> = client
        .appels
        .lock()
        .expect("mutex")
        .iter()
        .filter(|appel| appel.critere == "6.1")
        .cloned()
        .collect();
    assert_eq!(
        lots_6_1.len(),
        1,
        "les 7 liens sont regroupés en un seul appel 6.1"
    );
    assert_eq!(
        lots_6_1[0].elements.len(),
        7,
        "les 7 éléments restent présents dans le lot"
    );
    assert_eq!(liens(&snapshot).len(), 7);
}

#[test]
fn oracle_8_6_titre_repete_sur_douze_pages() {
    let pages = (1..=12)
        .map(|n| PageText {
            url: format!("https://example.test/page-{n}"),
            titre: "Accueil".to_owned(),
            h1: format!("Page {n}"),
            ..PageText::default()
        })
        .collect();
    let snapshot = TextSnapshot {
        page: PageText {
            url: "https://example.test/page-1".to_owned(),
            titre: "Accueil".to_owned(),
            ..PageText::default()
        },
        pages,
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    let titres: Vec<_> = preuves
        .iter()
        .filter(|preuve| preuve.tests.iter().any(|test| test == "8.6.1"))
        .collect();
    assert_eq!(titres.len(), 12);
    for preuve in &titres {
        assert_eq!(preuve.portee, rgaa_analysis::Portee::Mesuree);
        match mesures(preuve) {
            Mesures::TitrePage(mesure) => assert_eq!(mesure.nombre_pages_identiques, 12),
            autre => panic!("mesure inattendue : {autre:?}"),
        }
    }
}

#[test]
fn oracle_11_13_courriel_avec_autocomplete_tel() {
    let snapshot = TextSnapshot {
        page: PageText {
            champs: vec![Champ {
                id: "courriel".to_owned(),
                etiquette: "Courriel".to_owned(),
                autocomplete: Some("tel".to_owned()),
                ..Champ::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    let champ = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "11.13.1"))
        .expect("preuve 11.13");
    match mesures(champ) {
        Mesures::Champ(mesure) => {
            assert!(mesure.ecart_finalite, "écart de finalité attendu");
            assert_eq!(mesure.autocomplete_attendu.as_deref(), Some("email"));
            assert_eq!(mesure.autocomplete.as_deref(), Some("tel"));
        }
        autre => panic!("mesure inattendue : {autre:?}"),
    }
}

#[test]
fn oracle_11_10_11_11_messages_captures_sans_aucune_soumission() {
    let snapshot = TextSnapshot {
        page: PageText {
            champs: vec![Champ {
                id: "courriel".to_owned(),
                etiquette: "Courriel".to_owned(),
                requis: true,
                format_attendu: Some("AAAA-MM-JJ".to_owned()),
                ..Champ::default()
            }],
            formulaires: vec![Formulaire {
                id: "inscription".to_owned(),
                intitule: "Inscription".to_owned(),
                messages_erreur: vec![
                    "Champ invalide".to_owned(),
                    "Veuillez saisir une date".to_owned(),
                ],
                ..Formulaire::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    for test in ["11.10.1", "11.11.1"] {
        assert!(
            preuves
                .iter()
                .any(|preuve| preuve.tests.iter().any(|t| t == test)),
            "preuve attendue pour {test}"
        );
    }
    for preuve in &preuves {
        if let Mesures::Formulaire(mesure) = mesures(preuve) {
            assert!(
                !mesure.soumission_effectuee,
                "P-08 ne soumet jamais de formulaire"
            );
            assert!(mesure.messages_erreur.len() >= 2);
        }
    }
}

#[test]
fn oracle_11_12_bouton_payer_aucune_soumission_etre_sensible() {
    let snapshot = TextSnapshot {
        page: PageText {
            boutons: vec![Bouton {
                id: "payer".to_owned(),
                nom: "Payer".to_owned(),
                formulaire: Some("commande".to_owned()),
                ..Bouton::default()
            }],
            formulaires: vec![Formulaire {
                id: "commande".to_owned(),
                intitule: "Payer la commande".to_owned(),
                ..Formulaire::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    let sensible = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "11.12.1"))
        .expect("preuve 11.12");
    match mesures(sensible) {
        Mesures::Formulaire(mesure) => {
            assert!(mesure.sensible, "formulaire sensible détecté");
            assert!(!mesure.soumission_effectuee, "aucune soumission");
            assert_eq!(mesure.recapitulatif_observe, None);
        }
        autre => panic!("mesure inattendue : {autre:?}"),
    }
}

#[test]
fn oracle_2_2_cadre_sans_titre_est_mesure() {
    let snapshot = TextSnapshot {
        page: PageText {
            cadres: vec![Cadre {
                id: "frame".to_owned(),
                titre: String::new(),
                texte_document: "Formulaire de contact".to_owned(),
                ..Cadre::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    let cadre = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "2.2.1"))
        .expect("preuve 2.2");
    assert_eq!(cadre.portee, rgaa_analysis::Portee::Mesuree);
    match mesures(cadre) {
        Mesures::Cadre(mesure) => assert!(mesure.titre_generique),
        autre => panic!("mesure inattendue : {autre:?}"),
    }
}

#[test]
fn oracle_5_2_5_5_tableau_complexe_sans_resume_ni_caption() {
    let snapshot = TextSnapshot {
        page: PageText {
            tableaux: vec![Tableau {
                id: "t1".to_owned(),
                complexe: true,
                ..Tableau::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    for test in ["5.2.1", "5.5.1"] {
        let preuve = preuves
            .iter()
            .find(|preuve| preuve.tests.iter().any(|t| t == test))
            .expect("preuve tableau");
        assert_eq!(preuve.portee, rgaa_analysis::Portee::Mesuree);
    }
}

#[test]
fn oracle_12_3_plan_absent_puis_couverture_estimee() {
    let sans_plan = TextSnapshot::default();
    let preuves = analyser(&sans_plan, Arc::new(RecordingClient::default()));
    let plan = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "12.3.1"))
        .expect("preuve 12.3");
    assert_eq!(plan.portee, rgaa_analysis::Portee::Mesuree);
    match mesures(plan) {
        Mesures::PlanSite(mesure) => assert!(!mesure.present),
        autre => panic!("mesure inattendue : {autre:?}"),
    }

    let avec_plan = TextSnapshot {
        plan_du_site: Some(PlanSite {
            url: "https://example.test/plan".to_owned(),
            liens: vec!["https://example.test/a".to_owned()],
        }),
        urls_spider: vec![
            "https://example.test/a".to_owned(),
            "https://example.test/b".to_owned(),
        ],
        ..TextSnapshot::default()
    };
    let preuves = analyser(&avec_plan, Arc::new(RecordingClient::default()));
    let plan = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "12.3.1"))
        .expect("preuve 12.3");
    match mesures(plan) {
        Mesures::PlanSite(mesure) => {
            assert!(mesure.present);
            assert!((mesure.couverture - 0.5).abs() < 1e-9);
            assert_eq!(mesure.urls_manquantes, vec!["https://example.test/b"]);
        }
        autre => panic!("mesure inattendue : {autre:?}"),
    }
}

#[test]
fn oracle_13_4_similarite_et_couverture_de_la_version_alternative() {
    let snapshot = TextSnapshot {
        page: PageText {
            versions: vec![VersionAlternative {
                id: "v1".to_owned(),
                original: "Le rapport annuel présente les résultats financiers".to_owned(),
                alternative: "Le rapport annuel présente les résultats financiers".to_owned(),
                titre: "Rapport annuel".to_owned(),
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    let version = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "13.4.1"))
        .expect("preuve 13.4");
    assert_eq!(version.portee, rgaa_analysis::Portee::Estimee);
    match mesures(version) {
        Mesures::VersionAlternative(mesure) => {
            assert!((mesure.similarite - 1.0).abs() < 1e-9);
            assert!((mesure.couverture - 1.0).abs() < 1e-9);
        }
        autre => panic!("mesure inattendue : {autre:?}"),
    }
}

#[test]
fn oracle_1_5_7_2_13_6_alternative_absente_est_mesuree_presente_est_groupee() {
    let snapshot = TextSnapshot {
        page: PageText {
            alternatives: vec![
                Alternative {
                    id: "captcha".to_owned(),
                    critere: "1.5".to_owned(),
                    present: false,
                    intitule: "CAPTCHA".to_owned(),
                    contexte: "Formulaire de contact".to_owned(),
                },
                Alternative {
                    id: "script".to_owned(),
                    critere: "7.2".to_owned(),
                    present: true,
                    intitule: "Menu dynamique".to_owned(),
                    contexte: "Navigation".to_owned(),
                },
                Alternative {
                    id: "cryptique".to_owned(),
                    critere: "13.6".to_owned(),
                    present: true,
                    intitule: "Abréviations techniques".to_owned(),
                    contexte: "Article".to_owned(),
                },
            ],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, Arc::new(RecordingClient::default()));
    for test in ["1.5.1", "7.2.1", "13.6.1"] {
        assert!(
            preuves
                .iter()
                .any(|preuve| preuve.tests.iter().any(|t| t == test)),
            "preuve attendue pour {test}"
        );
    }
    let captcha = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "1.5.1"))
        .expect("preuve 1.5");
    assert_eq!(captcha.portee, rgaa_analysis::Portee::Mesuree);
}

#[test]
fn lot_et_reponse_sont_versionnes_et_l_avis_modele_ne_jamais_le_statut() {
    let client = Arc::new(RecordingClient {
        repondre: true,
        ..RecordingClient::default()
    });
    let snapshot = TextSnapshot {
        page: PageText {
            liens: vec![Lien {
                id: "l1".to_owned(),
                intitule: "En savoir plus".to_owned(),
                cible: "/offres".to_owned(),
                ..Lien::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, client.clone());
    let appel = &client.appels.lock().expect("mutex")[0];
    assert_eq!(appel.schema_version, 1);
    assert_eq!(appel.modele.reasoning_effort, "low");
    assert_eq!(appel.prompt_id, "pertinence-textuelle-v1");
    assert_eq!(appel.prompt_sha256.len(), 64, "empreinte sha-256 du prompt");
    let preuve = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "6.1.1"))
        .expect("preuve 6.1");
    assert!(
        preuve
            .version_modele
            .as_deref()
            .expect("version du modèle")
            .contains(&appel.prompt_sha256),
        "l'empreinte du prompt figure dans version_modele"
    );
    match mesures(preuve) {
        Mesures::Lien(mesure) => {
            let avis = mesure.avis_modele.expect("avis du modèle");
            assert_eq!(avis.pertinent, Pertinence::Non);
            assert_eq!(
                avis.explication,
                "intitulé générique sans contexte de liste"
            );
        }
        autre => panic!("mesure inattendue : {autre:?}"),
    }
    let json = serde_json::to_string(&preuves).expect("preuves sérialisables");
    assert!(!json.contains("\"status\""));
    assert!(!json.contains("\"verdict\""));
}

#[test]
fn cache_evite_un_second_appel_pour_le_meme_groupe() {
    let client = Arc::new(RecordingClient::default());
    let dom = DomSnapshot::new(
        serde_json::to_string(&TextSnapshot {
            page: PageText {
                liens: vec![Lien {
                    id: "l1".to_owned(),
                    intitule: "En savoir plus".to_owned(),
                    cible: "/offres".to_owned(),
                    ..Lien::default()
                }],
                ..PageText::default()
            },
            ..TextSnapshot::default()
        })
        .expect("snapshot"),
    );
    let resources = NoopResourceFetcher;
    let capture = page(&dom, &resources);
    let plugin = TextPlugin::with_client(client.clone());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime
        .block_on(plugin.analyze(&capture, &PluginOptions::default()))
        .expect("premier passage");
    runtime
        .block_on(plugin.analyze(&capture, &PluginOptions::default()))
        .expect("second passage");
    assert_eq!(client.appels.lock().expect("mutex").len(), 1);
}

#[test]
fn snapshot_illisible_est_une_erreur_domaine_sans_panic() {
    let dom = DomSnapshot::new("ceci n'est pas du JSON");
    let resources = NoopResourceFetcher;
    let capture = page(&dom, &resources);
    let plugin = TextPlugin::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let erreur = runtime
        .block_on(plugin.analyze(&capture, &PluginOptions::default()))
        .expect_err("snapshot illisible");
    assert!(erreur.to_string().contains("text"));
}

#[test]
fn lots_sont_decoupes_a_quarante_elements_et_un_seul_critere_par_requete() {
    let liens = (0..45)
        .map(|n| Lien {
            id: format!("l{n}"),
            intitule: format!("Lien {n}"),
            cible: format!("/cible/{n}"),
            extrait: format!("contexte {n}"),
            ..Lien::default()
        })
        .collect();
    let snapshot = TextSnapshot {
        page: PageText {
            url: "https://example.test/".to_owned(),
            titre: String::new(),
            liens,
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let client = Arc::new(RecordingClient::default());
    analyser(&snapshot, client.clone());
    let appels: Vec<_> = client.appels.lock().expect("mutex").clone();
    let lots_6_1: Vec<_> = appels
        .iter()
        .filter(|appel| appel.critere == "6.1")
        .collect();
    assert_eq!(lots_6_1.len(), 2, "40 puis 5 éléments");
    assert_eq!(lots_6_1[0].elements.len(), 40);
    assert_eq!(lots_6_1[1].elements.len(), 5);
    assert!(appels.iter().all(|appel| appel.schema_version == 1));
}

#[test]
fn une_reponse_d_un_autre_prompt_est_ignoree_sans_polluer_la_preuve() {
    let client = Arc::new(RecordingClient {
        repondre: true,
        mauvaise_version: true,
        ..RecordingClient::default()
    });
    let snapshot = TextSnapshot {
        page: PageText {
            liens: vec![Lien {
                id: "l1".to_owned(),
                intitule: "En savoir plus".to_owned(),
                cible: "/offres".to_owned(),
                ..Lien::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser(&snapshot, client);
    let preuve = preuves
        .iter()
        .find(|preuve| preuve.tests.iter().any(|test| test == "6.1.1"))
        .expect("preuve 6.1");
    match mesures(preuve) {
        Mesures::Lien(mesure) => assert!(mesure.avis_modele.is_none()),
        autre => panic!("mesure inattendue : {autre:?}"),
    }
}

#[test]
fn sonde_de_formulaire_observe_sans_soumettre_et_fournit_les_messages() {
    let sonde = Arc::new(CountingProbe::default());
    let plugin = TextPlugin::with_form_probe(sonde.clone());
    let snapshot = TextSnapshot {
        page: PageText {
            formulaires: vec![Formulaire {
                id: "inscription".to_owned(),
                intitule: "Inscription".to_owned(),
                ..Formulaire::default()
            }],
            ..PageText::default()
        },
        ..TextSnapshot::default()
    };
    let preuves = analyser_avec(&snapshot, &plugin);
    assert_eq!(*sonde.observations.lock().expect("mutex sonde"), 1);
    for preuve in &preuves {
        if let Mesures::Formulaire(mesure) = mesures(preuve) {
            assert!(!mesure.soumission_effectuee);
            assert_eq!(mesure.recapitulatif_observe, Some(false));
            assert!(mesure
                .messages_erreur
                .contains(&"Erreur sur inscription".to_owned()));
        }
    }
}
