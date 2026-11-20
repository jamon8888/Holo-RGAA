//! Classification of the registry's fixtures under Obscura (spec §4 point 1, points 6-7,
//! issue #261). Runs in the unconditional `E2E (navigateur)` CI job (Obscura 0.2.4 on
//! PATH, network for the axe-core CDN, no LLM key); skipped unless `RUN_E2E=1`, and
//! `RGAA_E2E_REQUIRED=1` turns that skip into a failure in CI.
//!
//! The expected outcome depends on the mechanism's class (spec §4 point 1), and it is
//! checked on **the mechanism's own source** — axe for `axe-native`, the criterion's
//! gap-fix probe otherwise — never on whatever else happens to speak for the criterion:
//! * decisive (`fail` declared): a `-fail` fixture must make that source emit `Fail`
//!   (for `axe-native`, through one of the mechanism's declared `axe_rules`);
//! * oriented (`review` only): a `-fail` fixture must make that source emit
//!   `NeedsReview` — never `Fail` — and a `-pass` fixture must make it emit neither;
//! * any `-pass` fixture: **no** source (axe or gap-fix) may emit `Fail` for the
//!   criterion — for a `partial` mechanism "conforming" means "emits no `fail`", silence
//!   never counts as `pass`;
//! * `complete` with `pass` declared, on a `-pass` fixture: the mechanism's own source
//!   must emit `Pass` (spec §4.1): its silence is the only one allowed to stand as
//!   conformance, so it is proven. On today's registry the only non-legacy mechanism in
//!   that class is `axe-8-5` (fixture `8.5-document-title-pass`): the eight `complete`
//!   gap-fix probes are all `legacy`, without fixtures, so the gap-fix half of this
//!   branch is exercised by no fixture yet.
//!
//! Every fixture URL must come back from **both** batches: a page axe or the gap-fix
//! script could not run on is a failure, never an empty result — an empty axe output
//! would have `AxeMapper` fabricate a `Pass` for every `complete` criterion.
//!
//! Limits, stated rather than hidden:
//! * the gap-fix batch returns one result per **criterion**, not per mechanism. Where
//!   two probe mechanisms share a criterion (today 4.12 and 12.9: the inventory and
//!   the keydown sweep, merged into one snippet), the result cannot be attributed to
//!   one of them; the check is on the criterion's merged probe;
//! * `site` mechanisms (12.1, 12.2, 12.4, 12.5) are not classified here: their verdict
//!   compares several pages, which this single-page harness does not crawl. Their
//!   comparator is tested on observations built in Rust (unit tests of
//!   `rgaa-orchestrator/src/site_comparison.rs`), **not** on their HTML fixtures under
//!   Obscura: those eight fixtures are classified by nothing yet. A multipage Obscura
//!   run is tracked by #389; see `docs/testing-layers.md`.
//!
//! A mechanism that misclassifies its fixture blocks the PR. The presence of the
//! fixture files is checked separately, without a browser, in
//! `rgaa-test-corpus/tests/registry_invariants.rs`.

use rgaa_core::catalog::AxeCoverage;
use rgaa_core::registry::Outcome;
use rgaa_core::{CriterionResult, CriterionStatus, MechanismKind, MechanismRegistry};
use rgaa_obscura::ObscuraBridge;
use rgaa_rules::{AxeMapper, GapFixRules};
use std::collections::{BTreeSet, HashMap};
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

/// What the mechanism's own source must say on the fixture (spec §4 point 1).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    /// Decisive mechanism, `-fail` fixture: `Fail`.
    Fail,
    /// Oriented mechanism, `-fail` fixture: `NeedsReview`, never `Fail`.
    Review,
    /// `complete` mechanism declaring `pass`, `-pass` fixture: `Pass`.
    Pass,
    /// Oriented mechanism, `-pass` fixture: neither `Fail` nor `NeedsReview`.
    Clean,
    /// Decisive `partial` mechanism, `-pass` fixture: no `Fail` (silence is no `pass`).
    NoFail,
}

struct Case {
    mechanism: String,
    criterion: String,
    fixture: String,
    expect: Expect,
    axe_native: bool,
    /// The mechanism's declared axe rules: an `axe-native` `Fail` must come from one.
    axe_rules: Vec<String>,
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for m in MechanismRegistry::builtin().mechanisms() {
        // Site mechanisms are evaluated by the crawl-level comparator over several
        // pages, not by axe/gap-fix on an isolated page: see the module doc.
        if m.kind == MechanismKind::Site {
            continue;
        }
        let decisive = m.outcomes.contains(&Outcome::Fail);
        let oriented = !decisive
            && !m.outcomes.contains(&Outcome::Pass)
            && m.outcomes.contains(&Outcome::Review);
        for f in &m.fixtures {
            let expect = if f.ends_with("-fail") {
                if decisive {
                    Expect::Fail
                } else if oriented {
                    Expect::Review
                } else {
                    // Unreachable today (`pass` requires `complete`, which carries
                    // `fail`); kept explicit so a new class cannot slip through.
                    panic!(
                        "{} declares no outcome a `-fail` fixture can be checked against: {:?}",
                        m.id, m.outcomes
                    );
                }
            } else if f.ends_with("-pass") {
                if m.coverage == AxeCoverage::Complete && m.outcomes.contains(&Outcome::Pass) {
                    Expect::Pass
                } else if oriented {
                    Expect::Clean
                } else {
                    Expect::NoFail
                }
            } else {
                continue;
            };
            out.push(Case {
                mechanism: m.id.clone(),
                criterion: m.criterion.clone(),
                fixture: f.clone(),
                expect,
                axe_native: m.kind == MechanismKind::AxeNative,
                axe_rules: m.axe_rules.clone(),
            });
        }
    }
    out
}

fn describe(source: &str, result: &CriterionResult) -> String {
    let rules: Vec<&str> = result
        .violations
        .iter()
        .map(|v| v.rule_id.as_str())
        .collect();
    format!(
        "{source}: {:?} rules {rules:?} {:?}",
        result.status, result.justification
    )
}

/// The verdict errors of one case, given what each source said about its criterion.
fn misclassifications(
    case: &Case,
    axe: Option<&CriterionResult>,
    gap: Option<&CriterionResult>,
) -> Vec<String> {
    let own = if case.axe_native { axe } else { gap };
    let own_status = own.map(|r| r.status.clone());
    let mut errors = Vec::new();
    match case.expect {
        Expect::Fail => {
            if own_status != Some(CriterionStatus::Fail) {
                errors.push(format!(
                    "expected the mechanism's own source to say Fail, it said {own_status:?}"
                ));
            } else if case.axe_native {
                // The criterion's axe result only carries its mapped rules, but the
                // mapping is the registry's: check the Fail really is this mechanism's.
                let foreign: Vec<&str> = own
                    .into_iter()
                    .flat_map(|r| r.violations.iter())
                    .map(|v| v.rule_id.as_str())
                    .filter(|rule| !case.axe_rules.iter().any(|r| r == rule))
                    .collect();
                let declared = own
                    .into_iter()
                    .flat_map(|r| r.violations.iter())
                    .any(|v| case.axe_rules.iter().any(|r| *r == v.rule_id));
                if !declared || !foreign.is_empty() {
                    errors.push(format!(
                        "Fail not produced by the declared axe rules {:?} (foreign: {foreign:?})",
                        case.axe_rules
                    ));
                }
            }
        }
        Expect::Review => {
            if own_status != Some(CriterionStatus::NeedsReview) {
                errors.push(format!(
                    "oriented mechanism expected NeedsReview (never Fail), it said {own_status:?}"
                ));
            }
        }
        Expect::Pass => {
            if own_status != Some(CriterionStatus::Pass) {
                errors.push(format!(
                    "complete mechanism expected Pass, it said {own_status:?}"
                ));
            }
        }
        Expect::Clean => {
            if matches!(
                own_status,
                Some(CriterionStatus::Fail | CriterionStatus::NeedsReview)
            ) {
                errors.push(format!(
                    "oriented mechanism expected neither Fail nor NeedsReview, it said \
                     {own_status:?}"
                ));
            }
        }
        Expect::NoFail => {}
    }
    // A conforming fixture: no source at all may fail the criterion.
    if matches!(case.expect, Expect::Pass | Expect::Clean | Expect::NoFail) {
        for (source, result) in [("axe", axe), ("gap-fix", gap)] {
            if result.is_some_and(|r| r.status == CriterionStatus::Fail) {
                errors.push(format!("{source} failed a conforming fixture"));
            }
        }
    }
    errors
}

#[tokio::test]
async fn registry_fixtures_are_classified_under_obscura() {
    if !rgaa_test_corpus::e2e::enabled("fixture_classification") {
        return;
    }
    let cases = cases();
    if cases.is_empty() {
        // Inherited mechanisms are `legacy` and exempt from fixtures (spec §4), so on
        // such a registry nothing is classified. That must never be silent, and a
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
    rgaa_test_corpus::e2e::allow_private_network();

    let port = serve(corpus_dir()).expect("fixture server must bind");
    // Never the fixed 9222: this test now runs in `E2E (navigateur)` beside the
    // coverage tests, and a shared CDP port lets concurrent runs steal each other's
    // server (CLAUDE.md, #382).
    let mut bridge = ObscuraBridge::from_env()
        .with_ephemeral_port()
        .expect("a free CDP port");
    bridge
        .start_server()
        .await
        .expect("obscura 0.2.4 must start");
    let binary = bridge.binary_path().to_string();
    let bridge = Arc::new(bridge);

    let url_of = |c: &Case| format!("http://127.0.0.1:{port}/{}.html", c.fixture);
    // Each page is audited once, whatever number of cases read it.
    let urls: Vec<String> = cases
        .iter()
        .map(url_of)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let axe_by_url = bridge
        .clone()
        .run_axe_batch(urls.clone(), 1)
        .await
        .expect("axe batch");
    let snippets: HashMap<String, String> = GapFixRules::snippets()
        .iter()
        .map(|(k, v)| (k.clone(), (*v).to_string()))
        .collect();
    let gap_by_url = ObscuraBridge::run_gap_fix_batch(binary, urls.clone(), snippets, 1)
        .await
        .expect("gap-fix batch");

    // A page with no result is a failure, never an empty output: `AxeMapper::map("[]")`
    // would fabricate a `Pass` for every complete criterion. The batch logs the reason
    // as a `warn!` (captured by the test subscriber).
    let missing: Vec<String> = urls
        .iter()
        .flat_map(|url| {
            let mut gone = Vec::new();
            if !axe_by_url.contains_key(url) {
                gone.push(format!("axe: {url}"));
            }
            if !gap_by_url.contains_key(url) {
                gone.push(format!("gap-fix: {url}"));
            }
            gone
        })
        .collect();
    assert!(
        missing.is_empty(),
        "no result for {} page(s) — the run proves nothing about them:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );

    let mut wrong = Vec::new();
    for case in &cases {
        let url = url_of(case);
        let axe_json = &axe_by_url[&url];
        let axe = match AxeMapper::map(axe_json) {
            Ok(axe) => axe,
            Err(error) => {
                wrong.push(format!(
                    "{} / {}: axe output does not parse: {error}; raw: {axe_json}",
                    case.mechanism, case.fixture
                ));
                continue;
            }
        };
        let gap_raw = &gap_by_url[&url];
        // A snippet that threw comes back as `{success: false, error}`, which
        // `parse_results` would read as a legacy `pass: false`, i.e. a `Fail`.
        if let Some(error) = gap_raw
            .get(&case.criterion)
            .filter(|v| v.get("success").and_then(serde_json::Value::as_bool) == Some(false))
            .and_then(|v| v.get("error"))
        {
            wrong.push(format!(
                "{} / {}: the {} gap-fix snippet threw: {error}",
                case.mechanism, case.fixture, case.criterion
            ));
            continue;
        }
        let gap = GapFixRules::parse_results(gap_raw);
        let axe_result = axe.get(&case.criterion);
        let gap_result = gap.get(&case.criterion);
        let errors = misclassifications(case, axe_result, gap_result);
        if !errors.is_empty() {
            let evidence: Vec<String> = [("axe", axe_result), ("gap-fix", gap_result)]
                .into_iter()
                .map(|(source, r)| match r {
                    Some(r) => describe(source, r),
                    None => format!("{source}: no result"),
                })
                .collect();
            wrong.push(format!(
                "{} / {} ({:?}): {} [{}]",
                case.mechanism,
                case.fixture,
                case.expect,
                errors.join("; "),
                evidence.join("; ")
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} fixture(s) misclassified under Obscura:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}
