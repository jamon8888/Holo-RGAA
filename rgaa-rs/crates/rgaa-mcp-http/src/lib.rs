//! HTTP + SSE transport for the RGAA MCP tool server (JSON-RPC over POST /mcp).

use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
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
use tower_http::cors::{AllowHeaders, CorsLayer};

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

/// Build the router with the CORS allowlist and bearer token taken from the
/// environment (`RGAA_CORS_ORIGINS` and `RGAA_MCP_TOKEN`).
pub fn app(state: AppState) -> Router {
    app_with_auth(
        state,
        std::env::var("RGAA_CORS_ORIGINS").ok().as_deref(),
        std::env::var("RGAA_MCP_TOKEN").ok().as_deref(),
    )
}

/// Build the router with an explicit CORS allowlist, so a caller that got the
/// origins from a command-line flag does not have to write them back into the
/// environment to be heard. The bearer token still comes from
/// `RGAA_MCP_TOKEN`.
pub fn app_with_cors(state: AppState, cors_origins: Option<&str>) -> Router {
    app_with_auth(
        state,
        cors_origins,
        std::env::var("RGAA_MCP_TOKEN").ok().as_deref(),
    )
}

/// Build the router for `/health`, `/mcp` (JSON-RPC) and `/mcp/events` (SSE)
/// with an explicit CORS allowlist and an explicit bearer token.
///
/// The CORS layer alone does not protect this endpoint: it governs what a
/// browser is allowed to *read*, not what the server is willing to *run*. A
/// simple cross-origin `text/plain` POST needs no preflight, so without the
/// guard below the browser fires it, `tools/call` dispatches, `analyze`
/// fetches a caller-chosen URL, and only the *response* is withheld from the
/// attacking page. [`guard`] therefore rejects the request before dispatch.
pub fn app_with_auth(state: AppState, cors_origins: Option<&str>, token: Option<&str>) -> Router {
    let policy = Arc::new(AuthPolicy::new(cors_origins, token));

    // The guarded routes live in their own router behind `route_layer`, not
    // on the whole tree behind `layer`. `layer` wraps *every* route the
    // router ends up holding, `/health` included, which turned the liveness
    // probe into a 401 the moment a token was configured — a supervisor
    // would then restart a perfectly healthy server in a loop. `route_layer`
    // runs only for requests that match a route in this sub-router.
    let guarded = Router::new()
        .route("/mcp", post(jsonrpc))
        .route("/mcp/events", get(sse_events))
        .route_layer(axum::middleware::from_fn(move |req, next| {
            let policy = Arc::clone(&policy);
            async move { guard(policy, req, next).await }
        }));

    Router::new()
        // `/health` stays outside the guard: a liveness probe that needs a
        // credential is one more thing to misconfigure, and it discloses
        // nothing but the version a supervisor already knows.
        .route("/health", get(health))
        .merge(guarded)
        .layer(cors_from_origins(cors_origins))
        .with_state(state)
}

/// Request-side authorization for the MCP endpoints (issue #198).
#[derive(Debug, Clone, Default)]
pub struct AuthPolicy {
    /// Origins a browser is allowed to call from — the same list the CORS
    /// layer uses, so the two cannot drift apart.
    allowed_origins: Vec<HeaderValue>,
    /// When set, every MCP request must carry `Authorization: Bearer <token>`.
    token: Option<String>,
}

/// Why a request was turned away, so the handler can answer with the status
/// that tells the caller what to change.
#[derive(Debug, PartialEq, Eq)]
pub enum Denied {
    /// The `Origin` header is absent from the allowlist: a browser page that
    /// is not one of ours. 403 — no credential would help.
    Origin,
    /// A token is required and the request did not present a valid one. 401.
    Token,
}

impl AuthPolicy {
    pub fn new(cors_origins: Option<&str>, token: Option<&str>) -> Self {
        Self {
            allowed_origins: parse_origins(cors_origins),
            token: token
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(ToOwned::to_owned),
        }
    }

    /// Decide on one request from its headers alone.
    ///
    /// Split from the middleware so the table of cases below is a unit test
    /// rather than a fleet of sockets.
    pub fn check(&self, headers: &HeaderMap) -> Result<(), Denied> {
        // An `Origin` means a browser sent this. Browsers attach it to every
        // cross-origin request including the simple `text/plain` POST that
        // skips preflight, which is exactly the hole the CORS layer leaves
        // open, so an origin we do not know is refused outright.
        if let Some(origin) = headers.get(axum::http::header::ORIGIN) {
            if !self.allowed_origins.iter().any(|a| a == origin) {
                return Err(Denied::Origin);
            }
        }

        // No `Origin` means a direct client (the CLI, curl, another service).
        // CORS never constrained those, so the token is the only thing that
        // does — and when one is configured it is required of browsers too.
        if let Some(expected) = &self.token {
            let presented = headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(str::trim)
                .unwrap_or_default();
            if !constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
                return Err(Denied::Token);
            }
        }
        Ok(())
    }
}

/// Compare without leaking the position of the first mismatch through timing.
///
/// Lengths are compared first and in the clear: the length of a bearer token
/// is not the secret, and branching on it keeps the loop a fixed shape.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Middleware that applies [`AuthPolicy`] before anything reaches a handler.
async fn guard(policy: Arc<AuthPolicy>, req: Request, next: Next) -> Response {
    match policy.check(req.headers()) {
        Ok(()) => next.run(req).await,
        Err(denied) => {
            let (status, message) = match denied {
                Denied::Origin => (
                    StatusCode::FORBIDDEN,
                    "origin not allowed; see --cors-origin / RGAA_CORS_ORIGINS",
                ),
                Denied::Token => (
                    StatusCode::UNAUTHORIZED,
                    "missing or invalid bearer token; see --auth-token / RGAA_MCP_TOKEN",
                ),
            };
            tracing::warn!(
                status = status.as_u16(),
                reason = message,
                "MCP request refused"
            );
            // A JSON-RPC envelope, not a bare status line: the caller is an
            // RPC client and should be able to read the refusal with the
            // parser it already has.
            (
                status,
                Json(rpc_err(None, ErrorCode::INVALID_REQUEST.0, message)),
            )
                .into_response()
        }
    }
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
        // `mirror_request`, not `Any`. `Any` emits
        // `Access-Control-Allow-Headers: *`, and under the Fetch spec that
        // wildcard deliberately does NOT cover `Authorization`. With a token
        // configured, an allowlisted browser origin would therefore fail its
        // preflight and never send the POST at all — the one case the docs
        // promise works. Mirroring echoes back exactly the headers the
        // preflight asked for, which covers `Authorization` and keeps the
        // previous "any header" latitude for everything else. The origin
        // allowlist, not this, is what decides who gets a usable preflight.
        .allow_headers(AllowHeaders::mirror_request());

    let origins = parse_origins(raw);
    if origins.is_empty() {
        if raw.is_some_and(|r| !r.trim().is_empty()) {
            tracing::warn!(
                "CORS allowlist contained no usable origin; denying all cross-origin requests"
            );
        }
        return base;
    }
    base.allow_origin(origins)
}

/// Parse a comma-separated allowlist into origins.
///
/// Shared by the CORS layer and [`AuthPolicy`] so the list a browser may read
/// a response from and the list it may *run a tool* from cannot drift apart.
fn parse_origins(raw: Option<&str>) -> Vec<HeaderValue> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let mut origins = Vec::new();
    for candidate in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match HeaderValue::from_str(candidate) {
            Ok(origin) => origins.push(origin),
            // Named rather than silently dropped: a typo used to widen the
            // policy to "any origin" instead of narrowing it.
            Err(_) => tracing::warn!(
                origin = candidate,
                "ignoring unparseable entry in the CORS allowlist"
            ),
        }
    }
    origins
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
        "lint_static" => {
            let req: rgaa_mcp::LintStaticRequest = parse_args(name, arguments)?;
            let McpJson(resp) = server.lint_static(Parameters(req))?;
            serde_json::to_value(resp).map_err(|e| {
                ErrorData::internal_error(format!("serialize lint_static response: {e}"), None)
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
    /// Shared secret every MCP request must present as
    /// `Authorization: Bearer <token>`.
    ///
    /// Unset means no token is checked, which is only safe while the server
    /// is bound to loopback and no untrusted process shares the machine: the
    /// Origin check stops browser pages, nothing else stops a local process.
    /// Set it whenever `--host` is anything but a loopback address.
    ///
    /// Prefer the `RGAA_MCP_TOKEN` environment variable over this flag: a
    /// command line is world-readable through `ps` and
    /// `/proc/<pid>/cmdline`, so passing the secret as an argument hands it
    /// to every local user and to anything sampling process listings.
    #[arg(long, env = "RGAA_MCP_TOKEN")]
    pub auth_token: Option<String>,
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

    if args.auth_token.is_none() && !is_loopback(&args.host) {
        tracing::warn!(
            host = %args.host,
            "serving MCP off loopback with no --auth-token: every tool is open to the network"
        );
    }
    let state = AppState::new(default_tool_server());
    let app = app_with_auth(
        state.clone(),
        args.cors_origin.as_deref(),
        args.auth_token.as_deref(),
    );
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

/// Whether `host` keeps the listener on this machine.
///
/// A hostname that is not an IP literal is treated as *not* loopback: the
/// warning it triggers is cheap, and guessing the other way would silence the
/// one case that matters.
fn is_loopback(host: &str) -> bool {
    host.trim_matches(['[', ']'])
        .parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
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

    /// Returns the `access-control-allow-headers` a preflight is answered
    /// with, for a preflight that asks to send `authorization`.
    async fn preflight_allow_headers(raw: Option<&str>, origin: &str) -> Option<String> {
        let svc = cors_from_origins(raw).layer(tower::service_fn(
            |_req: Request<axum::body::Body>| async move {
                Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                    axum::body::Body::empty(),
                ))
            },
        ));
        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/mcp")
            .header("origin", HeaderValue::from_str(origin).unwrap())
            .header("access-control-request-method", "POST")
            .header("access-control-request-headers", "authorization")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        resp.headers()
            .get("access-control-allow-headers")
            .map(|v| v.to_str().unwrap().to_lowercase())
    }

    /// The bug a `*` wildcard hides: under the Fetch spec
    /// `Access-Control-Allow-Headers: *` does **not** cover `Authorization`,
    /// so with a token configured an allowlisted browser origin would fail
    /// its preflight and never send the request. The header has to be named.
    #[tokio::test]
    async fn a_preflight_asking_for_authorization_is_granted_it_by_name() {
        let allowed =
            preflight_allow_headers(Some("https://plugin.example"), "https://plugin.example")
                .await
                .expect("an allowlisted preflight must be answered");
        assert!(
            allowed.contains("authorization"),
            "a wildcard does not cover Authorization under the Fetch spec, so it \
             must be named explicitly; got {allowed:?}"
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

#[cfg(test)]
mod auth_tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    /// The hole issue #198 is about: CORS lets the browser *send* this, it
    /// just hides the answer. The guard must stop it before dispatch.
    #[test]
    fn a_browser_origin_outside_the_allowlist_is_refused() {
        let policy = AuthPolicy::new(Some("https://plugin.example"), None);
        assert_eq!(
            policy.check(&headers(&[
                ("origin", "https://evil.example"),
                ("content-type", "text/plain"),
            ])),
            Err(Denied::Origin)
        );
    }

    #[test]
    fn an_allowlisted_origin_passes() {
        let policy = AuthPolicy::new(Some("https://plugin.example"), None);
        assert_eq!(
            policy.check(&headers(&[("origin", "https://plugin.example")])),
            Ok(())
        );
    }

    /// With no allowlist configured, *no* origin is allowed — the same
    /// fail-closed posture `cors_from_origins` takes.
    #[test]
    fn an_unset_allowlist_refuses_every_browser_origin() {
        let policy = AuthPolicy::new(None, None);
        assert_eq!(
            policy.check(&headers(&[("origin", "https://plugin.example")])),
            Err(Denied::Origin)
        );
    }

    /// A direct client sends no `Origin`. CORS never constrained it and this
    /// guard does not either, until a token is configured.
    #[test]
    fn a_request_with_no_origin_and_no_token_configured_passes() {
        assert_eq!(AuthPolicy::new(None, None).check(&HeaderMap::new()), Ok(()));
    }

    #[test]
    fn a_configured_token_is_required_of_direct_clients() {
        let policy = AuthPolicy::new(None, Some("s3cret"));
        assert_eq!(policy.check(&HeaderMap::new()), Err(Denied::Token));
        assert_eq!(
            policy.check(&headers(&[("authorization", "Bearer wrong")])),
            Err(Denied::Token)
        );
        assert_eq!(
            policy.check(&headers(&[("authorization", "Bearer s3cret")])),
            Ok(())
        );
    }

    /// Order matters for the status code: an unknown origin is 403 whatever
    /// credential it carries, because no credential makes that page ours.
    #[test]
    fn a_bad_origin_beats_a_good_token() {
        let policy = AuthPolicy::new(Some("https://plugin.example"), Some("s3cret"));
        assert_eq!(
            policy.check(&headers(&[
                ("origin", "https://evil.example"),
                ("authorization", "Bearer s3cret"),
            ])),
            Err(Denied::Origin)
        );
    }

    /// An allowlisted browser still needs the token when one is set.
    #[test]
    fn an_allowlisted_origin_without_the_token_is_refused() {
        let policy = AuthPolicy::new(Some("https://plugin.example"), Some("s3cret"));
        assert_eq!(
            policy.check(&headers(&[("origin", "https://plugin.example")])),
            Err(Denied::Token)
        );
    }

    /// A blank `RGAA_MCP_TOKEN` is an unset one, not a token equal to "".
    /// Otherwise exporting the variable empty would make `Bearer ` a valid
    /// credential — worse than no token at all, because it looks configured.
    #[test]
    fn a_blank_token_configures_no_token() {
        let policy = AuthPolicy::new(None, Some("   "));
        assert_eq!(policy.check(&HeaderMap::new()), Ok(()));
    }

    #[test]
    fn constant_time_eq_still_compares_correctly() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn loopback_hosts_are_recognised() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("::1"));
        assert!(is_loopback("[::1]"));
        assert!(!is_loopback("0.0.0.0"));
        assert!(!is_loopback("192.168.1.10"));
        // Not an IP literal: warn rather than assume.
        assert!(!is_loopback("localhost"));
    }
}
