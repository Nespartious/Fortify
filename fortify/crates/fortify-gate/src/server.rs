use crate::captcha_html::{
    render_captcha_page_with_timer, render_captcha_page_with_timer_and_reason,
};
use crate::captcha_types::{CaptchaData, CaptchaType};
use crate::Gate;
use crate::GateError;
use bytes::Bytes;
use fortify_core::safe_lock;
use fortify_core::templates::{BrandingVars, TemplateEngine, TemplateType};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

type BoxBody = Full<Bytes>;
// form_urlencoded is available via crate root if dependency added

/// HTTP server for the gate
pub struct GateServer {
    gate: Arc<Gate>,
    static_dir: String,
}

impl GateServer {
    pub fn new(gate: Arc<Gate>, static_dir: String) -> Self {
        Self { gate, static_dir }
    }

    pub async fn start(&self, addr: SocketAddr) -> anyhow::Result<()> {
        let gate = Arc::clone(&self.gate);
        let static_dir = self.static_dir.clone();

        let listener = TcpListener::bind(addr).await?;
        tracing::info!("Gate HTTP server listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let gate = Arc::clone(&gate);
            let static_dir = static_dir.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    handle_request(req, Arc::clone(&gate), static_dir.clone())
                });

                // Gate server with timeouts to protect against slow-loris attacks
                // 30s header timeout accommodates Tor latency
                let result = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .header_read_timeout(Duration::from_secs(30))
                    .max_buf_size(16 * 1024)
                    .serve_connection(io, service)
                    .await;

                if let Err(err) = result {
                    tracing::error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request(
    req: Request<Incoming>,
    gate: Arc<Gate>,
    _static_dir: String,
) -> Result<Response<BoxBody>, Infallible> {
    let path = req.uri().path();
    let method = req.method();

    // Extract cookies for session tracking
    let cookies = req
        .headers()
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Check if user was demoted - either via cookie OR via header from HTTP proxy
    // The header is set by HTTP proxy for immediate detection (before cookie round-trip)
    let was_demoted = cookies.contains("fortify_demoted=1")
        || req
            .headers()
            .get("X-Fortify-Demoted")
            .map(|v| v == "1")
            .unwrap_or(false);

    if was_demoted {
        tracing::info!("Demoted user detected at Gate (cookie or header)");
    }

    // Extract existing session ID for preservation (even if demoted)
    // This is stored when user is demoted so we can track them across re-verifications
    let existing_session_id = cookies
        .split(';')
        .map(|c| c.trim())
        .find(|c| c.starts_with("fortify_original_session="))
        .and_then(|c| c.strip_prefix("fortify_original_session="))
        .map(String::from);

    // Also check for pending session from HTTP proxy (new visitors assigned session before reaching Gate)
    let pending_session_id = cookies
        .split(';')
        .map(|c| c.trim())
        .find(|c| c.starts_with("fortify_pending_session="))
        .and_then(|c| c.strip_prefix("fortify_pending_session="))
        .map(String::from);

    // Use existing session > pending session > generate new
    let session_id_for_captcha = existing_session_id.clone().or(pending_session_id);

    // Query string for routing
    let query = req.uri().query().unwrap_or("");

    let response = match (method, path) {
        // Landing page: different content for new vs demoted users
        (&Method::GET, "/Fortify") => {
            // NOTE: Cookie compliance check removed - was causing false positives
            // with Tor Browser and privacy-focused clients. The CAPTCHA challenge
            // provides sufficient bot protection without pre-filtering.

            if was_demoted {
                // Demoted user: show "hold position" friendly message, clear demoted cookie
                serve_demoted_page(gate)
            } else {
                // New user: show the landing page (gate.html)
                serve_landing_page(gate)
            }
        }

        // The captcha challenge page - accessible by all
        // Pass the session ID from cookie (pending or existing) to preserve identity
        // Also pass demoted status to ensure threat sessions get proper treatment
        (&Method::GET, "/Fortify/Portcullis") => {
            // Parse query parameters for reason
            let reason = query
                .split('&')
                .find(|p| p.starts_with("reason="))
                .and_then(|p| p.strip_prefix("reason="));
            serve_captcha_challenge(gate, session_id_for_captcha, reason, was_demoted)
        }

        // Dynamic routes
        (&Method::POST, "/gate/verify") => verify_submission(req, gate).await,
        (&Method::POST, "/gate/upgrade-token") => handle_token_upgrade(req, gate).await,
        (&Method::GET, p) if p.starts_with("/gate/captcha/") => serve_captcha_image(p, gate).await,

        // Admin API: update captcha configuration
        (&Method::POST, "/gate/admin/captcha-config") => {
            handle_update_captcha_config(req, gate).await
        }

        // Admin API: update branding configuration
        (&Method::POST, "/gate/admin/branding") => handle_update_branding_config(req, gate).await,

        // API: Get pre-rendered CAPTCHA page for HTTP Proxy caching
        // Returns JSON with HTML, session_id, and cookie headers
        (&Method::GET, "/gate/api/prerendered-page") => serve_prerendered_page_api(gate),

        // Catch-all: redirect everyone to /Fortify landing
        // Also clear any stale session cookie to prevent redirect loops
        _ => Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/Fortify")
            .header(
                "Set-Cookie",
                "fortify_session=; Path=/; Max-Age=0; HttpOnly",
            )
            .body(Full::new(Bytes::new()))
            .expect("valid response"),
    };

    Ok(response)
}

fn serve_landing_page(gate: Arc<Gate>) -> Response<BoxBody> {
    // Landing page for NEW users (first-time visitors)
    // NOW serves the combined gate-challenge page with embedded CAPTCHA
    // This eliminates the 2-page hop (landing → captcha)
    // NO JAVASCRIPT ALLOWED

    // Generate a new session ID for this visitor
    let session_id = uuid::Uuid::new_v4().to_string();

    // Get captcha configuration
    let config = gate.get_captcha_config();
    let captcha_type = config.gate_captcha_type;

    // Create verification session and generate CAPTCHA
    // Multi-type CAPTCHAs (Emoji, Direction, etc.) are lightweight and should never fail
    // If BmpText fails, fall back to Emoji which is guaranteed to work
    let state = match gate.create_verification_with_type(
        session_id.clone(),
        captcha_type,
        crate::CaptchaDifficulty::Medium,
        false, // not threat mode for new visitors
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                "Failed to create verification session with {:?}: {}",
                captcha_type,
                e
            );
            tracing::warn!("Falling back to Emoji CAPTCHA type");
            // Fallback to lightweight Emoji CAPTCHA which is guaranteed to work
            match gate.create_verification_with_type(
                session_id.clone(),
                CaptchaType::Emoji,
                crate::CaptchaDifficulty::Medium,
                false,
            ) {
                Ok(s) => s,
                Err(e2) => {
                    tracing::error!("CRITICAL: Even Emoji CAPTCHA failed: {}", e2);
                    // Return 503 Service Unavailable instead of old landing page
                    return Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header("Content-Type", "text/html")
                        .body(Full::new(Bytes::from(
                            r#"<!DOCTYPE html>
                            <html><head><meta charset="UTF-8"><title>Service Unavailable</title></head>
                            <body style="background:#141417;color:#f5f0e8;font-family:sans-serif;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0;">
                                <div style="text-align:center;max-width:500px;padding:2rem;">
                                    <h1 style="color:#e4bc5e;margin-bottom:1rem;">⚠ Service Temporarily Unavailable</h1>
                                    <p>The security verification system is experiencing high load. Please wait a moment and try again.</p>
                                    <p style="margin-top:2rem;"><a href="/" style="color:#9ab893;text-decoration:none;">← Retry</a></p>
                                </div>
                            </body></html>"#
                        )))
                        .expect("valid response");
                }
            }
        }
    };

    // Generate the CAPTCHA content HTML dynamically based on type
    let (captcha_content, instruction, input_type) = if let Some(ref captcha_data) =
        state.captcha_data
    {
        crate::captcha_html::render_captcha_content_for_landing(
            &session_id,
            &session_id,
            captcha_data,
        )
    } else {
        // Fallback for legacy BmpText (no captcha_data, uses image URL)
        let content = format!(
            r#"<img src="/gate/captcha/{}" alt="Security Challenge" style="max-width: 100%; height: auto;">"#,
            session_id
        );
        (content, captcha_type.description().to_string(), "text")
    };

    // Generate input HTML based on whether this is text-based or selection-based
    let input_html = if input_type == "text" {
        r#"<div class="input-group">
                    <label for="captcha">Enter Code</label>
                    <input type="text" id="captcha" name="captcha" placeholder="• • • • • •" required autofocus autocomplete="off">
                </div>"#.to_string()
    } else {
        // Selection-based CAPTCHAs don't need a text input - buttons submit directly
        String::new()
    };

    // Generate submit button HTML (only for text-based CAPTCHAs)
    let submit_html = if input_type == "text" {
        r#"<button type="submit">Verify &amp; Enter</button>"#.to_string()
    } else {
        // Selection-based CAPTCHAs submit via the option buttons
        String::new()
    };

    // Render the combined gate-challenge template
    let engine = TemplateEngine::new();
    let branding = gate.branding().clone();
    let mut extra_vars = std::collections::HashMap::new();
    extra_vars.insert("CAPTCHA_CONTENT".to_string(), captcha_content);
    extra_vars.insert("CAPTCHA_INSTRUCTION".to_string(), instruction);
    extra_vars.insert("CAPTCHA_INPUT".to_string(), input_html);
    extra_vars.insert("CAPTCHA_SUBMIT".to_string(), submit_html);
    extra_vars.insert("SESSION_ID".to_string(), session_id.clone());
    extra_vars.insert("CAPTCHA_ID".to_string(), session_id.clone());

    let html =
        engine.render_with_branding(TemplateType::GateChallenge, &branding, Some(&extra_vars));

    tracing::info!(
        "Serving combined gate-challenge page for new session: {}, captcha_type={:?}",
        session_id,
        captcha_type
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        // Clear any stale fortify_session cookie to break redirect loops after service restart
        .header(
            "Set-Cookie",
            "fortify_session=; Path=/; Max-Age=0; HttpOnly",
        )
        // Set pending session cookie so the session is tracked through verification
        .header(
            "Set-Cookie",
            format!("fortify_pending_session={}; Path=/; HttpOnly", session_id),
        )
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

fn serve_demoted_page(gate: Arc<Gate>) -> Response<BoxBody> {
    // Demoted users see the "Hold Position" page with a friendly message
    // They click "Resume Access" to go to /Fortify/Portcullis for the 2-captcha challenge
    // This intermediate page reduces friction and explains what's happening

    // Use template engine to render demoted.html with branding
    let engine = TemplateEngine::new();
    let branding = gate.branding().clone();

    // Mirror list is currently not available from Gate context
    // Hide the section by providing an empty list with a message
    let mut extra_vars = std::collections::HashMap::new();
    extra_vars.insert(
        "MIRROR_LIST".to_string(),
        "<li><a href=\"/Fortify/Portcullis\">Click Resume Access above to continue</a></li>"
            .to_string(),
    );

    let html = engine.render_with_branding(TemplateType::Demoted, &branding, Some(&extra_vars));

    // Build response - do NOT clear demoted cookie here
    // The demoted cookie will be cleared when verification succeeds at /gate/verify
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

/// API endpoint for HTTP Proxy to fetch pre-rendered CAPTCHA pages
/// Returns JSON with the rendered HTML and session metadata
/// This allows HTTP Proxy to cache and serve pages without full proxy overhead
fn serve_prerendered_page_api(gate: Arc<Gate>) -> Response<BoxBody> {
    // Generate a new session ID for this visitor
    let session_id = uuid::Uuid::new_v4().to_string();

    // Get captcha configuration
    let config = gate.get_captcha_config();
    let captcha_type = config.gate_captcha_type;

    // Create verification session and generate CAPTCHA
    let state = match gate.create_verification_with_type(
        session_id.clone(),
        captcha_type,
        crate::CaptchaDifficulty::Medium,
        false, // not threat mode for new visitors
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create verification session for API: {}", e);
            let error_json = serde_json::json!({
                "error": "failed_to_create_session",
                "message": e.to_string()
            });
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(error_json.to_string())))
                .expect("valid response");
        }
    };

    // Generate the CAPTCHA content HTML dynamically based on type
    let (captcha_content, instruction, input_type) = if let Some(ref captcha_data) =
        state.captcha_data
    {
        crate::captcha_html::render_captcha_content_for_landing(
            &session_id,
            &session_id,
            captcha_data,
        )
    } else {
        // Fallback for legacy BmpText (no captcha_data, uses image URL)
        let content = format!(
            r#"<img src="/gate/captcha/{}" alt="Security Challenge" style="max-width: 100%; height: auto;">"#,
            session_id
        );
        (content, captcha_type.description().to_string(), "text")
    };

    // Generate input HTML based on whether this is text-based or selection-based
    let input_html = if input_type == "text" {
        r#"<div class="input-group">
                    <label for="captcha">Enter Code</label>
                    <input type="text" id="captcha" name="captcha" placeholder="• • • • • •" required autofocus autocomplete="off">
                </div>"#.to_string()
    } else {
        // Selection-based CAPTCHAs don't need a text input - buttons submit directly
        String::new()
    };

    // Generate submit button HTML (only for text-based CAPTCHAs)
    let submit_html = if input_type == "text" {
        r#"<button type="submit">Verify &amp; Enter</button>"#.to_string()
    } else {
        // Selection-based CAPTCHAs submit via the option buttons
        String::new()
    };

    // Render the combined gate-challenge template
    let engine = TemplateEngine::new();
    let branding = gate.branding().clone();
    let mut extra_vars = std::collections::HashMap::new();
    extra_vars.insert("CAPTCHA_CONTENT".to_string(), captcha_content);
    extra_vars.insert("CAPTCHA_INSTRUCTION".to_string(), instruction);
    extra_vars.insert("CAPTCHA_INPUT".to_string(), input_html);
    extra_vars.insert("CAPTCHA_SUBMIT".to_string(), submit_html);
    extra_vars.insert("SESSION_ID".to_string(), session_id.clone());
    extra_vars.insert("CAPTCHA_ID".to_string(), session_id.clone());

    let html =
        engine.render_with_branding(TemplateType::GateChallenge, &branding, Some(&extra_vars));

    tracing::info!(
        "API: Generated pre-rendered page for session: {}, captcha_type={:?}",
        session_id,
        captcha_type
    );

    // Return JSON with all the data HTTP Proxy needs
    let response_json = serde_json::json!({
        "html": html,
        "session_id": session_id,
        "captcha_id": session_id,
        "cookies": [
            format!("fortify_session=; Path=/; Max-Age=0; HttpOnly"),
            format!("fortify_pending_session={}; Path=/; HttpOnly", session_id)
        ]
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(response_json.to_string())))
        .expect("valid response")
}

/// Render a page for the second captcha challenge (for demoted/threat sessions)
/// Uses the existing captcha page renderer and adds a progress indicator
fn render_second_captcha_page(
    session_id: &str,
    captcha_id: &str,
    captcha_data: &CaptchaData,
    captchas_solved: u8,
    timeout_seconds: u64,
) -> String {
    // Use the existing captcha page renderer (with threat mode styling)
    let base_page =
        render_captcha_page_with_timer(session_id, captcha_id, captcha_data, true, timeout_seconds);

    // Add a progress badge to indicate this is the second captcha
    let step = captchas_solved + 1;
    let progress_badge = format!(
        r#"<div style="display: inline-block; background: #e4bc5e; color: #141417; padding: 6px 16px; font-size: 0.75rem; font-weight: 600; letter-spacing: 1px; text-transform: uppercase; margin-bottom: 15px; border-radius: 2px;">Step {} of 2</div>"#,
        step
    );

    // Insert the progress badge after the opening panel div
    // Look for the title element and insert before it
    if base_page.contains("<h1") {
        base_page.replacen("<h1", &format!("{}<h1", progress_badge), 1)
    } else {
        base_page
    }
}

fn serve_captcha_challenge(
    gate: Arc<Gate>,
    existing_session_id: Option<String>,
    reason: Option<&str>,
    is_demoted: bool,
) -> Response<BoxBody> {
    // Preserve existing session ID if available (demoted user re-verifying)
    // This keeps the same session ID so we can continue tracking them
    let session_id = existing_session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    tracing::debug!(
        "serve_captcha_challenge entry: session={}, existing_id={:?}, is_demoted={}",
        session_id,
        existing_session_id,
        is_demoted
    );

    // Check if this is an existing session - handle various states
    if let Some(existing_state) = gate.get_verification_state(&session_id) {
        tracing::debug!(
            "Found existing session {}: is_threat={}, captchas_remaining={}, captcha_solved={}",
            session_id,
            existing_state.is_threat,
            existing_state.captchas_remaining,
            existing_state.captcha_solved
        );

        // If session already completed all captchas, we need to create a fresh session
        // This happens when a demoted user returns after previously completing verification
        if existing_state.captchas_remaining == 0 {
            tracing::info!(
                "Session {} already completed (captchas_remaining=0), creating fresh session for re-verification",
                session_id
            );
            // Fall through to create new session below
        } else if existing_state.is_threat && existing_state.captchas_remaining > 0 {
            // This is an active threat session with captchas still needed - return the existing captcha page
            let timeout_seconds = gate.get_verification_timeout();
            if let Some(ref captcha_data) = existing_state.captcha_data {
                tracing::info!(
                    "Returning existing captcha for active threat session {}, captchas_remaining={}",
                    session_id, existing_state.captchas_remaining
                );
                let html = render_captcha_page_with_timer_and_reason(
                    &session_id,
                    &session_id,
                    captcha_data,
                    true,
                    timeout_seconds,
                    reason,
                );
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/html")
                    .body(Full::new(Bytes::from(html)))
                    .expect("valid response");
            }
        }
        // For non-threat sessions with remaining captchas, fall through to refresh
    }

    // Demoted users (from cookie) should be treated as threat mode with 2 captchas
    // This ensures they don't bypass threat handling by navigating to /Fortify/Portcullis directly
    let is_threat_mode = is_demoted;

    // Get captcha type from configuration
    // CRITICAL: For threat/demoted users, the FIRST captcha uses gate_captcha_type
    // The SECOND captcha (after AdditionalCaptchaRequired) uses threat_captcha_type
    // This ensures the two captchas are DIFFERENT types as required
    let config = gate.get_captcha_config();
    let captcha_type = if is_threat_mode {
        // First captcha for threat users: use gate type (BmpText by default)
        // Second captcha will use threat type (Emoji by default) - set in verify handler
        config.gate_captcha_type
    } else {
        config.get_captcha_type(false)
    };
    let timeout_seconds = gate.get_verification_timeout();

    let state = match gate.create_verification_with_type(
        session_id.clone(),
        captcha_type,
        if is_threat_mode {
            crate::CaptchaDifficulty::Hard
        } else {
            crate::CaptchaDifficulty::Medium
        },
        is_threat_mode, // threat mode based on demoted status
    ) {
        Ok(s) => s,
        Err(e) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("Gate busy: {}", e),
            )
        }
    };

    tracing::info!(
        "serve_captcha_challenge: session={}, is_demoted={}, is_threat_mode={}, captchas_remaining={}, captcha_type={:?}",
        session_id, is_demoted, is_threat_mode, state.captchas_remaining, captcha_type
    );

    // Render the appropriate captcha page based on type
    let html = if let Some(ref captcha_data) = state.captcha_data {
        render_captcha_page_with_timer_and_reason(
            &state.session_id,
            &state.session_id,
            captcha_data,
            is_threat_mode,
            timeout_seconds,
            reason,
        )
    } else {
        // Fallback: use template engine for BMP text captcha page
        let captcha_id = &state.session_id;
        let engine = TemplateEngine::new();
        let branding = gate.branding().clone();
        let mut extra_vars = std::collections::HashMap::new();
        extra_vars.insert(
            "CAPTCHA_IMAGE_URL".to_string(),
            format!("/gate/captcha/{}", captcha_id),
        );
        extra_vars.insert("SESSION_ID".to_string(), session_id.to_string());
        extra_vars.insert("CAPTCHA_TYPE".to_string(), "bmptext".to_string());
        engine.render_with_branding(TemplateType::Captcha, &branding, Some(&extra_vars))
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

/// Handle verification token upgrade to session token
async fn handle_token_upgrade(req: Request<Incoming>, gate: Arc<Gate>) -> Response<BoxBody> {
    // Extract User-Agent before consuming body
    let current_ua = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Parse JSON body
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid request body"),
    };

    let request: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON"),
    };

    let verification_token_str = match request["verification_token"].as_str() {
        Some(t) => t,
        None => return error_response(StatusCode::BAD_REQUEST, "Missing verification_token"),
    };

    // Decode and validate verification token
    let verification_token = match crate::VerificationToken::decode(verification_token_str) {
        Ok(token) => token,
        Err(_) => {
            tracing::warn!("Invalid verification token format");
            return error_response(StatusCode::UNAUTHORIZED, "Invalid verification token");
        }
    };

    // Check if token is expired
    if !verification_token.is_valid() {
        tracing::warn!("Expired verification token: {}", verification_token.user_id);
        return error_response(StatusCode::UNAUTHORIZED, "Verification token expired");
    }

    // Validate User-Agent matches
    if !verification_token.validate_user_agent(&current_ua) {
        tracing::warn!(
            "User-Agent mismatch for token {}",
            verification_token.user_id
        );
        return error_response(StatusCode::UNAUTHORIZED, "User-Agent mismatch");
    }

    // Check if token already used (atomic check-and-mark)
    let mut cache = safe_lock(&crate::VERIFICATION_TOKEN_CACHE);
    match cache.get_mut(&verification_token.user_id) {
        Some(cached_token) => {
            if cached_token.uses_remaining == 0 {
                tracing::warn!(
                    "Verification token already used: {}",
                    verification_token.user_id
                );
                return error_response(StatusCode::UNAUTHORIZED, "Token already used");
            }

            // Mark token as used
            cached_token.mark_used();
            tracing::info!(
                "Upgraded verification token {} to session",
                verification_token.user_id
            );
        }
        None => {
            tracing::warn!(
                "Verification token not found in cache: {}",
                verification_token.user_id
            );
            return error_response(StatusCode::UNAUTHORIZED, "Token not found");
        }
    }
    drop(cache); // Release lock

    // Create session token (long-lived, Verified tier) with User-Agent binding
    let session_token = gate.create_session_token(
        &verification_token.user_id,
        fortify_core::TrustTier::Verified,
        &current_ua,
    );

    tracing::info!(
        "Created session token for user {}, tier: Verified",
        verification_token.user_id
    );

    // Return session token as JSON
    let response_json = serde_json::json!({
        "session_token": session_token,
        "tier": "Verified",
        "message": "Token upgraded successfully"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(response_json.to_string())))
        .expect("valid response")
}

// Renamed for clarity in tool usage, originally serve_captcha in code
async fn serve_captcha_image(path: &str, gate: Arc<Gate>) -> Response<BoxBody> {
    serve_captcha(path, gate).await
}

// Keep original serve_captcha for valid signature
async fn serve_captcha(path: &str, gate: Arc<Gate>) -> Response<BoxBody> {
    let id = match path.strip_prefix("/gate/captcha/") {
        Some(id) => id,
        None => return not_found(),
    };

    // First try to get image data from the new captcha_data field
    if let Some(state) = gate.get_verification_state(id) {
        if let Some(CaptchaData::BmpText { ref image_data, .. }) = state.captcha_data {
            if !image_data.is_empty() {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "image/bmp")
                    .header("Cache-Control", "no-store, no-cache, must-revalidate")
                    .body(Full::new(Bytes::from(image_data.clone())))
                    .expect("valid response");
            }
        }
    }

    // Fallback to legacy captcha_challenge field
    match gate.get_captcha_challenge(id) {
        Some(challenge) => {
            if !challenge.image_data.is_empty() {
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "image/bmp")
                    .header("Cache-Control", "no-store, no-cache, must-revalidate")
                    .body(Full::new(Bytes::from(challenge.image_data.clone())))
                    .expect("valid response")
            } else {
                not_found()
            }
        }
        None => not_found(),
    }
}

async fn verify_submission(req: Request<Incoming>, gate: Arc<Gate>) -> Response<BoxBody> {
    // Extract User-Agent before consuming body
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body"),
    };

    let params: std::collections::HashMap<String, String> =
        form_urlencoded::parse(&body_bytes).into_owned().collect();

    let session_id = match params.get("session_id") {
        Some(s) => s,
        None => {
            return styled_error_response(
                StatusCode::BAD_REQUEST,
                "Missing session_id",
                "Session ID not found in form submission",
            )
        }
    };

    // Check for captcha answer - new captchas use 'selection' for button clicks,
    // while text-based captchas use 'captcha'
    let captcha = match params.get("captcha").or_else(|| params.get("selection")) {
        Some(c) => c,
        None => {
            return styled_error_response(
                StatusCode::BAD_REQUEST,
                "Missing Response",
                "No captcha answer was submitted. Please try again.",
            )
        }
    };

    let pow_nonce = match params.get("pow_nonce") {
        Some(n) => n.parse::<u64>().unwrap_or(0),
        None => 0,
    };

    // Log verification attempt details
    let is_threat = gate.is_threat_session(session_id);
    let captchas_remaining = gate.get_captchas_remaining(session_id);
    tracing::info!(
        "Verify submission for session {}: is_threat={}, captchas_remaining={}",
        session_id,
        is_threat,
        captchas_remaining
    );

    // Verify
    match gate.verify_submission(session_id, captcha, pow_nonce) {
        Ok(_token) => {
            // Success - Issue verification token instead of session token
            // Create verification token (60s TTL, single-use)
            let verification_token = crate::VerificationToken::new(&user_agent);
            let token_string = verification_token.encode();

            tracing::info!(
                "Issued verification token {} for User-Agent: {} (session will upgrade on first use)",
                verification_token.user_id, user_agent
            );

            // Store token in cache to track usage
            {
                let mut cache = safe_lock(&crate::VERIFICATION_TOKEN_CACHE);
                cache.insert(
                    verification_token.user_id.clone(),
                    verification_token.clone(),
                );
            }

            // Success page with verification token cookie
            // Random delay between 7-13 seconds for auto-redirect
            let delay_secs = {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                7 + (seed % 7) // 7 to 13 seconds
            };

            // Use the template engine for the verified page
            let engine = TemplateEngine::new();
            let branding = gate.branding().clone();
            let mut extra_vars = std::collections::HashMap::new();
            extra_vars.insert("REDIRECT_DELAY".to_string(), delay_secs.to_string());
            let html =
                engine.render_with_branding(TemplateType::Verified, &branding, Some(&extra_vars));

            // Set verification token cookie (60s expiry, single-use)
            // User must use this token on their next request to get a session token
            // Also clear fortify_demoted cookie now that verification is complete
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .header(
                    "Set-Cookie",
                    format!(
                        "fortify_verification={}; Path=/; HttpOnly; Max-Age=60; SameSite=Strict",
                        token_string
                    ),
                )
                .header(
                    "Set-Cookie",
                    format!(
                        "fortify_original_session={}; Path=/; HttpOnly; Max-Age=86400",
                        session_id
                    ),
                )
                .header(
                    "Set-Cookie",
                    "fortify_demoted=; Path=/; Max-Age=0; HttpOnly",
                )
                .body(Full::new(Bytes::from(html)))
                .expect("valid response")
        }
        Err(GateError::AdditionalCaptchaRequired) => {
            // First captcha solved - need second captcha for threat session
            // Get the threat captcha type from config (MUST be different from first captcha)
            // First captcha used gate_captcha_type, second uses threat_captcha_type
            let captcha_config = gate.get_captcha_config();
            let second_captcha_type = captcha_config.threat_captcha_type;

            tracing::info!(
                "Second captcha for threat session: type={:?} (first was {:?})",
                second_captcha_type,
                captcha_config.gate_captcha_type
            );

            // Regenerate captcha with the threat type for second challenge
            if gate
                .regenerate_captcha(session_id, second_captcha_type)
                .is_err()
            {
                return styled_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "System Error",
                    "Failed to generate second verification challenge. Please start over.",
                );
            }

            // Get the new captcha data and render the page
            let state = gate.get_verification_state(session_id);
            let captchas_solved = gate.get_captchas_solved(session_id);
            let timeout_seconds = gate.get_verification_timeout();

            let html = match state {
                Some(s) => {
                    if let Some(ref captcha_data) = s.captcha_data {
                        render_second_captcha_page(
                            session_id,
                            &s.session_id,
                            captcha_data,
                            captchas_solved,
                            timeout_seconds,
                        )
                    } else {
                        return styled_error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "System Error",
                            "Failed to generate second captcha. Please start over.",
                        );
                    }
                }
                None => {
                    return styled_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Session Error",
                        "Session state not found. Please start over.",
                    );
                }
            };

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .body(Full::new(Bytes::from(html)))
                .expect("valid response")
        }
        Err(_e) => {
            // Get failed attempts and calculate delay
            let failed_attempts = gate.get_failed_attempts(session_id);
            let delay_seconds = gate.calculate_delay(failed_attempts);

            // Use template engine for verification failed page
            let engine = TemplateEngine::new();
            let branding = gate.branding().clone();
            let mut extra_vars = std::collections::HashMap::new();
            extra_vars.insert("ATTEMPTS".to_string(), failed_attempts.to_string());
            extra_vars.insert("DELAY_SECONDS".to_string(), delay_seconds.to_string());
            extra_vars.insert(
                "DELAY_DISPLAY".to_string(),
                if delay_seconds > 0 {
                    "block".to_string()
                } else {
                    "none".to_string()
                },
            );
            let html = engine.render_with_branding(
                TemplateType::VerificationFailed,
                &branding,
                Some(&extra_vars),
            );

            Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Content-Type", "text/html")
                .body(Full::new(Bytes::from(html)))
                .expect("valid response")
        }
    }
}

/// Handle admin request to update captcha configuration
async fn handle_update_captcha_config(
    req: Request<Incoming>,
    gate: Arc<Gate>,
) -> Response<BoxBody> {
    // Read request body
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body"),
    };

    // Parse JSON body into CaptchaConfig
    let config: crate::captcha_types::CaptchaConfig = match serde_json::from_slice(&body_bytes) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    // Update the gate's captcha config
    gate.update_captcha_config(config);

    tracing::info!("Gate captcha config updated via admin API");

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
        .expect("valid response")
}

fn not_found() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .expect("valid response")
}

fn error_response(status: StatusCode, msg: &str) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(msg.to_string())))
        .expect("valid response")
}

/// Styled error response using template engine for consistent citadel/gold theme
/// TODO: Pass gate branding to error pages for full brand consistency
fn styled_error_response(status: StatusCode, title: &str, message: &str) -> Response<BoxBody> {
    let engine = TemplateEngine::new();
    // Use default branding for error pages (gate context not available here)
    let branding = BrandingVars::default();
    let mut extra_vars = std::collections::HashMap::new();
    extra_vars.insert("ERROR_TITLE".to_string(), title.to_string());
    extra_vars.insert("ERROR_CODE".to_string(), status.as_u16().to_string());
    extra_vars.insert("ERROR_MESSAGE".to_string(), message.to_string());
    let html = engine.render_with_branding(TemplateType::Error, &branding, Some(&extra_vars));

    Response::builder()
        .status(status)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}
// Handle admin request to update branding configuration
async fn handle_update_branding_config(
    req: Request<Incoming>,
    gate: Arc<Gate>,
) -> Response<BoxBody> {
    // Read request body
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body"),
    };

    // Parse JSON body into BrandingVars
    let branding: BrandingVars = match serde_json::from_slice(&body_bytes) {
        Ok(b) => b,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    // Update the gate's branding config
    gate.update_branding(branding);

    tracing::info!("Gate branding config updated via admin API");
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from("Branding updated")))
        .expect("valid response")
}
