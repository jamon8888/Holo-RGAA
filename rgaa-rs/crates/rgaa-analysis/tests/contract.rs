//! P-00 acceptance criteria exercised at the contract boundary.
//!
//! Expected values (the echo proof's fields, the rejected field names) come
//! from the spec `docs/specs/plugins-analyse/00-contrat-plugin.md`, not from
//! the implementation.

use rgaa_analysis::models;
use rgaa_analysis::sidecar;
use rgaa_analysis::{echo_plugin, run_plugin, Portee, Preuve, Registry, Subject};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("examples")
        .canonicalize()
        .expect("examples directory")
}

#[test]
fn preuve_schema_never_declares_a_status_or_verdict() {
    let schema = serde_json::to_value(schemars::schema_for!(Preuve)).expect("schema");
    let text = schema.to_string();
    assert!(!text.contains("\"status\""), "R-P1: no status in Preuve");
    assert!(!text.contains("\"verdict\""), "R-P1: no verdict in Preuve");
}

struct SecondEcho;

#[async_trait::async_trait]
impl rgaa_analysis::AnalysisPlugin for SecondEcho {
    fn id(&self) -> &'static str {
        "second-echo"
    }
    fn version(&self) -> &'static str {
        "0.1.0"
    }
    fn tests(&self) -> &'static [&'static str] {
        &["1.1.1"]
    }
    async fn analyze(
        &self,
        _page: &rgaa_analysis::PageCapture<'_>,
        _options: &rgaa_analysis::PluginOptions,
    ) -> rgaa_analysis::Result<Vec<Preuve>> {
        Ok(Vec::new())
    }
}

#[test]
fn registry_is_in_kebab_case_order_and_reports_shared_tests() {
    let registry = Registry::builder()
        .with(echo_plugin())
        .with(Box::new(SecondEcho))
        .build()
        .expect("registry");
    let ids: Vec<&str> = registry.iter().map(|plugin| plugin.id()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "the registry must be deterministic");
    assert!(
        ids.contains(&"echo"),
        "the P-00 dummy plugin must be registered"
    );

    let duplicates = registry.duplicate_tests();
    assert_eq!(
        duplicates.get("1.1.1"),
        Some(&vec!["echo".to_owned(), "second-echo".to_owned()]),
        "a test served by two plugins must be reported"
    );
}

#[tokio::test]
async fn native_echo_produces_the_canonical_proof() {
    let echo = echo_plugin();
    let options = serde_json::json!({ "message": "bonjour" });
    let proofs = run_plugin(echo.as_ref(), "https://example.test", &options)
        .await
        .expect("echo proof");
    assert_eq!(proofs.len(), 1);
    let proof = &proofs[0];
    assert_eq!(proof.plugin, "echo");
    assert_eq!(proof.tests, vec!["1.1.1".to_string()]);
    assert_eq!(proof.portee, Portee::Mesuree);
    assert_eq!(proof.artefacts.len(), 1);
    assert_eq!(proof.artefacts[0].nom, "echo.txt");
    assert!(!proof.empreinte_entree.is_empty());
}

#[test]
fn a_verdict_field_is_rejected_before_deserialization() {
    let mut value = serde_json::json!({
        "plugin": "echo",
        "version_plugin": "0.1.0",
        "version_modele": null,
        "tests": ["1.1.1"],
        "sujet": "Page",
        "portee": "Mesuree",
        "mesures": {},
        "indice": null,
        "artefacts": [],
        "empreinte_entree": "abc"
    });
    assert!(sidecar::validate_preuve(&value).is_ok());
    value["verdict"] = serde_json::json!("fail");
    let error = sidecar::validate_preuve(&value).expect_err("R-P1: verdict must be rejected");
    assert!(error.contains("verdict"), "{error}");
}

#[test]
fn a_missing_model_names_the_pull_command() {
    let spec = rgaa_analysis::ModelSpec {
        nom: "absent-model.bin",
        url: "https://example.test/absent-model.bin",
        sha256: Box::leak("0".repeat(64).into_boxed_str()),
        taille_octets: 1,
        licence: "Apache-2.0",
    };
    let dir = tempfile::tempdir().expect("temp dir");
    let error = models::ensure_present_in("images", &spec, dir.path())
        .expect_err("a missing model must be an explicit error");
    let message = error.to_string();
    assert!(message.contains("modèle absent"), "{message}");
    assert!(message.contains("rgaa models pull images"), "{message}");
}

#[tokio::test]
async fn sidecar_echo_matches_the_native_echo() {
    let dir = examples_dir().join("sidecar-echo");
    let plugins = sidecar::discover(&dir);
    assert_eq!(plugins.len(), 1, "the example manifest must be discovered");
    let options = serde_json::json!({ "message": "bonjour" });
    let from_sidecar = run_plugin(&plugins[0], "https://example.test", &options)
        .await
        .expect("sidecar proof");
    let echo = echo_plugin();
    let from_native = run_plugin(echo.as_ref(), "https://example.test", &options)
        .await
        .expect("native proof");
    assert_eq!(from_sidecar, from_native);
}

#[tokio::test]
async fn a_sleeping_sidecar_is_killed_and_reports_a_missing_proof() {
    let dir = examples_dir().join("sidecar-sleep");
    let plugins = sidecar::discover(&dir);
    assert_eq!(plugins.len(), 1, "the sleeping manifest must be discovered");
    let error = run_plugin(&plugins[0], "https://example.test", &serde_json::json!({}))
        .await
        .expect_err("timeout must be reported");
    let message = error.to_string();
    assert!(message.contains("preuve manquante"), "{message}");
    assert!(message.contains("sleep"), "{message}");
}

#[tokio::test]
async fn a_sidecar_returning_a_verdict_is_rejected() {
    let dir = examples_dir().join("sidecar-verdict");
    let plugins = sidecar::discover(&dir);
    assert_eq!(plugins.len(), 1, "the verdict manifest must be discovered");
    let error = run_plugin(&plugins[0], "https://example.test", &serde_json::json!({}))
        .await
        .expect_err("verdict must be rejected");
    assert!(error.to_string().contains("verdict"), "{error}");
}

#[test]
fn schemas_export_the_proof_and_each_mesures_shape() {
    let document = rgaa_analysis::schema::schemas().expect("schemas");
    assert!(document.get("preuve").is_some(), "Preuve schema missing");
    assert!(
        document["mesures"].get("echo").is_some(),
        "echo mesures schema missing"
    );
    let text = document.to_string();
    assert!(!text.contains("\"status\""));
    assert!(!text.contains("\"verdict\""));
}

#[test]
fn subject_is_serialized_as_a_tagged_enum() {
    let subject = Subject::Page;
    assert_eq!(
        serde_json::to_value(&subject).expect("json"),
        serde_json::json!("Page")
    );
}
