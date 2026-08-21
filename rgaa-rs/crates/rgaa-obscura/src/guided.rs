use crate::evidence::{EvidenceArtifact, EvidenceRef, EvidenceStore};
use crate::ObscuraError;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Maximum number of attempts for a single guided test step.
const MAX_STEP_ATTEMPTS: usize = 3;

/// A guided accessibility test definition.
///
/// Contains a sequence of steps to execute, along with metadata
/// for criterion mapping and evidence collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuidedTest {
    /// Unique identifier for the test.
    pub id: String,
    /// Version number of the test.
    pub version: u32,
    /// Preconditions that must be met before running the test.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// Steps to execute in order.
    pub steps: Vec<GuidedStep>,
    /// RGAA criteria this test maps to.
    #[serde(default)]
    pub criterion_mapping: Vec<String>,
    /// Types of evidence required by this test.
    #[serde(default)]
    pub evidence_requirements: Vec<String>,
}

/// A single step in a guided accessibility test.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuidedStep {
    /// Navigate to a URL.
    Navigate { url: String },
    /// Capture the accessibility tree.
    AccessibilityTree,
    /// Press a keyboard key.
    PressKey { key: String },
    /// Click an element by accessibility reference.
    ClickRef { reference: String },
    /// Fill an input field by accessibility reference.
    FillRef { reference: String, value: String },
    /// Take a screenshot.
    Screenshot,
    /// Assert the page state matches expected values.
    AssertState { expected: serde_json::Value },
}

/// An action to execute in a guided test (runtime representation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GuidedAction {
    /// Navigate to a URL.
    Navigate { url: String },
    /// Capture the accessibility tree.
    AccessibilityTree,
    /// Press a keyboard key.
    PressKey { key: String },
    /// Click an element by accessibility reference.
    ClickRef { reference: String },
    /// Fill an input field by accessibility reference.
    FillRef { reference: String, value: String },
    /// Take a screenshot.
    Screenshot,
    /// Assert the page state matches expected values.
    AssertState { expected: serde_json::Value },
}

/// Observations collected during guided test execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GuidedObservation {
    /// Accessibility tree references collected.
    #[serde(default)]
    pub tree_refs: Vec<String>,
    /// Page state snapshot.
    #[serde(default)]
    pub state: Option<serde_json::Value>,
    /// Evidence artifacts collected.
    #[serde(default)]
    pub evidence: Vec<EvidenceArtifact>,
}

impl GuidedObservation {
    /// Creates an observation with accessibility tree references.
    ///
    /// # Arguments
    ///
    /// * `refs` - Accessibility tree references to include.
    pub fn tree(refs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tree_refs: refs.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }
}

/// Executor trait for guided accessibility tests.
///
/// Implementations drive browser interaction via CDP or other mechanisms.
#[allow(async_fn_in_trait)]
pub trait GuidedExecutor {
    /// Executes a guided action and returns observations.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to execute.
    ///
    /// # Errors
    ///
    /// Returns `ObscuraError` if the action fails.
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError>;
}

/// Checks if a reference is a stable accessibility reference.
///
/// Stable references use the format `ax:{number}` or `ax-role={role};name={name}`
/// and can be used reliably across page loads.
///
/// # Arguments
///
/// * `reference` - The reference string to check.
///
/// # Returns
///
/// `true` if the reference is stable, `false` otherwise.
pub fn is_stable_accessibility_reference(reference: &str) -> bool {
    reference
        .strip_prefix("ax:")
        .is_some_and(|value| value.parse::<u64>().is_ok())
        || (reference.starts_with("ax-role=") && reference.contains(";name="))
}

/// Reason why a guided test terminated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TerminationReason {
    /// Test completed successfully.
    Completed,
    /// An accessibility reference was missing.
    MissingReference,
    /// An assertion failed.
    AssertionFailed,
    /// A keyboard trap was detected.
    KeyboardTrap,
    /// The test timed out.
    Timeout,
    /// Navigation to a URL failed.
    NavigationError,
    /// An execution error occurred.
    #[default]
    ExecutionError,
    /// Steps were executed in an invalid order.
    InvalidOrdering,
}

/// Result of running a guided accessibility test.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GuidedRunResult {
    /// Issues discovered during the test.
    pub issues: Vec<String>,
    /// Elements that could not be analyzed.
    pub unanalyzed_elements: Vec<String>,
    /// Reason the test terminated.
    pub terminated_reason: TerminationReason,
    /// Number of steps completed.
    pub completed_steps: usize,
    /// Evidence collected during the test.
    pub evidence: Vec<EvidenceRef>,
    /// Whether manual review is required.
    pub manual_review_required: bool,
    /// Trace of actions executed.
    pub action_trace: Vec<GuidedAction>,
    /// RGAA criteria this test maps to.
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

pub(crate) struct ObscuraGuidedExecutor {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    target_id: String,
    session_id: String,
    current_url: Option<String>,
    accessibility_refs: HashMap<String, u64>,
}

impl ObscuraGuidedExecutor {
    pub(crate) async fn connect(bridge: &crate::ObscuraBridge) -> Result<Self, ObscuraError> {
        let ws_url = bridge
            .get_browser_ws_url()
            .await
            .map_err(crate::ObscuraBridge::classify_error)?;
        let (mut ws, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|error| ObscuraError::CdpTransport(error.to_string()))?;
        let target = crate::ObscuraBridge::cdp_send(
            &mut ws,
            "Target.createTarget",
            serde_json::json!({"url": "about:blank"}),
        )
        .await
        .map_err(crate::ObscuraBridge::classify_error)?;
        let target_id = target
            .get("targetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ObscuraError::CdpTransport("guided target id is missing".into()))?
            .to_owned();
        let session = crate::ObscuraBridge::cdp_send(
            &mut ws,
            "Target.attachToTarget",
            serde_json::json!({"targetId": target_id, "flatten": true}),
        )
        .await;
        let session_id = match session {
            Ok(value) => value
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| ObscuraError::CdpTransport("guided session id is missing".into()))?,
            Err(error) => {
                let _ = crate::ObscuraBridge::cdp_send(
                    &mut ws,
                    "Target.closeTarget",
                    serde_json::json!({"targetId": target_id}),
                )
                .await;
                return Err(crate::ObscuraBridge::classify_error(error));
            }
        };
        Ok(Self {
            ws,
            target_id,
            session_id,
            current_url: None,
            accessibility_refs: HashMap::new(),
        })
    }

    pub(crate) async fn close(&mut self) -> Result<(), ObscuraError> {
        crate::ObscuraBridge::cleanup_target(&mut self.ws, &self.session_id, &self.target_id)
            .await
            .map_err(crate::ObscuraBridge::classify_error)
    }

    async fn send(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ObscuraError> {
        crate::ObscuraBridge::cdp_send_session(&mut self.ws, &self.session_id, method, params)
            .await
            .map_err(crate::ObscuraBridge::classify_error)
    }

    async fn observe_state(&mut self) -> Result<serde_json::Value, ObscuraError> {
        let result = self
            .send(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": "JSON.stringify({url: location.href, title: document.title, active_tag: document.activeElement && document.activeElement.tagName, values: Array.from(document.querySelectorAll('input,textarea,select')).map(e => ({name:e.name, id:e.id, value:e.value}))})",
                    "returnByValue": true
                }),
            )
            .await?;
        let value = result
            .get("result")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ObscuraError::Evaluation("browser state was not returned".into()))?;
        serde_json::from_str(value).map_err(|error| ObscuraError::Json(error.to_string()))
    }

    async fn active_element_signature(&mut self) -> Result<Option<String>, ObscuraError> {
        let result = self
            .send(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": "document.activeElement ? document.activeElement.outerHTML : null",
                    "returnByValue": true
                }),
            )
            .await?;
        Ok(result
            .get("result")
            .and_then(|value| value.get("value"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned))
    }

    async fn resolve_reference(&mut self, reference: &str) -> Result<String, ObscuraError> {
        let backend_node_id = reference
            .strip_prefix("ax:")
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| self.accessibility_refs.get(reference).copied())
            .filter(|_| is_stable_accessibility_reference(reference))
            .ok_or_else(|| {
                ObscuraError::Evaluation(format!(
                    "missing element reference: {reference} (expected stable ax:<backendNodeId>)"
                ))
            })?;
        let result = self
            .send(
                "DOM.resolveNode",
                serde_json::json!({"backendNodeId": backend_node_id}),
            )
            .await?;
        result
            .get("object")
            .and_then(|object| object.get("objectId"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                ObscuraError::Evaluation(format!("missing element reference: {reference}"))
            })
    }

    async fn click_or_fill(
        &mut self,
        reference: &str,
        value: Option<&str>,
    ) -> Result<(), ObscuraError> {
        let object_id = self.resolve_reference(reference).await?;
        let function = if value.is_some() {
            "function(value) { this.focus(); this.value = value; this.dispatchEvent(new Event('input', {bubbles:true})); this.dispatchEvent(new Event('change', {bubbles:true})); return true; }"
        } else {
            "function() { this.click(); return true; }"
        };
        let mut params = serde_json::json!({
            "objectId": object_id,
            "functionDeclaration": function,
            "returnByValue": true
        });
        if let Some(value) = value {
            params["arguments"] = serde_json::json!([{"value": value}]);
        }
        let response = self.send("Runtime.callFunctionOn", params).await?;
        if response.get("exceptionDetails").is_some() {
            return Err(ObscuraError::Evaluation(format!(
                "failed to operate on element reference: {reference}"
            )));
        }
        Ok(())
    }
}

impl GuidedExecutor for ObscuraGuidedExecutor {
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError> {
        if !matches!(action, GuidedAction::Navigate { .. }) && self.current_url.is_none() {
            return Err(ObscuraError::Navigation(
                "guided test has no current URL".into(),
            ));
        }
        match action {
            GuidedAction::Navigate { url } => {
                let result = self
                    .send("Page.navigate", serde_json::json!({"url": url}))
                    .await?;
                if result.get("errorText").is_some() {
                    return Err(ObscuraError::Navigation(result["errorText"].to_string()));
                }
                crate::ObscuraBridge::wait_for_load(
                    &mut self.ws,
                    &self.session_id,
                    std::time::Duration::from_secs(30),
                )
                .await
                .map_err(crate::ObscuraBridge::classify_error)?;
                self.current_url = Some(url.clone());
                Ok(GuidedObservation::default())
            }
            GuidedAction::AccessibilityTree => {
                self.send("Accessibility.enable", serde_json::json!({}))
                    .await?;
                let value = self
                    .send("Accessibility.getFullAXTree", serde_json::json!({}))
                    .await?;
                let refs = value
                    .get("nodes")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|node| {
                        let backend_id = node.get("backendDOMNodeId")?.as_u64()?;
                        let role = node.get("role")?.get("value")?.as_str()?;
                        let name = node.get("name")?.get("value")?.as_str()?;
                        if !role.is_empty() && !name.is_empty() {
                            self.accessibility_refs
                                .insert(format!("ax-role={role};name={name}"), backend_id);
                        }
                        Some(format!("ax:{backend_id}"))
                    });
                let tree_refs = refs.collect::<Vec<_>>();
                let evidence = serde_json::to_vec(&value)
                    .map_err(|error| ObscuraError::Json(error.to_string()))?;
                Ok(GuidedObservation {
                    tree_refs,
                    evidence: vec![EvidenceArtifact::new("tree", evidence)],
                    ..Default::default()
                })
            }
            GuidedAction::PressKey { key } => {
                let before = self.active_element_signature().await?;
                self.send(
                    "Input.dispatchKeyEvent",
                    serde_json::json!({"type":"keyDown", "key":key}),
                )
                .await?;
                self.send(
                    "Input.dispatchKeyEvent",
                    serde_json::json!({"type":"keyUp", "key":key}),
                )
                .await?;
                let after = self.active_element_signature().await?;
                if key == "Tab" && before.is_some() && before == after {
                    return Err(ObscuraError::Evaluation("keyboard trap detected".into()));
                }
                Ok(GuidedObservation::default())
            }
            GuidedAction::ClickRef { reference } => {
                self.click_or_fill(reference, None).await?;
                Ok(GuidedObservation::default())
            }
            GuidedAction::FillRef { reference, value } => {
                self.click_or_fill(reference, Some(value)).await?;
                Ok(GuidedObservation::default())
            }
            GuidedAction::Screenshot => {
                let value = self
                    .send(
                        "Page.captureScreenshot",
                        serde_json::json!({"format":"png"}),
                    )
                    .await?;
                let encoded = value
                    .get("data")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        ObscuraError::Evidence("CDP returned no screenshot data".into())
                    })?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|error| ObscuraError::Evidence(error.to_string()))?;
                if !bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
                    return Err(ObscuraError::Evidence(
                        "CDP screenshot was not PNG data".into(),
                    ));
                }
                Ok(GuidedObservation {
                    evidence: vec![EvidenceArtifact::new("screenshot", bytes)],
                    ..Default::default()
                })
            }
            GuidedAction::AssertState { .. } => Ok(GuidedObservation {
                state: Some(self.observe_state().await?),
                ..Default::default()
            }),
        }
    }
}
