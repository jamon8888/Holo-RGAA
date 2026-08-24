use std::collections::HashMap;

/// Detect which RGAA criteria are not applicable based on page context.
///
/// Returns a map `criterion_id -> applicable` where `false` means Not Applicable.
pub fn detect_na(page_context: &serde_json::Value) -> HashMap<String, bool> {
    let mut applicable = HashMap::new();

    // Default: assume all criteria are applicable.
    // We will only explicitly set the criteria covered by detection rules.
    // Uncovered criteria remain absent from the map and are treated as applicable by callers.

    // Helper to check if an array field is non-empty
    let has_non_empty_array = |key: &str| -> bool {
        page_context
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    };

    // 1.x images criteria 1.1-1.9
    let has_images = has_non_empty_array("images");
    if !has_images {
        for j in 1..=9 {
            applicable.insert(format!("1.{j}"), false);
        }
    } else {
        for j in 1..=9 {
            applicable.entry(format!("1.{j}")).or_insert(true);
        }
    }

    // 11.x forms criteria 11.1-11.13
    let has_forms = has_non_empty_array("forms");
    if !has_forms {
        for j in 1..=13 {
            applicable.insert(format!("11.{j}"), false);
        }
    } else {
        for j in 1..=13 {
            applicable.entry(format!("11.{j}")).or_insert(true);
        }
    }

    // 5.x tables criteria 5.1-5.8
    // Check landmarks for role="table"
    let has_tables = page_context
        .get("landmarks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().any(|l| {
                l.get("role")
                    .and_then(|r| r.as_str())
                    .map(|s| s == "table")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if !has_tables {
        for j in 1..=8 {
            applicable.insert(format!("5.{j}"), false);
        }
    } else {
        for j in 1..=8 {
            applicable.entry(format!("5.{j}")).or_insert(true);
        }
    }

    // 2.1 iframes
    let has_iframes = has_non_empty_array("iframes");
    if !has_iframes {
        applicable.insert("2.1".to_string(), false);
    } else {
        applicable.entry("2.1".to_string()).or_insert(true);
    }

    // 4.x media criteria 4.1-4.13
    let has_media = has_non_empty_array("media");
    if !has_media {
        for j in 1..=13 {
            applicable.insert(format!("4.{j}"), false);
        }
    } else {
        for j in 1..=13 {
            applicable.entry(format!("4.{j}")).or_insert(true);
        }
    }

    applicable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_na_detection_no_images() {
        let context = serde_json::json!({
            "images": [],
            "forms": [],
            "iframes": [],
            "media": [],
            "landmarks": []
        });
        let na = detect_na(&context);
        assert_eq!(na.get("1.1"), Some(&false));
        assert_eq!(na.get("1.2"), Some(&false));
        assert_eq!(na.get("1.9"), Some(&false));
        // forms also absent
        assert_eq!(na.get("11.1"), Some(&false));
        // tables absent
        assert_eq!(na.get("5.1"), Some(&false));
        // iframes absent
        assert_eq!(na.get("2.1"), Some(&false));
        // media absent
        assert_eq!(na.get("4.1"), Some(&false));
    }

    #[test]
    fn test_na_detection_with_images() {
        let context = serde_json::json!({
            "images": [{"src": "test.png", "alt": "test"}],
            "forms": [],
            "iframes": [],
            "media": [],
            "landmarks": []
        });
        let na = detect_na(&context);
        assert_eq!(na.get("1.1"), Some(&true));
        assert_eq!(na.get("1.9"), Some(&true));
        // others still NA
        assert_eq!(na.get("11.1"), Some(&false));
    }

    #[test]
    fn test_na_detection_tables_via_landmarks() {
        let context = serde_json::json!({
            "images": [{"src": "a.png"}],
            "forms": [{"id":"f1"}],
            "iframes": [{"src":"i.html"}],
            "media": [{"media_type":"video"}],
            "landmarks": [
                {"tag":"main","role":"main"},
                {"tag":"div","role":"table","label":"Data"}
            ]
        });
        let na = detect_na(&context);
        assert_eq!(na.get("5.1"), Some(&true));
        assert_eq!(na.get("5.8"), Some(&true));
        assert_eq!(na.get("2.1"), Some(&true));
        assert_eq!(na.get("4.1"), Some(&true));
        assert_eq!(na.get("11.1"), Some(&true));
    }

    #[test]
    fn test_na_detection_no_tables() {
        let context = serde_json::json!({
            "images": [{"src":"a.png"}],
            "forms": [{"id":"f1"}],
            "iframes": [{"src":"i.html"}],
            "media": [{"media_type":"video"}],
            "landmarks": [
                {"tag":"main","role":"main"}
            ]
        });
        let na = detect_na(&context);
        assert_eq!(na.get("5.1"), Some(&false));
        assert_eq!(na.get("5.8"), Some(&false));
    }

    #[test]
    fn test_na_detection_missing_fields() {
        let context = serde_json::json!({});
        let na = detect_na(&context);
        // All should be NA
        assert_eq!(na.get("1.1"), Some(&false));
        assert_eq!(na.get("11.13"), Some(&false));
        assert_eq!(na.get("2.1"), Some(&false));
        assert_eq!(na.get("4.13"), Some(&false));
        assert_eq!(na.get("5.8"), Some(&false));
    }
}
