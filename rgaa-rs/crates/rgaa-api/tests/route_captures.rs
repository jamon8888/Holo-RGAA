//! Path captures must use axum 0.7's `:id`, not 0.8's `{id}`.
//!
//! Every id-bearing route in this crate used braces and so matched nothing:
//! `GET /v1/audit-bundles/<uuid>`, `DELETE` on the same path, and
//! `GET /audit/<id>` always 404'd.
//!
//! It survived for two reasons, both worth guarding against rather than
//! trusting people to remember:
//!
//! 1. Braces *read* correctly — they are the syntax everyone knows from
//!    newer axum, and nothing in a diff flags them.
//! 2. The only test touching those routes asserted that an unknown id
//!    returns 404. It passed for the wrong reason: every id returned 404.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;

async fn ok() -> &'static str {
    "ok"
}

async fn status_for(pattern: &'static str, path: &str) -> StatusCode {
    let app = Router::new().route(pattern, get(ok));
    app.oneshot(
        Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response")
    .status()
}

/// Pins the axum-0.7 semantics this crate depends on.
///
/// If this starts failing, axum has been bumped to 0.8+ and **every** `:id`
/// in the crate has to become `{id}` in the same change — the meaning of
/// both syntaxes inverts at that boundary.
#[tokio::test]
async fn axum_07_captures_with_a_colon_and_treats_braces_as_a_literal() {
    assert_eq!(
        status_for("/v1/audit-bundles/:id", "/v1/audit-bundles/abc-123").await,
        StatusCode::OK,
        "`:id` must capture"
    );
    assert_eq!(
        status_for("/v1/audit-bundles/{id}", "/v1/audit-bundles/abc-123").await,
        StatusCode::NOT_FOUND,
        "on axum 0.7 `{{id}}` is a literal segment, so a real id must not match"
    );
    assert_eq!(
        status_for("/v1/audit-bundles/{id}", "/v1/audit-bundles/{id}").await,
        StatusCode::OK,
        "...and the literal string itself is what it matches"
    );
}

/// Guards the router declarations themselves, not just the semantics above.
///
/// Reading the source is crude, but it is the only check that fails when
/// someone adds a *new* brace route — a behavioural test only covers the
/// paths it happens to name.
#[test]
fn no_route_in_this_crate_uses_brace_captures() {
    for (file, source) in [
        ("src/lib.rs", include_str!("../src/lib.rs")),
        ("src/batch.rs", include_str!("../src/batch.rs")),
    ] {
        for (number, line) in source.lines().enumerate() {
            let Some(rest) = line.split_once(".route(\"").map(|(_, r)| r) else {
                continue;
            };
            let Some((pattern, _)) = rest.split_once('"') else {
                continue;
            };
            assert!(
                !pattern.contains('{'),
                "{file}:{} declares `{pattern}`; on axum 0.7 braces are a \
                 literal segment and this route will match nothing. Use `:name`.",
                number + 1
            );
        }
        // `.route(` spread over several lines puts the pattern on its own.
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('"') && trimmed.ends_with("\",") && trimmed.contains('{') {
                assert!(
                    !trimmed.starts_with("\"/"),
                    "{file}:{} looks like a route pattern with brace captures: {trimmed}",
                    number + 1
                );
            }
        }
    }
}
