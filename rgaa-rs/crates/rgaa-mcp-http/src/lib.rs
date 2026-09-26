//! HTTP + SSE transport for the RGAA MCP tool server (JSON-RPC over POST /mcp).

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use rmcp::handler::server::wrapper::{Json as McpJson, Parameters};
use rmcp::model::{CallToolRequestParams, ErrorCode, Tool};
use rmcp::ErrorData;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

pub use rgaa_mcp::ToolServer;

/// Shared server state: the tool server plus a progress event bus for SSE.
#[derive(Clone)]
pub struct AppState {
    server: Arc<ToolServer>,
    events: tokio::sync::broadcast::Sender<ProgressEvent>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ProgressEvent {
    pub event: &'static str,
    pub tool: String,
}

impl AppState {
    pub fn new(server: ToolServer) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(64);
        Self {
            server: Arc::new(server),
            events,
        }
    }
}

/// Build the router for `/mcp` (JSON-RPC) and `/mcp/events` (SSE).
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/mcp", post(jsonrpc))
        .route("/mcp/events", get(sse_events))
        .layer(cors_from_env())
        .with_state(state)
}

/// CORS for the MCP endpoint, driven by `RGAA_CORS_ORIGINS`
/// (comma-separated origins).
///
/// Fails closed: with the variable unset, empty, or holding nothing that
/// parses as an origin, no cross-origin request is allowed. The previous
/// default allowed any origin, which is not safe for this endpoint even bound
/// to loopback — a page open in the user's browser can POST to
/// `http://127.0.0.1:3000/mcp`, and `jsonrpc` takes the body as a `String`
/// without checking `Content-Type`, so a `text/plain` POST (no preflight)
/// reaches `tools/call`. `analyze` and `audit_url` then fetch
/// caller-controlled URLs and return the results. CWE-942.
///
/// Note this layer is not authentication: anything that can reach the port
/// directly, rather than through a browser, is unaffected by CORS.
pub fn cors_from_env() -> CorsLayer {
    cors_from_origins(std::env::var("RGAA_CORS_ORIGINS").ok().as_deref())
}

/// The policy behind [`cors_from_env`], taking the raw allowlist directly so
/// it can be exercised without mutating the process environment.
pub fn cors_from_origins(raw: Option<&str>) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers(Any);

    let Some(raw) = raw else {
        return base;
    };
    if raw.trim().is_empty() {
        return base;
    }

    let mut origins = Vec::new();
    for candidate in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match candidate.parse() {
            Ok(origin) => origins.push(origin),
            // Named rather than silently dropped: a typo used to widen the
            // policy to "any origin" instead of narrowing it.
            Err(_) => tracing::warn!(
                origin = candidate,
                "ignoring unparseable entry in RGAA_CORS_ORIGINS"
            ),
        }
    }

    if origins.is_empty() {
        tracing::warn!(
            "RGAA_CORS_ORIGINS contained no usable origin; denying all cross-origin requests"
        );
        return base;
    }
    base.allow_origin(origins)
}

#[derive(serde::Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

fn rpc_ok(id: Option<Value>, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn rpc_err(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

fn error_data_to_rpc(id: Option<Value>, error: ErrorData) -> Value {
    rpc_err(id, error.code.0, error.message.as_ref())
}

async fn jsonrpc(State(state): State<AppState>, headers: HeaderMap, body: String) -> Response {
    let _ = &headers; // CORS layer already applied; headers kept for future diagnostics
    let request: JsonRpcRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Json(rpc_err(None, ErrorCode::PARSE_ERROR.0, e.to_string())).into_response();
        }
    };
    let id = request.id.clone();
    let envelope = match request.method.as_str() {
        "initialize" => rpc_ok(
            id,
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "rgaa-mcp-http", "version": env!("CARGO_PKG_VERSION")}
            }),
        ),
        "notifications/initialized" | "notifications/cancelled" => {
            return StatusCode::NO_CONTENT.into_response();
        }
        "tools/list" => {
            let tools: Vec<Tool> = ToolServer::tool_router().list_all();
            rpc_ok(id, json!({"tools": tools}))
        }
        "tools/call" => {
            let params: CallToolRequestParams = match request
                .params
                .clone()
                .map(serde_json::from_value)
                .transpose()
            {
                Ok(Some(p)) => p,
                Ok(None) => {
                    return Json(rpc_err(id, ErrorCode::INVALID_PARAMS.0, "missing params"))
                        .into_response();
                }
                Err(e) => {
                    return Json(rpc_err(id, ErrorCode::INVALID_PARAMS.0, e.to_string()))
                        .into_response();
                }
            };
            let tool_name = params.name.to_string();
            let _ = state.events.send(ProgressEvent {
                event: "tool_started",
                tool: tool_name.clone(),
            });
            let outcome = call_tool(&state.server, params).await;
            let _ = state.events.send(ProgressEvent {
                event: "tool_completed",
                tool: tool_name,
            });
            match outcome {
                Ok(result) => rpc_ok(id, result),
                Err(error) => error_data_to_rpc(id, error),
            }
        }
        other => rpc_err(id, ErrorCode::METHOD_NOT_FOUND.0, other),
    };
    Json(envelope).into_response()
}

fn parse_args<T: serde::de::DeserializeOwned>(name: &str, args: Value) -> Result<T, ErrorData> {
    serde_json::from_value(args)
        .map_err(|e| ErrorData::invalid_params(format!("invalid arguments for {name}: {e}"), None))
}

async fn call_tool(server: &ToolServer, params: CallToolRequestParams) -> Result<Value, ErrorData> {
    let arguments = Value::Object(params.arguments.clone().unwrap_or_default());
    let name = params.name.as_ref();
    let value: Value = match name {
        "analyze" => {
            let req: rgaa_mcp::AnalyzeRequest = parse_args(name, arguments)?;
            let McpJson(resp) = server.analyze(Parameters(req)).await?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize analyze response: {e}"), None)
            })?
        }
        "remediate" => {
            let req: rgaa_mcp::RemediationRequest = parse_args(name, arguments)?;
            let McpJson(resp) = server.remediate(Parameters(req))?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize remediate response: {e}"), None)
            })?
        }
        "igt" => {
            let req: rgaa_mcp::GuidedTestRequest = parse_args(name, arguments)?;
            let McpJson(resp) = server.igt(Parameters(req)).await?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize igt response: {e}"), None)
            })?
        }
        "audit_url" => {
            let req: rgaa_mcp::AuditUrlInput = parse_args(name, arguments)?;
            let McpJson(resp) = server.audit_url(Parameters(req)).await?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize audit_url response: {e}"), None)
            })?
        }
        "get_audit_result" => {
            let req: rgaa_mcp::GetAuditInput = parse_args(name, arguments)?;
            let McpJson(resp) = server.get_audit_result(Parameters(req)).await?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize get_audit_result response: {e}"), None)
            })?
        }
        "list_criteria" => {
            let McpJson(resp) = server.list_criteria()?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize list_criteria response: {e}"), None)
            })?
        }
        other => {
            return Err(ErrorData::invalid_params(
                format!("unknown tool: {other}"),
                None,
            ));
        }
    };
    Ok(json!({
        "content": [{"type": "text", "text": value.to_string()}],
        "structuredContent": value,
        "isError": false
    }))
}

async fn sse_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
                    let event = Event::default().event(ev.event).data(data);
                    return Some((Ok(event), rx));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

// Re-export for callers that need the tool server type.
pub use rgaa_mcp::ToolServer as _ToolServerReexportGuard;

#[cfg(test)]
mod cors_tests {
    use super::*;
    use axum::http::{HeaderValue, Method, Request};
    use tower::{Layer, ServiceExt};

    /// Runs one cross-origin GET through the layer and returns the
    /// `access-control-allow-origin` it answered with, if any.
    async fn allow_origin_for(raw: Option<&str>, origin: &str) -> Option<String> {
        let svc = cors_from_origins(raw).layer(tower::service_fn(
            |_req: Request<axum::body::Body>| async move {
                Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                    axum::body::Body::empty(),
                ))
            },
        ));
        let req = Request::builder()
            .method(Method::GET)
            .uri("/mcp")
            .header("origin", HeaderValue::from_str(origin).unwrap())
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        resp.headers()
            .get("access-control-allow-origin")
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[tokio::test]
    async fn an_unset_allowlist_denies_every_origin() {
        assert_eq!(allow_origin_for(None, "https://evil.example").await, None);
    }

    #[tokio::test]
    async fn a_blank_allowlist_denies_every_origin() {
        assert_eq!(
            allow_origin_for(Some("   "), "https://evil.example").await,
            None
        );
    }

    #[tokio::test]
    async fn an_allowlist_of_only_garbage_fails_closed() {
        // The old code widened to "any origin" here, so a typo in the
        // variable silently removed the protection it was meant to configure.
        assert_eq!(
            allow_origin_for(Some("not-an-origin, also bad"), "https://evil.example").await,
            None
        );
    }

    #[tokio::test]
    async fn a_configured_origin_is_allowed_and_others_are_not() {
        assert_eq!(
            allow_origin_for(Some("https://plugin.example"), "https://plugin.example").await,
            Some("https://plugin.example".to_string())
        );
        assert_eq!(
            allow_origin_for(Some("https://plugin.example"), "https://evil.example").await,
            None
        );
    }

    #[tokio::test]
    async fn a_valid_origin_survives_an_invalid_neighbour() {
        assert_eq!(
            allow_origin_for(
                Some("nonsense, https://plugin.example"),
                "https://plugin.example"
            )
            .await,
            Some("https://plugin.example".to_string())
        );
    }
}
