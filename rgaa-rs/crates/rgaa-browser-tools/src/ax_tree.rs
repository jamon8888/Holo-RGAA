use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AXNode {
    pub backend_node_id: String,
    pub role: String,
    pub name: String,
    pub children: Vec<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AXTree {
    pub nodes: Vec<AXNode>,
}

impl AXTree {
    pub fn find_by_ref(&self, ref_id: &str) -> Option<&AXNode> {
        self.nodes.iter().find(|n| n.backend_node_id == ref_id)
    }

    pub fn focused_element(&self) -> Option<&AXNode> {
        self.nodes.iter().find(|n| {
            n.properties
                .get("focused")
                .map(|v| v == "true")
                .unwrap_or(false)
        })
    }

    pub fn focusable_elements(&self) -> Vec<&AXNode> {
        self.nodes
            .iter()
            .filter(|n| {
                n.role == "button"
                    || n.role == "link"
                    || n.role == "textbox"
                    || n.role == "checkbox"
                    || n.role == "radio"
                    || n.role == "combobox"
                    || n.role == "listbox"
                    || n.role == "slider"
                    || n.role == "tab"
                    || n.properties.contains_key("tabindex")
            })
            .collect()
    }
}
