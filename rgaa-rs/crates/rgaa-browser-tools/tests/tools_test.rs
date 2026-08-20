use rgaa_browser_tools::tools::{
    AccessibilityTreeTool, AssertStateTool, ClickTool, EvalJsTool, NavigateTool, PressKeyTool,
    ScreenshotTool, TabOrderTool, TypeTool,
};
use rgaa_browser_tools::{AXNode, AXTree, BrowserSession};
use rgaa_obscura::ObscuraBridge;

#[test]
fn screenshot_tool_is_unit_struct() {
    let tool = ScreenshotTool;
    assert!(std::mem::size_of_val(&tool) == 0);
}

#[test]
fn navigate_tool_holds_url() {
    let tool = NavigateTool {
        url: "https://example.com".to_string(),
    };
    assert_eq!(tool.url, "https://example.com");
}

#[test]
fn eval_js_tool_holds_snippet() {
    let tool = EvalJsTool {
        snippet: "document.title".to_string(),
    };
    assert_eq!(tool.snippet, "document.title");
}

#[test]
fn click_tool_holds_ref_id() {
    let tool = ClickTool {
        ref_id: "42".to_string(),
    };
    assert_eq!(tool.ref_id, "42");
}

#[test]
fn type_tool_holds_ref_id_and_text() {
    let tool = TypeTool {
        ref_id: "7".to_string(),
        text: "hello".to_string(),
    };
    assert_eq!(tool.ref_id, "7");
    assert_eq!(tool.text, "hello");
}

#[test]
fn press_key_tool_holds_key() {
    let tool = PressKeyTool {
        key: "Tab".to_string(),
    };
    assert_eq!(tool.key, "Tab");
}

#[test]
fn tab_order_tool_is_unit_struct() {
    let tool = TabOrderTool;
    assert!(std::mem::size_of_val(&tool) == 0);
}

#[test]
fn assert_state_tool_holds_predicate() {
    let tool = AssertStateTool {
        predicate: "document.title === 'Home'".to_string(),
    };
    assert_eq!(tool.predicate, "document.title === 'Home'");
}

#[tokio::test]
async fn navigate_tool_execute_without_cdp_returns_ok() {
    let bridge = ObscuraBridge::new();
    let mut session = BrowserSession::new(bridge);
    let tool = NavigateTool {
        url: "https://example.com".to_string(),
    };
    let result = tool.execute(&mut session).await;
    assert!(result.is_ok());
    assert!(result.unwrap().contains("example.com"));
}

#[tokio::test]
async fn screenshot_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = ScreenshotTool;
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn a11y_tree_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let mut session = BrowserSession::new(bridge);
    let tool = AccessibilityTreeTool;
    let result = tool.execute(&mut session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn eval_js_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = EvalJsTool {
        snippet: "1+1".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn click_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = ClickTool {
        ref_id: "1".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn type_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = TypeTool {
        ref_id: "1".to_string(),
        text: "test".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn press_key_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = PressKeyTool {
        key: "Tab".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tab_order_tool_execute_without_a11y_tree_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = TabOrderTool;
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn assert_state_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = AssertStateTool {
        predicate: "true".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[test]
fn ax_node_has_stable_ref() {
    let node = AXNode {
        backend_node_id: "123".to_string(),
        role: "button".to_string(),
        name: "Submit".to_string(),
        children: vec![],
        properties: std::collections::HashMap::new(),
    };
    assert_eq!(node.backend_node_id, "123");
    assert_eq!(node.role, "button");
}

#[test]
fn ax_tree_find_node_by_ref() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".to_string(),
                role: "root".to_string(),
                name: "".to_string(),
                children: vec!["2".to_string()],
                properties: std::collections::HashMap::new(),
            },
            AXNode {
                backend_node_id: "2".to_string(),
                role: "button".to_string(),
                name: "OK".to_string(),
                children: vec![],
                properties: std::collections::HashMap::new(),
            },
        ],
    };
    let node = tree.find_by_ref("2");
    assert!(node.is_some());
    assert_eq!(node.unwrap().name, "OK");
}
