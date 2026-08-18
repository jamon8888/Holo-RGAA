use rgaa_obscura::{
    EvidenceArtifact, EvidenceStore, GuidedExecutor, GuidedObservation, GuidedRunResult,
    GuidedStep, GuidedTest, TerminationReason,
};
use rgaa_obscura::{GuidedAction, ObscuraError};
use std::collections::VecDeque;
use std::path::PathBuf;

struct FakeExecutor {
    calls: Vec<GuidedAction>,
    results: VecDeque<Result<GuidedObservation, ObscuraError>>,
}

impl FakeExecutor {
    fn new(results: impl IntoIterator<Item = Result<GuidedObservation, ObscuraError>>) -> Self {
        Self {
            calls: Vec::new(),
            results: results.into_iter().collect(),
        }
    }
}

impl GuidedExecutor for FakeExecutor {
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError> {
        self.calls.push(action.clone());
        self.results
            .pop_front()
            .unwrap_or_else(|| Ok(GuidedObservation::default()))
    }
}

fn test_case(steps: Vec<GuidedStep>) -> GuidedTest {
    GuidedTest {
        id: "keyboard-flow".into(),
        version: 1,
        preconditions: vec!["page loaded".into()],
        steps,
        criterion_mapping: vec!["7.3".into()],
        evidence_requirements: vec!["tree".into(), "screenshot".into()],
    }
}

#[tokio::test]
async fn mutating_actions_are_followed_by_observation() {
    let mut executor = FakeExecutor::new([
        Ok(GuidedObservation::default()),
        Ok(GuidedObservation::tree(["dialog-close"])),
        Ok(GuidedObservation::default()),
    ]);
    let test = test_case(vec![
        GuidedStep::PressKey { key: "Tab".into() },
        GuidedStep::AccessibilityTree,
        GuidedStep::Screenshot,
    ]);

    let result = test.run(&mut executor, None).await.expect("run succeeds");

    assert!(result.is_pass());
    assert_eq!(
        executor.calls[0],
        GuidedAction::PressKey { key: "Tab".into() }
    );
    assert_eq!(executor.calls[1], GuidedAction::AccessibilityTree);
    assert_eq!(result.completed_steps, 3);
    assert_eq!(result.criterion_mapping, vec!["7.3"]);
    assert_eq!(result.action_trace.len(), 3);
}

#[tokio::test]
async fn missing_reference_is_incomplete_and_records_unanalyzed_target() {
    let mut executor = FakeExecutor::new([Err(ObscuraError::Evaluation(
        "missing element reference: save-button".into(),
    ))]);
    let test = test_case(vec![
        GuidedStep::ClickRef {
            reference: "save-button".into(),
        },
        GuidedStep::AccessibilityTree,
    ]);

    let result = test
        .run(&mut executor, None)
        .await
        .expect("run returns envelope");

    assert!(!result.is_pass());
    assert_eq!(
        result.terminated_reason,
        TerminationReason::MissingReference
    );
    assert_eq!(result.unanalyzed_elements, vec!["save-button"]);
    assert!(result.manual_review_required);
}

#[tokio::test]
async fn assertion_failure_cannot_serialize_as_pass() {
    let mut executor = FakeExecutor::new([
        Ok(GuidedObservation::default()),
        Err(ObscuraError::Evaluation("assertion failed".into())),
    ]);
    let test = test_case(vec![
        GuidedStep::FillRef {
            reference: "name".into(),
            value: "Ada".into(),
        },
        GuidedStep::AssertState {
            expected: serde_json::json!({"name": "Ada"}),
        },
    ]);

    let result = test
        .run(&mut executor, None)
        .await
        .expect("run returns envelope");

    assert!(!result.is_pass());
    assert_eq!(result.terminated_reason, TerminationReason::AssertionFailed);
    assert!(!serde_json::to_string(&result)
        .expect("result serializes")
        .contains("\"status\":\"pass\""));
}

#[tokio::test]
async fn keyboard_trap_and_timeout_are_incomplete() {
    for error in [
        ObscuraError::Evaluation("keyboard trap detected".into()),
        ObscuraError::Timeout("step timed out".into()),
    ] {
        let mut executor = if matches!(error, ObscuraError::Timeout(_)) {
            FakeExecutor::new([Err(error.clone()), Err(error.clone()), Err(error)])
        } else {
            FakeExecutor::new([Err(error)])
        };
        let test = test_case(vec![
            GuidedStep::PressKey { key: "Tab".into() },
            GuidedStep::AccessibilityTree,
        ]);
        let result = test
            .run(&mut executor, None)
            .await
            .expect("run returns envelope");
        assert!(!result.is_pass());
        assert!(matches!(
            result.terminated_reason,
            TerminationReason::KeyboardTrap | TerminationReason::Timeout
        ));
    }
}

#[test]
fn evidence_store_writes_content_hashes_and_replays_deterministically() {
    let root = std::env::temp_dir().join(format!("rgaa-evidence-{}", uuid::Uuid::new_v4()));
    let store = EvidenceStore::new(root.clone());
    let artifact = EvidenceArtifact::new("screenshot", b"png-bytes".to_vec());

    let first = store.write(artifact.clone()).expect("write evidence");
    let second = store.write(artifact).expect("replay evidence");

    assert_eq!(first.sha256, second.sha256);
    assert_eq!(first.path, second.path);
    assert!(PathBuf::from(&first.path).exists());
    assert!(first.sha256.starts_with("sha256:"));
    std::fs::remove_dir_all(root).expect("remove test evidence");
}

#[test]
fn guided_result_defaults_to_incomplete_without_completion() {
    let result = GuidedRunResult::default();
    assert!(!result.is_pass());
}
