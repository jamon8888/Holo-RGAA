use rgaa_obscura::{
    is_stable_accessibility_reference, EvidenceArtifact, EvidenceStore, GuidedExecutor,
    GuidedObservation, GuidedRunResult, GuidedStep, GuidedTest, TerminationReason,
};
use rgaa_obscura::{GuidedAction, ObscuraError};
use std::collections::VecDeque;
use std::path::PathBuf;

struct FakeExecutor {
    calls: Vec<GuidedAction>,
    results: VecDeque<Result<GuidedObservation, ObscuraError>>,
}

struct StatefulExecutor {
    value: Option<String>,
}

impl GuidedExecutor for StatefulExecutor {
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError> {
        match action {
            GuidedAction::FillRef { value, .. } => {
                self.value = Some(value.clone());
                Ok(GuidedObservation::default())
            }
            GuidedAction::AssertState { .. } => Ok(GuidedObservation {
                state: Some(serde_json::json!({"value": self.value})),
                ..Default::default()
            }),
            _ => Ok(GuidedObservation::default()),
        }
    }
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

fn no_required_evidence(mut test: GuidedTest) -> GuidedTest {
    test.evidence_requirements.clear();
    test
}

#[tokio::test]
async fn mutating_actions_are_followed_by_observation() {
    let mut executor = FakeExecutor::new([
        Ok(GuidedObservation::default()),
        Ok(GuidedObservation::tree(["dialog-close"])),
        Ok(GuidedObservation::default()),
    ]);
    let test = no_required_evidence(test_case(vec![
        GuidedStep::PressKey { key: "Tab".into() },
        GuidedStep::AccessibilityTree,
        GuidedStep::Screenshot,
    ]));

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
    assert_eq!(
        result.unanalyzed_elements,
        vec![
            "save-button",
            "accessibility-tree",
            "evidence:tree",
            "evidence:screenshot"
        ]
    );
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
async fn observed_state_mismatch_preserves_assertion_failure() {
    let mut executor = FakeExecutor::new([Ok(GuidedObservation {
        state: Some(serde_json::json!({"actual": true})),
        ..Default::default()
    })]);
    let test = test_case(vec![GuidedStep::AssertState {
        expected: serde_json::json!({"expected": true}),
    }]);

    let result = test
        .run(&mut executor, None)
        .await
        .expect("run returns envelope");

    assert_eq!(result.terminated_reason, TerminationReason::AssertionFailed);
    assert!(!result.is_pass());
}

#[tokio::test]
async fn stateful_executor_preserves_typed_values_between_steps() {
    let mut executor = StatefulExecutor { value: None };
    let mut test = test_case(vec![
        GuidedStep::FillRef {
            reference: "ax:42".into(),
            value: "Ada".into(),
        },
        GuidedStep::AssertState {
            expected: serde_json::json!({"value": "Ada"}),
        },
    ]);
    test.evidence_requirements.clear();

    let result = test.run(&mut executor, None).await.expect("run succeeds");

    assert!(result.is_pass());
    assert_eq!(executor.value.as_deref(), Some("Ada"));
}

#[test]
fn accessibility_references_are_stable_backend_node_ids() {
    assert!(is_stable_accessibility_reference("ax:42"));
    assert!(is_stable_accessibility_reference(
        "ax-role=textbox;name=Name"
    ));
    assert!(!is_stable_accessibility_reference("#save-button"));
    assert!(!is_stable_accessibility_reference("button:3"));
}

#[tokio::test]
async fn invalid_action_ordering_is_incomplete() {
    let mut executor = FakeExecutor::new([]);
    let test = test_case(vec![GuidedStep::ClickRef {
        reference: "ax:42".into(),
    }]);

    let result = test
        .run(&mut executor, None)
        .await
        .expect("run returns envelope");

    assert_eq!(result.terminated_reason, TerminationReason::InvalidOrdering);
    assert!(!result.is_pass());
    assert!(executor.calls.is_empty());
}

#[tokio::test]
async fn failed_step_marks_all_remaining_steps_unanalyzed() {
    let mut executor =
        FakeExecutor::new([Err(ObscuraError::Navigation("navigation failed".into()))]);
    let test = test_case(vec![
        GuidedStep::Navigate {
            url: "https://example.test".into(),
        },
        GuidedStep::AccessibilityTree,
        GuidedStep::PressKey { key: "Tab".into() },
        GuidedStep::AccessibilityTree,
        GuidedStep::ClickRef {
            reference: "ax:42".into(),
        },
        GuidedStep::AccessibilityTree,
    ]);

    let result = test
        .run(&mut executor, None)
        .await
        .expect("run returns envelope");

    assert_eq!(result.terminated_reason, TerminationReason::NavigationError);
    assert_eq!(result.unanalyzed_elements.len(), 6);
}

#[test]
fn screenshot_evidence_requires_a_real_png_signature() {
    let root = std::env::temp_dir().join(format!("rgaa-png-{}", uuid::Uuid::new_v4()));
    let store = EvidenceStore::new(root.clone());
    let invalid = store.write(EvidenceArtifact::new("screenshot", b"html".to_vec()));
    assert!(matches!(invalid, Err(ObscuraError::Evidence(_))));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn required_evidence_is_part_of_pass_semantics() {
    let test = test_case(vec![]);
    let result = GuidedRunResult {
        terminated_reason: TerminationReason::Completed,
        criterion_mapping: test.criterion_mapping,
        evidence_requirements: test.evidence_requirements,
        ..Default::default()
    };
    assert!(!result.is_pass());
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
    let artifact = EvidenceArtifact::new(
        "screenshot",
        [vec![137, 80, 78, 71, 13, 10, 26, 10], b"png-bytes".to_vec()].concat(),
    );

    let first = store.write(artifact.clone()).expect("write evidence");
    let second = store.write(artifact).expect("replay evidence");

    assert_eq!(first.sha256, second.sha256);
    assert_eq!(first.path, second.path);
    assert!(PathBuf::from(&first.path).exists());
    assert!(first.sha256.starts_with("sha256:"));
    assert!(std::fs::read(&first.path)
        .expect("read screenshot")
        .starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
    std::fs::remove_dir_all(root).expect("remove test evidence");
}

#[test]
fn guided_result_defaults_to_incomplete_without_completion() {
    let result = GuidedRunResult::default();
    assert!(!result.is_pass());
}
