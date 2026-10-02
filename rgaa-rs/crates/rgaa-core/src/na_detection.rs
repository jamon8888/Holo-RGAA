use std::collections::HashMap;
use std::sync::OnceLock;

/// Themes the table covers, and criteria per theme.
const THEMES: usize = 13;
const CRITERIA_PER_THEME: usize = 15;

/// The ids the table is keyed by: all 13 themes × criteria 1-15, formatted once per
/// process. The table itself is per-page, but its keys never change, so an audit no
/// longer allocates 195 strings to say "everything is applicable" before narrowing.
fn table_ids() -> &'static [String] {
    static IDS: OnceLock<Vec<String>> = OnceLock::new();
    IDS.get_or_init(|| {
        (1..=THEMES)
            .flat_map(|theme| (1..=CRITERIA_PER_THEME).map(move |crit| format!("{theme}.{crit}")))
            .collect()
    })
}

/// `"{theme}.{crit}"` from the id table, by position — no formatting, and no
/// scan. `None` outside the table's range, which is also what the pre-index
/// behavior amounted to: a key no caller of this map ever asks about.
fn table_id(theme: usize, crit: usize) -> Option<&'static str> {
    if theme == 0 || theme > THEMES || crit == 0 || crit > CRITERIA_PER_THEME {
        return None;
    }
    table_ids()
        .get((theme - 1) * CRITERIA_PER_THEME + (crit - 1))
        .map(String::as_str)
}

/// Detect which RGAA criteria are not applicable based on page context.
///
/// Returns a map `criterion_id -> applicable` where `false` means Not Applicable.
pub fn detect_na(page_context: &serde_json::Value) -> HashMap<&'static str, bool> {
    // Default: assume all criteria are applicable.
    let mut applicable: HashMap<&'static str, bool> =
        table_ids().iter().map(|id| (id.as_str(), true)).collect();

    // Narrowing below re-inserts under the *same* key the default pass created,
    // taken from the id table by position.
    let mut mark_na = |theme: usize, crit: usize| {
        if let Some(id) = table_id(theme, crit) {
            applicable.insert(id, false);
        }
    };

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
            mark_na(1, j);
        }
    }

    // 11.x forms criteria 11.1-11.13
    let has_forms = has_non_empty_array("forms");
    if !has_forms {
        for j in 1..=13 {
            mark_na(11, j);
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
            mark_na(5, j);
        }
    }

    // 2.1 iframes
    let has_iframes = has_non_empty_array("iframes");
    if !has_iframes {
        mark_na(2, 1);
    }

    // 4.x media criteria 4.1-4.13
    let has_media = has_non_empty_array("media");
    if !has_media {
        for j in 1..=13 {
            mark_na(4, j);
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
        // Covered criteria should be NA
        assert_eq!(na.get("1.1"), Some(&false));
        assert_eq!(na.get("11.13"), Some(&false));
        assert_eq!(na.get("2.1"), Some(&false));
        assert_eq!(na.get("4.13"), Some(&false));
        assert_eq!(na.get("5.8"), Some(&false));
        // Uncovered criteria should be present and true
        assert_eq!(na.get("3.2"), Some(&true));
        assert_eq!(na.get("6.1"), Some(&true));
        // Map should contain all 13 themes × 15 criteria
        assert!(na.len() >= 195);
    }

    /// The keys are formatted once per process and handed out by position, so
    /// narrowing a criterion must land on the key the default pass created
    /// rather than add a second one (#43).
    #[test]
    fn narrowing_reuses_the_default_keys() {
        let empty = detect_na(&serde_json::json!({}));
        let full = detect_na(&serde_json::json!({
            "images":[{"src":"a.png"}],
            "forms":[{"id":"f1"}],
            "iframes":[{"src":"i.html"}],
            "media":[{"media_type":"video"}],
            "landmarks":[{"tag":"div","role":"table"}]
        }));
        assert_eq!(empty.len(), THEMES * CRITERIA_PER_THEME);
        assert_eq!(empty.len(), full.len());

        // The audit path looks a criterion up by its owned `criterion_id`.
        let owned = String::from("1.1");
        assert_eq!(empty.get(owned.as_str()), Some(&false));

        let mut ids: Vec<&str> = empty.keys().copied().collect();
        ids.sort_unstable();
        let mut expected: Vec<&str> = table_ids().iter().map(String::as_str).collect();
        expected.sort_unstable();
        assert_eq!(ids, expected);
    }
}
