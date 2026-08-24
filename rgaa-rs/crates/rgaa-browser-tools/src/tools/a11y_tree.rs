use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the a11y tree tool.
#[derive(Debug, thiserror::Error)]
pub enum A11yTreeError {
    #[error("a11y tree not yet connected to CDP")]
    NotConnected,
    #[error("a11y tree capture failed: {0}")]
    CaptureFailed(String),
}

/// Arguments for the a11y tree tool (no parameters needed).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct A11yTreeArgs {}

/// Output from the a11y tree tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct A11yTreeOutput {
    pub tree_json: serde_json::Value,
    pub node_count: usize,
}

/// Tool that retrieves the full accessibility tree of the current page.
pub struct A11yTreeTool {
    ctx: ToolContext,
}

impl A11yTreeTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for A11yTreeTool {
    const NAME: &str = "a11y_tree";
    type Error = A11yTreeError;
    type Args = A11yTreeArgs;
    type Output = A11yTreeOutput;

    fn description(&self) -> String {
        "Get the full accessibility tree of the current page".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(A11yTreeArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session = self.ctx.session().lock().await;
        let tree = session
            .get_a11y_tree()
            .await
            .map_err(A11yTreeError::CaptureFailed)?;

        let node_count = count_ax_nodes(&tree);

        Ok(A11yTreeOutput {
            tree_json: tree,
            node_count,
        })
    }
}

/// Legacy unit-struct tool for backward compatibility with existing tests.
pub struct AccessibilityTreeLegacy;

impl AccessibilityTreeLegacy {
    /// Fetch the accessibility tree via CDP Accessibility.getFullAXTree.
    /// Returns a structured AXTree with stable backendNodeIds.
    pub async fn execute(
        &self,
        session: &mut crate::BrowserSession,
    ) -> Result<crate::AXTree, String> {
        let tree = session.get_a11y_tree().await?;
        // Convert raw JSON tree to AXTree structure
        let nodes = flatten_ax_tree(&tree);
        Ok(crate::AXTree { nodes })
    }
}

fn count_ax_nodes(tree: &serde_json::Value) -> usize {
    let mut count = 1; // count the root node itself
    if let Some(children) = tree.get("children").and_then(|c| c.as_array()) {
        for child in children {
            count += 1;
            count += count_ax_nodes(child);
        }
    }
    count
}

fn flatten_ax_tree(tree: &serde_json::Value) -> Vec<crate::ax_tree::AXNode> {
    let mut nodes = Vec::new();
    flatten_ax_node(tree, &mut nodes);
    nodes
}

fn flatten_ax_node(node: &serde_json::Value, acc: &mut Vec<crate::ax_tree::AXNode>) {
    let backend_node_id = node
        .get("backendDOMNodeId")
        .or_else(|| node.get("backendNodeId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let role = node
        .get("role")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let name = node
        .get("name")
        .and_then(|n| n.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let children: Vec<String> = node
        .get("children")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|child| {
                    child
                        .get("backendDOMNodeId")
                        .or_else(|| child.get("backendNodeId"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    let mut properties = std::collections::HashMap::new();
    if let Some(props) = node.get("properties").and_then(|p| p.as_array()) {
        for prop in props {
            if let (Some(key), Some(val)) = (
                prop.get("name").and_then(|v| v.as_str()),
                prop.get("value"),
            ) {
                let value_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    other => other.to_string(),
                };
                properties.insert(key.to_string(), value_str);
            }
        }
    }

    acc.push(crate::ax_tree::AXNode {
        backend_node_id,
        role,
        name,
        children,
        properties,
    });

    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child in children {
            flatten_ax_node(child, acc);
        }
    }
}
