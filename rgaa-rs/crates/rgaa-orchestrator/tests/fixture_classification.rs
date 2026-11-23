//! Classification of the registry's fixtures under Obscura (spec §4 points 6-7,
//! issue #261). Runs in the unconditional `E2E (navigateur)` CI job (Obscura 0.2.4 on
//! PATH, network for the axe-core CDN, no LLM key); skipped unless `RUN_E2E=1`, and
//! `RGAA_E2E_REQUIRED=1` turns that skip into a failure in CI.
//!
//! For every fixture a registry mechanism names:
//! * where the mechanism declares `fail` as an outcome, a `-fail` fixture must make
//!   axe-core or a gap-fix probe emit a `Fail` for the fixture's criterion;
//! * a `-fail` fixture of a review-only mechanism must make that mechanism emit
//!   `NeedsReview`, never `Fail` (spec §4.1, "orienté") — unless a decisive probe
//!   shares its criterion, since gap-fix reports one verdict per criterion;
//! * a `-pass` fixture must not draw a `Fail` from any engine (for a `partial`
//!   mechanism "conforming" means "emits no `fail`" — silence never counts as `pass`);
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
            let dir = dir.clone();
            // One thread per connection: a hook probe keeps its CDP page open, and a serial
            // server would stall every later fixture behind it.
            std::thread::spawn(move || {
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
            });
        }
    });
    Ok(port)
}

struct Case {
    mechanism: String,
    criterion: String,
    fixture: String,
    expect_fail: bool,
    /// `-review` fixture: an adverse case the mechanism itself must flag as `NeedsReview`
    /// (never `Fail`), because the probe cannot prove the failure.
    expect_review: bool,
    /// A criterion-wide `Fail` is only mandatory when the mechanism declares it:
    /// a review-only probe can flag an adverse fixture as `NeedsReview` instead.
    expect_failure: bool,
    /// Statuses the declared mechanism may report on this fixture. A review-only
    /// probe is constrained too, but either a detected failure or an explicit
    /// review request is an acceptable adverse answer.
    expected_statuses: Vec<CriterionStatus>,
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
            let expect_review = f.ends_with("-review");
            if !expect_fail && !expect_review && !f.ends_with("-pass") {
                continue;
            }
            let expect_pass = !expect_fail
                && !expect_review
                && m.coverage == rgaa_core::catalog::AxeCoverage::Complete
                && m.outcomes.contains(&Outcome::Pass);
            let declares_fail = m.outcomes.contains(&Outcome::Fail);
            let declares_review = m.outcomes.contains(&Outcome::Review);
            let expect_failure = expect_fail && declares_fail;
            let expected_statuses = if expect_review {
                vec![CriterionStatus::NeedsReview]
            } else if expect_fail && declares_fail {
                vec![CriterionStatus::Fail]
            } else if expect_fail && declares_review && shares_a_decisive_probe(m) {
                // Gap-fix reports one verdict per criterion: a sibling probe that
                // declares `fail` on the same criterion (12.9) may decide this fixture.
                vec![CriterionStatus::NeedsReview, CriterionStatus::Fail]
            } else if expect_fail && declares_review {
                // Review-only ("orienté", spec §4.1): an adverse fixture is `review`,
                // never `fail`.
                vec![CriterionStatus::NeedsReview]
            } else if expect_pass {
                vec![CriterionStatus::Pass]
            } else if declares_review {
                vec![CriterionStatus::Pass, CriterionStatus::NeedsReview]
            } else {
                Vec::new()
            };
            out.push(Case {
                mechanism: m.id.clone(),
                criterion: m.criterion.clone(),
                fixture: f.clone(),
                expect_fail,
                expect_review,
                expect_failure,
                expected_statuses,
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

/// Retry when another process claims the selected port before Obscura binds it.
async fn start_fixture_browser() -> Result<ObscuraBridge, String> {
    const MAX_RETRIES: usize = 2;

    let mut retries = 0;
    loop {
        let mut bridge = ObscuraBridge::from_env().with_ephemeral_port()?;
        let error = match bridge.start_server().await {
            Ok(()) => return Ok(bridge),
            Err(error) => error,
        };
        let port = bridge.server_port();
        let collision = error.starts_with(&format!("port {port} is already in use"))
            || (error.contains("obscura serve exited during startup")
                && TcpListener::bind(("127.0.0.1", port))
                    .is_err_and(|err| err.kind() == std::io::ErrorKind::AddrInUse));
        if !collision || retries == MAX_RETRIES {
            return Err(error);
        }
        retries += 1;
        tracing::warn!(port, retries, "retrying Obscura after a CDP port collision");
    }
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
    let bridge = start_fixture_browser()
        .await
        .expect("obscura 0.2.4 must start");
    let binary = bridge.binary_path().to_string();
    let bridge = Arc::new(bridge);

    let url_of = |c: &Case| format!("http://127.0.0.1:{port}/{}.html", c.fixture);
    // Several mechanisms may name the same fixture (probe-13-9 and
    // probe-13-9-orientation-render): each page is evaluated once and its result read
    // by every case that names it.
    let mut urls: Vec<String> = cases.iter().map(url_of).collect();
    urls.sort();
    urls.dedup();

    let axe_by_url = bridge
        .clone()
        .run_axe_batch(urls.clone(), 1)
        .await
        .expect("axe batch");
    let snippets: HashMap<String, String> = GapFixRules::snippets()
        .iter()
        .map(|(k, v)| (k.clone(), (*v).to_string()))
        .collect();
    let gap_by_url = ObscuraBridge::run_gap_fix_batch(binary, urls, snippets, 1)
        .await
        .expect("gap-fix batch");

    let mut wrong = Vec::new();
    for case in &cases {
        let url = url_of(case);
        let Some(axe_json) = axe_by_url.get(&url).cloned() else {
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
        let Some(gap_js) = gap_by_url.get(&url).cloned() else {
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
        if case.mechanism == "probe-13-1-script-redirect"
            || case.mechanism == "probe-13-2-popup-hook"
        {
            own_status = None;
            let popup = case.mechanism == "probe-13-2-popup-hook";
            // Hook probe: installed before the page's scripts, ~6 s without interaction.
            match bridge
                .observe_with_hook(
                    &url,
                    if popup {
                        rgaa_rules::script_hooks::POPUP_HOOK
                    } else {
                        rgaa_rules::script_hooks::REDIRECT_HOOK
                    },
                    rgaa_rules::script_hooks::REDIRECT_READ,
                    std::time::Duration::from_millis(
                        rgaa_rules::script_hooks::REDIRECT_OBSERVATION_MS,
                    ),
                )
                .await
            {
                Ok(observation) => {
                    let finding = if popup {
                        rgaa_rules::script_hooks::parse_popup_observation(&observation)
                    } else {
                        rgaa_rules::script_hooks::parse_redirect_observation(&observation)
                    };
                    evidence.push(format!("script-hook: {finding:?}"));
                    if let Some(finding) = finding {
                        // The dedicated observation is this mechanism's own verdict;
                        // it supersedes the generic gap-fix answer on the criterion.
                        own_status = Some(finding.status.into());
                        statuses.push(finding.status.into());
                    }
                }
                Err(error) => {
                    evidence.push(format!("script-hook error: {error}"));
                    wrong.push(format!(
                        "{} / {}: script hook observation failed: {error}",
                        case.mechanism, case.fixture
                    ));
                }
            }
        }
        if case.mechanism == "probe-13-9-orientation-render" {
            own_status = None;
            match bridge
                .observe_viewports(
                    &url,
                    rgaa_rules::script_hooks::ORIENTATION_HOOK,
                    rgaa_rules::script_hooks::ORIENTATION_READ,
                    &rgaa_rules::script_hooks::ORIENTATION_VIEWPORTS,
                )
                .await
            {
                Ok(snapshots) => {
                    let finding =
                        rgaa_rules::script_hooks::parse_orientation_observation(&snapshots);
                    evidence.push(format!("orientation: {finding:?}"));
                    if let Some(finding) = finding {
                        // The dedicated observation is this mechanism's own verdict;
                        // it supersedes the generic gap-fix answer on the criterion.
                        own_status = Some(finding.status.into());
                        statuses.push(finding.status.into());
                    }
                }
                Err(error) => {
                    evidence.push(format!("orientation error: {error}"));
                    wrong.push(format!(
                        "{} / {}: orientation observation failed: {error}",
                        case.mechanism, case.fixture
                    ));
                }
            }
        }
        if case.mechanism == "probe-13-8-script-motion" {
            own_status = None;
            match bridge
                .observe_with_hook(
                    &url,
                    rgaa_rules::script_hooks::MOTION_HOOK,
                    rgaa_rules::script_hooks::MOTION_READ,
                    std::time::Duration::from_millis(
                        rgaa_rules::script_hooks::MOTION_OBSERVATION_MS,
                    ),
                )
                .await
            {
                Ok(observation) => {
                    let finding = rgaa_rules::script_hooks::parse_motion_observation(&observation);
                    evidence.push(format!("motion: {finding:?}"));
                    if let Some(finding) = finding {
                        // The dedicated observation is this mechanism's own verdict;
                        // it supersedes the generic gap-fix answer on the criterion.
                        own_status = Some(finding.status.into());
                        statuses.push(finding.status.into());
                    }
                }
                Err(error) => {
                    evidence.push(format!("motion error: {error}"));
                    wrong.push(format!(
                        "{} / {}: motion observation failed: {error}",
                        case.mechanism, case.fixture
                    ));
                }
            }
        }
        if matches!(
            case.mechanism.as_str(),
            "probe-12-10-listener-inventory"
                | "probe-13-10-listener-inventory"
                | "probe-13-11-listener-inventory"
                | "probe-13-12-listener-inventory"
        ) {
            own_status = None;
            match bridge
                .observe_with_hook(
                    &url,
                    rgaa_rules::script_hooks::LISTENER_HOOK,
                    rgaa_rules::script_hooks::LISTENER_READ,
                    std::time::Duration::from_millis(
                        rgaa_rules::script_hooks::LISTENER_OBSERVATION_MS,
                    ),
                )
                .await
            {
                Ok(observation) => {
                    let result = rgaa_rules::script_hooks::listener_results(&observation);
                    if let Some(result) = result.get(&case.criterion) {
                        own_status = Some(result.status.clone());
                        statuses.push(result.status.clone());
                        evidence.push(format!("listener-hook: {:?}", result.justification));
                    }
                }
                Err(error) => {
                    evidence.push(format!("listener-hook error: {error}"));
                    wrong.push(format!(
                        "{} / {}: listener observation failed: {error}",
                        case.mechanism, case.fixture
                    ));
                }
            }
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
        let silent_review_fixture = !case.expect_fail
            && !case.expect_review
            && !case.expect_pass
            && case
                .expected_statuses
                .contains(&CriterionStatus::NeedsReview)
            && own_status.is_none();
        if !case.expected_statuses.is_empty()
            && !case
                .expected_statuses
                .iter()
                .any(|expected| own_status.as_ref() == Some(expected))
            && !silent_review_fixture
        {
            wrong.push(format!(
                "{} / {}: expected its own mechanism to report one of {:?}, it said {own_status:?} ({})",
                case.mechanism,
                case.fixture,
                case.expected_statuses,
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
        // A `Fail` is mandatory only where the mechanism declares it; a conforming
        // fixture must draw none from any engine on its criterion.
        let fail_expected = case.expect_failure;
        let fail_forbidden = !case.expect_fail;
        if (fail_expected && !failed) || (fail_forbidden && failed) {
            wrong.push(format!(
                "{} / {}: expected {}, mechanisms said {:?} ({})",
                case.mechanism,
                case.fixture,
                if fail_expected { "fail" } else { "no fail" },
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
