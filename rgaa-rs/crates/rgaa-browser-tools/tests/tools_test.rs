use rgaa_browser_tools::tools::{
    A11yTreeTool, AccessibilityTreeLegacy, AssertStateTool, AssertStateToolLegacy, ClickTool,
    ClickToolLegacy, EvalJsTool, EvalJsToolLegacy, NavigateLegacy, NavigateTool, PressKeyTool,
    PressKeyToolLegacy, ScreenshotLegacy, ScreenshotTool, TabOrderTool, TabOrderToolLegacy,
    TypeTool, TypeToolLegacy,
};
use rgaa_browser_tools::{AXNode, AXTree, BrowserSession, ToolContext};
use rgaa_obscura::ObscuraBridge;
use rig_core::tool::PortableTool;
use std::collections::HashMap;

#[test]
fn screenshot_legacy_is_unit_struct() {
    let tool = ScreenshotLegacy;
    assert!(std::mem::size_of_val(&tool) == 0);
}

#[test]
fn navigate_tool_holds_url() {
    let tool = NavigateLegacy {
        url: "https://example.com".to_string(),
    };
    assert_eq!(tool.url, "https://example.com");
}

#[test]
fn eval_js_tool_holds_snippet() {
    let tool = EvalJsToolLegacy {
        snippet: "document.title".to_string(),
    };
    assert_eq!(tool.snippet, "document.title");
}

#[test]
fn click_tool_holds_ref_id() {
    let tool = ClickToolLegacy {
        ref_id: "42".to_string(),
    };
    assert_eq!(tool.ref_id, "42");
}

#[test]
fn type_tool_holds_ref_id_and_text() {
    let tool = TypeToolLegacy {
        ref_id: "7".to_string(),
        text: "hello".to_string(),
    };
    assert_eq!(tool.ref_id, "7");
    assert_eq!(tool.text, "hello");
}

#[test]
fn press_key_tool_holds_key() {
    let tool = PressKeyToolLegacy {
        key: "Tab".to_string(),
    };
    assert_eq!(tool.key, "Tab");
}

#[test]
fn tab_order_tool_is_unit_struct() {
    let tool = TabOrderToolLegacy;
    assert!(std::mem::size_of_val(&tool) == 0);
}

#[test]
fn assert_state_tool_holds_predicate() {
    let tool = AssertStateToolLegacy {
        predicate: "document.title === 'Home'".to_string(),
    };
    assert_eq!(tool.predicate, "document.title === 'Home'");
}

#[tokio::test]
async fn navigate_tool_execute_without_cdp_returns_ok() {
    let bridge = ObscuraBridge::new();
    let mut session = BrowserSession::new(bridge);
    let tool = NavigateLegacy {
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
    let tool = ScreenshotLegacy;
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn a11y_tree_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let mut session = BrowserSession::new(bridge);
    let tool = AccessibilityTreeLegacy;
    let result = tool.execute(&mut session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn eval_js_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = EvalJsToolLegacy {
        snippet: "1+1".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn click_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = ClickToolLegacy {
        ref_id: "1".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn type_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = TypeToolLegacy {
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
    let tool = PressKeyToolLegacy {
        key: "Tab".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tab_order_tool_execute_without_a11y_tree_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = TabOrderToolLegacy;
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn assert_state_tool_execute_without_cdp_returns_err() {
    let bridge = ObscuraBridge::new();
    let session = BrowserSession::new(bridge);
    let tool = AssertStateToolLegacy {
        predicate: "true".to_string(),
    };
    let result = tool.execute(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn navigate_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = NavigateTool::new(ctx);
    let desc = tool.description();
    assert!(!desc.is_empty());
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn navigate_tool_calls_successfully() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = NavigateTool::new(ctx);
    let args: rgaa_browser_tools::tools::NavigateArgs =
        serde_json::from_value(serde_json::json!({"url": "https://example.com"}))
            .expect("valid args");
    let result = tool.call(args).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn click_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ClickTool::new(ctx);
    let desc = tool.description();
    assert!(desc.to_lowercase().contains("click"));
}

#[tokio::test]
async fn click_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ClickTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn click_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ClickTool::new(ctx);
    let args = rgaa_browser_tools::tools::ClickArgs {
        ref_id: "42".to_string(),
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn type_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TypeTool::new(ctx);
    let desc = tool.description();
    assert!(desc.to_lowercase().contains("type"));
}

#[tokio::test]
async fn type_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TypeTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn type_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TypeTool::new(ctx);
    let args = rgaa_browser_tools::tools::TypeArgs {
        ref_id: "7".to_string(),
        text: "hello".to_string(),
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn press_key_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = PressKeyTool::new(ctx);
    let desc = tool.description();
    assert!(desc.to_lowercase().contains("press"));
}

#[tokio::test]
async fn press_key_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = PressKeyTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn press_key_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = PressKeyTool::new(ctx);
    let args = rgaa_browser_tools::tools::PressKeyArgs {
        key: "Tab".to_string(),
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tab_order_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TabOrderTool::new(ctx);
    let desc = tool.description();
    assert!(desc.contains("tab"));
}

#[tokio::test]
async fn tab_order_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TabOrderTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn tab_order_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TabOrderTool::new(ctx);
    let args = rgaa_browser_tools::tools::TabOrderArgs {};
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn eval_js_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = EvalJsTool::new(ctx);
    let desc = tool.description();
    assert!(desc.contains("JavaScript"));
}

#[tokio::test]
async fn eval_js_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = EvalJsTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn eval_js_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = EvalJsTool::new(ctx);
    let args = rgaa_browser_tools::tools::EvalJsArgs {
        expression: "1+1".to_string(),
    };
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn assert_state_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = AssertStateTool::new(ctx);
    let desc = tool.description();
    assert!(desc.to_lowercase().contains("assert"));
}

#[tokio::test]
async fn assert_state_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = AssertStateTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn assert_state_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = AssertStateTool::new(ctx);
    let args = rgaa_browser_tools::tools::AssertStateArgs {
        predicate: "dialog-visible".to_string(),
    };
    let result = tool.call(args).await;
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
        nodes: vec![AXNode {
            backend_node_id: "1".to_string(),
            role: "button".to_string(),
            name: "OK".to_string(),
            children: vec![],
            properties: HashMap::new(),
        }],
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
    let ids: Vec<&str> = focusable
        .iter()
        .map(|n| n.backend_node_id.as_str())
        .collect();
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
async fn screenshot_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ScreenshotTool::new(ctx);
    let desc = tool.description();
    assert!(desc.contains("screenshot"));
}

#[tokio::test]
async fn screenshot_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ScreenshotTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn screenshot_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ScreenshotTool::new(ctx);
    let args = rgaa_browser_tools::tools::ScreenshotArgs {};
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn a11y_tree_tool_description() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = A11yTreeTool::new(ctx);
    let desc = tool.description();
    assert!(desc.contains("accessibility tree"));
}

#[tokio::test]
async fn a11y_tree_tool_parameters_is_object() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = A11yTreeTool::new(ctx);
    let params = tool.parameters();
    assert!(params.is_object());
}

#[tokio::test]
async fn a11y_tree_tool_calls_err_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = A11yTreeTool::new(ctx);
    let args = rgaa_browser_tools::tools::A11yTreeArgs {};
    let result = tool.call(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tool_context_creation() {
    let session = BrowserSession::new_placeholder();
    let ctx = ToolContext::new(session);
    assert!(ctx.session().lock().await.current_url().is_none());
}
