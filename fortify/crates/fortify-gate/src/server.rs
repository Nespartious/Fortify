use crate::Gate;
use crate::GateError;
use crate::captcha_types::CaptchaData;
use crate::captcha_html::{render_captcha_page_with_timer_and_reason, render_captcha_page_with_timer};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
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

        let make_svc = make_service_fn(move |_conn| {
            let gate = Arc::clone(&gate);
            let static_dir = static_dir.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    handle_request(req, Arc::clone(&gate), static_dir.clone())
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        tracing::info!("Gate HTTP server listening on {}", addr);

        server.await?;
        Ok(())
    }
}

async fn handle_request(
    req: Request<Body>,
    gate: Arc<Gate>,
    _static_dir: String,
) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path();
    let method = req.method();
    
    // Extract cookies for session tracking
    let cookies = req.headers()
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    // Check if user was demoted (has the demoted cookie from node redirect)
    let was_demoted = cookies.contains("fortify_demoted=1");
    
    // Extract existing session ID for preservation (even if demoted)
    // This is stored when user is demoted so we can track them across re-verifications
    let existing_session_id = cookies.split(';')
        .map(|c| c.trim())
        .find(|c| c.starts_with("fortify_original_session="))
        .and_then(|c| c.strip_prefix("fortify_original_session="))
        .map(String::from);
    
    // Also check for pending session from HTTP proxy (new visitors assigned session before reaching Gate)
    let pending_session_id = cookies.split(';')
        .map(|c| c.trim())
        .find(|c| c.starts_with("fortify_pending_session="))
        .and_then(|c| c.strip_prefix("fortify_pending_session="))
        .map(String::from);
    
    // Use existing session > pending session > generate new
    let session_id_for_captcha = existing_session_id.clone()
        .or(pending_session_id);
    
    // Cookie compliance check - filter out bots that don't handle cookies
    let has_cookie_test = cookies.contains("fortify_test=1");
    let query = req.uri().query().unwrap_or("");
    let is_cookie_check = query.contains("check=1");

    let response = match (method, path) {
        // Landing page: different content for new vs demoted users
        (&Method::GET, "/Fortify") => {
            // Cookie compliance check (skip for demoted users who already passed)
            if !was_demoted && !has_cookie_test && !is_cookie_check {
                // First visit - set test cookie and redirect to check
                return Ok(Response::builder()
                    .status(StatusCode::FOUND)
                    .header("Location", "/Fortify?check=1")
                    .header("Set-Cookie", "fortify_test=1; Path=/; Max-Age=60; HttpOnly; SameSite=Lax")
                    .body(Body::empty())
                    .unwrap());
            }
            
            if is_cookie_check && !has_cookie_test {
                // Came back without cookie - likely a bot
                return Ok(serve_cookie_blocked_page());
            }
            
            if was_demoted {
                // Demoted user: show "hold position" friendly message, clear demoted cookie
                serve_demoted_page(gate)
            } else {
                // New user: show the landing page (gate.html)
                serve_landing_page(gate)
            }
        },
        
        // The captcha challenge page - accessible by all
        // Pass the session ID from cookie (pending or existing) to preserve identity
        (&Method::GET, "/Fortify/Portcullis") => {
            // Parse query parameters for reason
            let reason = query.split('&')
                .find(|p| p.starts_with("reason="))
                .and_then(|p| p.strip_prefix("reason="));
            serve_captcha_challenge(gate, session_id_for_captcha, reason)
        },
        
        // Dynamic routes
        (&Method::POST, "/gate/verify") => verify_submission(req, gate).await,
        (&Method::POST, "/gate/upgrade-token") => handle_token_upgrade(req, gate).await,
        (&Method::GET, p) if p.starts_with("/gate/captcha/") => serve_captcha_image(p, gate).await,
        
        // Admin API: update captcha configuration
        (&Method::POST, "/gate/admin/captcha-config") => handle_update_captcha_config(req, gate).await,
        
        // Catch-all: redirect everyone to /Fortify landing
        // Also clear any stale session cookie to prevent redirect loops
        _ => {
            Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", "/Fortify")
                .header("Set-Cookie", "fortify_session=; Path=/; Max-Age=0; HttpOnly")
                .body(Body::empty())
                .unwrap()
        }
    };

    Ok(response)
}

/// Block page for clients that don't support cookies (likely bots)
fn serve_cookie_blocked_page() -> Response<Body> {
    let html = r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FORTIFY /// ACCESS DENIED</title>
    <style>
        :root {
            --bg-deep: #0a0012;
            --neon-pink: #ff2a6d;
            --neon-red: #ff3366;
            --neon-cyan: #05d9e8;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            background: linear-gradient(180deg, #1a0a2e 0%, #0a0012 50%, #05020a 100%);
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-red);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
        }
        .container {
            background: rgba(18, 3, 24, 0.95);
            border: 2px solid var(--neon-red);
            box-shadow: 0 0 40px rgba(255, 51, 102, 0.3);
            padding: 40px 45px;
            max-width: 500px;
            width: 100%;
            text-align: center;
        }
        .icon { font-size: 4rem; margin-bottom: 20px; }
        h1 {
            font-size: 1.5rem;
            color: var(--neon-red);
            text-transform: uppercase;
            letter-spacing: 4px;
            margin-bottom: 20px;
            text-shadow: 0 0 10px currentColor;
        }
        .message {
            color: #888;
            font-size: 0.85rem;
            line-height: 1.6;
            margin-bottom: 25px;
        }
        .code {
            font-family: 'Courier New', monospace;
            background: rgba(255, 51, 102, 0.1);
            padding: 12px 18px;
            border: 1px solid var(--neon-red);
            color: var(--neon-red);
            font-size: 0.75rem;
            margin-bottom: 25px;
        }
        .requirement {
            font-size: 0.75rem;
            color: var(--neon-cyan);
            text-transform: uppercase;
            letter-spacing: 2px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">⛔</div>
        <h1>Access Denied</h1>
        <p class="message">Your client does not support the required security mechanisms to access this service.</p>
        <div class="code">ERROR: COOKIE_SUPPORT_REQUIRED</div>
        <p class="requirement">Enable cookies and try again</p>
    </div>
</body>
</html>"###;

    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("Content-Type", "text/html")
        .body(Body::from(html))
        .unwrap()
}

fn serve_landing_page(_gate: Arc<Gate>) -> Response<Body> {
    // Landing page for NEW users (first-time visitors)
    // Castle/Fortification themed - Retrosynth style
    // NO JAVASCRIPT ALLOWED
    let html = r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FORTIFY /// SHIELD ACTIVE</title>
    <style>
        :root {
            --bg-color: #0b0014;
            --neon-pink: #d500f9;
            --neon-cyan: #00e5ff;
            --grid-color: rgba(213, 0, 249, 0.1);
        }
        * { box-sizing: border-box; }
        body {
            background-color: var(--bg-color);
            background-image: 
                linear-gradient(var(--grid-color) 1px, transparent 1px),
                linear-gradient(90deg, var(--grid-color) 1px, transparent 1px);
            background-size: 50px 50px;
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-cyan);
            min-height: 100vh;
            margin: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
            text-align: center;
        }
        .container { max-width: 520px; width: 100%; }
        .icon { font-size: 4rem; margin-bottom: 15px; }
        h1 {
            font-size: 2rem;
            margin: 0 0 8px 0;
            color: var(--neon-pink);
            text-transform: uppercase;
            letter-spacing: 4px;
        }
        .tagline {
            font-size: 0.8rem;
            color: #888;
            letter-spacing: 2px;
            margin-bottom: 30px;
        }
        .info-box {
            background: rgba(213, 0, 249, 0.08);
            border: 1px solid var(--neon-pink);
            padding: 20px;
            margin-bottom: 25px;
            text-align: left;
        }
        .info-box p {
            margin: 0 0 12px 0;
            color: #aaa;
            font-size: 0.9rem;
            line-height: 1.5;
        }
        .info-box p:last-child { margin-bottom: 0; }
        @keyframes fadeInDelay {
            0% { opacity: 0; pointer-events: none; }
            60% { opacity: 0; pointer-events: none; }
            100% { opacity: 1; pointer-events: auto; }
        }
        .delay-btn {
            animation: fadeInDelay 1.5s forwards;
            opacity: 0;
        }
        .proceed-btn {
            background: transparent;
            color: var(--neon-cyan);
            border: 2px solid var(--neon-cyan);
            padding: 16px 40px;
            font-family: inherit;
            font-weight: 700;
            font-size: 1rem;
            text-transform: uppercase;
            letter-spacing: 2px;
            text-decoration: none;
            display: inline-block;
            transition: all 0.2s;
        }
        .proceed-btn:hover {
            background: var(--neon-cyan);
            color: #000;
            box-shadow: 0 0 20px rgba(0, 229, 255, 0.4);
        }
        .status-bar {
            margin-top: 30px;
            font-size: 0.65rem;
            color: #444;
            display: flex;
            justify-content: center;
            gap: 20px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">🏰</div>
        <h1>FORTIFY SHIELD</h1>
        <div class="tagline">▸ GATEWAY PROTECTION ACTIVE ◂</div>
        <div class="info-box">
            <p>This site is protected by <strong style="color: var(--neon-pink);">Fortify</strong> - a decentralized verification system designed to defend against automated threats.</p>
            <p>Complete a quick verification to proceed. No accounts or tracking required.</p>
        </div>
        <div class="delay-btn">
            <a href="/Fortify/Portcullis" class="proceed-btn">INITIALIZE HANDSHAKE</a>
        </div>
        <div class="status-bar">
            <span>ONION-V3</span>
            <span>NO-JS</span>
            <span>ZERO-TRACK</span>
        </div>
    </div>
</body>
</html>"###;
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        // Clear any stale fortify_session cookie to break redirect loops after service restart
        .header("Set-Cookie", "fortify_session=; Path=/; Max-Age=0; HttpOnly")
        .body(Body::from(html))
        .unwrap()
}

fn serve_demoted_page(gate: Arc<Gate>) -> Response<Body> {
    // Demoted users get an inline captcha on the same page
    // This reduces the friction vs requiring another click
    // Use HARD difficulty for demoted users as they've exhibited suspicious behavior
    
    // Create a verification session with harder difficulty
    let session_id = uuid::Uuid::new_v4().to_string();
    
    // Get captcha type from config - use threat captcha type for demoted users
    let config = gate.get_captcha_config();
    let captcha_type = config.get_captcha_type(true); // threat mode
    let timeout_seconds = gate.get_verification_timeout();
    
    // Force create with hard difficulty for demoted users using configured captcha type
    let captcha_state = match gate.create_verification_with_type(
        session_id.clone(),
        captcha_type,
        crate::CaptchaDifficulty::Hard,
        true, // threat mode - requires 2 captchas
    ) {
        Ok(s) => s,
        Err(_) => {
            // Fallback: use default captcha type but still set threat mode for 2 captchas
            match gate.create_verification_with_type(
                session_id.clone(), 
                crate::CaptchaType::BmpText,  // Default type
                crate::CaptchaDifficulty::Hard,
                true,  // CRITICAL: still threat mode for 2 captchas
            ) {
                Ok(s) => s,
                Err(e) => return error_response(StatusCode::SERVICE_UNAVAILABLE, &format!("Gate busy: {}", e)),
            }
        }
    };

    // If we have the new captcha_data, render the modern captcha page with threat styling
    if let Some(ref captcha_data) = captcha_state.captcha_data {
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html")
            .header("Set-Cookie", "fortify_demoted=; Path=/; Max-Age=0; HttpOnly")
            .body(Body::from(render_captcha_page_with_timer(
                &captcha_state.session_id, 
                &captcha_state.session_id, 
                captcha_data, 
                true, // threat styling
                timeout_seconds
            )))
            .unwrap();
    }

    let captcha_id = &captcha_state.session_id;

    // Amber warning theme with inline captcha - harder difficulty for demoted users
    let html = format!(r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FORTIFY /// SESSION REVIEW</title>
    <style>
        :root {{
            --bg-deep: #0a0012;
            --panel-bg: #120318;
            --neon-pink: #ff2a6d;
            --neon-cyan: #05d9e8;
            --neon-amber: #ffab00;
            --neon-orange: #ff6b35;
            --grid-color: rgba(255, 107, 53, 0.08);
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: linear-gradient(180deg, #1a0a2e 0%, #0a0012 50%, #05020a 100%);
            background-attachment: fixed;
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-cyan);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
            position: relative;
        }}
        body::before {{
            content: '';
            position: fixed;
            bottom: 0;
            left: -50%;
            right: -50%;
            height: 40%;
            background: 
                linear-gradient(to top, rgba(255, 107, 53, 0.1) 0%, transparent 100%),
                repeating-linear-gradient(90deg, transparent, transparent 60px, rgba(255, 107, 53, 0.15) 60px, rgba(255, 107, 53, 0.15) 61px);
            transform: perspective(500px) rotateX(60deg);
            transform-origin: bottom;
            pointer-events: none;
        }}
        .container {{
            background: rgba(18, 3, 24, 0.95);
            border: 2px solid var(--neon-orange);
            box-shadow: 0 0 40px rgba(255, 107, 53, 0.3), inset 0 0 60px rgba(0, 0, 0, 0.5);
            padding: 35px 40px;
            max-width: 460px;
            width: 100%;
            text-align: center;
            position: relative;
            z-index: 1;
        }}
        .warning-badge {{
            display: inline-block;
            background: linear-gradient(135deg, var(--neon-orange), var(--neon-amber));
            color: #000;
            padding: 6px 20px;
            font-size: 0.65rem;
            font-weight: bold;
            letter-spacing: 2px;
            text-transform: uppercase;
            margin-bottom: 20px;
        }}
        .icon {{ font-size: 3rem; margin-bottom: 12px; }}
        h1 {{
            font-size: 1.5rem;
            color: var(--neon-orange);
            text-transform: uppercase;
            letter-spacing: 3px;
            margin-bottom: 6px;
        }}
        .tagline {{
            font-size: 0.7rem;
            color: var(--neon-pink);
            letter-spacing: 2px;
            margin-bottom: 20px;
        }}
        .message-box {{
            background: rgba(255, 107, 53, 0.08);
            border-left: 3px solid var(--neon-orange);
            padding: 15px;
            margin-bottom: 25px;
            text-align: left;
        }}
        .message-box p {{
            color: #999;
            font-size: 0.8rem;
            line-height: 1.5;
            margin: 0;
        }}
        .message-box strong {{ color: var(--neon-cyan); }}
        
        /* Captcha Section */
        .captcha-section {{
            background: rgba(0, 0, 0, 0.3);
            border: 1px solid var(--neon-cyan);
            padding: 20px;
            margin-bottom: 20px;
        }}
        .captcha-label {{
            font-size: 0.7rem;
            color: var(--neon-pink);
            letter-spacing: 2px;
            margin-bottom: 12px;
            text-transform: uppercase;
        }}
        .captcha-display {{
            background: #000;
            border: 1px solid var(--neon-cyan);
            padding: 10px;
            margin-bottom: 15px;
            min-height: 100px;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .captcha-display img {{ max-width: 100%; height: auto; }}
        input[type="text"] {{
            width: 100%;
            background: #000;
            border: 1px solid var(--neon-cyan);
            color: var(--neon-cyan);
            padding: 12px 14px;
            font-family: inherit;
            font-size: 1rem;
            letter-spacing: 3px;
            text-align: center;
            text-transform: uppercase;
            margin-bottom: 15px;
        }}
        input[type="text"]:focus {{
            outline: none;
            border-color: var(--neon-orange);
            box-shadow: 0 0 10px rgba(255, 107, 53, 0.3);
        }}
        input[type="text"]::placeholder {{ color: #444; letter-spacing: 1px; }}
        button {{
            width: 100%;
            background: transparent;
            color: var(--neon-orange);
            border: 2px solid var(--neon-orange);
            padding: 14px;
            font-family: inherit;
            font-weight: 700;
            font-size: 0.9rem;
            text-transform: uppercase;
            letter-spacing: 2px;
            cursor: pointer;
            transition: all 0.2s;
        }}
        button:hover {{
            background: var(--neon-orange);
            color: #000;
            box-shadow: 0 0 20px rgba(255, 107, 53, 0.5);
        }}
        .footer-note {{
            margin-top: 20px;
            font-size: 0.6rem;
            color: #444;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="warning-badge">⚠ SESSION REVIEW</div>
        <div class="icon">🛡️</div>
        <h1>HOLD POSITION</h1>
        <div class="tagline">▸ ENHANCED VERIFICATION REQUIRED ◂</div>
        
        <div class="message-box">
            <p>Elevated activity detected from your session. Complete verification to <strong>resume access</strong>. This is a routine security measure.</p>
        </div>
        
        <form method="POST" action="/gate/verify" class="captcha-section">
            <input type="hidden" name="session_id" value="{captcha_id}">
            <input type="hidden" name="pow_nonce" value="0">
            <div class="captcha-label">Security Challenge</div>
            <div class="captcha-display">
                <img src="/gate/captcha/{captcha_id}" alt="Verification Code">
            </div>
            <input type="text" name="captcha" placeholder="Enter code above" required autofocus autocomplete="off">
            <button type="submit">VERIFY &amp; RESUME</button>
        </form>
        
        <div class="footer-note">
            No scripts • Server-verified • Session preserved
        </div>
    </div>
</body>
</html>"###, captcha_id = captcha_id);
    
    // Build response - clear demoted cookie after showing the page
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .header("Set-Cookie", "fortify_demoted=; Path=/; Max-Age=0; HttpOnly")
        .body(Body::from(html))
        .unwrap();
    response.headers_mut().append("Set-Cookie", "fortify_session=; Path=/; Max-Age=0; HttpOnly".parse().unwrap());
    response
}

/// Render a page for the second captcha challenge (for demoted/threat sessions)
/// Uses the existing captcha page renderer and adds a progress indicator
fn render_second_captcha_page(session_id: &str, captcha_id: &str, captcha_data: &CaptchaData, captchas_solved: u8, timeout_seconds: u64) -> String {
    // Use the existing captcha page renderer (with threat mode styling)
    let base_page = render_captcha_page_with_timer(session_id, captcha_id, captcha_data, true, timeout_seconds);
    
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

fn serve_captcha_challenge(gate: Arc<Gate>, existing_session_id: Option<String>, reason: Option<&str>) -> Response<Body> {
    // Preserve existing session ID if available (demoted user re-verifying)
    // This keeps the same session ID so we can continue tracking them
    let session_id = existing_session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    // Check if this is an existing threat session - if so, don't overwrite it!
    // This prevents demoted users who navigate to /Fortify/Portcullis from losing their threat status
    if let Some(existing_state) = gate.get_verification_state(&session_id) {
        if existing_state.is_threat {
            // This is an existing threat session - return the existing captcha page
            let timeout_seconds = gate.get_verification_timeout();
            if let Some(ref captcha_data) = existing_state.captcha_data {
                let html = render_captcha_page_with_timer_and_reason(&session_id, &session_id, captcha_data, true, timeout_seconds, reason);
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/html")
                    .body(Body::from(html))
                    .unwrap();
            }
        }
    }
    
    // Get captcha type from configuration (supports random cycling)
    let config = gate.get_captcha_config();
    let captcha_type = config.get_captcha_type(false); // not threat mode
    let timeout_seconds = gate.get_verification_timeout();
    
    let state = match gate.create_verification_with_type(
        session_id.clone(),
        captcha_type,
        crate::CaptchaDifficulty::Medium,
        false, // not threat mode
    ) {
        Ok(s) => s,
        Err(e) => return error_response(StatusCode::SERVICE_UNAVAILABLE, &format!("Gate busy: {}", e)),
    };

    // Render the appropriate captcha page based on type
    let html = if let Some(ref captcha_data) = state.captcha_data {
        render_captcha_page_with_timer_and_reason(&state.session_id, &state.session_id, captcha_data, false, timeout_seconds, reason)
    } else {
        // Fallback to legacy BMP text captcha page if no captcha_data
        let captcha_id = &state.session_id;
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>
        :root {{
            --bg-color: #0d0211;
            --panel-bg: #150520;
            --neon-pink: #d500f9;
            --neon-cyan: #00e5ff;
            --neon-green: #00e676;
            --grid-color: rgba(213, 0, 249, 0.15);
        }}
        body {{
            background-color: var(--bg-color);
            background-image: 
                linear-gradient(var(--grid-color) 1px, transparent 1px),
                linear-gradient(90deg, var(--grid-color) 1px, transparent 1px);
            background-size: 50px 50px;
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-cyan);
            height: 100vh;
            margin: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            overflow: hidden;
        }}
        .panel {{
            background: rgba(21, 5, 32, 0.95);
            border: 2px solid var(--neon-cyan);
            box-shadow: 0 0 20px rgba(0, 229, 255, 0.3), inset 0 0 30px rgba(0,0,0,0.8);
            padding: 3rem 2rem;
            width: 100%;
            max-width: 480px;
            position: relative;
            box-sizing: border-box;
            border-radius: 4px;
        }}
        .scanline {{
            width: 100%;
            height: 100px;
            z-index: 10;
            background: linear-gradient(0deg, rgba(0,0,0,0) 0%, rgba(255, 255, 255, 0.04) 50%, rgba(0,0,0,0) 100%);
            opacity: 0.1;
            position: absolute;
            bottom: 100%;
            animation: scanline 10s linear infinite;
            pointer-events: none;
        }}
        @keyframes scanline {{
            0% {{ bottom: 100%; }}
            100% {{ bottom: -100px; }}
        }}
        h1 {{
            text-align: center;
            margin: 0 0 10px 0;
            color: var(--neon-pink);
            text-shadow: 2px 2px 0px rgba(255,0,255,0.4);
            font-size: 2.5rem;
            letter-spacing: 6px;
            text-transform: uppercase;
            font-weight: 900;
        }}
        .subtitle {{
            text-align: center;
            color: #fff;
            margin-bottom: 30px;
            font-size: 0.8rem;
            letter-spacing: 3px;
            opacity: 0.7;
            border-bottom: 1px solid var(--neon-pink);
            padding-bottom: 10px;
            display: inline-block;
            width: 100%;
        }}
        .captcha-container {{
            background: #000;
            border: 1px solid var(--neon-cyan);
            padding: 15px;
            margin-bottom: 25px;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 130px;
            box-shadow: inset 0 0 20px rgba(0, 229, 255, 0.1);
            position: relative;
        }}
        .captcha-container::after {{
            content: "VISUAL CHALLENGE";
            position: absolute;
            bottom: 5px;
            right: 5px;
            font-size: 0.6em;
            color: #555;
        }}
        .captcha-container img {{
            display: block;
            border: 1px solid #222;
        }}
        input[type="text"] {{
            width: 100%;
            box-sizing: border-box;
            background: rgba(0, 0, 0, 0.6);
            border: 1px solid var(--neon-cyan);
            border-left: 5px solid var(--neon-cyan);
            color: var(--neon-green);
            padding: 15px;
            font-family: inherit;
            font-size: 1.4rem;
            text-align: center;
            outline: none;
            margin-bottom: 25px;
            text-transform: uppercase;
            letter-spacing: 2px;
            transition: all 0.3s ease;
        }}
        input[type="text"]:focus {{
            box-shadow: 0 0 15px rgba(0, 229, 255, 0.4);
            background: rgba(0, 229, 255, 0.1);
        }}
        button {{
            width: 100%;
            box-sizing: border-box;
            background: var(--neon-cyan);
            border: none;
            color: #000;
            padding: 18px;
            font-family: inherit;
            font-size: 1.2rem;
            font-weight: 900;
            cursor: pointer;
            text-transform: uppercase;
            letter-spacing: 4px;
            transition: all 0.2s;
            clip-path: polygon(10px 0, 100% 0, 100% calc(100% - 10px), calc(100% - 10px) 100%, 0 100%, 0 10px);
        }}
        button:hover {{
            background: var(--neon-pink);
            color: #fff;
            text-shadow: 0 0 5px rgba(0,0,0,0.5);
            box-shadow: 0 0 30px var(--neon-pink);
        }}
        .footer-status {{
            margin-top: 20px;
            display: flex;
            justify-content: space-between;
            font-size: 0.7rem;
            color: #555;
            text-transform: uppercase;
        }}
    </style>
</head>
<body>
    <div class="panel">
        <div class="scanline"></div>
        <h1>FORTIFY</h1>
        <div class="subtitle">SECURE GATEWAY ACCESS</div>
        
        <form method="POST" action="/gate/verify">
            <div class="captcha-container">
                <img src="/gate/captcha/{}" alt="Security Challenge">
            </div>
            
            <input type="text" name="captcha" placeholder="ENTER CODE" required autocomplete="off" autofocus>
            
            <input type="hidden" name="session_id" value="{}">
            
            <button type="submit">AUTHENTICATE</button>
        </form>
        
        <div class="footer-status">
            <span>ENCRYPTION: ONION-V3</span>
            <span>NO-JS: ACTIVE</span>
        </div>
    </div>
</body>
</html>"#, captcha_id, session_id)
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(html))
        .unwrap()
}

/// Handle verification token upgrade to session token
async fn handle_token_upgrade(
    mut req: Request<Body>,
    gate: Arc<Gate>,
) -> Response<Body> {
    // Parse JSON body
    let body_bytes = match hyper::body::to_bytes(req.body_mut()).await {
        Ok(b) => b,
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
    let current_ua = req.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    if !verification_token.validate_user_agent(current_ua) {
        tracing::warn!("User-Agent mismatch for token {}", verification_token.user_id);
        return error_response(StatusCode::UNAUTHORIZED, "User-Agent mismatch");
    }

    // Check if token already used (atomic check-and-mark)
    let mut cache = crate::VERIFICATION_TOKEN_CACHE.lock().unwrap();
    match cache.get_mut(&verification_token.user_id) {
        Some(cached_token) => {
            if cached_token.uses_remaining == 0 {
                tracing::warn!("Verification token already used: {}", verification_token.user_id);
                return error_response(StatusCode::UNAUTHORIZED, "Token already used");
            }

            // Mark token as used
            cached_token.mark_used();
            tracing::info!("Upgraded verification token {} to session", verification_token.user_id);
        }
        None => {
            tracing::warn!("Verification token not found in cache: {}", verification_token.user_id);
            return error_response(StatusCode::UNAUTHORIZED, "Token not found");
        }
    }
    drop(cache); // Release lock

    // Create session token (long-lived, Verified tier) with User-Agent binding
    let session_token = gate.create_session_token(
        &verification_token.user_id, 
        fortify_core::TrustTier::Verified,
        current_ua
    );

    tracing::info!("Created session token for user {}, tier: Verified", verification_token.user_id);

    // Return session token as JSON
    let response_json = serde_json::json!({
        "session_token": session_token,
        "tier": "Verified",
        "message": "Token upgraded successfully"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(response_json.to_string()))
        .unwrap()
}

// Renamed for clarity in tool usage, originally serve_captcha in code
async fn serve_captcha_image(path: &str, gate: Arc<Gate>) -> Response<Body> {
    serve_captcha(path, gate).await
}

// Keep original serve_captcha for valid signature
async fn serve_captcha(path: &str, gate: Arc<Gate>) -> Response<Body> {
    let id = match path.strip_prefix("/gate/captcha/") {
        Some(id) => id,
        None => return not_found(),
    };

    // First try to get image data from the new captcha_data field
    if let Some(state) = gate.get_verification_state(id) {
        if let Some(ref captcha_data) = state.captcha_data {
            if let CaptchaData::BmpText { image_data, .. } = captcha_data {
                if !image_data.is_empty() {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "image/bmp")
                        .header("Cache-Control", "no-store, no-cache, must-revalidate")
                        .body(Body::from(image_data.clone()))
                        .unwrap();
                }
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
                    .body(Body::from(challenge.image_data.clone()))
                    .unwrap()
            } else {
                 not_found()
            }
        },
        None => not_found(),
    }
}

async fn verify_submission(
    mut req: Request<Body>,
    gate: Arc<Gate>,
) -> Response<Body> {
    let body_bytes = match hyper::body::to_bytes(req.body_mut()).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body"),
    };

    let params: std::collections::HashMap<String, String> = form_urlencoded::parse(&body_bytes)
        .into_owned()
        .collect();

    let session_id = match params.get("session_id") {
        Some(s) => s,
        None => return styled_error_response(StatusCode::BAD_REQUEST, "Missing session_id", "Session ID not found in form submission"),
    };

    // Check for captcha answer - new captchas use 'selection' for button clicks, 
    // while text-based captchas use 'captcha'
    let captcha = match params.get("captcha").or_else(|| params.get("selection")) {
        Some(c) => c,
        None => return styled_error_response(StatusCode::BAD_REQUEST, "Missing Response", "No captcha answer was submitted. Please try again."),
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
        session_id, is_threat, captchas_remaining
    );

    // Verify
    match gate.verify_submission(session_id, captcha, pow_nonce) {
        Ok(_token) => {
            // Success - Issue verification token instead of session token
            // Get User-Agent for binding
            let user_agent = req.headers()
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            
            // Create verification token (60s TTL, single-use)
            let verification_token = crate::VerificationToken::new(user_agent);
            let token_string = verification_token.encode();
            
            tracing::info!(
                "Issued verification token {} for User-Agent: {} (session will upgrade on first use)",
                verification_token.user_id, user_agent
            );
            
            // Store token in cache to track usage
            {
                let mut cache = crate::VERIFICATION_TOKEN_CACHE.lock().unwrap();
                cache.insert(verification_token.user_id.clone(), verification_token.clone());
            }
            
            // Success page with verification token cookie
            // Random delay between 7-13 seconds for auto-redirect
            let delay_secs = {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
                7 + (seed % 7) // 7 to 13 seconds
            };
            
            let html = format!(r###"<!DOCTYPE html>>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="refresh" content="{delay};url=/">
    <title>FORTIFY /// VERIFIED</title>
    <style>
        :root {{
            --bg-deep: #0a0012;
            --bg-panel: #120318;
            --neon-pink: #ff2a6d;
            --neon-cyan: #05d9e8;
            --neon-purple: #d300c5;
            --sunset-orange: #ff6b35;
            --sunset-yellow: #f7c80e;
            --grid-color: rgba(213, 0, 197, 0.08);
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: linear-gradient(180deg, #1a0a2e 0%, #0a0012 50%, #05020a 100%);
            background-attachment: fixed;
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-cyan);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
            position: relative;
            overflow: hidden;
        }}
        /* Retro grid floor effect */
        body::before {{
            content: '';
            position: fixed;
            bottom: 0;
            left: -50%;
            right: -50%;
            height: 45%;
            background: 
                linear-gradient(to top, rgba(255, 42, 109, 0.15) 0%, transparent 100%),
                repeating-linear-gradient(
                    90deg,
                    transparent,
                    transparent 60px,
                    rgba(255, 42, 109, 0.2) 60px,
                    rgba(255, 42, 109, 0.2) 61px
                ),
                repeating-linear-gradient(
                    0deg,
                    transparent,
                    transparent 30px,
                    rgba(5, 217, 232, 0.15) 30px,
                    rgba(5, 217, 232, 0.15) 31px
                );
            transform: perspective(500px) rotateX(60deg);
            transform-origin: bottom;
            pointer-events: none;
        }}
        /* Horizon glow */
        body::after {{
            content: '';
            position: fixed;
            bottom: 30%;
            left: 0;
            right: 0;
            height: 4px;
            background: linear-gradient(90deg, transparent, var(--neon-pink), var(--sunset-orange), var(--neon-pink), transparent);
            box-shadow: 0 0 40px var(--neon-pink), 0 0 80px var(--sunset-orange);
            opacity: 0.7;
        }}
        .container {{
            background: rgba(18, 3, 24, 0.9);
            border: 2px solid var(--neon-pink);
            box-shadow: 0 0 40px rgba(255, 42, 109, 0.3), inset 0 0 60px rgba(0, 0, 0, 0.5);
            padding: 40px 50px;
            max-width: 480px;
            width: 100%;
            text-align: center;
            position: relative;
            z-index: 1;
        }}
        .container::before {{
            content: '';
            position: absolute;
            top: -2px;
            left: 20%;
            right: 20%;
            height: 2px;
            background: linear-gradient(90deg, transparent, var(--neon-cyan), transparent);
        }}
        .success-badge {{
            display: inline-block;
            background: linear-gradient(135deg, var(--neon-pink), var(--neon-purple));
            color: #fff;
            padding: 8px 25px;
            font-size: 0.7rem;
            font-weight: bold;
            letter-spacing: 3px;
            text-transform: uppercase;
            margin-bottom: 25px;
            box-shadow: 0 0 20px rgba(255, 42, 109, 0.5);
        }}
        .icon-row {{
            font-size: 3rem;
            margin-bottom: 15px;
            text-shadow: 0 0 30px var(--neon-cyan);
        }}
        h1 {{
            font-size: 1.8rem;
            color: #fff;
            text-transform: uppercase;
            letter-spacing: 5px;
            margin-bottom: 8px;
            text-shadow: 0 0 10px var(--neon-cyan), 2px 2px 0 var(--neon-pink);
        }}
        .tagline {{
            font-size: 0.75rem;
            color: var(--neon-pink);
            letter-spacing: 2px;
            margin-bottom: 30px;
        }}
        .message-box {{
            background: rgba(5, 217, 232, 0.05);
            border-left: 3px solid var(--neon-cyan);
            padding: 20px;
            margin-bottom: 25px;
            text-align: left;
        }}
        .message-box p {{
            color: #aaa;
            font-size: 0.85rem;
            line-height: 1.6;
            margin: 0;
        }}
        .message-box strong {{
            color: var(--neon-cyan);
        }}
        .status-indicators {{
            display: flex;
            justify-content: center;
            gap: 20px;
            margin-bottom: 30px;
            flex-wrap: wrap;
        }}
        .indicator {{
            font-size: 0.7rem;
            padding: 6px 12px;
            border: 1px solid;
            letter-spacing: 1px;
        }}
        .indicator.active {{
            border-color: var(--neon-cyan);
            color: var(--neon-cyan);
            animation: pulse-glow 2s ease-in-out infinite;
        }}
        @keyframes pulse-glow {{
            0%, 100% {{ box-shadow: 0 0 5px currentColor; }}
            50% {{ box-shadow: 0 0 15px currentColor; }}
        }}
        .redirect-notice {{
            color: #555;
            font-size: 0.75rem;
            margin-bottom: 20px;
            letter-spacing: 1px;
        }}
        /* Button appears after 3 seconds using CSS animation */
        .proceed-btn {{
            display: inline-block;
            background: transparent;
            color: var(--sunset-orange);
            border: 2px solid var(--sunset-orange);
            padding: 14px 35px;
            font-family: inherit;
            font-weight: bold;
            font-size: 0.9rem;
            text-transform: uppercase;
            letter-spacing: 2px;
            text-decoration: none;
            transition: all 0.2s ease;
            opacity: 0;
            animation: fadeInButton 0.5s ease-out 3s forwards;
        }}
        @keyframes fadeInButton {{
            to {{ opacity: 1; }}
        }}
        .proceed-btn:hover {{
            background: var(--sunset-orange);
            color: #000;
            box-shadow: 0 0 25px rgba(255, 107, 53, 0.6);
        }}
        .footer-text {{
            margin-top: 25px;
            font-size: 0.6rem;
            color: #333;
            letter-spacing: 1px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="success-badge">★ VERIFIED ★</div>
        <div class="icon-row">🔓</div>
        <h1>Access Granted</h1>
        <div class="tagline">▸ IDENTITY CONFIRMED ◂</div>
        
        <div class="message-box">
            <p>Your verification is complete. You will be automatically transferred to the <strong>secure zone</strong> in a few moments.</p>
        </div>
        
        <div class="status-indicators">
            <span class="indicator active">◉ ENCRYPTED</span>
            <span class="indicator active">◉ VERIFIED</span>
            <span class="indicator active">◉ SECURE</span>
        </div>
        
        <div class="redirect-notice">
            [ Preparing secure connection... ]
        </div>
        
        <a href="/" class="proceed-btn">Continue Now →</a>
        
        <div class="footer-text">
            FORTIFY SECURITY LAYER • TRUST ESTABLISHED
        </div>
    </div>
</body>
</html>"###, delay = delay_secs);

             // Set verification token cookie (60s expiry, single-use)
             // User must use this token on their next request to get a session token
             Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .header("Set-Cookie", format!("fortify_verification={}; Path=/; HttpOnly; Max-Age=60; SameSite=Strict", token_string))
                .header("Set-Cookie", format!("fortify_original_session={}; Path=/; HttpOnly; Max-Age=86400", session_id))
                .body(Body::from(html))
                .unwrap()
        }
        Err(GateError::AdditionalCaptchaRequired) => {
            // First captcha solved - need second captcha for threat session
            // Get the threat captcha type from config and generate new captcha
            let captcha_config = gate.get_captcha_config();
            let second_captcha_type = captcha_config.threat_captcha_type;
            
            // Regenerate captcha with the threat type for second challenge
            if let Err(_) = gate.regenerate_captcha(session_id, second_captcha_type) {
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
                        render_second_captcha_page(session_id, &s.session_id, captcha_data, captchas_solved, timeout_seconds)
                    } else {
                        return styled_error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "System Error",
                            "Failed to generate second captcha. Please start over.",
                        );
                    }
                },
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
                .body(Body::from(html))
                .unwrap()
        }
        Err(_e) => {
            // Get failed attempts and calculate delay
            let failed_attempts = gate.get_failed_attempts(session_id);
            let delay_seconds = gate.calculate_delay(failed_attempts);
            
            // Generate themed error page with retry functionality and progressive delay
            let html = format!(r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FORTIFY /// VERIFICATION FAILED</title>
    <style>
        :root {{
            --bg-deep: #0a0012;
            --neon-pink: #ff2a6d;
            --neon-cyan: #05d9e8;
            --neon-red: #ff3366;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: linear-gradient(180deg, #1a0a2e 0%, #0a0012 50%, #05020a 100%);
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-cyan);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
        }}
        .container {{
            background: rgba(18, 3, 24, 0.95);
            border: 2px solid var(--neon-red);
            box-shadow: 0 0 40px rgba(255, 51, 102, 0.3);
            padding: 35px 40px;
            max-width: 450px;
            width: 100%;
            text-align: center;
        }}
        .icon {{ font-size: 3rem; margin-bottom: 15px; }}
        h1 {{
            font-size: 1.4rem;
            color: var(--neon-red);
            text-transform: uppercase;
            letter-spacing: 3px;
            margin-bottom: 15px;
        }}
        .message {{
            color: #999;
            font-size: 0.85rem;
            line-height: 1.5;
            margin-bottom: 20px;
        }}
        .attempts {{
            font-size: 0.75rem;
            color: var(--neon-pink);
            margin-bottom: 20px;
        }}
        .delay-notice {{
            background: rgba(255, 51, 102, 0.1);
            border: 1px solid var(--neon-red);
            padding: 15px;
            margin-bottom: 20px;
            font-size: 0.8rem;
            color: var(--neon-red);
            display: {delay_display};
        }}
        .retry-btn {{
            display: inline-block;
            background: transparent;
            color: var(--neon-cyan);
            border: 2px solid var(--neon-cyan);
            padding: 14px 35px;
            font-family: inherit;
            font-size: 0.9rem;
            text-transform: uppercase;
            letter-spacing: 2px;
            text-decoration: none;
            transition: all 0.2s;
            opacity: 0;
            animation: fadeIn 0.5s ease-out {delay}s forwards;
        }}
        @keyframes fadeIn {{ to {{ opacity: 1; }} }}
        .retry-btn:hover {{
            background: var(--neon-cyan);
            color: #000;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">✗</div>
        <h1>Verification Failed</h1>
        <p class="message">The code you entered was incorrect. Please try again with a new challenge.</p>
        <p class="attempts">Attempts: {attempts}</p>
        <div class="delay-notice">Please wait before retrying...</div>
        <a href="/Fortify/Portcullis" class="retry-btn">Try Again</a>
    </div>
</body>
</html>"###,
                delay = delay_seconds,
                delay_display = if delay_seconds > 0 { "block" } else { "none" },
                attempts = failed_attempts
            );
            
            Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Content-Type", "text/html")
                .body(Body::from(html))
                .unwrap()
        }
    }
}

/// Handle admin request to update captcha configuration
async fn handle_update_captcha_config(
    mut req: Request<Body>,
    gate: Arc<Gate>,
) -> Response<Body> {
    // Read request body
    let body_bytes = match hyper::body::to_bytes(req.body_mut()).await {
        Ok(b) => b,
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
        .body(Body::from(r#"{"status":"ok"}"#))
        .unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}

fn error_response(status: StatusCode, msg: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(msg.to_string()))
        .unwrap()
}

/// Styled error response matching Fortify theme
fn styled_error_response(status: StatusCode, title: &str, message: &str) -> Response<Body> {
    let html = format!(r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FORTIFY /// ERROR</title>
    <style>
        :root {{
            --bg-deep: #0a0012;
            --panel-bg: #120318;
            --neon-pink: #ff2a6d;
            --neon-cyan: #05d9e8;
            --neon-red: #ff3366;
            --neon-orange: #ff6b35;
            --grid-color: rgba(255, 51, 102, 0.08);
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: linear-gradient(180deg, #1a0a2e 0%, #0a0012 50%, #05020a 100%);
            background-attachment: fixed;
            font-family: 'Courier New', Courier, monospace;
            color: var(--neon-cyan);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
            position: relative;
        }}
        body::before {{
            content: '';
            position: fixed;
            bottom: 0;
            left: -50%;
            right: -50%;
            height: 40%;
            background: 
                linear-gradient(to top, rgba(255, 51, 102, 0.1) 0%, transparent 100%),
                repeating-linear-gradient(90deg, transparent, transparent 60px, rgba(255, 51, 102, 0.15) 60px, rgba(255, 51, 102, 0.15) 61px);
            transform: perspective(500px) rotateX(60deg);
            transform-origin: bottom;
            pointer-events: none;
        }}
        .container {{
            background: rgba(18, 3, 24, 0.95);
            border: 2px solid var(--neon-red);
            box-shadow: 0 0 40px rgba(255, 51, 102, 0.3), inset 0 0 60px rgba(0, 0, 0, 0.5);
            padding: 40px 45px;
            max-width: 500px;
            width: 100%;
            text-align: center;
            position: relative;
        }}
        .container::before {{
            content: '';
            position: absolute;
            top: -2px;
            left: 50%;
            transform: translateX(-50%);
            width: 60%;
            height: 4px;
            background: linear-gradient(90deg, transparent, var(--neon-red), transparent);
        }}
        .icon {{
            font-size: 4rem;
            margin-bottom: 20px;
            animation: pulse 2s ease-in-out infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}
        h1 {{
            font-size: 1.8rem;
            color: var(--neon-red);
            text-transform: uppercase;
            letter-spacing: 4px;
            margin-bottom: 15px;
            text-shadow: 0 0 10px currentColor;
        }}
        .code {{
            font-family: 'Courier New', monospace;
            background: rgba(255, 51, 102, 0.1);
            padding: 10px 15px;
            border: 1px solid var(--neon-red);
            color: var(--neon-red);
            font-size: 0.8rem;
            margin-bottom: 20px;
            letter-spacing: 2px;
        }}
        .message {{
            color: #888;
            font-size: 0.9rem;
            line-height: 1.6;
            margin-bottom: 25px;
        }}
        .retry-btn {{
            display: inline-block;
            background: var(--neon-cyan);
            border: none;
            color: #000;
            padding: 14px 40px;
            font-family: inherit;
            font-size: 1rem;
            font-weight: 900;
            cursor: pointer;
            text-transform: uppercase;
            letter-spacing: 2px;
            text-decoration: none;
            transition: all 0.2s;
        }}
        .retry-btn:hover {{
            background: var(--neon-pink);
            color: #fff;
            box-shadow: 0 0 20px var(--neon-pink);
        }}
        .footer {{
            margin-top: 25px;
            display: flex;
            justify-content: space-between;
            font-size: 0.65rem;
            color: #444;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">⚠</div>
        <h1>{title}</h1>
        <div class="code">ERROR {status_code}</div>
        <p class="message">{message}</p>
        <a href="/Fortify/Portcullis" class="retry-btn">⟳ Try Again</a>
        <div class="footer">
            <span>FORTIFY</span>
            <span>ERROR HANDLER</span>
        </div>
    </div>
</body>
</html>"###, title = title, status_code = status.as_u16(), message = message);

    Response::builder()
        .status(status)
        .header("Content-Type", "text/html")
        .body(Body::from(html))
        .unwrap()
}
