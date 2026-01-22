use crate::Orchestrator;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;

/// Type alias for response body
type BoxBody = Full<Bytes>;

/// Auth token header for admin API calls
const AUTH_TOKEN_HEADER: &str = "X-Fortify-Admin-Token";

/// Admin password (must match HTTP service)
const ADMIN_PASSWORD: &str = "pleaseletmein123";

/// Generate auth token from password (must match HTTP service)
fn generate_auth_token(password: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Check if request has valid auth token
fn is_authenticated(req: &Request<Incoming>) -> bool {
    if let Some(token_header) = req.headers().get(AUTH_TOKEN_HEADER) {
        if let Ok(provided_token) = token_header.to_str() {
            let expected_token = generate_auth_token(ADMIN_PASSWORD);
            return provided_token == expected_token;
        }
    }
    false
}

/// Return 401 Unauthorized response
fn unauthorized() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::json!({
                "error": "Unauthorized",
                "message": "Valid authentication token required for administrative operations"
            })
            .to_string(),
        )))
        .unwrap()
}

/// Orchestrator HTTP server
pub struct OrchestratorServer {
    bind_addr: SocketAddr,
    gate_address: String,
    orchestrator: Arc<Orchestrator>,
}

impl OrchestratorServer {
    pub fn new(
        bind_addr: SocketAddr,
        gate_address: String,
        orchestrator: Arc<Orchestrator>,
    ) -> Self {
        Self {
            bind_addr,
            gate_address,
            orchestrator,
        }
    }

    /// Start the HTTP server
    pub async fn start(&self) -> anyhow::Result<()> {
        let gate_address = self.gate_address.clone();
        let orchestrator = Arc::clone(&self.orchestrator);

        let listener = TcpListener::bind(&self.bind_addr).await?;
        tracing::info!("Orchestrator HTTP server listening on {}", self.bind_addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let gate_address = gate_address.clone();
            let orchestrator = Arc::clone(&orchestrator);

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    handle_request(req, gate_address.clone(), Arc::clone(&orchestrator))
                });

                if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                    tracing::debug!("Connection error: {}", e);
                }
            });
        }
    }
}

/// Handle incoming request
async fn handle_request(
    req: Request<Incoming>,
    gate_address: String,
    orchestrator: Arc<Orchestrator>,
) -> std::result::Result<Response<BoxBody>, Infallible> {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Administrative endpoints require authentication
    let admin_endpoints = [
        "/mirror/create",
        "/mirror/create-standby",
        "/mirror/activate",
        "/mirror/pause",
        "/mirror/resume",
        "/mirror/destroy",
    ];

    if admin_endpoints.iter().any(|endpoint| path == *endpoint) && !is_authenticated(&req) {
        tracing::warn!("🚫 Unauthorized attempt to access {} from {}", path, method);
        let duration = start.elapsed();
        tracing::debug!(
            "{} {} - {} - {:?}",
            method,
            path,
            StatusCode::UNAUTHORIZED,
            duration
        );
        return Ok(unauthorized());
    }

    let response = match (req.method().as_str(), req.uri().path()) {
        ("GET", "/health") => health_check(Arc::clone(&orchestrator)),
        ("GET", "/stats") => get_stats(Arc::clone(&orchestrator)),
        ("GET", "/mirrors") => list_mirrors(Arc::clone(&orchestrator)),
        ("GET", "/mirrors/all") => list_all_mirrors(Arc::clone(&orchestrator)),
        ("GET", "/mirrors/extended") => list_extended_mirrors(Arc::clone(&orchestrator)),
        ("GET", "/status") => status_page(Arc::clone(&orchestrator)),
        ("POST", "/mirror/create") => create_mirror(Arc::clone(&orchestrator)).await,
        ("POST", "/mirror/create-standby") => {
            create_standby_mirror(Arc::clone(&orchestrator)).await
        }
        ("POST", "/mirror/activate") => {
            activate_standby_mirror(req, Arc::clone(&orchestrator)).await
        }
        ("POST", "/mirror/pause") => pause_mirror(req, Arc::clone(&orchestrator)).await,
        ("POST", "/mirror/resume") => resume_mirror(req, Arc::clone(&orchestrator)).await,
        ("POST", "/mirror/destroy") => destroy_mirror(req, Arc::clone(&orchestrator)).await,
        _ => {
            // Check if this is a request to a paused mirror
            let host = req
                .headers()
                .get("Host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            if host.contains(".onion") && orchestrator.is_mirror_paused(host) {
                // Serve paused mirror page
                return Ok(serve_paused_mirror_page(&orchestrator));
            }

            // Forward all other requests to gate
            proxy_to_gate(req, gate_address).await
        }
    };

    let duration = start.elapsed();
    tracing::debug!(
        "{} {} - {} - {:?}",
        method,
        path,
        response.status(),
        duration
    );

    Ok(response)
}

/// Health check endpoint
fn health_check(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    let active_mirrors = orchestrator.get_active_mirrors();

    if active_mirrors.is_empty() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(Full::new(Bytes::from("No active mirrors")))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::json!({
                "status": "healthy",
                "active_mirrors": active_mirrors.len(),
            })
            .to_string(),
        )))
        .unwrap()
}

/// List active mirrors
fn list_mirrors(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    let mirrors = orchestrator.get_active_mirrors();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::json!({
                "mirrors": mirrors,
                "count": mirrors.len(),
            })
            .to_string(),
        )))
        .unwrap()
}

/// List ALL mirrors with status (for admin panel)
fn list_all_mirrors(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    let mirrors = orchestrator.get_all_mirrors();

    let mirror_data: Vec<serde_json::Value> = mirrors
        .iter()
        .map(|(id, onion, status)| {
            serde_json::json!({
                "id": id,
                "onion_address": onion,
                "status": status,
            })
        })
        .collect();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::json!({
                "mirrors": mirror_data,
                "count": mirrors.len(),
            })
            .to_string(),
        )))
        .unwrap()
}

/// List ALL mirrors with extended info (PoW status, standby status, etc.)
fn list_extended_mirrors(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    let mirrors = orchestrator.get_all_mirrors_extended();

    let mirror_data: Vec<serde_json::Value> = mirrors
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "onion_address": m.onion_address,
                "status": m.status,
                "pow_enabled": m.pow_enabled,
                "is_standby": m.is_standby,
                "file_based": m.file_based,
            })
        })
        .collect();

    let active_count = mirrors.iter().filter(|m| m.status == "active").count();
    let standby_count = mirrors.iter().filter(|m| m.is_standby).count();
    let pow_count = mirrors.iter().filter(|m| m.pow_enabled).count();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::json!({
                "mirrors": mirror_data,
                "count": mirrors.len(),
                "active_count": active_count,
                "standby_count": standby_count,
                "pow_enabled_count": pow_count,
            })
            .to_string(),
        )))
        .unwrap()
}

/// Generate HTML list of active mirrors
fn generate_mirror_list_html(orchestrator: &Orchestrator) -> String {
    let mirrors = orchestrator.get_all_mirrors_extended();
    let active_mirrors: Vec<_> = mirrors.iter().filter(|m| m.status == "active").collect();

    if active_mirrors.is_empty() {
        return r#"<li class="no-mirrors">No active mirrors available at this time</li>"#
            .to_string();
    }

    active_mirrors
        .iter()
        .map(|m| {
            let pow_badge = if m.pow_enabled {
                r#"<span class="pow-badge">PoW</span>"#
            } else {
                ""
            };
            format!(
                r#"<li><a href="http://{}">{}{}</a></li>"#,
                m.onion_address, m.onion_address, pow_badge
            )
        })
        .collect::<Vec<_>>()
        .join("\n                ")
}

/// Serve a static page for paused/maintenance mirrors
fn serve_paused_mirror_page(orchestrator: &Orchestrator) -> Response<BoxBody> {
    let mirror_list = generate_mirror_list_html(orchestrator);

    // Try to load the maintenance.html template
    let html = match std::fs::read_to_string("assets/html/maintenance.html") {
        Ok(template) => template.replace("{{MIRROR_LIST}}", &mirror_list),
        Err(_) => {
            // Fallback inline HTML if file not found
            let active_mirrors = orchestrator.get_active_mirrors();
            let alt_mirror_links = if active_mirrors.is_empty() {
                "<p style='color: #888;'>No alternative mirrors available</p>".to_string()
            } else {
                active_mirrors.iter()
                    .map(|m| format!(r#"<a href="http://{}" style="display: block; color: #00ff88; margin: 10px 0;">{}</a>"#, m, m))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mirror Maintenance - Fortify</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(135deg, #0a0a0f 0%, #1a1a2e 50%, #0a0a0f 100%);
            font-family: 'Segoe UI', system-ui, sans-serif;
            color: #e0e0e0;
            padding: 20px;
        }}
        .container {{
            max-width: 600px;
            text-align: center;
            background: rgba(20, 20, 35, 0.9);
            border: 1px solid rgba(255, 165, 0, 0.4);
            border-radius: 12px;
            padding: 40px;
            box-shadow: 0 0 40px rgba(255, 165, 0, 0.2);
        }}
        .icon {{ font-size: 64px; margin-bottom: 20px; }}
        h1 {{ color: #ffa500; font-size: 2em; margin-bottom: 15px; }}
        p {{ color: #aaa; line-height: 1.6; margin-bottom: 25px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">⚙️</div>
        <h1>Mirror Under Maintenance</h1>
        <p>This mirror is temporarily offline. Please use an alternative mirror:</p>
        {}
        <p style="margin-top: 30px; font-size: 0.8em; color: #666;">🛡️ Protected by Fortify</p>
    </div>
</body>
</html>"#,
                alt_mirror_links
            )
        }
    };

    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

/// Create a new mirror (triggered by admin panel)
async fn create_mirror(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    tracing::info!("Admin requested new mirror creation");

    match orchestrator.spawn_mirror().await {
        Ok(onion_addr) => {
            tracing::info!("Successfully created new mirror: {}", onion_addr);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "created",
                        "onion_address": onion_addr,
                    })
                    .to_string(),
                )))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to create mirror: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    })
                    .to_string(),
                )))
                .unwrap()
        }
    }
}

/// Create a new standby mirror (paused, ready for activation)
async fn create_standby_mirror(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    tracing::info!("Admin requested new standby mirror creation");

    match orchestrator.spawn_standby_mirror().await {
        Ok(mirror_id) => {
            // Get the onion address
            let onion_addr = orchestrator
                .get_mirror(&mirror_id)
                .and_then(|m| m.onion_address)
                .unwrap_or_else(|| "unknown".to_string());

            tracing::info!(
                "Successfully created standby mirror: {} ({})",
                mirror_id,
                onion_addr
            );
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "created_standby",
                        "mirror_id": mirror_id,
                        "onion_address": onion_addr,
                        "message": "Mirror created as standby (paused). Use /mirror/activate to make it active."
                    })
                    .to_string(),
                )))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to create standby mirror: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    })
                    .to_string(),
                )))
                .unwrap()
        }
    }
}

/// Activate a standby mirror (change from paused to active)
async fn activate_standby_mirror(
    req: Request<Incoming>,
    orchestrator: Arc<Orchestrator>,
) -> Response<BoxBody> {
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid body")))
                .unwrap();
        }
    };

    let json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(j) => j,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid JSON")))
                .unwrap();
        }
    };

    let onion_address = match json.get("onion_address").and_then(|v| v.as_str()) {
        Some(addr) => addr.to_string(),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing onion_address")))
                .unwrap();
        }
    };

    tracing::info!(
        "Admin requested activation for standby mirror: {}",
        onion_address
    );

    match orchestrator.activate_standby(&onion_address).await {
        Ok(_) => {
            tracing::info!("Successfully activated standby mirror: {}", onion_address);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "activated",
                        "onion_address": onion_address,
                        "message": "Mirror is now active and accepting traffic"
                    })
                    .to_string(),
                )))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to activate standby mirror {}: {}", onion_address, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    })
                    .to_string(),
                )))
                .unwrap()
        }
    }
}

/// Pause a mirror (triggered by admin panel)
async fn pause_mirror(
    req: Request<Incoming>,
    orchestrator: Arc<Orchestrator>,
) -> Response<BoxBody> {
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid body")))
                .unwrap();
        }
    };

    let json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(j) => j,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid JSON")))
                .unwrap();
        }
    };

    let onion_address = match json.get("onion_address").and_then(|v| v.as_str()) {
        Some(addr) => addr.to_string(),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing onion_address")))
                .unwrap();
        }
    };

    tracing::info!("Admin requested pause for mirror: {}", onion_address);

    match orchestrator.pause_mirror(&onion_address).await {
        Ok(_) => {
            tracing::info!("Successfully paused mirror: {}", onion_address);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "paused",
                        "onion_address": onion_address,
                    })
                    .to_string(),
                )))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to pause mirror {}: {}", onion_address, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    })
                    .to_string(),
                )))
                .unwrap()
        }
    }
}

/// Resume a paused mirror (triggered by admin panel)
async fn resume_mirror(
    req: Request<Incoming>,
    orchestrator: Arc<Orchestrator>,
) -> Response<BoxBody> {
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid body")))
                .unwrap();
        }
    };

    let json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(j) => j,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid JSON")))
                .unwrap();
        }
    };

    let onion_address = match json.get("onion_address").and_then(|v| v.as_str()) {
        Some(addr) => addr.to_string(),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing onion_address")))
                .unwrap();
        }
    };

    tracing::info!("Admin requested resume for mirror: {}", onion_address);

    match orchestrator.resume_mirror(&onion_address).await {
        Ok(_) => {
            tracing::info!("Successfully resumed mirror: {}", onion_address);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "active",
                        "onion_address": onion_address,
                    })
                    .to_string(),
                )))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to resume mirror {}: {}", onion_address, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    })
                    .to_string(),
                )))
                .unwrap()
        }
    }
}

/// Destroy a mirror permanently (triggered by admin panel)
async fn destroy_mirror(
    req: Request<Incoming>,
    orchestrator: Arc<Orchestrator>,
) -> Response<BoxBody> {
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid body")))
                .unwrap();
        }
    };

    let json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(j) => j,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid JSON")))
                .unwrap();
        }
    };

    let onion_address = match json.get("onion_address").and_then(|v| v.as_str()) {
        Some(addr) => addr.to_string(),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing onion_address")))
                .unwrap();
        }
    };

    tracing::warn!("Admin requested DESTROY for mirror: {}", onion_address);

    match orchestrator.destroy_mirror(&onion_address).await {
        Ok(_) => {
            tracing::warn!("Successfully destroyed mirror: {}", onion_address);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "destroyed",
                        "onion_address": onion_address,
                    })
                    .to_string(),
                )))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to destroy mirror {}: {}", onion_address, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    })
                    .to_string(),
                )))
                .unwrap()
        }
    }
}

/// Get orchestrator statistics as JSON (for TUI polling)
fn get_stats(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    let captcha_stats = orchestrator.captcha_pool_stats();
    let all_mirrors = orchestrator.get_all_mirrors_extended();
    let active_count = all_mirrors
        .iter()
        .filter(|m| m.status == "active" && !m.is_standby)
        .count();
    let standby_count = all_mirrors.iter().filter(|m| m.is_standby).count();

    let stats = serde_json::json!({
        "captcha_pool": {
            "current_size": captcha_stats.current_size,
            "target_size": captcha_stats.target_size,
            "min_size": captcha_stats.min_size,
            "max_size": captcha_stats.max_size,
            "total_generated": captcha_stats.total_generated,
            "total_served": captcha_stats.total_served,
            "needs_refill": captcha_stats.needs_refill
        },
        "mirrors": {
            "active": active_count,
            "standby": standby_count
        },
        "orchestrator_count": 1
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(stats.to_string())))
        .unwrap()
}

/// Status page (HTML)
fn status_page(orchestrator: Arc<Orchestrator>) -> Response<BoxBody> {
    let active_mirrors = orchestrator.get_active_mirrors();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Fortify Orchestrator Status</title>
    <style>
        body {{
            font-family: monospace;
            background: #111;
            color: #0f0;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
        }}
        h1 {{ text-align: center; border-bottom: 1px solid #0f0; padding-bottom: 10px; }}
        .mirror {{ background: #222; padding: 10px; margin: 10px 0; border: 1px solid #0f0; }}
        .count {{ color: #ff0; font-size: 1.2em; }}
    </style>
</head>
<body>
    <h1>⚡ FORTIFY ORCHESTRATOR ⚡</h1>
    <p class="count">Active Mirrors: {}</p>
    <div>
        {}
    </div>
</body>
</html>"#,
        active_mirrors.len(),
        active_mirrors
            .iter()
            .enumerate()
            .map(|(i, addr)| format!("<div class=\"mirror\">Mirror {}: {}</div>", i + 1, addr))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

/// Proxy request to gate
async fn proxy_to_gate(req: Request<Incoming>, gate_address: String) -> Response<BoxBody> {
    // Build gate URI
    let gate_uri = format!(
        "{}{}",
        gate_address,
        req.uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/")
    );

    let uri: hyper::Uri = match gate_uri.parse() {
        Ok(u) => u,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("Invalid gate URI")))
                .unwrap();
        }
    };

    // Build a new request with the gate URI
    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("Failed to read request body")))
                .unwrap();
        }
    };

    let mut builder = Request::builder().method(parts.method).uri(uri);

    // Copy headers
    for (key, value) in parts.headers.iter() {
        builder = builder.header(key, value);
    }

    let new_req = match builder.body(Full::new(body_bytes)) {
        Ok(r) => r,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("Failed to build request")))
                .unwrap();
        }
    };

    // Forward to gate using legacy client
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();
    match client.request(new_req).await {
        Ok(response) => {
            // Convert the response body
            let (parts, body) = response.into_parts();
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(_) => Bytes::from("Failed to read response"),
            };
            Response::from_parts(parts, Full::new(body_bytes))
        }
        Err(e) => {
            tracing::error!("Failed to proxy to gate: {}", e);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("Gate unavailable")))
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OrchestratorConfig;

    #[tokio::test]
    async fn test_health_check_no_mirrors() {
        let config = OrchestratorConfig::default();
        let orch = Arc::new(Orchestrator::new(config));

        let response = health_check(orch);
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_health_check_with_mirrors() {
        let config = OrchestratorConfig::default();
        let orch = Arc::new(Orchestrator::new(config));

        orch.spawn_mirror().await.unwrap();

        let response = health_check(orch);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_mirrors() {
        let config = OrchestratorConfig::default();
        let orch = Arc::new(Orchestrator::new(config));

        orch.spawn_mirror().await.unwrap();
        orch.spawn_mirror().await.unwrap();

        let response = list_mirrors(orch);
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("count"));
        assert!(body_str.contains("mirrors"));
    }
}
