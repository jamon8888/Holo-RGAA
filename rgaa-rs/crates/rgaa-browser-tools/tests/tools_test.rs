use rgaa_browser_tools::{AXNode, AXTree};

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
