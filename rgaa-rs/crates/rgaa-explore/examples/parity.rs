//! Dev-only AX / screenshot parity study.
//!
//! Compares agent-browser's Obscura adapter against the audit-side
//! `ObscuraBridge` on a corpus of public and local fixture pages. Results are
//! a go/no-go input for exposing the exploration tool over MCP; they are not
//! audit evidence and do not establish Chrome parity on their own.

use base64::Engine;
use rgaa_explore::{ExploreOptions, ObservationKind};
use rgaa_obscura::ObscuraBridge;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Page {
    url: String,
    category: String,
    #[serde(default)]
    expected: Vec<String>,
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn json_string(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Object(map) => map.get("value").and_then(Value::as_str),
        _ => None,
    }
}

fn collect_ax_names(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let (Some(role), Some(name)) = (map.get("role"), map.get("name")) {
                if let (Some(role), Some(name)) = (json_string(role), json_string(name)) {
                    let name = normalize(name);
                    if !name.is_empty() {
                        out.insert(format!("{}:{}", role.to_ascii_lowercase(), name));
                    }
                }
            }
            for child in map.values() {
                collect_ax_names(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_ax_names(child, out);
            }
        }
        _ => {}
    }
}

fn snapshot_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in text.lines() {
        let body = line.trim_start().trim_start_matches('-').trim_start();
        let role = match body.split_whitespace().next() {
            Some(role) => role,
            None => continue,
        };
        let remainder = body[role.len()..].trim_start();
        if !remainder.starts_with('"') {
            continue;
        }
        if let Some(end) = remainder[1..].find('"') {
            let name = normalize(&remainder[1..1 + end]);
            if !name.is_empty() {
                names.insert(format!("{}:{}", role.to_ascii_lowercase(), name));
            }
        }
    }
    names
}

fn png_dimensions(path: &Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    Some((
        u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
    ))
}

fn jaccard(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let intersection = left.intersection(right).count();
    let union = left.union(right).count();
    intersection as f64 / union as f64
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus_path = args
        .get(1)
        .map(PathBuf::from)
        .expect("usage: parity <corpus.json> <report.json> <obscura-binary>");
    let report_path = args
        .get(2)
        .map(PathBuf::from)
        .expect("usage: parity <corpus.json> <report.json> <obscura-binary>");
    let executable = args
        .get(3)
        .cloned()
        .expect("usage: parity <corpus.json> <report.json> <obscura-binary>");

    let corpus: Vec<Page> =
        serde_json::from_str(&std::fs::read_to_string(&corpus_path).expect("read corpus"))
            .expect("parse corpus");
    std::env::set_var("RGAA_OBSCURA_BIN", &executable);
    // The corpus deliberately includes local fixtures; both engines need the
    // explicit private-network opt-in before a server is started.
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");

    let mut bridge = ObscuraBridge::from_env();
    if let Err(error) = bridge.start_server().await {
        eprintln!("ObscuraBridge start_server failed: {error}");
        return;
    }

    let shots_root = report_path
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"))
        .join("shots");
    let mut pages = Vec::new();
    for (index, page) in corpus.iter().enumerate() {
        let mut options = ExploreOptions {
            url: page.url.clone(),
            executable_path: PathBuf::from(&executable),
            include: vec![ObservationKind::Snapshot, ObservationKind::Screenshot],
            allow_private_network: true,
            artifact_dir: Some(shots_root.join(index.to_string())),
            viewport: Some((1920, 1080)),
            timeout_ms: 25_000,
            ..ExploreOptions::default()
        };
        let agent = rgaa_explore::explore(options.clone()).await;
        let (agent_names, agent_dims, agent_error) = match &agent {
            Ok(result) => {
                let names = result
                    .observations()
                    .snapshot
                    .as_ref()
                    .map(|snapshot| snapshot_names(&snapshot.text))
                    .unwrap_or_default();
                let dims = result
                    .observations()
                    .screenshot
                    .as_ref()
                    .and_then(|shot| png_dimensions(Path::new(&shot.path)))
                    .map(|(width, height)| json!({"width": width, "height": height}));
                (names, dims, None)
            }
            Err(error) => (BTreeSet::new(), None, Some(error.to_string())),
        };

        options.include.clear();
        let bridge_ax = bridge.get_accessibility_tree(&page.url).await;
        let bridge_shot = bridge.screenshot_with_dimensions(&page.url).await;
        let agent_shot_path = agent.as_ref().ok().and_then(|result| {
            result
                .observations()
                .screenshot
                .as_ref()
                .map(|shot| shot.path.clone())
        });
        let (bridge_names, bridge_dims, bridge_shot_path, bridge_error) = match &bridge_ax {
            Ok(tree) => {
                let mut names = BTreeSet::new();
                collect_ax_names(tree, &mut names);
                let shot_dir = shots_root.join(index.to_string());
                let (dims, shot_path) = match bridge_shot.as_ref() {
                    Ok((data, width, height)) => {
                        let path = shot_dir.join("bridge.png");
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD
                            .decode(data.trim_start_matches("data:image/png;base64,"))
                        {
                            let _ = std::fs::create_dir_all(&shot_dir);
                            let _ = std::fs::write(&path, bytes);
                        }
                        (
                            Some(json!({"width": width, "height": height})),
                            Some(path.to_string_lossy().into_owned()),
                        )
                    }
                    Err(_) => (None, None),
                };
                let err = bridge_shot.as_ref().err().map(ToString::to_string);
                (names, dims, shot_path, err)
            }
            Err(error) => (BTreeSet::new(), None, None, Some(error.clone())),
        };

        let missing_in_agent: Vec<String> =
            bridge_names.difference(&agent_names).cloned().collect();
        let missing_in_bridge: Vec<String> =
            agent_names.difference(&bridge_names).cloned().collect();
        let expected_found: BTreeSet<String> = page
            .expected
            .iter()
            .filter(|name| {
                let needle = normalize(name);
                agent_names.iter().chain(bridge_names.iter()).any(|entry| {
                    entry
                        .rsplit_once(':')
                        .is_some_and(|(_, n)| n.contains(&needle))
                })
            })
            .cloned()
            .collect();

        let expected_coverage = if page.expected.is_empty() {
            1.0
        } else {
            expected_found.len() as f64 / page.expected.len() as f64
        };

        pages.push(json!({
            "url": page.url,
            "category": page.category,
            "agent_names": agent_names,
            "bridge_names": bridge_names,
            "jaccard": jaccard(&agent_names, &bridge_names),
            "missing_in_agent": missing_in_agent,
            "missing_in_bridge": missing_in_bridge,
            "agent_screenshot": agent_dims,
            "bridge_screenshot": bridge_dims,
            "agent_shot_path": agent_shot_path,
            "bridge_shot_path": bridge_shot_path,
            "expected_found": expected_found,
            "expected_total": page.expected.len(),
            "expected_coverage": expected_coverage,
            "agent_error": agent_error,
            "bridge_error": bridge_error,
        }));
        eprintln!(
            "{} -> jaccard {:.2}",
            page.url,
            pages.last().unwrap()["jaccard"]
        );
    }

    let successful: Vec<&Value> = pages
        .iter()
        .filter(|page| page["agent_error"].is_null() && page["bridge_error"].is_null())
        .collect();
    let mean_jaccard = if successful.is_empty() {
        0.0
    } else {
        successful
            .iter()
            .map(|page| page["jaccard"].as_f64().unwrap())
            .sum::<f64>()
            / successful.len() as f64
    };
    let screenshot_matches = successful
        .iter()
        .filter(|page| !page["agent_screenshot"].is_null() && !page["bridge_screenshot"].is_null())
        .filter(|page| page["agent_screenshot"] == page["bridge_screenshot"])
        .count();
    let screenshots_compared = successful
        .iter()
        .filter(|page| !page["agent_screenshot"].is_null() && !page["bridge_screenshot"].is_null())
        .count();

    let expected_total: usize = successful
        .iter()
        .map(|page| page["expected_total"].as_u64().unwrap_or(0) as usize)
        .sum();
    let expected_found: usize = successful
        .iter()
        .map(|page| page["expected_found"].as_array().map_or(0, Vec::len))
        .sum();
    let expected_coverage = if expected_total == 0 {
        1.0
    } else {
        expected_found as f64 / expected_total as f64
    };

    let report = json!({
        "pinned": {"obscura": "obscura 0.2.4", "agent_browser": "0.45.0"},
        "summary": {
            "pages": pages.len(),
            "successful": successful.len(),
            "mean_jaccard": mean_jaccard,
            "screenshot_dimensions_match": screenshot_matches,
            "screenshot_dimensions_compared": screenshots_compared,
            "expected_name_coverage": expected_coverage,
            "mcp_gate": {
                "threshold_mean_jaccard": 0.6,
                "threshold_expected_coverage": 0.8,
                "candidate": mean_jaccard >= 0.6 && expected_coverage >= 0.8
            }
        },
        "pages": pages
    });
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap())
        .expect("write report");
    println!("wrote {}", report_path.display());
}
