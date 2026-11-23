//! Axum middleware over [`LicenseGate`].
//!
//! Attach with `route_layer`, never `layer`, on the routes that need it, so
//! `/health` stays outside (a gated liveness probe causes restart loops):
//!
//! ```ignore
//! use axum::middleware::from_fn_with_state;
//! use rgaa_license::middleware::{enforce, Classification, LicenseLayerState};
//!
//! let state = LicenseLayerState::new(gate, |_method, path| match path {
//!     p if p.starts_with("/v1/batches") => Classification::Access(Feature::Batch),
//!     _ => Classification::Metered(Feature::FullAudit, 1),
//! });
//! let routes = routes.route_layer(from_fn_with_state(state, enforce));
//! ```
//!
//! It must sit *outside* timeout / concurrency / load-shed layers, so a refused
//! request takes no slot and its 429 is not rewritten into a 503.
//!
//! Quota is consumed before the handler runs; a request that later fails
//! still counts.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::gate::{Denial, LicenseGate};
use crate::tiers::Feature;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// No licence check, no headers.
    Exempt,
    /// Feature must be in the tier; no quota consumed.
    Access(Feature),
    /// Feature must be in the tier, and `units` are consumed.
    Metered(Feature, u32),
}

pub type Classifier = fn(&Method, &str) -> Classification;

#[derive(Clone)]
pub struct LicenseLayerState {
    gate: Arc<LicenseGate>,
    classify: Classifier,
}

impl LicenseLayerState {
    pub fn new(gate: Arc<LicenseGate>, classify: Classifier) -> Self {
        Self { gate, classify }
    }
}

fn append_headers(response: &mut Response, pairs: Vec<(&'static str, String)>) {
    let headers = response.headers_mut();
    for (name, value) in pairs {
        if let Ok(value) = HeaderValue::from_str(&value) {
            headers.insert(HeaderName::from_static(name), value);
        }
    }
}

pub async fn enforce(State(state): State<LicenseLayerState>, req: Request, next: Next) -> Response {
    let (feature, cost) = match (state.classify)(req.method(), req.uri().path()) {
        Classification::Exempt => return next.run(req).await,
        Classification::Access(f) => (f, 0),
        Classification::Metered(f, n) => (f, n),
    };
    // `check` holds the usage `persist_lock` across a read-modify-write of
    // `usage.toml` (blocking file I/O): keep it off the async worker threads.
    let gate = Arc::clone(&state.gate);
    let Ok(checked) = tokio::task::spawn_blocking(move || gate.check(feature, cost)).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match checked {
        Ok(admission) => {
            let mut response = next.run(req).await;
            append_headers(&mut response, admission.headers());
            response
        }
        Err(denial) => (*denial).into_response(),
    }
}

impl IntoResponse for Denial {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.http_status()).unwrap_or(StatusCode::PAYMENT_REQUIRED);
        let mut response = (status, Json(self.body())).into_response();
        append_headers(&mut response, self.headers());
        response
    }
}
