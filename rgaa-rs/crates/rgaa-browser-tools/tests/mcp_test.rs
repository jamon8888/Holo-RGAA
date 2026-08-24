use rgaa_browser_tools::mcp::BrowserMcpServer;

#[test]
fn mcp_server_exposes_screenshot_tool() {
    let server = BrowserMcpServer::new_placeholder();
    let tools = server.tool_names();
    assert!(tools.contains(&"screenshot".to_string()));
    assert!(tools.contains(&"navigate".to_string()));
    assert!(tools.contains(&"a11y_tree".to_string()));
    assert!(tools.contains(&"eval_js".to_string()));
    assert!(tools.contains(&"click".to_string()));
    assert!(tools.contains(&"type".to_string()));
    assert!(tools.contains(&"press_key".to_string()));
    assert!(tools.contains(&"tab_order".to_string()));
    assert!(tools.contains(&"assert_state".to_string()));
}
