//! Classification of the registry's fixtures under Obscura (spec §4 points 6-7,
//! issue #261). Runs in the unconditional `E2E (navigateur)` CI job (Obscura 0.2.4 on
//! PATH, network for the axe-core CDN, no LLM key); skipped unless `RUN_E2E=1`, and
//! `RGAA_E2E_REQUIRED=1` turns that skip into a failure in CI.
//!
//! For every fixture a registry mechanism names (spec §4.1):
//! * a `-fail` fixture must make **that mechanism** (axe for `axe-native`, the gap-fix
//!   probe otherwise) emit `Fail` if it declares `fail`, `NeedsReview` if it is
//!   review-only — never another engine in its place;
//! * a `-pass` fixture must draw no `Fail` from any engine (for a `partial` mechanism
//!   "conforming" means "emits no `fail`" — silence never counts as `pass`);
//! * a `-pass` fixture of a `complete` mechanism that declares `pass` must make **that
//!   mechanism** (axe for `axe-native`, the gap-fix probe otherwise) emit `Pass` (spec
//!   §4.1): its silence is the only one allowed to stand as conformance, so it is proven.
//!
//! A mechanism that misclassifies its fixture blocks the PR. The presence of the
//! fixture files is checked separately, without a browser, in
//! `rgaa-test-corpus/tests/registry_invariants.rs`.

use rgaa_core::{registry::Outcome, CriterionStatus, MechanismKind, MechanismRegistry};
use rgaa_obscura::ObscuraBridge;
use rgaa_rules::{AxeMapper, GapFixRules};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../rgaa-test-corpus/criteria")
}

/// Serve the fixture directory over loopback HTTP: Obscura refuses `file://`.
fn serve(dir: PathBuf) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let name = request
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/')
                .to_string();
            let body = if name.contains("..") || name.contains('/') {
                None
            } else {
                std::fs::read(dir.join(&name)).ok()
            };
            let _ = match body {
                Some(b) => {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n",
                        b.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .and_then(|()| stream.write_all(&b))
                }
                None => stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ),
            };
        }
    });
    Ok(port)
}

struct Case {
    mechanism: String,
    criterion: String,
    fixture: String,
    expect_fail: bool,
    /// On a `-fail` fixture, what the mechanism's own source may report (spec §4.1):
    /// `Fail` for a decisive mechanism, `NeedsReview` for a review-only one.
    accepted_on_fail: Vec<CriterionStatus>,
    /// `complete` with `pass` declared, on a `-pass` fixture: the mechanism's own
    /// source must report `Pass`, not merely stay silent.
    expect_pass: bool,
    axe_native: bool,
    axe_rules: Vec<String>,
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for m in MechanismRegistry::builtin().mechanisms() {
        // Site mechanisms are evaluated by the crawl-level comparator, not by
        // axe/gap-fix on an isolated page. The comparator's logic is unit-tested on
        // synthetic observations in `site_comparison`; their fixture files are not
        // classified anywhere yet (docs/testing-layers.md).
        if m.kind == MechanismKind::Site {
            continue;
        }
        for f in &m.fixtures {
            let expect_fail = f.ends_with("-fail");
            if expect_fail
                && !m.outcomes.contains(&Outcome::Fail)
                && !m.outcomes.contains(&Outcome::Review)
            {
                continue;
            }
            if !expect_fail && !f.ends_with("-pass") {
                continue;
            }
            let expect_pass = !expect_fail
                && m.coverage == rgaa_core::catalog::AxeCoverage::Complete
                && m.outcomes.contains(&Outcome::Pass);
            let accepted_on_fail = if !expect_fail {
                Vec::new()
            } else if m.outcomes.contains(&Outcome::Fail) {
                vec![CriterionStatus::Fail]
            } else if shares_a_decisive_probe(m) {
                // Gap-fix reports one verdict per criterion. When a sibling probe on
                // the same criterion declares `fail`, it may legitimately decide this
                // fixture; review-only on its own, this mechanism never may.
                vec![CriterionStatus::NeedsReview, CriterionStatus::Fail]
            } else {
                vec![CriterionStatus::NeedsReview]
            };
            out.push(Case {
                mechanism: m.id.clone(),
                criterion: m.criterion.clone(),
                fixture: f.clone(),
                expect_fail,
                accepted_on_fail,
                expect_pass,
                axe_native: m.kind == MechanismKind::AxeNative,
                axe_rules: m.axe_rules.clone(),
            });
        }
    }
    out
}

/// Another non-axe, non-site mechanism on the same criterion declares `fail`.
fn shares_a_decisive_probe(m: &rgaa_core::registry::Mechanism) -> bool {
    MechanismRegistry::builtin()
        .mechanisms()
        .iter()
        .any(|other| {
            other.id != m.id
                && other.criterion == m.criterion
                && !matches!(other.kind, MechanismKind::AxeNative | MechanismKind::Site)
                && other.outcomes.contains(&Outcome::Fail)
        })
}

/// The ids of the violations in an axe result, whichever shape the bridge returns:
/// `{"violations": [...], "incomplete": [...]}` under Obscura, or a bare array.
fn violation_ids(axe_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(axe_json) else {
        return Vec::new();
    };
    let violations = match &value {
        serde_json::Value::Array(items) => items.as_slice(),
        serde_json::Value::Object(map) => map
            .get("violations")
            .and_then(serde_json::Value::as_array)
            .map_or(&[][..], Vec::as_slice),
        _ => &[],
    };
    violations
        .iter()
        .filter_map(|v| v.get("id").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect()
}

#[tokio::test]
async fn registry_fixtures_are_classified_under_obscura() {
    if !rgaa_test_corpus::e2e::enabled("fixture_classification") {
        return;
    }
    let cases = cases();
    if cases.is_empty() {
        // Inherited mechanisms are `legacy` and exempt from fixtures (spec §4), so on
        // today's registry nothing is classified. That must never be silent, and a
        // non-legacy mechanism with no classifiable fixture must never pass here.
        let unclassifiable: Vec<&str> = MechanismRegistry::builtin()
            .mechanisms()
            .iter()
            .filter(|m| !m.legacy && m.kind != MechanismKind::Site)
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            unclassifiable.is_empty(),
            "non-legacy mechanisms with no `-pass`/`-fail` fixture to classify: {unclassifiable:?}"
        );
        eprintln!(
            "NOTHING CLASSIFIED: every registry mechanism is legacy (exempt from fixtures, \
             spec §4); this run validates no classification under Obscura"
        );
        return;
    }

    // The fixtures are served on 127.0.0.1, which Obscura blocks by default (SSRF
    // guard): without this, `obscura scrape` gives up in ~3 ms and the gap-fix batch
    // fails with "missing 'eval'". Child processes inherit the variable.
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");

    let port = serve(corpus_dir()).expect("fixture server must bind");
    // Never the fixed 9222: this test now runs in `E2E (navigateur)` beside the
    // coverage tests, and a shared CDP port lets concurrent runs steal each other's
    // server (CLAUDE.md, #382).
    //
    // `with_ephemeral_port` releases the port it picked before Obscura binds it, so
    // another process can take it in between; `start_server` then refuses the port.
    // That is a lost race, not an Obscura fault: retry on a fresh port.
    let mut attempts = 0;
    let bridge = loop {
        attempts += 1;
        let mut bridge = ObscuraBridge::from_env()
            .with_ephemeral_port()
            .expect("a free CDP port");
        match bridge.start_server().await {
            Ok(()) => break bridge,
            Err(error) if attempts < 3 && error.contains("already in use") => {
                eprintln!(
                    "CDP port {} lost to another process, retrying: {error}",
                    bridge.server_port()
                );
            }
            Err(error) => panic!("obscura 0.2.4 must start: {error}"),
        }
    };
    let binary = bridge.binary_path().to_string();
    let bridge = Arc::new(bridge);

    let url_of = |c: &Case| format!("http://127.0.0.1:{port}/{}.html", c.fixture);
    let urls: Vec<String> = cases.iter().map(url_of).collect();

    let mut axe_by_url = bridge
        .clone()
        .run_axe_batch(urls.clone(), 1)
        .await
        .expect("axe batch");
    let snippets: HashMap<String, String> = GapFixRules::snippets()
        .iter()
        .map(|(k, v)| (k.clone(), (*v).to_string()))
        .collect();
    let mut gap_by_url = ObscuraBridge::run_gap_fix_batch(binary, urls, snippets, 1)
        .await
        .expect("gap-fix batch");

    let mut wrong = Vec::new();
    for case in &cases {
        let url = url_of(case);
        let Some(axe_json) = axe_by_url.remove(&url) else {
            wrong.push(format!(
                "{} / {}: axe batch returned no result for {url}",
                case.mechanism, case.fixture
            ));
            continue;
        };
        let mut statuses: Vec<CriterionStatus> = Vec::new();
        let mut own_status: Option<CriterionStatus> = None;
        let mut evidence = Vec::new();
        match AxeMapper::map(&axe_json) {
            Ok(axe) => {
                if let Some(result) = axe.get(&case.criterion) {
                    statuses.push(result.status.clone());
                    if case.axe_native {
                        own_status = Some(result.status.clone());
                    }
                    evidence.push(format!(
                        "axe: {:?} {:?}",
                        result.status, result.justification
                    ));
                } else {
                    evidence.push(format!(
                        "axe had no mapped result; raw violations: {axe_json}"
                    ));
                }
            }
            Err(error) => evidence.push(format!("axe parse error: {error}; raw: {axe_json}")),
        }
        let Some(gap_js) = gap_by_url.remove(&url) else {
            wrong.push(format!(
                "{} / {}: gap-fix batch returned no result for {url}",
                case.mechanism, case.fixture
            ));
            continue;
        };
        if let Some(result) = GapFixRules::parse_results(&gap_js).get(&case.criterion) {
            statuses.push(result.status.clone());
            if !case.axe_native {
                own_status = Some(result.status.clone());
            }
            evidence.push(format!(
                "gap-fix: {:?} {:?}",
                result.status, result.justification
            ));
        }
        let failed = statuses.iter().any(|s| matches!(s, CriterionStatus::Fail));
        if case.axe_native && case.expect_fail {
            let declared_rule_fired = violation_ids(&axe_json)
                .iter()
                .any(|id| case.axe_rules.contains(id));
            if !declared_rule_fired {
                wrong.push(format!(
                    "{} / {}: expected this axe mechanism to fail, but none of its declared rules fired ({})",
                    case.mechanism,
                    case.fixture,
                    evidence.join("; ")
                ));
            }
        }
        // A `-fail` fixture is judged on the mechanism's own source only: another
        // engine failing the same criterion must not classify it in its place.
        if case.expect_fail
            && !own_status
                .as_ref()
                .is_some_and(|s| case.accepted_on_fail.contains(s))
        {
            wrong.push(format!(
                "{} / {}: expected its own mechanism to report one of {:?}, it said {own_status:?} ({})",
                case.mechanism,
                case.fixture,
                case.accepted_on_fail,
                evidence.join("; ")
            ));
        }
        if case.expect_pass && own_status != Some(CriterionStatus::Pass) {
            wrong.push(format!(
                "{} / {}: complete mechanism expected Pass, it said {own_status:?} ({})",
                case.mechanism,
                case.fixture,
                evidence.join("; ")
            ));
        }
        // A conforming fixture must draw no `Fail` from any engine on its criterion.
        if !case.expect_fail && failed {
            wrong.push(format!(
                "{} / {}: expected no fail, mechanisms said {:?} ({})",
                case.mechanism,
                case.fixture,
                statuses,
                evidence.join("; ")
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "fixtures misclassified under Obscura:\n  {}",
        wrong.join("\n  ")
    );
}
