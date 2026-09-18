use crate::evidence::{EvidenceArtifact, EvidenceRef, EvidenceStore};
use crate::ObscuraError;
use serde::{Deserialize, Serialize};

const MAX_STEP_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuidedTest {
    pub id: String,
    pub version: u32,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub steps: Vec<GuidedStep>,
    #[serde(default)]
    pub criterion_mapping: Vec<String>,
    #[serde(default)]
    pub evidence_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuidedStep {
    Navigate { url: String },
    AccessibilityTree,
    PressKey { key: String },
    ClickRef { reference: String },
    FillRef { reference: String, value: String },
    Screenshot,
    AssertState { expected: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GuidedAction {
    Navigate { url: String },
    AccessibilityTree,
    PressKey { key: String },
    ClickRef { reference: String },
    FillRef { reference: String, value: String },
    Screenshot,
    AssertState { expected: serde_json::Value },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GuidedObservation {
    #[serde(default)]
    pub tree_refs: Vec<String>,
    #[serde(default)]
    pub state: Option<serde_json::Value>,
    #[serde(default)]
    pub evidence: Vec<EvidenceArtifact>,
}

impl GuidedObservation {
    pub fn tree(refs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tree_refs: refs.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait GuidedExecutor {
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError>;
}

pub fn is_stable_accessibility_reference(reference: &str) -> bool {
    reference
        .strip_prefix("ax:")
        .is_some_and(|value| value.parse::<u64>().is_ok())
        || (reference.starts_with("ax-role=") && reference.contains(";name="))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TerminationReason {
    Completed,
    MissingReference,
    AssertionFailed,
    KeyboardTrap,
    Timeout,
    NavigationError,
    #[default]
    ExecutionError,
    InvalidOrdering,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GuidedRunResult {
    pub issues: Vec<String>,
    pub unanalyzed_elements: Vec<String>,
    pub terminated_reason: TerminationReason,
    pub completed_steps: usize,
    pub evidence: Vec<EvidenceRef>,
    pub manual_review_required: bool,
    pub action_trace: Vec<GuidedAction>,
    pub criterion_mapping: Vec<String>,
    #[serde(default)]
    pub evidence_requirements: Vec<String>,
}

fn state_matches(expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (expected, actual) {
        (serde_json::Value::Object(expected), serde_json::Value::Object(actual)) => {
            expected.iter().all(|(key, value)| {
                actual
                    .get(key)
                    .is_some_and(|candidate| state_matches(value, candidate))
            })
        }
        (serde_json::Value::Array(expected), serde_json::Value::Array(actual)) => {
            expected.len() == actual.len()
                && expected
                    .iter()
                    .zip(actual)
                    .all(|(expected, actual)| state_matches(expected, actual))
        }
        _ => expected == actual,
    }
}

impl GuidedRunResult {
    pub fn is_pass(&self) -> bool {
        self.terminated_reason == TerminationReason::Completed
            && self.issues.is_empty()
            && self.unanalyzed_elements.is_empty()
            && !self.manual_review_required
            && self.evidence_requirements.iter().all(|required| {
                self.evidence
                    .iter()
                    .any(|evidence| evidence.kind == *required)
            })
    }
}

impl GuidedTest {
    pub async fn run<E: GuidedExecutor>(
        &self,
        executor: &mut E,
        evidence_store: Option<&EvidenceStore>,
    ) -> Result<GuidedRunResult, ObscuraError> {
        let mut result = GuidedRunResult {
            criterion_mapping: self.criterion_mapping.clone(),
            evidence_requirements: self.evidence_requirements.clone(),
            ..Default::default()
        };
        let mut index = 0;
        while index < self.steps.len() {
            let step = &self.steps[index];
            if is_mutating(step)
                && !self
                    .steps
                    .get(index + 1)
                    .is_some_and(is_observation_or_assertion)
            {
                result.terminated_reason = TerminationReason::InvalidOrdering;
                result
                    .issues
                    .push("mutating action must be followed by observation or assertion".into());
                result.manual_review_required = true;
                mark_unanalyzed(&mut result, &self.steps[index..]);
                break;
            }
            let action: GuidedAction = step.clone().into();
            result.action_trace.push(action.clone());
            match execute_bounded(executor, &action).await {
                Ok(observation) => {
                    result.completed_steps += 1;
                    if !observation.tree_refs.is_empty() {
                        let bytes = serde_json::to_vec(&observation.tree_refs)
                            .map_err(|error| ObscuraError::Json(error.to_string()))?;
                        if let Some(store) = evidence_store {
                            result
                                .evidence
                                .push(store.write(EvidenceArtifact::new("tree", bytes))?);
                        }
                    }
                    for artifact in observation.evidence {
                        if let Some(store) = evidence_store {
                            result.evidence.push(store.write(artifact)?);
                        }
                    }
                    if let GuidedStep::AssertState { expected } = step {
                        if !observation
                            .state
                            .as_ref()
                            .is_some_and(|actual| state_matches(expected, actual))
                        {
                            result.terminated_reason = TerminationReason::AssertionFailed;
                            result.issues.push("assertion failed".into());
                            result.manual_review_required = true;
                            mark_unanalyzed(&mut result, &self.steps[index + 1..]);
                            break;
                        }
                    }
                }
                Err(error) => {
                    result.terminated_reason = reason_for(&error);
                    result.issues.push(error.to_string());
                    mark_unanalyzed(&mut result, &self.steps[index..]);
                    result.manual_review_required = true;
                    break;
                }
            }
            index += 1;
        }
        if result.completed_steps == self.steps.len()
            && result.terminated_reason == TerminationReason::ExecutionError
        {
            result.terminated_reason = TerminationReason::Completed;
        }
        for required in &result.evidence_requirements {
            if !result
                .evidence
                .iter()
                .any(|evidence| evidence.kind == *required)
            {
                result
                    .issues
                    .push(format!("required evidence is missing: {required}"));
                result
                    .unanalyzed_elements
                    .push(format!("evidence:{required}"));
                result.manual_review_required = true;
            }
        }
        Ok(result)
    }
}

async fn execute_bounded<E: GuidedExecutor>(
    executor: &mut E,
    action: &GuidedAction,
) -> Result<GuidedObservation, ObscuraError> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        match executor.execute(action).await {
            Ok(observation) => return Ok(observation),
            Err(error) if attempts < MAX_STEP_ATTEMPTS && is_retryable(&error) => continue,
            Err(error) => return Err(error),
        }
    }
}

fn is_retryable(error: &ObscuraError) -> bool {
    matches!(
        error,
        ObscuraError::Timeout(_) | ObscuraError::CdpTransport(_)
    )
}

impl From<GuidedStep> for GuidedAction {
    fn from(step: GuidedStep) -> Self {
        match step {
            GuidedStep::Navigate { url } => Self::Navigate { url },
            GuidedStep::AccessibilityTree => Self::AccessibilityTree,
            GuidedStep::PressKey { key } => Self::PressKey { key },
            GuidedStep::ClickRef { reference } => Self::ClickRef { reference },
            GuidedStep::FillRef { reference, value } => Self::FillRef { reference, value },
            GuidedStep::Screenshot => Self::Screenshot,
            GuidedStep::AssertState { expected } => Self::AssertState { expected },
        }
    }
}

fn is_mutating(step: &GuidedStep) -> bool {
    matches!(
        step,
        GuidedStep::Navigate { .. }
            | GuidedStep::PressKey { .. }
            | GuidedStep::ClickRef { .. }
            | GuidedStep::FillRef { .. }
    )
}

fn is_observation_or_assertion(step: &GuidedStep) -> bool {
    matches!(
        step,
        GuidedStep::AccessibilityTree | GuidedStep::Screenshot | GuidedStep::AssertState { .. }
    )
}

fn mark_unanalyzed(result: &mut GuidedRunResult, steps: &[GuidedStep]) {
    for step in steps {
        let target = match step {
            GuidedStep::Navigate { url } => format!("navigate:{url}"),
            GuidedStep::AccessibilityTree => "accessibility-tree".into(),
            GuidedStep::PressKey { key } => format!("key:{key}"),
            GuidedStep::ClickRef { reference } | GuidedStep::FillRef { reference, .. } => {
                reference.clone()
            }
            GuidedStep::Screenshot => "screenshot".into(),
            GuidedStep::AssertState { .. } => "assert-state".into(),
        };
        if !result.unanalyzed_elements.contains(&target) {
            result.unanalyzed_elements.push(target);
        }
    }
}

fn reason_for(error: &ObscuraError) -> TerminationReason {
    let message = error.to_string().to_ascii_lowercase();
    if matches!(error, ObscuraError::Timeout(_)) {
        TerminationReason::Timeout
    } else if message.contains("keyboard trap") {
        TerminationReason::KeyboardTrap
    } else if message.contains("missing element reference") {
        TerminationReason::MissingReference
    } else if matches!(error, ObscuraError::Navigation(_)) {
        TerminationReason::NavigationError
    } else if message.contains("assertion") {
        TerminationReason::AssertionFailed
    } else {
        TerminationReason::ExecutionError
    }
}
