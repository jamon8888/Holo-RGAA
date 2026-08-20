use crate::ax_tree::AXNode;

pub struct TabOrderTool;

impl TabOrderTool {
    /// Return the ordered list of focusable elements from the a11y tree.
    /// Each element has a stable backendNodeId for click/type operations.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<Vec<AXNode>, String> {
        let tree = session
            .last_a11y()
            .ok_or("no a11y tree available — call AccessibilityTreeTool first")?;
        Ok(tree.focusable_elements().into_iter().cloned().collect())
    }
}
