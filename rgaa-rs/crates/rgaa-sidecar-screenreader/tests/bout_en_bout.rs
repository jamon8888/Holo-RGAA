//! Acceptation simulée du ticket #437, sans NVDA : le binaire est lancé par
//! le client sidecar de `rgaa-analysis` (P-00 §7 bis) avec des fixtures
//! écrites à la main d'après le format de la recherche §1.2 et ses instants
//! §4.2. Aucune horloge réelle, aucun `sleep`.

use std::fs;
use std::path::Path;

use rgaa_analysis::{run_plugin, sidecar, AnalysisPlugin, Portee, Subject};
use serde_json::{json, Value};

const ENVIRONNEMENT: &str = "NVDA 2026.3.1 + Firefox 141.0";

/// Journal Speech Logger (recherche §1.2), bouton « menu » sans
/// `aria-expanded` : l'état est annoncé avant et après Entrée.
const JOURNAL: &str = "### Log started on 11/22/26 at 09:14:02 ###\n\
menu, bouton, pas coché\n\
menu, bouton, coché\n\
### Log ended on 11/22/26 at 09:14:31 ###\n";

const INSTANTS_ACTIONS: &str = "{\"id\":\"a1\",\"t\":\"2026-11-22T09:14:00.204Z\"}\n\
{\"id\":\"a2\",\"t\":\"2026-11-22T09:14:02.120Z\"}\n";

fn ecrire_manifest(dossier: &Path) {
    let binaire = env!("CARGO_BIN_EXE_rgaa-sidecar-screenreader");
    let manifeste = format!(
        "id = \"screenreader-nvda\"\n\
version = \"0.1.0\"\n\
tests = [\"7.1.1\", \"7.1.2\", \"7.1.3\", \"7.5.1\", \"7.5.2\", \"7.5.3\"]\n\
commande = [\"{binaire}\"]\n\
mode = \"persistant\"\n\
timeout_ms = 10000\n\
licence = \"GPL-2.0 (add-on Speech Logger) ; sidecar seul, aucun code NVDA lie\"\n"
    );
    fs::write(dossier.join("sidecar.toml"), manifeste).expect("manifeste");
}

fn parametres(dossier: &Path) -> Value {
    json!({
        "url": "https://exemple.fr/fixture-menu",
        "sujets": [{
            "selecteur": "#menu-button",
            "xpath": "/html/body/button[1]",
            "bbox": {"x": 120.0, "y": 210.0, "width": 90.0, "height": 32.0}
        }],
        "fichiers": {
            "parole": dossier.join("speech.log").to_str().expect("chemin UTF-8"),
            "actions": dossier.join("actions.jsonl").to_str().expect("chemin UTF-8")
        },
        "actions": [
            {"id": "a1", "type": "focus", "selecteur": "#menu-button", "fenetre_ms": 1500},
            {"id": "a2", "type": "activation", "selecteur": "#menu-button",
             "declencheur": "Enter", "fenetre_ms": 2500, "nom_accessible_attendu": "menu"}
        ],
        "options": {
            "environnement": ENVIRONNEMENT,
            "horodatage_simule": [
                "2026-11-22T09:14:00.500Z",
                "2026-11-22T09:14:02.481Z"
            ]
        }
    })
}

fn install_fixtures(dossier: &Path) {
    ecrire_manifest(dossier);
    fs::write(dossier.join("speech.log"), JOURNAL).expect("journal");
    fs::write(dossier.join("actions.jsonl"), INSTANTS_ACTIONS).expect("instants");
}

#[tokio::test]
async fn le_binaire_livree_la_preuve_71_avant_apres_via_le_client_sidecar() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    install_fixtures(dossier.path());

    let plugins = sidecar::discover(dossier.path());
    assert_eq!(plugins.len(), 1, "le manifeste doit être découvert");
    assert_eq!(plugins[0].id(), "screenreader-nvda");
    assert_eq!(plugins[0].kind(), rgaa_analysis::PluginKind::Sidecar);

    let plugin = plugins[0]
        .clone()
        .avec_parametres(parametres(dossier.path()));
    let preuves = run_plugin(&plugin, "https://exemple.fr/fixture-menu", &json!({}))
        .await
        .expect("preuve 7.1");

    assert_eq!(preuves.len(), 1, "une activation = une preuve");
    let preuve = &preuves[0];
    assert_eq!(preuve.plugin, "screenreader-nvda");
    assert_eq!(preuve.portee, Portee::Mesuree);
    assert!(preuve.tests.contains(&"7.1.1".to_owned()));
    assert!(matches!(
        &preuve.sujet,
        Subject::Element { selecteur, .. } if selecteur == "#menu-button"
    ));
    assert_eq!(preuve.mesures["action"]["id"], "a2");
    assert_eq!(preuve.mesures["action"]["declencheur"], "Enter");
    assert_eq!(preuve.mesures["texte_prononce"], "menu, bouton, coché");
    assert_eq!(preuve.mesures["horodatage_type"], "reception");
    assert_eq!(
        preuve.mesures["avant"]["texte_prononce"],
        "menu, bouton, pas coché"
    );
    assert_eq!(
        preuve.mesures["apres"]["texte_prononce"],
        "menu, bouton, coché"
    );
    assert_eq!(preuve.mesures["environnement"], ENVIRONNEMENT);
    let texte = serde_json::to_string(preuve).expect("preuve JSON");
    assert!(!texte.contains("\"status\""), "R-P1");
    assert!(!texte.contains("\"verdict\""), "R-P1");

    assert_eq!(preuve.artefacts.len(), 1);
    let artefact = &preuve.artefacts[0];
    assert_eq!(artefact.nom, "parole.jsonl");
    assert_eq!(
        artefact.sha256,
        rgaa_analysis::sha256_hex(JOURNAL_PAROLE_ATTENDUE.as_bytes())
    );
    let ecrit = std::env::temp_dir()
        .join("rgaa-screenreader")
        .join(format!("{}.jsonl", artefact.sha256));
    assert!(ecrit.is_file(), "l'artefact doit être matérialisé");
}

/// Contenu NDJSON attendu, calculé à partir du format documenté P-00 §5 et
/// des deux énoncés de la recherche — pas depuis notre sérialiseur.
const JOURNAL_PAROLE_ATTENDUE: &str = "{\"horodatage\":\"2026-11-22T09:14:00.500Z\",\"horodatage_type\":\"reception\",\"texte\":\"menu, bouton, pas coché\"}\n{\"horodatage\":\"2026-11-22T09:14:02.481Z\",\"horodatage_type\":\"reception\",\"texte\":\"menu, bouton, coché\"}\n";

#[tokio::test]
async fn la_preuve_du_binaire_ajoute_l_environnement_a_la_declaration() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    install_fixtures(dossier.path());
    let plugins = sidecar::discover(dossier.path());
    let plugin = plugins[0]
        .clone()
        .avec_parametres(parametres(dossier.path()));
    let preuves = run_plugin(&plugin, "https://exemple.fr/fixture-menu", &json!({}))
        .await
        .expect("preuve 7.1");

    // Chaîne complète : binaire → Preuve P-00 → déclaration #430. Le rendu
    // HTML lui-même est couvert par le test de rgaa-report.
    let environnements = rgaa_report::environnements_utilises_avec_preuves(None, &[], &preuves);
    assert_eq!(environnements.len(), 1);
    assert_eq!(
        environnements[0].technologie_assistance.as_deref(),
        Some("NVDA 2026.3.1")
    );
    assert_eq!(
        environnements[0].navigateur.as_deref(),
        Some("Firefox 141.0")
    );
}

#[tokio::test]
async fn un_fichier_de_parole_absent_donne_une_preuve_manquante_sans_panique() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    ecrire_manifest(dossier.path());
    fs::write(dossier.path().join("actions.jsonl"), INSTANTS_ACTIONS).expect("instants");
    let mut params = parametres(dossier.path());
    params["fichiers"]["parole"] =
        json!(dossier.path().join("absent.log").to_str().expect("chemin"));

    let plugins = sidecar::discover(dossier.path());
    let plugin = plugins[0].clone().avec_parametres(params);
    let erreur = run_plugin(&plugin, "https://exemple.fr/fixture-menu", &json!({}))
        .await
        .expect_err("fichier absent => preuve manquante");
    let message = erreur.to_string();
    assert!(message.contains("preuve manquante"), "{message}");
    assert!(message.contains("absent.log"), "{message}");
}

#[tokio::test]
async fn un_nvda_non_detecte_donne_une_preuve_manquante_sans_panique() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    install_fixtures(dossier.path());
    let mut params = parametres(dossier.path());
    params["options"]["nvda_detecte"] = json!(false);

    let plugins = sidecar::discover(dossier.path());
    let plugin = plugins[0].clone().avec_parametres(params);
    let erreur = run_plugin(&plugin, "https://exemple.fr/fixture-menu", &json!({}))
        .await
        .expect_err("NVDA non détecté => preuve manquante");
    assert!(erreur.to_string().contains("NVDA non détecté"), "{erreur}");
}

#[test]
fn le_manifeste_est_decouvert_via_rgaa_plugins_dir() {
    // Mécanisme de P-00 §7 bis : `$RGAA_PLUGINS_DIR` puis
    // `~/.config/rgaa/plugins`. On pose la variable dans un sous-processus
    // pour ne pas modifier l'environnement global des autres tests.
    if std::env::var_os("P10_ENFANT").is_some() {
        let plugins = sidecar::discover_default();
        assert!(
            plugins
                .iter()
                .any(|plugin| plugin.id() == "screenreader-nvda"),
            "le manifeste du sidecar doit être découvert"
        );
        return;
    }
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    install_fixtures(dossier.path());
    let exe = std::env::current_exe().expect("binaire de test");
    let sortie = std::process::Command::new(exe)
        .env("P10_ENFANT", "1")
        .env("RGAA_PLUGINS_DIR", dossier.path())
        .args(["--exact", "le_manifeste_est_decouvert_via_rgaa_plugins_dir"])
        .status()
        .expect("rejeu du test de découverte");
    assert!(
        sortie.success(),
        "la découverte par RGAA_PLUGINS_DIR échoue"
    );
}
