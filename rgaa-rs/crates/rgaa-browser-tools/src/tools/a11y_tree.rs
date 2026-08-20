use crate::ax_tree::AXTree;

pub struct AccessibilityTreeTool;

impl AccessibilityTreeTool {
    /// Fetch the accessibility tree via CDP Accessibility.getFullAXTree.
    /// Returns a structured AXTree with stable backendNodeIds.
    pub async fn execute(
        &self,
        _session: &mut crate::BrowserSession,
    ) -> Result<AXTree, String> {
        Err("a11y tree not yet connected to CDP".to_string())
    }
}
