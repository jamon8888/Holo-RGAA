use rgaa_agent::{AgentConfig, RgaaAgent};
use rgaa_core::RgaaCriteria;
use rgaa_holo::prompts::HeadingInfo;
use rgaa_holo::PageContext;
use rgaa_test_corpus::{EvaluationManifest, ExpectedVerdict};
use std::collections::BTreeMap;
use std::path::Path;

/// This evaluation can spend provider credits and is always ignored by ordinary
/// test runs. Even when explicitly unignored, it requires an additional opt-in.
#[tokio::test]
#[ignore = "live provider evaluation requires RGAA_RUN_LIVE_VERDICT_EVAL=1"]
async fn live_verdict_evaluation_reports_false_pass_and_false_fail_by_family() {
    if std::env::var("RGAA_RUN_LIVE_VERDICT_EVAL").as_deref() != Ok("1") {
        eprintln!("live RGAA evaluation skipped: set RGAA_RUN_LIVE_VERDICT_EVAL=1 to opt in");
        return;
    }
    let api_key = std::env::var("MYIA_API_KEY")
        .or_else(|_| std::env::var("RGAA_LLM_API_KEY"))
        .expect("live RGAA evaluation requires MYIA_API_KEY or RGAA_LLM_API_KEY");
    assert!(!api_key.trim().is_empty(), "API key must not be empty");

    let criteria_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rgaa-test-corpus/criteria");
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/verdict-evaluation.json");
    let raw = std::fs::read_to_string(manifest_path).expect("evaluation manifest is readable");
    let manifest =
        EvaluationManifest::from_json(&raw, &criteria_dir).expect("evaluation manifest validates");
    let mut config = AgentConfig::from_env().expect("agent configuration is valid");
    config.api_key = api_key;
    let agent = RgaaAgent::new(&config).await.expect("agent initializes");

    #[derive(Default)]
    struct Counts {
        false_pass: usize,
        false_fail: usize,
        evaluated: usize,
        unresolved: usize,
    }
    let mut counts = BTreeMap::<String, Counts>::new();
    for case in &manifest.cases {
        let fixture_path = criteria_dir.join(&case.fixture);
        let html = std::fs::read_to_string(&fixture_path).expect("fixture is readable");
        let context = PageContext {
            title: Some(case.fixture.clone()),
            lang: Some("fr".into()),
            headings: vec![HeadingInfo {
                level: 1,
                text: format!("Fixture HTML source (untrusted):\n{html}"),
            }],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        };
        let criterion = RgaaCriteria::find(&case.criterion_id)
            .expect("validated case criterion exists")
            .clone();
        let results = agent
            .run_automatic_estimates(std::slice::from_ref(&criterion), &context, &[])
            .await;
        let family = case
            .criterion_id
            .split('.')
            .next()
            .unwrap_or("unknown")
            .to_owned();
        let family_counts = counts.entry(family).or_default();
        let result = results.get(&case.criterion_id);
        let predicted = result.and_then(|result| {
            result
                .tests
                .iter()
                .find(|test| test.test_key == case.test_key)
                .map(|test| test.status.clone())
        });
        let Some(predicted) = predicted else {
            family_counts.unresolved += 1;
            continue;
        };
        family_counts.evaluated += 1;
        match (case.expected_verdict, predicted) {
            (ExpectedVerdict::Fail, rgaa_core::CriterionStatus::Pass) => {
                family_counts.false_pass += 1;
            }
            (ExpectedVerdict::Pass, rgaa_core::CriterionStatus::Fail) => {
                family_counts.false_fail += 1;
            }
            _ => {}
        }
    }

    for (family, result) in counts {
        println!(
            "family={family} evaluated={} unresolved={} false_pass={} false_fail={}",
            result.evaluated, result.unresolved, result.false_pass, result.false_fail
        );
    }
}
