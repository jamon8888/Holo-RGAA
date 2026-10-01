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
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

pub use rgaa_mcp::ToolServer;

/// Shared server state: the tool server plus a progress event bus for SSE.
#[derive(Clone)]
pub struct AppState {
    server: Arc<ToolServer>,
    events: tokio::sync::broadcast::Sender<ProgressEvent>,
    /// Flipped to `true` once the process has decided to stop. Only the SSE
    /// handler reads it; see [`AppState::begin_shutdown`].
    shutdown: tokio::sync::watch::Sender<bool>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ProgressEvent {
    pub event: &'static str,
    pub tool: String,
}

impl AppState {
    pub fn new(server: ToolServer) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(64);
        let (shutdown, _) = tokio::sync::watch::channel(false);
        Self {
            server: Arc::new(server),
            events,
            shutdown,
        }
    }

    /// Ends every open `/mcp/events` stream.
    ///
    /// Must be called before the graceful shutdown starts waiting. Axum's
    /// graceful shutdown drains *connections*, and an SSE response never
    /// completes on its own: a single attached browser would hold the
    /// process open past SIGTERM until something killed it, which is exactly
    /// the hang a drain is supposed to avoid.
    pub fn begin_shutdown(&self) {
        // `send_replace`, not `send`: `watch::Sender::send` returns `Err`
        // when there is no live receiver and — this is the part that bites —
        // leaves the stored value untouched. `AppState::new` drops the
        // initial receiver, so with no SSE client attached the flag would
        // stay `false`, and a request already accepted that subscribes
        // *after* this call would never see the shutdown. That stream then
        // holds the drain open: exactly the hang this whole mechanism
        // exists to prevent, in a narrower window.
        let _ = self.shutdown.send_replace(true);
    }
}

/// Build the router with the CORS allowlist taken from the environment.
pub fn app(state: AppState) -> Router {
    app_with_cors(state, std::env::var("RGAA_CORS_ORIGINS").ok().as_deref())
}

/// Build the router for `/health`, `/mcp` (JSON-RPC) and `/mcp/events` (SSE)
/// with an explicit CORS allowlist, so a caller that got the origins from a
/// command-line flag does not have to write them back into the environment
/// to be heard.
pub fn app_with_cors(state: AppState, cors_origins: Option<&str>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/mcp", post(jsonrpc))
        .route("/mcp/events", get(sse_events))
        .layer(cors_from_origins(cors_origins))
        .with_state(state)
}

/// Liveness probe. Supervisors and the plugin front-end poll this to decide
/// the server is up, so it must answer without touching the tool services:
/// a health check that can hang behind an audit is not a health check.
async fn health() -> Json<Value> {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

/// CORS for the MCP endpoint, from a comma-separated allowlist (the
/// `--cors-origin` flag, defaulting to `RGAA_CORS_ORIGINS`).
///
/// Fails closed: with the allowlist absent, empty, or holding nothing that
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
                "ignoring unparseable entry in the CORS allowlist"
            ),
        }
    }

    if origins.is_empty() {
        tracing::warn!(
            "CORS allowlist contained no usable origin; denying all cross-origin requests"
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
        // A tool is only reachable over HTTP once it has an arm here: this
        // dispatch is hand-written, while `tools/list` comes from the macro
        // router, so a missing arm advertises a tool that then 404s.
        "source_map" => {
            let req: rgaa_mcp::SourceMapRequest = parse_args(name, arguments)?;
            let McpJson(resp) = server.source_map(Parameters(req)).await?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize source_map response: {e}"), None)
            })?
        }
        "verify_fix" => {
            let req: rgaa_mcp::VerifyFixRequest = parse_args(name, arguments)?;
            let McpJson(resp) = server.verify_fix(Parameters(req)).await?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize verify_fix response: {e}"), None)
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
    let stop = state.shutdown.subscribe();
    let stream = futures::stream::unfold((rx, stop), |(mut rx, mut stop)| async move {
        // Checked before awaiting as well as inside the select: a client that
        // connects after `begin_shutdown` would otherwise never observe the
        // `changed()` edge and would keep the connection — and the drain —
        // open.
        if *stop.borrow_and_update() {
            return None;
        }
        loop {
            let received = tokio::select! {
                _ = stop.changed() => return None,
                received = rx.recv() => received,
            };
            match received {
                Ok(ev) => {
                    let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
                    let event = Event::default().event(ev.event).data(data);
                    return Some((Ok(event), (rx, stop)));
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

/// Listen, CORS and transport flags for the MCP server.
///
/// Lives here rather than in either binary because both `rgaa mcp-server`
/// and the standalone `rgaa-mcp-http` start the same server: two hand-rolled
/// copies of the flags is how `--cors-origin` ends up meaning two different
/// things depending on which entry point you used.
#[derive(Debug, Clone, clap::Args)]
pub struct McpServerArgs {
    /// TCP port to listen on.
    #[arg(long, env = "PORT", default_value_t = 3000)]
    pub port: u16,
    /// Address to bind. Defaults to loopback; binding elsewhere exposes the
    /// tool server to the network, which has no authentication of its own.
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    pub host: String,
    /// Comma-separated browser origins allowed to call the server.
    ///
    /// Unset means no cross-origin request is allowed. That is deliberate:
    /// see [`cors_from_origins`].
    #[arg(long, env = "RGAA_CORS_ORIGINS")]
    pub cors_origin: Option<String>,
    /// Serve MCP over stdin/stdout instead of HTTP.
    ///
    /// Kept so the MCP client configurations written by `install.sh` before
    /// the HTTP transport existed keep working unchanged.
    #[arg(long)]
    pub stdio: bool,
}

/// Why the server stopped early. The variants are separate because the
/// operator's next move differs: a bind failure is a port clash, an HTTP
/// failure is not.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("cannot bind {addr}: {source}")]
    Bind {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("HTTP server failed: {0}")]
    Http(#[source] std::io::Error),
    #[error("stdio transport failed: {0}")]
    Stdio(String),
}

/// The tool server the binaries expose, wired to the real Obscura bridge.
///
/// The bridge is lazy so that starting the server does not require a browser
/// substrate to be installed: tools that need one fail when called, rather
/// than the process refusing to start and taking `/health` down with it.
pub fn default_tool_server() -> ToolServer {
    let bridge = Arc::new(rgaa_mcp::LazyObscuraBridge::new(
        rgaa_obscura::ObscuraBridge::from_env(),
    ));
    ToolServer::new(
        Arc::new(rgaa_mcp::ObscuraAnalyzeService::new(Arc::clone(&bridge))),
        Arc::new(rgaa_mcp::RemediationServiceImpl::default()),
        Arc::new(rgaa_mcp::ObscuraGuidedService::new(bridge)),
        Arc::new(rgaa_mcp::OrchestrationService::new()),
        Arc::new(rgaa_mcp::NoOpStorageService),
    )
}

/// Run the MCP server as configured by `args`, returning once it has shut
/// down cleanly.
pub async fn run(args: McpServerArgs) -> Result<(), ServeError> {
    if args.stdio {
        return run_stdio().await;
    }

    let state = AppState::new(default_tool_server());
    let app = app_with_cors(state.clone(), args.cors_origin.as_deref());
    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|source| ServeError::Bind {
            addr: addr.clone(),
            source,
        })?;
    tracing::info!("rgaa mcp-server listening on http://{addr}");

    serve_until(listener, app, async move {
        terminate_signal().await;
        tracing::info!("termination signal received, draining in-flight requests");
        state.begin_shutdown();
    })
    .await
    .map_err(ServeError::Http)
}

async fn run_stdio() -> Result<(), ServeError> {
    use rmcp::ServiceExt;
    let service = default_tool_server()
        .serve(rmcp::transport::io::stdio())
        .await
        .map_err(|e| ServeError::Stdio(e.to_string()))?;
    service
        .waiting()
        .await
        .map_err(|e| ServeError::Stdio(e.to_string()))?;
    Ok(())
}

/// Serve `app` until `shutdown` resolves, then wait for the requests already
/// being handled to produce their responses.
///
/// Split out from [`run`] so the drain can be driven by something other than
/// a real signal in tests.
pub async fn serve_until<F>(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown: F,
) -> std::io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
}

/// Resolves on SIGTERM or SIGINT.
///
/// SIGTERM is the one that matters: it is what a container runtime, systemd
/// and the plugin supervisor send, and the default disposition kills the
/// process outright, cutting whatever response was mid-flight.
pub async fn terminate_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "cannot listen for SIGTERM; falling back to Ctrl-C");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod shutdown_tests {
    /// A stream that subscribes *after* `begin_shutdown` must still see it.
    ///
    /// This is the case `watch::Sender::send` gets wrong: with no live
    /// receiver it returns `Err` and leaves the stored value `false`, so a
    /// request accepted just before shutdown — then reaching the SSE handler
    /// and subscribing — would never be told to stop, and would hold the
    /// drain open indefinitely.
    ///
    /// `AppState::new` drops its initial receiver, so "no live receiver" is
    /// the *normal* state of a server with nobody streaming.
    #[tokio::test]
    async fn a_stream_subscribing_after_shutdown_still_sees_it() {
        let (shutdown, _) = tokio::sync::watch::channel(false);
        let _ = shutdown.send_replace(true);
        assert!(
            *shutdown.subscribe().borrow(),
            "a late subscriber must observe the shutdown flag"
        );
    }

    /// Pins the tokio behaviour the fix works around, so the reason for
    /// `send_replace` survives someone "simplifying" it back to `send`.
    #[tokio::test]
    async fn plain_send_loses_the_flag_when_nobody_is_listening() {
        let (shutdown, _) = tokio::sync::watch::channel(false);
        assert!(
            shutdown.send(true).is_err(),
            "send must report the absence of receivers"
        );
        assert!(
            !*shutdown.subscribe().borrow(),
            "and must leave the stored value untouched — which is the bug"
        );
    }
}

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
