//! The per-file time budget from #166: under 100 ms for a typical TSX, Vue or
//! HTML file.
//!
//! This test measures rather than asserts a number pulled from intuition, and
//! it prints what it measured (`cargo nextest run -p rgaa-linter --no-capture`)
//! so the figure quoted in a review is one someone actually observed.
//!
//! The assertion threshold is deliberately far above the observed cost. A tight
//! timing assertion is the classic flaky CI test: a shared runner under load can
//! stall a thread for tens of milliseconds for reasons that have nothing to do
//! with this code, and a linter test that fails at random gets muted. The guard
//! below therefore catches an algorithmic regression — something quadratic
//! creeping into the scanner — and nothing finer.

use rgaa_linter::{lint_sources, LintConfig, LintOptions, Profile, Source};
use std::path::Path;
use std::time::Instant;

/// Well below the 100 ms requirement but orders of magnitude above the cost
/// measured on a developer machine (tens of microseconds per file).
const GUARD_MS: u128 = 25;

const ITERATIONS: u32 = 50;

fn read(name: &str) -> Source {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    Source {
        path: name.to_string(),
        content: std::fs::read_to_string(path).expect("fixture readable"),
    }
}

fn options() -> LintOptions {
    LintOptions {
        profile: Profile::Rgaa41,
        config: LintConfig::default(),
        config_source: "test".into(),
    }
}

/// Median of `ITERATIONS` runs, in microseconds. The median, not the mean, so a
/// single scheduler stall does not decide the result.
fn median_micros(source: &Source) -> u128 {
    let options = options();
    let sources = [source.clone()];
    let mut samples: Vec<u128> = (0..ITERATIONS)
        .map(|_| {
            let started = Instant::now();
            let report = lint_sources(&sources, &options).expect("lint");
            let elapsed = started.elapsed().as_micros();
            // Keep the optimiser from removing the work being timed.
            assert!(report.files.len() == 1);
            elapsed
        })
        .collect();
    samples.sort_unstable();
    samples[samples.len() / 2]
}

#[test]
fn a_typical_source_file_lints_well_inside_the_hundred_millisecond_budget() {
    for name in ["sample.tsx", "sample.vue", "sample.html"] {
        let source = read(name);
        let micros = median_micros(&source);
        println!(
            "{name}: {} bytes, median {} µs/file over {ITERATIONS} runs",
            source.content.len(),
            micros
        );
        assert!(
            micros < GUARD_MS * 1000,
            "{name} took {micros} µs per file, above the {GUARD_MS} ms guard"
        );
    }
}

/// A file with many elements must cost time proportional to its size. This is
/// the regression the guard above cannot see on a small fixture: a nested-loop
/// lookup added to a rule would pass the fixtures and fall over on a real page.
#[test]
fn cost_grows_roughly_linearly_with_file_size() {
    let unit = read("sample.html").content;
    let small = Source {
        path: "scaled.html".into(),
        content: unit.repeat(4),
    };
    let large = Source {
        path: "scaled.html".into(),
        content: unit.repeat(32),
    };
    let small_us = median_micros(&small).max(1);
    let large_us = median_micros(&large).max(1);
    let ratio = large_us as f64 / small_us as f64;
    println!(
        "4x fixture: {small_us} µs, 32x fixture: {large_us} µs, ratio {ratio:.2} for 8x the input"
    );
    // 8x the input, allowed up to 24x the time: generous enough for cache
    // effects and timer noise, tight enough that anything quadratic (64x) fails.
    assert!(
        ratio < 24.0,
        "cost grew {ratio:.2}x for 8x the input, which is not linear"
    );
}
