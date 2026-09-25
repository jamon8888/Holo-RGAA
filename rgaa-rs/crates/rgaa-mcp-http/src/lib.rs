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

/// CORS: `RGAA_CORS_ORIGINS` (comma-separated) or permissive for local plugins.
pub fn cors_from_env() -> CorsLayer {
    match std::env::var("RGAA_CORS_ORIGINS") {
        Ok(raw) if !raw.trim().is_empty() => {
            let origins: Vec<_> = raw
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if origins.is_empty() {
                CorsLayer::new().allow_origin(Any)
            } else {
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::OPTIONS,
                    ])
                    .allow_headers(Any)
            }
        }
        _ => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers(Any),
    }
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
