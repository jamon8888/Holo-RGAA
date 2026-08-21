# rgaa-browser-tools

Browser automation tools for RGAA accessibility auditing via CDP (Chrome DevTools Protocol).

## Features
- AXTree types for accessibility tree navigation
- BrowserSession for CDP connection management
- 9 tool definitions for browser interaction (navigate, screenshot, a11y_tree, eval_js, click, type_input, press_key, tab_order, assert_state)
- MCP server skeleton for tool exposure

## Usage
```rust
use rgaa_browser_tools::{BrowserSession, ax_tree::{AXTree, AXNode}};

// Create session (requires ObscuraBridge)
let session = BrowserSession::new(bridge);

// Get accessibility tree
if let Some(tree) = session.last_a11y() {
    if let Some(node) = tree.find_by_ref("node-123") {
        println!("Found: {:?}", node.name);
    }
}
```
