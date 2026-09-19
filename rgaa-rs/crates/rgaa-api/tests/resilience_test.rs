//! Verifies the resilience middleware stack (timeout, concurrency limit,
//! load shed) against a minimal router with a slow dummy handler — no live
//! orchestrator/storage needed, since `apply_resilience` is generic over the
//! router's state and doesn't touch `AppState`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use rgaa_api::apply_resilience;
use std::time::Duration;
use tower::ServiceExt;

async fn slow_handler() -> &'static str {
    tokio::time::sleep(Duration::from_millis(300)).await;
    "ok"
}

fn slow_app(max_concurrent: usize, timeout: Duration) -> Router {
    let router = Router::new().route("/slow", get(slow_handler));
    apply_resilience(router, max_concurrent, timeout)
}

#[tokio::test(flavor = "multi_thread")]
async fn burst_beyond_concurrency_limit_is_shed_fast() {
    // A real bound server, not `.oneshot()`: concurrent in-flight requests
    // against one persistent `Service` instance (as `axum::serve` runs it in
    // production) is what actually exercises the shared concurrency-limit
    // state — a fresh `.oneshot()` per request doesn't reliably reproduce
    // production's single-listener concurrency semantics.
    let app = slow_app(2, Duration::from_secs(5));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let url = format!("http://{addr}/slow");

    // Fire 10 requests at once against a concurrency limit of 2: some must
    // succeed (in-flight, unaffected) and some must be shed immediately
    // (503) rather than queue behind the limit — and the whole burst must
    // resolve well before 10 sequential 300ms handlers would (3s), proving
    // the shed responses didn't wait in a queue.
    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let client = client.clone();
            let url = url.clone();
            tokio::spawn(async move { client.get(&url).send().await.unwrap().status().as_u16() })
        })
        .collect();

    let mut ok_count = 0;
    let mut shed_count = 0;
    for handle in handles {
        match handle.await.unwrap() {
            200 => ok_count += 1,
            503 => shed_count += 1,
            other => panic!("unexpected status {other}"),
        }
    }
    let elapsed = start.elapsed();

    assert!(ok_count >= 1, "at least one request should succeed");
    assert!(
        shed_count >= 1,
        "at least one request beyond the concurrency limit should be shed with 503 \
         (got {ok_count} ok, {shed_count} shed)"
    );
    assert_eq!(ok_count + shed_count, 10);
    assert!(
        elapsed < Duration::from_secs(2),
        "shed requests must not queue behind the concurrency limit (took {:?} for 10 requests \
         at concurrency 2 with a 300ms handler)",
        elapsed
    );
}

#[tokio::test]
async fn request_within_limit_succeeds() {
    let app = slow_app(4, Duration::from_secs(5));
    let response = app
        .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn request_exceeding_timeout_is_marked_not_hung() {
    let app = slow_app(4, Duration::from_millis(50));
    let start = std::time::Instant::now();
    let response = app
        .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    // tower_http's TimeoutLayer returns its own 408 directly rather than an
    // error, so this never reaches `handle_overload_error`.
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    assert!(
        elapsed < Duration::from_millis(250),
        "a timed-out request must return promptly, not hang for the handler's full duration (took {:?})",
        elapsed
    );
}
