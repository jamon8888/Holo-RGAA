//! Classification of the registry's fixtures under Obscura (spec §4 points 6-7,
//! issue #261). Runs in the E2E job (Obscura 0.2.2 on PATH, network for the axe-core
//! CDN); skipped unless `RUN_E2E=1`.
//!
//! For every fixture a registry mechanism names:
//! * where the mechanism declares `fail` as an outcome, a `-fail` fixture must make
//!   axe-core or a gap-fix probe emit a `Fail` for the fixture's criterion;
//! * a `-pass` fixture must not (for a `partial` mechanism "conforming" means "emits no
//!   `fail`" — silence never counts as `pass`).
//!
//! A mechanism that misclassifies its fixture blocks the PR. The presence of the
//! fixture files is checked separately, without a browser, in
//! `rgaa-test-corpus/tests/registry_invariants.rs`.

use rgaa_core::{CriterionStatus, MechanismKind, MechanismRegistry};
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
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for m in MechanismRegistry::builtin().mechanisms() {
        // Site mechanisms are evaluated by the crawl-level comparator, not by
        // axe/gap-fix on an isolated page. Their paired page observations are
        // covered by `site_comparison` tests instead of this single-page harness.
        if m.kind == MechanismKind::Site {
            continue;
        }
        for f in &m.fixtures {
            let expect_fail = f.ends_with("-fail");
            // A review-only inventory can show a candidate, but by contract it
            // cannot classify even a deliberately adverse single-page fixture.
            // Its non-failing signal is checked through the normal `-pass` case;
            // site mechanisms use the separate crawl-level comparator tests.
            if expect_fail && !m.outcomes.contains(&rgaa_core::registry::Outcome::Fail) {
                continue;
            }
            if !expect_fail && !f.ends_with("-pass") {
                continue;
            }
            out.push(Case {
                mechanism: m.id.clone(),
                criterion: m.criterion.clone(),
                fixture: f.clone(),
                expect_fail,
            });
        }
    }
    out
}

#[tokio::test]
async fn registry_fixtures_are_classified_under_obscura() {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("skipping fixture classification (set RUN_E2E=1 to enable)");
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
            .filter(|m| !m.legacy)
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
    let mut bridge = ObscuraBridge::from_env();
    bridge
        .start_server()
        .await
        .expect("obscura 0.2.2 must start");
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
        let axe_json = axe_by_url.remove(&url).unwrap_or_else(|| "[]".to_string());
        let mut statuses: Vec<CriterionStatus> = Vec::new();
        let mut evidence = Vec::new();
        match AxeMapper::map(&axe_json) {
            Ok(axe) => {
                if let Some(result) = axe.get(&case.criterion) {
                    statuses.push(result.status.clone());
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
        let gap_js = gap_by_url.remove(&url).unwrap_or_default();
        if let Some(result) = GapFixRules::parse_results(&gap_js).get(&case.criterion) {
            statuses.push(result.status.clone());
            evidence.push(format!(
                "gap-fix: {:?} {:?}",
                result.status, result.justification
            ));
        }
        let failed = statuses.iter().any(|s| matches!(s, CriterionStatus::Fail));
        if failed != case.expect_fail {
            wrong.push(format!(
                "{} / {}: expected {}, mechanisms said {:?} ({})",
                case.mechanism,
                case.fixture,
                if case.expect_fail { "fail" } else { "no fail" },
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
