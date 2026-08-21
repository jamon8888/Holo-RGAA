use rgaa_browser_tools::tools::{
    AccessibilityTreeTool, AssertStateTool, ClickTool, EvalJsTool, NavigateTool, PressKeyTool,
    ScreenshotTool, TabOrderTool, TypeTool,
};
use rgaa_browser_tools::{AXNode, AXTree, BrowserSession, ToolContext};
use std::collections::HashMap;
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

#[test]
fn focused_element_returns_node_with_focused_true() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".to_string(),
                role: "textbox".to_string(),
                name: "Email".to_string(),
                children: vec![],
                properties: HashMap::from([("focused".to_string(), "true".to_string())]),
            },
            AXNode {
                backend_node_id: "2".to_string(),
                role: "button".to_string(),
                name: "Submit".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
        ],
    };
    let focused = tree.focused_element();
    assert!(focused.is_some());
    assert_eq!(focused.unwrap().backend_node_id, "1");
    assert_eq!(focused.unwrap().name, "Email");
}

#[test]
fn focused_element_returns_none_when_no_focused_node() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".to_string(),
                role: "button".to_string(),
                name: "OK".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
        ],
    };
    assert!(tree.focused_element().is_none());
}

#[test]
fn focused_element_returns_none_on_empty_tree() {
    let tree = AXTree { nodes: vec![] };
    assert!(tree.focused_element().is_none());
}

#[test]
fn focused_element_ignores_non_true_focused_value() {
    let tree = AXTree {
        nodes: vec![AXNode {
            backend_node_id: "1".to_string(),
            role: "textbox".to_string(),
            name: "Input".to_string(),
            children: vec![],
            properties: HashMap::from([("focused".to_string(), "false".to_string())]),
        }],
    };
    assert!(tree.focused_element().is_none());
}

#[test]
fn focusable_elements_returns_interactive_roles() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".to_string(),
                role: "button".to_string(),
                name: "Submit".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "2".to_string(),
                role: "link".to_string(),
                name: "Home".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "3".to_string(),
                role: "textbox".to_string(),
                name: "Search".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "4".to_string(),
                role: "checkbox".to_string(),
                name: "Accept".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "5".to_string(),
                role: "radio".to_string(),
                name: "Option A".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "6".to_string(),
                role: "combobox".to_string(),
                name: "Country".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "7".to_string(),
                role: "listbox".to_string(),
                name: "Options".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "8".to_string(),
                role: "slider".to_string(),
                name: "Volume".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "9".to_string(),
                role: "tab".to_string(),
                name: "Settings".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "10".to_string(),
                role: "div".to_string(),
                name: "Container".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
        ],
    };
    let focusable = tree.focusable_elements();
    assert_eq!(focusable.len(), 9);
    assert!(focusable.iter().all(|n| n.role != "div"));
}

#[test]
fn focusable_elements_returns_nodes_with_tabindex() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".to_string(),
                role: "div".to_string(),
                name: "Custom Widget".to_string(),
                children: vec![],
                properties: HashMap::from([("tabindex".to_string(), "0".to_string())]),
            },
            AXNode {
                backend_node_id: "2".to_string(),
                role: "img".to_string(),
                name: "Icon".to_string(),
                children: vec![],
                properties: HashMap::from([("tabindex".to_string(), "-1".to_string())]),
            },
            AXNode {
                backend_node_id: "3".to_string(),
                role: "span".to_string(),
                name: "Text".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
        ],
    };
    let focusable = tree.focusable_elements();
    assert_eq!(focusable.len(), 2);
    let ids: Vec<&str> = focusable.iter().map(|n| n.backend_node_id.as_str()).collect();
    assert!(ids.contains(&"1"));
    assert!(ids.contains(&"2"));
}

#[test]
fn focusable_elements_returns_empty_for_empty_tree() {
    let tree = AXTree { nodes: vec![] };
    assert!(tree.focusable_elements().is_empty());
}

#[test]
fn focusable_elements_returns_empty_when_no_interactive_nodes() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".to_string(),
                role: "div".to_string(),
                name: "Container".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
            AXNode {
                backend_node_id: "2".to_string(),
                role: "span".to_string(),
                name: "Text".to_string(),
                children: vec![],
                properties: HashMap::new(),
            },
        ],
    };
    assert!(tree.focusable_elements().is_empty());
}

#[tokio::test]
async fn test_tool_context_creation() {
    let session = BrowserSession::new_placeholder();
    let ctx = ToolContext::new(session);
    assert!(ctx.session().lock().await.current_url().is_none());
}

#[tokio::test]
async fn test_tool_context_shares_state_across_clones() {
    let session = BrowserSession::new_placeholder();
    let ctx1 = ToolContext::new(session);
    let ctx2 = ctx1.clone();

    ctx1.session().lock().await.set_current_url("https://example.com".to_string());
    let url = ctx2.session().lock().await.current_url().map(String::from);
    assert_eq!(url.as_deref(), Some("https://example.com"));
}
