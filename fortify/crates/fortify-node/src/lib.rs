pub mod detection;
pub mod server;

use bytes::Bytes;
use fortify_core::{Session, SessionManager, SessionToken};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Session not found")]
    SessionNotFound,
    #[error("Backend unavailable")]
    BackendUnavailable,
    #[error("Violation detected: {0}")]
    ViolationDetected(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

pub type Result<T> = std::result::Result<T, NodeError>;

/// Node operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeMode {
    /// Healthy mode: fast path, minimal inspection
    Healthy,
    /// Threat mode: additional scrutiny and rate limiting
    Threat,
}

impl NodeMode {
    pub fn should_inspect_deeply(&self) -> bool {
        matches!(self, NodeMode::Threat)
    }

    pub fn max_requests_per_minute(&self) -> u32 {
        match self {
            // Low limit for testing - in production use higher values
            NodeMode::Healthy => 20,
            NodeMode::Threat => 10,
        }
    }

    pub fn request_timeout(&self) -> Duration {
        match self {
            NodeMode::Healthy => Duration::from_secs(30),
            NodeMode::Threat => Duration::from_secs(10),
        }
    }
}

/// Violation type detected during request processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    RateLimitExceeded,
    MalformedRequest,
    SuspiciousPattern,
    InvalidPath,
    OversizedRequest,
}

impl ViolationType {
    pub fn severity(&self) -> u32 {
        match self {
            ViolationType::RateLimitExceeded => 1,
            ViolationType::MalformedRequest => 2,
            ViolationType::SuspiciousPattern => 3,
            ViolationType::InvalidPath => 1,
            ViolationType::OversizedRequest => 2,
        }
    }
}

/// Violation record
#[derive(Debug, Clone)]
pub struct Violation {
    pub violation_type: ViolationType,
    pub session_id: String,
    pub timestamp: u64,
    pub description: String,
}

impl Violation {
    pub fn new(violation_type: ViolationType, session_id: String, description: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            violation_type,
            session_id,
            timestamp,
            description,
        }
    }
}

/// Node metrics
#[derive(Debug, Default, Clone)]
pub struct NodeMetrics {
    pub requests_total: u64,
    pub requests_forwarded: u64,
    pub requests_blocked: u64,
    pub violations_detected: u64,
    pub sessions_demoted: u64,
    pub sessions_promoted: u64,
    pub backend_errors: u64,
    pub average_response_time_ms: f64,
}

impl NodeMetrics {
    pub fn record_request(&mut self, forwarded: bool, response_time_ms: f64) {
        self.requests_total += 1;
        if forwarded {
            self.requests_forwarded += 1;
        } else {
            self.requests_blocked += 1;
        }

        // Update running average
        let n = self.requests_forwarded as f64;
        if n > 0.0 {
            self.average_response_time_ms =
                (self.average_response_time_ms * (n - 1.0) + response_time_ms) / n;
        }
    }

    pub fn record_violation(&mut self) {
        self.violations_detected += 1;
    }

    pub fn record_demotion(&mut self) {
        self.sessions_demoted += 1;
    }

    pub fn record_promotion(&mut self) {
        self.sessions_promoted += 1;
    }

    pub fn record_backend_error(&mut self) {
        self.backend_errors += 1;
    }
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub mode: NodeMode,
    pub bind_addr: SocketAddr,
    pub backend_address: String,
    pub gate_address: String,
    pub socks_proxy: Option<String>,
    pub max_request_size: usize,
    pub violation_threshold: u32,
    pub promotion_threshold: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            mode: NodeMode::Healthy,
            bind_addr: "0.0.0.0:8083".parse().unwrap(),
            backend_address: "http://127.0.0.1:9000".to_string(),
            gate_address: "http://127.0.0.1:8081".to_string(),
            socks_proxy: None,
            max_request_size: 10 * 1024 * 1024, // 10 MB
            violation_threshold: 3,
            promotion_threshold: 50,
        }
    }
}

/// Backend node
pub struct Node {
    config: NodeConfig,
    session_manager: Arc<SessionManager>,
    secret_key: Vec<u8>,
    metrics: Arc<Mutex<NodeMetrics>>,
    violations: Arc<Mutex<HashMap<String, Vec<Violation>>>>,
    request_counts: Arc<Mutex<HashMap<String, u32>>>,
    /// Track which sessions have used their one-time burst exception
    burst_exceptions: Arc<Mutex<HashMap<String, bool>>>,
    /// Optional callback to report demotions to controller
    demotion_callback: Option<Arc<dyn Fn(String, u8) + Send + Sync>>,
}

impl Node {
    pub fn new(
        config: NodeConfig,
        session_manager: Arc<SessionManager>,
        secret_key: Vec<u8>,
    ) -> Self {
        Self {
            config,
            session_manager,
            secret_key,
            metrics: Arc::new(Mutex::new(NodeMetrics::default())),
            violations: Arc::new(Mutex::new(HashMap::new())),
            request_counts: Arc::new(Mutex::new(HashMap::new())),
            burst_exceptions: Arc::new(Mutex::new(HashMap::new())),
            demotion_callback: None,
        }
    }

    /// Set the demotion callback for reporting to controller
    pub fn set_demotion_callback<F>(&mut self, callback: F)
    where
        F: Fn(String, u8) + Send + Sync + 'static,
    {
        self.demotion_callback = Some(Arc::new(callback));
    }

    /// Start the node server
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!(
            "Node starting on {} in {:?} mode",
            self.config.bind_addr,
            self.config.mode
        );

        // Start background cleanup task
        self.start_cleanup_task();

        Ok(())
    }

    /// Process request from proxy
    pub async fn process_request(
        &self,
        session_id: String,
        req: Request<Incoming>,
    ) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
        let start = std::time::Instant::now();

        // Get session - Try stateless token verification first
        let session = if let Ok(token) = SessionToken::decode(&session_id) {
            if token.verify(&self.secret_key).is_ok() && token.is_valid() {
                let mut s = Session::new(token.clone());
                s.token.trust_tier = token.trust_tier;
                s
            } else {
                // Invalid token, try lookup in case it's a raw ID
                match self.session_manager.get_session(&session_id) {
                    Some(s) => s,
                    None => {
                        return Ok(
                            self.error_response(StatusCode::UNAUTHORIZED, "Invalid session token")
                        );
                    }
                }
            }
        } else {
            // Not a token, try raw session ID lookup
            match self.session_manager.get_session(&session_id) {
                Some(s) => s,
                None => {
                    return Ok(self.error_response(StatusCode::UNAUTHORIZED, "Session not found"));
                }
            }
        };

        // Check if session is already demoted (from previous violations)
        {
            let violations = self.violations.lock().unwrap();
            if let Some(viols) = violations.get(&session_id) {
                if viols.len() >= self.config.violation_threshold as usize {
                    tracing::warn!(
                        "Session {} has {} violations, redirecting to Gate",
                        session_id,
                        viols.len()
                    );
                    return Ok(self.redirect_to_gate());
                }
            }
        }

        // Check for new violations
        if let Err(e) = self.check_violations(&session_id, &req, &session) {
            self.metrics.lock().unwrap().record_request(false, 0.0);

            // Check if this violation pushed them over the threshold
            let should_redirect = {
                let violations = self.violations.lock().unwrap();
                violations
                    .get(&session_id)
                    .map(|v| v.len() >= self.config.violation_threshold as usize)
                    .unwrap_or(false)
            };

            if should_redirect {
                tracing::warn!(
                    "Session {} reached violation threshold, redirecting to Gate",
                    session_id
                );
                return Ok(self.redirect_to_gate());
            }

            return Ok(self.error_response(StatusCode::FORBIDDEN, &format!("Violation: {}", e)));
        }

        // Forward to backend
        let response = match self.forward_to_backend(req).await {
            Ok(resp) => {
                let duration = start.elapsed();
                self.metrics
                    .lock()
                    .unwrap()
                    .record_request(true, duration.as_millis() as f64);

                // Check for promotion
                self.check_promotion(&session_id);

                resp
            }
            Err(e) => {
                self.metrics.lock().unwrap().record_backend_error();
                tracing::error!("Backend error: {}", e);
                self.error_response(StatusCode::BAD_GATEWAY, "Backend unavailable")
            }
        };

        Ok(response)
    }

    /// Check for violations
    fn check_violations(
        &self,
        session_id: &str,
        req: &Request<Incoming>,
        _session: &Session,
    ) -> Result<()> {
        // Rate limiting
        if let Err(e) = self.check_rate_limit(session_id) {
            self.record_violation(
                session_id,
                ViolationType::RateLimitExceeded,
                "Rate limit exceeded",
            );
            return Err(e);
        }

        // Request size check
        if let Some(content_length) = req.headers().get("content-length") {
            if let Ok(size) = content_length.to_str().unwrap_or("0").parse::<usize>() {
                if size > self.config.max_request_size {
                    self.record_violation(
                        session_id,
                        ViolationType::OversizedRequest,
                        "Request too large",
                    );
                    return Err(NodeError::ViolationDetected("Request too large".into()));
                }
            }
        }

        // Path validation
        if self.config.mode.should_inspect_deeply() {
            if let Err(e) = self.validate_path(req.uri().path()) {
                self.record_violation(session_id, ViolationType::InvalidPath, "Invalid path");
                return Err(e);
            }
        }

        Ok(())
    }

    /// Check rate limit
    fn check_rate_limit(&self, session_id: &str) -> Result<()> {
        let mut counts = self.request_counts.lock().unwrap();
        let count = counts.entry(session_id.to_string()).or_insert(0);

        *count += 1;

        let limit = self.config.mode.max_requests_per_minute();
        tracing::debug!("Session {} request count: {}/{}", session_id, *count, limit);

        if *count > limit {
            // Check if session qualifies for burst exception
            // Burst exception: Clean sessions (no violations) get ONE burst allowance
            // for loading pages with many assets (e.g., 20+ images)
            let violations = self.violations.lock().unwrap();
            let session_violations = violations.get(session_id).map(|v| v.len()).unwrap_or(0);
            drop(violations); // Release lock before checking burst

            if session_violations == 0 && *count <= 20 {
                // Check if burst exception already used
                let mut burst_exceptions = self.burst_exceptions.lock().unwrap();
                let burst_used = burst_exceptions
                    .entry(session_id.to_string())
                    .or_insert(false);

                if !*burst_used {
                    // Grant one-time burst exception
                    *burst_used = true;
                    tracing::info!(
                        "Session {} granted burst exception: {} requests (clean session, no violations)",
                        session_id, *count
                    );
                    return Ok(()); // Allow burst
                }
            }

            tracing::warn!("Session {} RATE LIMITED ({}/{})", session_id, *count, limit);
            return Err(NodeError::RateLimitExceeded);
        }

        Ok(())
    }

    /// Validate request path
    fn validate_path(&self, path: &str) -> Result<()> {
        // Block suspicious patterns
        let suspicious_patterns = ["../", "..\\", "<script", "' OR ", "DROP TABLE"];

        for pattern in &suspicious_patterns {
            if path.contains(pattern) {
                return Err(NodeError::ViolationDetected(format!(
                    "Suspicious pattern: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }

    /// Record violation
    fn record_violation(&self, session_id: &str, violation_type: ViolationType, description: &str) {
        let violation = Violation::new(
            violation_type,
            session_id.to_string(),
            description.to_string(),
        );

        let violation_count = {
            let mut violations = self.violations.lock().unwrap();
            let session_viols = violations.entry(session_id.to_string()).or_default();
            session_viols.push(violation);
            session_viols.len()
        };

        tracing::warn!(
            "VIOLATION recorded for session {}: {:?} - {} (total: {}/{})",
            session_id,
            violation_type,
            description,
            violation_count,
            self.config.violation_threshold
        );

        self.metrics.lock().unwrap().record_violation();

        // Check if should demote
        self.check_demotion(session_id);
    }

    /// Check if session should be demoted
    fn check_demotion(&self, session_id: &str) {
        let violations = self.violations.lock().unwrap();
        let session_violations = violations.get(session_id);

        if let Some(viols) = session_violations {
            if viols.len() >= self.config.violation_threshold as usize {
                tracing::warn!(
                    "Session {} reached violation threshold, demoting",
                    session_id
                );

                if let Some(mut session) = self.session_manager.get_session(session_id) {
                    match session.demote() {
                        Ok(()) => {
                            self.session_manager.update_session(session);
                            let mut metrics = self.metrics.lock().unwrap();
                            metrics.record_demotion();
                            let demotion_count = metrics.sessions_demoted as u8;
                            drop(metrics);

                            // Report to controller blacklist if callback set
                            if let Some(ref callback) = self.demotion_callback {
                                callback(session_id.to_string(), demotion_count);
                            }
                        }
                        Err(err) => {
                            tracing::error!("Failed to demote session {}: {}", session_id, err);
                        }
                    }
                }
            }
        }
    }

    /// Check if session should be promoted
    fn check_promotion(&self, session_id: &str) {
        let counts = self.request_counts.lock().unwrap();
        let count = counts.get(session_id).copied().unwrap_or(0);

        let violations = self.violations.lock().unwrap();
        let violation_count = violations.get(session_id).map(|v| v.len()).unwrap_or(0);

        // Promote if many successful requests and few violations
        if count >= self.config.promotion_threshold && violation_count == 0 {
            if let Some(mut session) = self.session_manager.get_session(session_id) {
                if session.promote().is_ok() {
                    tracing::info!(
                        "Session {} promoted to {:?}",
                        session_id,
                        session.token.trust_tier
                    );
                    self.session_manager.update_session(session);
                    self.metrics.lock().unwrap().record_promotion();
                }
            }
        }
    }

    /// Forward request to backend
    async fn forward_to_backend(
        &self,
        req: Request<Incoming>,
    ) -> std::result::Result<Response<Full<Bytes>>, String> {
        let path = req
            .uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/");

        let backend_url = format!("{}{}", self.config.backend_address, path);

        // Build reqwest client with SOCKS proxy if backend is .onion
        let client_builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()); // Don't follow redirects - let the browser handle them

        let client = if self.config.backend_address.contains(".onion") {
            if let Some(socks_addr) = &self.config.socks_proxy {
                // Use socks5h:// scheme for remote DNS resolution (required for .onion)
                let socks_url = format!("socks5h://{}", socks_addr);
                tracing::info!(
                    "Using SOCKS proxy {} for .onion backend: {}",
                    socks_url,
                    self.config.backend_address
                );
                client_builder
                    .proxy(
                        reqwest::Proxy::all(&socks_url)
                            .map_err(|e| format!("Invalid proxy: {}", e))?,
                    )
                    .timeout(Duration::from_secs(60)) // Longer timeout for Tor circuit building
                    .connect_timeout(Duration::from_secs(30)) // Connection timeout
                    .build()
                    .map_err(|e| format!("Failed to build client: {}", e))?
            } else {
                tracing::error!("Backend is .onion but no SOCKS proxy configured");
                return Err("Backend is .onion but no SOCKS proxy configured".to_string());
            }
        } else {
            // For non-.onion backends, use standard timeout
            client_builder
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .map_err(|e| format!("Failed to build client: {}", e))?
        };

        // Forward the request
        let method = req.method().clone();
        let headers = req.headers().clone();

        // Read the body from the incoming request
        let body_bytes = req
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to read request body: {}", e))?
            .to_bytes();

        let mut req_builder = client.request(method, &backend_url);
        for (name, value) in headers.iter() {
            req_builder = req_builder.header(name, value);
        }

        // Forward the body if present
        if !body_bytes.is_empty() {
            req_builder = req_builder.body(body_bytes.to_vec());
        }

        match req_builder.send().await {
            Ok(resp) => {
                // Convert reqwest response to hyper response
                let status = StatusCode::from_u16(resp.status().as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                let mut response = Response::builder().status(status);

                // Forward headers, rewriting Location and Set-Cookie as needed
                for (name, value) in resp.headers().iter() {
                    let name_lower = name.as_str().to_lowercase();

                    if name_lower == "location" {
                        // Rewrite Location headers to use relative paths
                        if let Ok(location) = value.to_str() {
                            let rewritten = self.rewrite_location(location);
                            response = response.header(name, rewritten);
                        }
                    } else if name_lower == "set-cookie" {
                        // Remove domain restrictions from cookies so they work with proxy address
                        if let Ok(cookie) = value.to_str() {
                            let rewritten = self.rewrite_cookie(cookie);
                            response = response.header(name, rewritten);
                        }
                    } else {
                        response = response.header(name, value);
                    }
                }

                let body_bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read body: {}", e))?;
                Ok(response
                    .body(Full::new(Bytes::from(body_bytes.to_vec())))
                    .unwrap())
            }
            Err(e) => {
                tracing::warn!("Backend request failed: {}", e);
                Ok(self.serve_backend_fallback())
            }
        }
    }

    /// Rewrite Location header to use relative path instead of absolute backend URL
    fn rewrite_location(&self, location: &str) -> String {
        // ONLY rewrite if Location points to our backend .onion address
        // External redirects (like monitor/go links to other sites) should pass through unchanged
        if location.starts_with(&self.config.backend_address) {
            // Extract the path after the backend address and make it relative
            location[self.config.backend_address.len()..].to_string()
        } else {
            // Keep everything else as-is:
            // - External redirects to other .onion addresses
            // - Relative paths
            // - Any other URLs
            location.to_string()
        }
    }

    /// Rewrite Set-Cookie header to remove domain restrictions
    fn rewrite_cookie(&self, cookie: &str) -> String {
        // Remove Domain= attributes so cookies work with any domain (proxy or direct)
        let parts: Vec<&str> = cookie.split(';').collect();
        let filtered: Vec<&str> = parts
            .into_iter()
            .filter(|part| {
                let trimmed = part.trim().to_lowercase();
                !trimmed.starts_with("domain=")
            })
            .collect();
        filtered.join("; ")
    }

    /// Serve fallback page when backend is unavailable
    fn serve_backend_fallback(&self) -> Response<Full<Bytes>> {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Fortify Protected</title>
    <style>
        body { background: linear-gradient(135deg, #141417 0%, #18181b 50%, #141417 100%); color: #e4e4e7; font-family: 'Courier New', monospace; min-height: 100vh; display: flex; align-items: center; justify-content: center; margin: 0; }
        .container { text-align: center; max-width: 600px; padding: 40px; }
        .shield { font-size: 80px; margin-bottom: 20px; animation: pulse 2s infinite; }
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.7; } }
        h1 { color: #c9a227; font-size: 2em; margin-bottom: 20px; }
        .box { border: 2px solid #c9a227; padding: 30px; background: rgba(201, 162, 39, 0.1); border-radius: 8px; }
        p { line-height: 1.8; color: #a1a1aa; margin: 15px 0; }
        .status { color: #22c55e; font-weight: bold; }
        .note { font-size: 0.9em; color: #71717a; margin-top: 30px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="shield">🛡️</div>
        <h1>FORTIFY PROTECTION ACTIVE</h1>
        <div class="box">
            <p class="status">✓ You have passed verification</p>
            <p class="status">✓ Session authenticated</p>
            <p class="status">✓ Connected to healthy node</p>
            <p>⏳ The protected backend service is temporarily unreachable.</p>
            <p class="note">This may occur during initial deployment while Tor circuits are being established (typically 5-10 minutes).<br/>If this persists, please try refreshing or contact support.</p>
        </div>
    </div>
</body>
</html>"#;

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(html)))
            .unwrap()
    }

    /// Generate error response
    fn error_response(&self, status: StatusCode, message: &str) -> Response<Full<Bytes>> {
        Response::builder()
            .status(status)
            .header("Content-Type", "text/plain")
            .body(Full::new(Bytes::from(message.to_string())))
            .unwrap()
    }

    /// Redirect to Gate with cookie clear (forces re-verification)
    fn redirect_to_gate(&self) -> Response<Full<Bytes>> {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="2;url=/">
    <title>Redirecting...</title>
    <style>
        body { background: #141417; color: #e4e4e7; font-family: 'Courier New', monospace; text-align: center; padding: 50px; }
        .icon { font-size: 64px; margin-bottom: 20px; }
        .box { border: 2px solid #c9a227; padding: 40px; margin: 20px auto; max-width: 600px; background: rgba(201, 162, 39, 0.1); border-radius: 8px; }
        h1 { color: #c9a227; margin-bottom: 20px; }
        p { line-height: 1.6; color: #a1a1aa; }
        a { color: #c9a227; }
    </style>
</head>
<body>
    <div class="box">
        <div class="icon">☕</div>
        <p>Redirecting...</p>
    </div>
</body>
</html>"#;

        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "text/html")
            // Signal to proxy that this session should be demoted
            .header("X-Fortify-Demote", "true")
            // Clear the session cookie to force re-verification at Gate
            .header(
                "Set-Cookie",
                "fortify_session=; Path=/; Max-Age=0; HttpOnly",
            )
            // Set the demoted cookie so Gate knows to show friendly message
            .header(
                "Set-Cookie",
                "fortify_demoted=1; Path=/; Max-Age=300; HttpOnly",
            )
            .body(Full::new(Bytes::from(html)))
            .unwrap()
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> NodeMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Get violation count for session
    pub fn get_violation_count(&self, session_id: &str) -> usize {
        let violations = self.violations.lock().unwrap();
        violations.get(session_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Start background cleanup task
    fn start_cleanup_task(&self) {
        let request_counts = Arc::clone(&self.request_counts);
        let violations = Arc::clone(&self.violations);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Reset request counts every minute
                {
                    let mut counts = request_counts.lock().unwrap();
                    counts.clear();
                }

                // Clean old violations (older than 5 minutes)
                {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let mut viols = violations.lock().unwrap();
                    for violations_list in viols.values_mut() {
                        violations_list.retain(|v| now - v.timestamp < 300);
                    }
                    viols.retain(|_, v| !v.is_empty());
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortify_core::TrustTier;

    fn create_test_node(mode: NodeMode) -> Node {
        let secret = b"test-secret";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
        let config = NodeConfig {
            mode,
            ..Default::default()
        };
        Node::new(config, session_manager, secret.to_vec())
    }

    #[test]
    fn test_node_mode_properties() {
        // Note: Low limits for testing - production uses higher values
        assert_eq!(NodeMode::Healthy.max_requests_per_minute(), 20);
        assert_eq!(NodeMode::Threat.max_requests_per_minute(), 10);

        assert!(!NodeMode::Healthy.should_inspect_deeply());
        assert!(NodeMode::Threat.should_inspect_deeply());
    }

    #[test]
    fn test_violation_severity() {
        assert_eq!(ViolationType::RateLimitExceeded.severity(), 1);
        assert_eq!(ViolationType::SuspiciousPattern.severity(), 3);
    }

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = NodeMetrics::default();

        metrics.record_request(true, 100.0);
        assert_eq!(metrics.requests_total, 1);
        assert_eq!(metrics.requests_forwarded, 1);
        assert_eq!(metrics.average_response_time_ms, 100.0);

        metrics.record_request(false, 0.0);
        assert_eq!(metrics.requests_total, 2);
        assert_eq!(metrics.requests_blocked, 1);
    }

    #[test]
    fn test_violation_recording() {
        let node = create_test_node(NodeMode::Healthy);
        let session_manager = &node.session_manager;

        // Create session
        let mut session = session_manager.create_session("test-123".into());
        session.promote().unwrap();
        session_manager.update_session(session);

        // Record violations
        node.record_violation("test-123", ViolationType::RateLimitExceeded, "Test");
        node.record_violation("test-123", ViolationType::MalformedRequest, "Test");

        assert_eq!(node.get_violation_count("test-123"), 2);
        assert_eq!(node.get_metrics().violations_detected, 2);
    }

    #[test]
    fn test_path_validation() {
        let node = create_test_node(NodeMode::Threat);

        assert!(node.validate_path("/api/users").is_ok());
        assert!(node.validate_path("/api/../etc/passwd").is_err());
        assert!(node.validate_path("/api/<script>alert</script>").is_err());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let node = create_test_node(NodeMode::Threat);
        let limit = NodeMode::Threat.max_requests_per_minute(); // 10

        // Record a violation first to disable burst exception
        node.record_violation("session-1", ViolationType::RateLimitExceeded, "Test");

        // Should allow up to limit (session has violations, no burst exception)
        for i in 0..limit {
            let result = node.check_rate_limit("session-1");
            assert!(result.is_ok(), "Request {} should be allowed", i + 1);
        }

        // Should reject after limit (no burst exception due to violation)
        assert!(node.check_rate_limit("session-1").is_err());
    }

    #[test]
    fn test_demotion_threshold() {
        let node = create_test_node(NodeMode::Healthy);
        let session_manager = &node.session_manager;

        // Create session
        let mut session = session_manager.create_session("test-123".into());
        session.promote().unwrap();
        session_manager.update_session(session);

        // Record violations up to threshold
        for i in 0..3 {
            node.record_violation(
                "test-123",
                ViolationType::RateLimitExceeded,
                &format!("Test {}", i),
            );
        }

        // Check session was demoted
        let session = session_manager.get_session("test-123").unwrap();
        assert_eq!(session.token.trust_tier, TrustTier::Suspicious);
    }
}
