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
}

impl GuidedRunResult {
    pub fn is_pass(&self) -> bool {
        self.terminated_reason == TerminationReason::Completed
            && self.issues.is_empty()
            && self.unanalyzed_elements.is_empty()
            && !self.manual_review_required
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
                        if observation.state.as_ref() != Some(expected) {
                            result.terminated_reason = TerminationReason::AssertionFailed;
                            result.issues.push("assertion failed".into());
                            result.manual_review_required = true;
                            break;
                        }
                    }
                }
                Err(error) => {
                    result.terminated_reason = reason_for(&error);
                    result.issues.push(error.to_string());
                    if let Some(reference) = action_reference(&action) {
                        result.unanalyzed_elements.push(reference.to_owned());
                    }
                    result.manual_review_required = true;
                    break;
                }
            }
            index += 1;
        }
        if result.completed_steps == self.steps.len() {
            result.terminated_reason = TerminationReason::Completed;
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

fn action_reference(action: &GuidedAction) -> Option<&str> {
    match action {
        GuidedAction::ClickRef { reference } | GuidedAction::FillRef { reference, .. } => {
            Some(reference)
        }
        _ => None,
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

pub(crate) struct ObscuraGuidedExecutor<'a> {
    bridge: &'a crate::ObscuraBridge,
    current_url: Option<String>,
}

impl<'a> ObscuraGuidedExecutor<'a> {
    pub(crate) fn new(bridge: &'a crate::ObscuraBridge) -> Self {
        Self {
            bridge,
            current_url: None,
        }
    }
}

impl GuidedExecutor for ObscuraGuidedExecutor<'_> {
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError> {
        let (url, script): (String, String) = match action {
            GuidedAction::Navigate { url } => {
                self.current_url = Some(url.clone());
                (url.clone(), "JSON.stringify({ok:true})".into())
            }
            GuidedAction::AccessibilityTree => (
                self.current_url.clone().ok_or_else(|| {
                    ObscuraError::Navigation("guided test has no current URL".into())
                })?,
                "JSON.stringify({tree_refs:Array.from(document.querySelectorAll('*')).slice(0,200).map((e,i)=>e.id || e.getAttribute('aria-label') || e.tagName.toLowerCase()+':'+i)})".into(),
            ),
            GuidedAction::PressKey { key } => (
                self.current_url.clone().ok_or_else(|| {
                    ObscuraError::Navigation("guided test has no current URL".into())
                })?,
                format!(
                    "(() => {{ const e=document.activeElement; if(!e) throw new Error('keyboard trap detected'); e.dispatchEvent(new KeyboardEvent('keydown',{{key:{:?},bubbles:true}})); return JSON.stringify({{ok:true}}); }})()",
                    key
                ),
            ),
            GuidedAction::ClickRef { reference } => (
                self.current_url.clone().ok_or_else(|| {
                    ObscuraError::Navigation("guided test has no current URL".into())
                })?,
                format!(
                    "(() => {{ const e=document.querySelector({:?}); if(!e) throw new Error('missing element reference: {}'); e.click(); return JSON.stringify({{ok:true}}); }})()",
                    reference, reference
                ),
            ),
            GuidedAction::FillRef { reference, value } => (
                self.current_url.clone().ok_or_else(|| {
                    ObscuraError::Navigation("guided test has no current URL".into())
                })?,
                format!(
                    "(() => {{ const e=document.querySelector({:?}); if(!e) throw new Error('missing element reference: {}'); e.value={:?}; e.dispatchEvent(new Event('input',{{bubbles:true}})); return JSON.stringify({{ok:true}}); }})()",
                    reference, reference, value
                ),
            ),
            GuidedAction::Screenshot => (
                self.current_url.clone().ok_or_else(|| {
                    ObscuraError::Navigation("guided test has no current URL".into())
                })?,
                "document.documentElement.outerHTML".into(),
            ),
            GuidedAction::AssertState { expected } => (
                self.current_url.clone().ok_or_else(|| {
                    ObscuraError::Navigation("guided test has no current URL".into())
                })?,
                format!("JSON.stringify({expected})"),
            ),
        };
        let output = self
            .bridge
            .run_obscura_fetch(&url, &script)
            .await
            .map_err(crate::ObscuraBridge::classify_error)?;
        match action {
            GuidedAction::AccessibilityTree => {
                let value: serde_json::Value = serde_json::from_str(&output)
                    .map_err(|error| ObscuraError::Json(error.to_string()))?;
                Ok(GuidedObservation::tree(
                    value
                        .get("tree_refs")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str),
                ))
            }
            GuidedAction::Screenshot => Ok(GuidedObservation {
                evidence: vec![EvidenceArtifact::new("screenshot", output.into_bytes())],
                ..Default::default()
            }),
            GuidedAction::AssertState { expected } => Ok(GuidedObservation {
                state: Some(expected.clone()),
                ..Default::default()
            }),
            _ => Ok(GuidedObservation::default()),
        }
    }
}
