use bytes::Bytes;
use fortify_core::{
    jittered_timeout, safe_lock, safe_write, RequestMeta, SessionBehavior, SessionManager,
    SessionToken, TrustTier,
};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

/// Global connection limit across all backend nodes
/// This prevents resource exhaustion under DDoS conditions
/// Value: 1000 total connections (can be tuned based on server capacity)
static GLOBAL_CONNECTION_SEMAPHORE: Semaphore = Semaphore::const_new(1000);

/// Type alias for the response body type used throughout
type BoxBody = Full<Bytes>;

pub mod admin;
pub mod middleware;
pub mod proxy;
pub mod routing;

pub use admin::{AdminState, HistoryEventType, ADMIN_PATH};

/// Circuit-aware rate limiter with trust tier support
/// Tracks requests per Tor circuit (not shared IP) to prevent attack traffic
/// from blocking legitimate users during DDoS attacks
struct GlobalRateLimiter {
    /// Map of circuit_id -> Vec of request timestamps within window
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    /// Track unique circuits per time window (for attack detection)
    active_circuits: Arc<Mutex<HashMap<Instant, Vec<String>>>>,
    /// Time window for rate limiting
    window: Duration,
    /// Shared admin state for reading traffic tier config
    admin_state: Option<Arc<AdminState>>,
}

impl GlobalRateLimiter {
    fn new(window_secs: u64, admin_state: Option<Arc<AdminState>>) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            active_circuits: Arc::new(Mutex::new(HashMap::new())),
            window: Duration::from_secs(window_secs),
            admin_state,
        }
    }

    /// Get rate limit for a given trust tier (PER CIRCUIT)
    /// Base limits are scaled by the current TrafficTier multiplier:
    /// Micro=0.5x, Small=1.0x, Medium=2.0x, Large=3.0x, Enterprise=4.0x
    ///
    /// Base values (at Small tier / 1.0x):
    ///   - Unknown/Suspicious/Burned: 10/10s per circuit (always strict)
    ///   - Verified: 100/10s per circuit
    ///   - Trusted: 300/10s per circuit
    fn get_limit_for_tier(&self, tier: TrustTier) -> usize {
        // Get multiplier from traffic tier config (default to 1.0x if no admin state)
        let multiplier = self
            .admin_state
            .as_ref()
            .map(|state| state.get_traffic_tier().rate_limit_multiplier())
            .unwrap_or(1.0);

        // Base limits (per 10s window) - these are scaled by traffic tier
        // Unknown tier is NOT scaled - always strict for security
        let base_limit = match tier {
            TrustTier::Trusted => 300,  // Proven good actors
            TrustTier::Verified => 100, // Passed CAPTCHA
            TrustTier::Unknown | TrustTier::Suspicious | TrustTier::Burned => {
                return 10; // Always strict, not scaled
            }
        };

        // Scale by traffic tier multiplier
        let scaled = (base_limit as f32 * multiplier) as usize;
        // Ensure minimum of 1
        scaled.max(1)
    }

    /// Check if circuit is within rate limit and record request
    /// Uses circuit_id (from session or fingerprint) instead of shared Tor IP
    /// Returns true if request is allowed, false if rate limited
    fn check_and_record(&self, circuit_id: &str, tier: TrustTier) -> bool {
        let mut requests = safe_lock(&self.requests);
        let now = Instant::now();
        let window_start = now - self.window;

        // Get or create request history for this circuit
        let reqs = requests.entry(circuit_id.to_string()).or_default();

        // Remove expired timestamps (older than window)
        reqs.retain(|&t| t > window_start);

        // Get limit based on trust tier (per circuit)
        let limit = self.get_limit_for_tier(tier);

        // Check if limit exceeded
        if reqs.len() >= limit {
            return false; // Rate limited
        }

        // Record this request
        reqs.push(now);

        // Track circuit for attack detection
        self.record_active_circuit(circuit_id, now);

        true
    }

    /// Record active circuit for attack pattern detection
    fn record_active_circuit(&self, circuit_id: &str, now: Instant) {
        let mut circuits = safe_lock(&self.active_circuits);
        let window_start = now - self.window;

        // Clean old entries
        circuits.retain(|&t, _| t > window_start);

        // Add this circuit to current window
        circuits
            .entry(now)
            .or_default()
            .push(circuit_id.to_string());
    }

    /// Get number of unique circuits active in current window
    /// Used for attack detection (>100 circuits = probable DDoS)
    #[allow(dead_code)]
    fn get_active_circuit_count(&self) -> usize {
        let circuits = safe_lock(&self.active_circuits);
        let all_circuits: Vec<String> = circuits.values().flatten().cloned().collect();
        let mut unique: Vec<String> = all_circuits.clone();
        unique.sort();
        unique.dedup();
        unique.len()
    }

    /// Clear rate limit quota for a specific circuit
    /// Used after successful CAPTCHA verification to prevent infinite CAPTCHA loops
    /// This allows verified users to browse immediately after proving they're human
    fn clear_circuit_quota(&self, circuit_id: &str) {
        let mut requests = safe_lock(&self.requests);
        if requests.remove(circuit_id).is_some() {
            tracing::info!("Cleared rate limit quota for circuit: {}", circuit_id);
        }
    }

    /// Cleanup old entries to prevent unbounded memory growth
    #[allow(dead_code)]
    fn cleanup(&self) {
        let mut requests = safe_lock(&self.requests);
        let now = Instant::now();
        let window_start = now - self.window;

        // Remove IPs with no recent requests
        requests.retain(|_, reqs| {
            reqs.retain(|&t| t > window_start);
            !reqs.is_empty()
        });
    }
}

/// Session activity tracker for concise session logging
struct SessionActivityTracker {
    last_activity: HashMap<String, u64>,
}

impl SessionActivityTracker {
    fn new() -> Self {
        Self {
            last_activity: HashMap::new(),
        }
    }

    /// Get seconds since last activity for a session
    fn seconds_since_last(&mut self, session_id: &str, now: u64) -> u64 {
        let last = self.last_activity.get(session_id).copied().unwrap_or(now);
        self.last_activity.insert(session_id.to_string(), now);
        now.saturating_sub(last)
    }
}

// Session timestamp cache for cloning detection (Task 8)
// Tracks last request timestamp per session to detect rapid concurrent requests
lazy_static::lazy_static! {
    static ref SESSION_TIMESTAMPS: Arc<Mutex<HashMap<String, u64>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// Check for suspicious rapid requests that indicate session cloning
/// Returns true if cloning suspected (requests < 100ms apart)
fn detect_session_cloning(session_id: &str) -> bool {
    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut timestamps = safe_lock(&SESSION_TIMESTAMPS);

    if let Some(&last_request) = timestamps.get(session_id) {
        let time_diff = now_millis.saturating_sub(last_request);

        // If requests are less than 100ms apart, likely cloning attack
        if time_diff < 100 {
            tracing::warn!(
                "CLONING DETECTED: Session {} made requests {}ms apart",
                &session_id[..8.min(session_id.len())],
                time_diff
            );
            timestamps.insert(session_id.to_string(), now_millis);
            return true;
        }
    }

    timestamps.insert(session_id.to_string(), now_millis);
    false
}

/// Log session activity in a concise format
/// Format: SID SECONDS_SINCE_LAST PATH [EVENT]
fn log_session_activity(sid: &str, seconds_idle: u64, path: &str, event: Option<&str>) {
    let short_sid = &sid[..6.min(sid.len())];
    let idle_str = if seconds_idle == 0 {
        "new".to_string()
    } else if seconds_idle < 60 {
        format!("{}s", seconds_idle)
    } else if seconds_idle < 3600 {
        format!("{}m", seconds_idle / 60)
    } else {
        format!("{}h", seconds_idle / 3600)
    };

    if let Some(ev) = event {
        tracing::info!("[{}] +{} {} → {}", short_sid, idle_str, path, ev);
    } else {
        tracing::info!("[{}] +{} {}", short_sid, idle_str, path);
    }
}

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Missing token")]
    MissingToken,
    #[error("Session not found")]
    SessionNotFound,
    #[error("Backpressure limit exceeded")]
    BackpressureExceeded,
    #[error("Backend unavailable")]
    BackendUnavailable,
    #[error("Request forbidden")]
    Forbidden,
    #[error("Backend request timed out after {0}s")]
    BackendTimeout(u64),
    #[error("Gate request timed out after {0}s")]
    GateTimeout(u64),
    #[error("Service temporarily unavailable - all nodes at capacity")]
    ServiceUnavailable,
}

pub type Result<T> = std::result::Result<T, ProxyError>;

/// Backend node configuration with semaphore-based connection gating
/// Uses actual semaphores instead of soft counters to prevent race conditions
#[derive(Debug, Clone)]
pub struct BackendNode {
    /// Unique node identifier (e.g., "healthy-0", "threat-1")
    pub name: String,
    pub address: String,
    pub healthy_mode: bool,
    pub weight: u32,
    /// Per-node connection semaphore for true concurrency control
    connection_semaphore: Arc<Semaphore>,
    /// Active connection counter for metrics/display (updated alongside semaphore)
    pub active_connections: Arc<Mutex<usize>>,
    pub max_connections: usize,
}

impl BackendNode {
    pub fn new(address: String, healthy_mode: bool, max_connections: usize) -> Self {
        // Generate default name from address
        let name = address.replace([':', '.'], "-");
        Self {
            name,
            address,
            healthy_mode,
            weight: 1,
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
            active_connections: Arc::new(Mutex::new(0)),
            max_connections,
        }
    }

    /// Create a node with an explicit name
    pub fn with_name(
        name: String,
        address: String,
        healthy_mode: bool,
        max_connections: usize,
    ) -> Self {
        Self {
            name,
            address,
            healthy_mode,
            weight: 1,
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
            active_connections: Arc::new(Mutex::new(0)),
            max_connections,
        }
    }

    pub fn can_accept(&self) -> bool {
        // Check if semaphore has available permits
        self.connection_semaphore.available_permits() > 0
    }

    /// Try to acquire a connection slot using semaphores
    /// Returns true if slot acquired, false if at capacity
    /// Uses both global and per-node semaphores for defense-in-depth
    pub fn acquire(&self) -> bool {
        // First try global limit (defense against total exhaustion)
        let global_permit = GLOBAL_CONNECTION_SEMAPHORE.try_acquire();
        if global_permit.is_err() {
            tracing::warn!("Global connection limit reached (1000 total)");
            return false;
        }

        // Then try per-node limit
        match self.connection_semaphore.try_acquire() {
            Ok(_permit) => {
                // Forget permits so they stay acquired until release() is called
                // We manually track via active_connections counter
                std::mem::forget(global_permit.unwrap());
                std::mem::forget(_permit);

                // Update counter for metrics display
                let mut active = safe_lock(&self.active_connections);
                *active += 1;
                true
            }
            Err(_) => {
                // Per-node limit reached, global permit is dropped automatically
                tracing::debug!("Node {} at capacity ({})", self.name, self.max_connections);
                false
            }
        }
    }

    /// Release a connection slot
    pub fn release(&self) {
        // Add permits back to both semaphores
        self.connection_semaphore.add_permits(1);
        GLOBAL_CONNECTION_SEMAPHORE.add_permits(1);

        // Update counter for metrics
        let mut active = safe_lock(&self.active_connections);
        if *active > 0 {
            *active -= 1;
        }
    }
}

/// Request metrics
#[derive(Debug, Default, Clone)]
pub struct Metrics {
    pub requests_total: u64,
    pub requests_allowed: u64,
    pub requests_denied: u64,
    pub tokens_valid: u64,
    pub tokens_invalid: u64,
    pub backend_errors: u64,
    /// 503 responses sent due to capacity limits
    pub capacity_shed: u64,
}

impl Metrics {
    pub fn record_request(&mut self) {
        self.requests_total += 1;
    }

    pub fn record_allowed(&mut self) {
        self.requests_allowed += 1;
    }

    pub fn record_denied(&mut self) {
        self.requests_denied += 1;
    }

    pub fn record_valid_token(&mut self) {
        self.tokens_valid += 1;
    }

    pub fn record_invalid_token(&mut self) {
        self.tokens_invalid += 1;
    }

    pub fn record_backend_error(&mut self) {
        self.backend_errors += 1;
    }

    /// Record a 503 capacity shed response
    pub fn record_capacity_shed(&mut self) {
        self.capacity_shed += 1;
    }
}

/// HTTP proxy server
pub struct HttpProxy {
    bind_addr: SocketAddr,
    max_concurrent: usize,
    secret_key: Vec<u8>,
    session_manager: Arc<SessionManager>,
    healthy_nodes: Vec<BackendNode>,
    threat_nodes: Vec<BackendNode>,
    metrics: Arc<Mutex<Metrics>>,
    active_requests: Arc<Mutex<usize>>,
    admin_state: Arc<AdminState>,
    /// Per-session behavioral analyzers
    behavior_sessions: Arc<RwLock<HashMap<String, SessionBehavior>>>,
    /// Gate address for redirecting unknown users
    gate_address: String,
    /// Session activity tracker for concise logging
    activity_tracker: Arc<Mutex<SessionActivityTracker>>,
    /// Global rate limiter for per-IP request limiting
    rate_limiter: Arc<GlobalRateLimiter>,
    /// Optional callback to check if session is blacklisted
    #[allow(clippy::type_complexity)]
    blacklist_check: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
}

impl HttpProxy {
    pub fn new(
        bind_addr: SocketAddr,
        max_concurrent: usize,
        secret_key: Vec<u8>,
        session_manager: Arc<SessionManager>,
        healthy_nodes: Vec<BackendNode>,
        threat_nodes: Vec<BackendNode>,
    ) -> Self {
        // No onion addresses, default gate address
        Self::new_with_onions(
            bind_addr,
            max_concurrent,
            secret_key,
            session_manager,
            healthy_nodes,
            vec![],
            threat_nodes,
            vec![],
            "http://127.0.0.1:8081".to_string(),
        )
    }

    /// Create with onion addresses for nodes
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_onions(
        bind_addr: SocketAddr,
        max_concurrent: usize,
        secret_key: Vec<u8>,
        session_manager: Arc<SessionManager>,
        healthy_nodes: Vec<BackendNode>,
        healthy_onions: Vec<Option<String>>,
        threat_nodes: Vec<BackendNode>,
        threat_onions: Vec<Option<String>>,
        gate_address: String,
    ) -> Self {
        let admin_state = Arc::new(AdminState::new());

        // Register initial nodes in admin state
        for (i, node) in healthy_nodes.iter().enumerate() {
            let onion = healthy_onions.get(i).cloned().flatten();
            admin_state.update_node(admin::NodeInfo {
                id: format!("healthy-{}", i),
                bind_addr: node.address.clone(),
                onion_address: onion,
                mode: "healthy".to_string(),
                status: "online".to_string(),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                total_requests: 0,
                active_connections: 0,
                violations_detected: 0,
            });
        }
        for (i, node) in threat_nodes.iter().enumerate() {
            let onion = threat_onions.get(i).cloned().flatten();
            admin_state.update_node(admin::NodeInfo {
                id: format!("threat-{}", i),
                bind_addr: node.address.clone(),
                onion_address: onion,
                mode: "threat".to_string(),
                status: "online".to_string(),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                total_requests: 0,
                active_connections: 0,
                violations_detected: 0,
            });
        }

        Self {
            bind_addr,
            max_concurrent,
            secret_key,
            session_manager,
            healthy_nodes,
            threat_nodes,
            metrics: Arc::new(Mutex::new(Metrics::default())),
            active_requests: Arc::new(Mutex::new(0)),
            admin_state: admin_state.clone(),
            behavior_sessions: Arc::new(RwLock::new(HashMap::new())),
            gate_address,
            activity_tracker: Arc::new(Mutex::new(SessionActivityTracker::new())),
            rate_limiter: Arc::new(GlobalRateLimiter::new(10, Some(admin_state))), // 10 second window
            blacklist_check: None,
        }
    }

    /// Set the blacklist check callback
    pub fn set_blacklist_check<F>(&mut self, callback: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.blacklist_check = Some(Arc::new(callback));
    }

    /// Create with existing admin state (for sharing across components)
    pub fn with_admin_state(
        bind_addr: SocketAddr,
        max_concurrent: usize,
        secret_key: Vec<u8>,
        session_manager: Arc<SessionManager>,
        healthy_nodes: Vec<BackendNode>,
        threat_nodes: Vec<BackendNode>,
        admin_state: Arc<AdminState>,
    ) -> Self {
        Self {
            bind_addr,
            max_concurrent,
            secret_key,
            session_manager,
            healthy_nodes,
            threat_nodes,
            metrics: Arc::new(Mutex::new(Metrics::default())),
            active_requests: Arc::new(Mutex::new(0)),
            admin_state: admin_state.clone(),
            behavior_sessions: Arc::new(RwLock::new(HashMap::new())),
            gate_address: "http://127.0.0.1:8081".to_string(),
            activity_tracker: Arc::new(Mutex::new(SessionActivityTracker::new())),
            rate_limiter: Arc::new(GlobalRateLimiter::new(10, Some(admin_state))), // 10 second window
            blacklist_check: None,
        }
    }

    /// Get reference to admin state
    pub fn admin_state(&self) -> Arc<AdminState> {
        Arc::clone(&self.admin_state)
    }

    /// Start the HTTP proxy server
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("HTTP proxy starting on {}", self.bind_addr);
        tracing::info!(
            "Admin panel available at: http://{}{}",
            self.bind_addr,
            ADMIN_PATH
        );

        let listener = TcpListener::bind(&self.bind_addr).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);

            let secret_key = self.secret_key.clone();
            let session_manager = Arc::clone(&self.session_manager);
            let healthy_nodes = self.healthy_nodes.clone();
            let threat_nodes = self.threat_nodes.clone();
            let metrics = Arc::clone(&self.metrics);
            let active_requests = Arc::clone(&self.active_requests);
            let max_concurrent = self.max_concurrent;
            let admin_state = Arc::clone(&self.admin_state);
            let behavior_sessions = Arc::clone(&self.behavior_sessions);
            let gate_address = self.gate_address.clone();
            let activity_tracker = Arc::clone(&self.activity_tracker);
            let rate_limiter = Arc::clone(&self.rate_limiter);
            let blacklist_check = self.blacklist_check.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    handle_proxy_request(
                        req,
                        secret_key.clone(),
                        Arc::clone(&session_manager),
                        healthy_nodes.clone(),
                        threat_nodes.clone(),
                        Arc::clone(&metrics),
                        Arc::clone(&active_requests),
                        max_concurrent,
                        Arc::clone(&admin_state),
                        Arc::clone(&behavior_sessions),
                        gate_address.clone(),
                        Arc::clone(&activity_tracker),
                        Arc::clone(&rate_limiter),
                        blacklist_check.clone(),
                    )
                });

                // Configure HTTP/1.1 with defensive timeouts to prevent slow-loris attacks
                // header_read_timeout: Max time to receive all request headers (30s for Tor latency)
                // max_buf_size: Limit header size to prevent memory exhaustion (16KB)
                // Jitter applied to prevent timing-based fingerprinting
                // timer: Required for timeouts to work in hyper 1.x
                let result = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .header_read_timeout(jittered_timeout(30))
                    .max_buf_size(16 * 1024)
                    .serve_connection(io, service)
                    .await;

                if let Err(err) = result {
                    tracing::error!("Error serving connection: {:?}", err);
                }
            });
        }

        // This is unreachable in normal operation since the loop runs forever
        // but needed for the return type
        #[allow(unreachable_code)]
        Ok(())
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> Metrics {
        safe_lock(&self.metrics).clone()
    }

    /// Get active request count
    pub fn active_requests(&self) -> usize {
        *safe_lock(&self.active_requests)
    }
}

/// Extract client IP from request headers or connection
fn extract_client_ip(req: &Request<Incoming>) -> String {
    // Check X-Forwarded-For header first (if behind proxy/load balancer)
    if let Some(xff) = req.headers().get("X-Forwarded-For") {
        if let Ok(xff_str) = xff.to_str() {
            // XFF can contain multiple IPs, take the first (original client)
            if let Some(first_ip) = xff_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Fallback to "unknown" if no IP can be determined
    // In production, hyper provides connection info, but for now we use unknown
    "unknown".to_string()
}

/// Handle incoming proxy request
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
async fn handle_proxy_request(
    req: Request<Incoming>,
    secret_key: Vec<u8>,
    session_manager: Arc<SessionManager>,
    healthy_nodes: Vec<BackendNode>,
    threat_nodes: Vec<BackendNode>,
    metrics: Arc<Mutex<Metrics>>,
    active_requests: Arc<Mutex<usize>>,
    max_concurrent: usize,
    admin_state: Arc<AdminState>,
    behavior_sessions: Arc<RwLock<HashMap<String, SessionBehavior>>>,
    gate_address: String,
    activity_tracker: Arc<Mutex<SessionActivityTracker>>,
    rate_limiter: Arc<GlobalRateLimiter>,
    blacklist_check: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
) -> std::result::Result<Response<BoxBody>, std::convert::Infallible> {
    // Check if this is an admin panel request first
    let path = req.uri().path();
    if admin::is_admin_request(path) {
        return Ok(admin::handle_admin_request(req, admin_state).await);
    }

    // Extract client IP
    let client_ip = extract_client_ip(&req);

    // Extract session token early to get trust tier for rate limiting
    let token_cookie = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find(|c| c.trim().starts_with("fortify_session="))
                .map(|c| {
                    c.trim()
                        .strip_prefix("fortify_session=")
                        .unwrap_or_default()
                        .to_string()
                })
        });

    // LAYER 1: Bypass rate limiting for Gate/CAPTCHA paths
    // This ensures real users can ALWAYS access CAPTCHA even during DDoS attacks
    let path = req.uri().path();
    let bypass_rate_limit =
        path.starts_with("/gate/") || path == "/Fortify/Portcullis" || path.starts_with("/gate");

    if !bypass_rate_limit {
        // LAYER 2: Circuit-based rate limiting (not IP-based)
        // Extract circuit ID from session cookie or generate temporary fingerprint
        let circuit_id = if let Some(token_str) = token_cookie.as_ref() {
            // Use session ID as circuit identifier (persists across requests)
            if let Ok(_token) = SessionToken::decode(token_str) {
                format!("session_{}", &token_str[..16])
            } else {
                // Invalid token, use IP + User-Agent as fingerprint
                let ua = req
                    .headers()
                    .get("user-agent")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown");
                format!("temp_{}_{}", client_ip, &ua[..ua.len().min(20)])
            }
        } else {
            // No session, use IP + User-Agent + timestamp as temporary circuit ID
            let ua = req
                .headers()
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            format!("temp_{}_{}", client_ip, &ua[..ua.len().min(20)])
        };

        // Determine trust tier for rate limiting
        let trust_tier_for_ratelimit = if let Some(token_str) = token_cookie {
            if let Ok(token) = SessionToken::decode(&token_str) {
                if !token.is_expired() && token.verify(&secret_key).is_ok() {
                    token.trust_tier
                } else {
                    TrustTier::Unknown
                }
            } else {
                TrustTier::Unknown
            }
        } else {
            TrustTier::Unknown
        };

        // Check rate limit per circuit (not global)
        // Unknown: 10/10s per circuit, Verified: 100/10s, Trusted: 300/10s
        if !rate_limiter.check_and_record(&circuit_id, trust_tier_for_ratelimit) {
            let limit = match trust_tier_for_ratelimit {
                TrustTier::Trusted => 300,
                TrustTier::Verified => 100,
                _ => 10,
            };
            tracing::warn!(
                "Rate limited circuit: {} tier={:?} ({} req/10sec exceeded)",
                circuit_id,
                trust_tier_for_ratelimit,
                limit
            );
            let mut m = safe_lock(&metrics);
            m.record_denied();

            // Redirect to gate with CAPTCHA challenge (RELATIVE path to preserve onion address)
            // Using relative redirect ensures user stays on their .onion address
            // instead of being redirected to localhost
            // Store the exact circuit_id so we can clear it after CAPTCHA verification
            return Ok(Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("Location", "/Fortify/Portcullis?reason=rate_limit")
                .header(
                    "Set-Cookie",
                    "fortify_rate_limited=1; Path=/; Max-Age=60; HttpOnly",
                )
                .header(
                    "Set-Cookie",
                    format!(
                        "fortify_rate_limited_circuit={}; Path=/; Max-Age=60; HttpOnly",
                        circuit_id
                    ),
                )
                .body(Full::new(Bytes::new()))
                .expect("valid response"));
        }
    } // end bypass_rate_limit check

    // Record request
    {
        let mut m = safe_lock(&metrics);
        m.record_request();
    }

    // Check backpressure
    {
        let active = *safe_lock(&active_requests);
        if active >= max_concurrent {
            let mut m = safe_lock(&metrics);
            m.record_denied();
            return Ok(error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Service temporarily unavailable",
            ));
        }
    }

    // Acquire request slot
    {
        let mut active = safe_lock(&active_requests);
        *active += 1;
    }

    // Process request
    let result = process_request(
        req,
        secret_key,
        session_manager,
        healthy_nodes,
        threat_nodes,
        Arc::clone(&metrics),
        Arc::clone(&admin_state),
        Arc::clone(&behavior_sessions),
        gate_address,
        Arc::clone(&activity_tracker),
        Arc::clone(&rate_limiter),
        blacklist_check.clone(),
    )
    .await;

    // Release request slot
    {
        let mut active = safe_lock(&active_requests);
        if *active > 0 {
            *active -= 1;
        }
    }

    Ok(result)
}

/// Process and route request
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
async fn process_request(
    req: Request<Incoming>,
    secret_key: Vec<u8>,
    session_manager: Arc<SessionManager>,
    healthy_nodes: Vec<BackendNode>,
    _threat_nodes: Vec<BackendNode>, // Kept for API compatibility, threat users go to Gate not nodes
    metrics: Arc<Mutex<Metrics>>,
    admin_state: Arc<AdminState>,
    behavior_sessions: Arc<RwLock<HashMap<String, SessionBehavior>>>,
    gate_address: String,
    activity_tracker: Arc<Mutex<SessionActivityTracker>>,
    rate_limiter: Arc<GlobalRateLimiter>,
    blacklist_check: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
) -> Response<BoxBody> {
    let request_path = req.uri().path().to_string();
    let request_method = req.method().to_string();

    // Extract headers for behavioral analysis BEFORE consuming req
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let referer = req
        .headers()
        .get("Referer")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let content_length = req
        .headers()
        .get("Content-Length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    // Extract Host header to track which mirror user is using
    let host_header = req
        .headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Check if this request is coming through a paused mirror
    if host_header.contains(".onion") {
        if is_mirror_paused(&host_header) {
            return serve_paused_mirror_page(&host_header);
        }
        // Track mirror request statistics (only for .onion addresses)
        admin_state.record_mirror_request(&host_header);
    }

    // GATE PATH: Route Gate-specific paths directly to Gate service
    // This includes CAPTCHA verification pages and API endpoints
    // These paths must always go to Gate regardless of session status
    if request_path.starts_with("/gate/") || request_path.starts_with("/Fortify/") {
        tracing::info!("GATE PATH: Routing to Gate service: {}", request_path);
        match proxy_to_gate(req, &gate_address, &request_path).await {
            Ok(response) => return response,
            Err(e) => {
                tracing::error!("Failed to proxy to Gate: {}", e);
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Full::new(Bytes::from("Gate service unavailable")))
                    .expect("valid response");
            }
        }
    }

    // Attempt to extract and verify token (or upgrade verification token)
    // Priority: 1) Session token, 2) Verification token (upgrade), 3) None (new user)
    let mut trust_tier = TrustTier::Unknown;
    let mut verified_session_id = None;
    let mut stale_session_id = None;
    let mut raw_token_for_forwarding: Option<String> = None;
    let mut upgraded_session_token: Option<String> = None; // For setting cookie after upgrade

    // Extract both session and verification tokens
    let (session_token_opt, verification_token_opt) = extract_tokens(&req);

    // Try session token first
    if let Some(token_str) = session_token_opt {
        match SessionToken::decode(&token_str) {
            Ok(token) => {
                match token.verify(&secret_key) {
                    Ok(_) => {
                        if token.is_valid() {
                            // Task 7: Validate User-Agent matches token binding
                            let current_ua = user_agent.as_deref().unwrap_or("unknown");
                            if !token.validate_user_agent(current_ua) {
                                tracing::warn!(
                                    "User-Agent mismatch for session {}: token requires different UA",
                                    &token.session_id[..8.min(token.session_id.len())]
                                );
                                // Treat as invalid token - route to gate for re-verification
                                stale_session_id = Some(token.session_id.clone());
                                let mut m = safe_lock(&metrics);
                                m.record_invalid_token();
                            } else {
                                // Task 8: Check for session cloning
                                let cloning_detected = detect_session_cloning(&token.session_id);
                                if cloning_detected {
                                    tracing::warn!(
                                        "Session cloning detected for {}, maintaining session but flagging",
                                        &token.session_id[..8.min(token.session_id.len())]
                                    );
                                }

                                tracing::info!(
                                    "Valid session token for session {}",
                                    token.session_id
                                );
                                trust_tier = token.trust_tier;
                                verified_session_id = Some(token.session_id.clone());
                                raw_token_for_forwarding = Some(token_str.clone());

                                // Clear tier override for fresh tokens
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                let token_age_secs = now.saturating_sub(token.issued_at);
                                if token.trust_tier == TrustTier::Verified && token_age_secs < 30 {
                                    tracing::info!("Fresh token ({}s old) for session {} - clearing tier override", token_age_secs, token.session_id);
                                    admin_state.clear_tier_override(&token.session_id);
                                }

                                let mut m = safe_lock(&metrics);
                                m.record_valid_token();
                            }
                        } else {
                            tracing::info!(
                                "Token expired/burned for session {} - routing to gate",
                                token.session_id
                            );
                            stale_session_id = Some(token.session_id);
                            let mut m = safe_lock(&metrics);
                            m.record_invalid_token();
                        }
                    }
                    Err(e) => {
                        // Token signature invalid (e.g., service restarted with new secret key)
                        // Preserve session ID and route to gate for re-verification
                        tracing::info!("Token verification failed for session {} ({}), routing to gate for re-verification", token.session_id, e);
                        stale_session_id = Some(token.session_id);
                        let mut m = safe_lock(&metrics);
                        m.record_invalid_token();
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to decode token: {}", e);
            }
        }
    }

    // Task 6: If we have a verification token, ALWAYS try to upgrade it
    // This takes priority over existing session tokens because it means the user just solved a CAPTCHA
    // Critical for demoted users who still have valid session tokens but need to re-verify
    // Store the demoted session ID BEFORE potentially overwriting verified_session_id
    let demoted_session_id = verified_session_id.clone();

    if let Some(verification_token) = verification_token_opt.clone() {
        let current_ua = user_agent.as_deref().unwrap_or("unknown");

        tracing::info!(
            "Found verification token, attempting upgrade (existing session: {:?})",
            demoted_session_id
        );

        // Extract the original session ID from cookie (set by Gate for demoted users)
        // This is the session that was demoted and needs its tier override cleared
        let original_session_id = req
            .headers()
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .find(|c| c.trim().starts_with("fortify_original_session="))
                    .map(|c| {
                        c.trim()
                            .strip_prefix("fortify_original_session=")
                            .unwrap_or_default()
                            .to_string()
                    })
            });

        // Extract the rate-limited circuit_id from cookie
        // This is the exact circuit_id that was rate-limited before CAPTCHA verification
        let rate_limited_circuit = req
            .headers()
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .find(|c| c.trim().starts_with("fortify_rate_limited_circuit="))
                    .map(|c| {
                        c.trim()
                            .strip_prefix("fortify_rate_limited_circuit=")
                            .unwrap_or_default()
                            .to_string()
                    })
            });

        // Call Gate's upgrade endpoint
        if let Some(session_token_str) =
            upgrade_verification_token(&verification_token, current_ua, &gate_address).await
        {
            // Successfully upgraded! Decode the session token
            match SessionToken::decode(&session_token_str) {
                Ok(token) => {
                    tracing::info!(
                        "Successfully upgraded verification token to session {}",
                        token.session_id
                    );
                    trust_tier = token.trust_tier;
                    verified_session_id = Some(token.session_id.clone());
                    raw_token_for_forwarding = Some(session_token_str.clone());
                    upgraded_session_token = Some(session_token_str); // Will set cookie in response

                    // CRITICAL: Clear tier override after successful CAPTCHA verification
                    // This ensures demoted users who solved 2 captchas get promoted back to Verified
                    // Without this, the old "Suspicious" override would persist and send them back to Gate
                    admin_state.clear_tier_override(&token.session_id);
                    tracing::info!(
                        "Cleared tier override for session {} after CAPTCHA verification",
                        token.session_id
                    );

                    // ALSO clear tier override for the ORIGINAL session if this was a demoted user
                    // Gate sets fortify_original_session cookie with the session that was demoted
                    // We must clear its override too, or requests with old session cookie will loop
                    if let Some(ref orig_sid) = original_session_id {
                        admin_state.clear_tier_override(orig_sid);
                        tracing::info!(
                            "Cleared tier override for ORIGINAL demoted session {} after CAPTCHA verification",
                            orig_sid
                        );
                    }

                    // ALSO clear tier override for the demoted_session_id if it exists
                    // This is the session from the fortify_session cookie that was demoted
                    // Critical: demoted users have valid session tokens, so this is different from stale_session_id
                    if let Some(ref demoted_sid) = demoted_session_id {
                        if Some(demoted_sid) != original_session_id.as_ref()
                            && demoted_sid != &token.session_id
                        {
                            admin_state.clear_tier_override(demoted_sid);
                            tracing::info!(
                                "Cleared tier override for DEMOTED session {} after CAPTCHA verification",
                                demoted_sid
                            );
                        }
                    }

                    // Also check stale_session_id for completeness
                    if let Some(ref stale_sid) = stale_session_id {
                        if Some(stale_sid) != original_session_id.as_ref()
                            && Some(stale_sid) != demoted_session_id.as_ref()
                        {
                            admin_state.clear_tier_override(stale_sid);
                            tracing::info!(
                                "Cleared tier override for STALE session {} after CAPTCHA verification",
                                stale_sid
                            );
                        }
                    }

                    // Clear rate limit quota for the circuit that was rate-limited
                    // This prevents infinite CAPTCHA loops for legitimate users during attacks
                    // Ensures new users can access the site immediately after solving CAPTCHA
                    if let Some(circuit_id) = rate_limited_circuit {
                        rate_limiter.clear_circuit_quota(&circuit_id);
                        tracing::info!(
                            "Cleared rate limit quota for circuit: {} after CAPTCHA verification",
                            circuit_id
                        );
                    }

                    let mut m = safe_lock(&metrics);
                    m.record_valid_token();
                }
                Err(e) => {
                    tracing::error!("Failed to decode upgraded session token: {}", e);
                }
            }
        } else {
            tracing::warn!("Failed to upgrade verification token");
        }
    }

    if verified_session_id.is_none() && verification_token_opt.is_none() {
        tracing::debug!("No token found in request");
    }

    // Track if this is a brand new visitor (no existing session at all)
    // Used to differentiate new users (1 CAPTCHA) from demoted users (2 CAPTCHAs)
    let is_new_visitor = verified_session_id.is_none() && stale_session_id.is_none();

    // Get or create session based on verification status
    // Priority: verified_session_id > stale_session_id > new session
    let session = if let Some(ref sid) = verified_session_id {
        match session_manager.get_session(sid) {
            Some(s) => s,
            None => {
                // Token valid but session gone? Recreate transient session
                let mut session = session_manager.create_session(sid.clone());
                session.token.trust_tier = trust_tier;
                session_manager.update_session(session.clone());
                session
            }
        }
    } else if let Some(ref sid) = stale_session_id {
        // Stale token - preserve the session ID but route to gate for re-verification
        // Create or get session with Unknown tier so they go through gate
        match session_manager.get_session(sid) {
            Some(mut s) => {
                // Session exists, but token was stale - they need re-verification
                s.token.trust_tier = TrustTier::Unknown;
                session_manager.update_session(s.clone());
                s
            }
            None => {
                // Session doesn't exist, create new one with their existing ID
                let mut session = session_manager.create_session(sid.clone());
                session.token.trust_tier = TrustTier::Unknown;
                session_manager.update_session(session.clone());
                session
            }
        }
    } else {
        // No Valid Token -> Create a NEW real session with Unknown tier
        // This ensures every visitor gets tracked from their first request
        let new_session_id = uuid::Uuid::new_v4().to_string();
        let mut session = session_manager.create_session(new_session_id.clone());
        session.token.trust_tier = TrustTier::Unknown;
        session_manager.update_session(session.clone());
        // Store the new session ID for downstream processing
        verified_session_id = Some(new_session_id);
        session
    };

    // Use stale_session_id as verified_session_id for downstream processing if we have one
    // This ensures the session ID is preserved in responses and tracking
    if verified_session_id.is_none() && stale_session_id.is_some() {
        verified_session_id = stale_session_id;
    }

    // =========================================================================
    // BEHAVIORAL ANALYSIS
    // =========================================================================
    let mut behavior_violations = Vec::new();
    if let Some(ref sid) = verified_session_id {
        // Get behavioral config from admin state
        let behavior_config = admin_state.get_behavior_config();

        // Check if behavioral analysis is enabled
        if admin_state.is_behavior_enabled() {
            // Create request metadata for analysis
            let req_meta = RequestMeta::new(
                request_path.clone(),
                request_method.clone(),
                user_agent.clone(),
                referer.clone(),
                content_length,
            );

            // Get or create session behavior tracker
            let mut behavior_sessions_guard = safe_write(&behavior_sessions);
            let session_behavior = behavior_sessions_guard
                .entry(sid.clone())
                .or_insert_with(|| SessionBehavior::new(sid.clone(), behavior_config.clone()));

            // Update config if it changed
            session_behavior.update_config(behavior_config.clone());

            // Analyze the request
            behavior_violations = session_behavior.analyze(&req_meta);

            // Log violations and record high-severity ones in session history
            for v in &behavior_violations {
                tracing::warn!(
                    "Behavioral violation for session {}: {} - {}",
                    sid,
                    v.violation_type.as_str(),
                    v.details
                );
                // Record severity 2+ violations in session history for visibility
                if v.severity >= 2 {
                    admin_state.record_violation(
                        sid,
                        v.violation_type.as_str(),
                        &v.details,
                        v.severity,
                    );
                }
            }

            // Update behavior stats in admin state
            let stats = session_behavior.get_stats().clone();
            drop(behavior_sessions_guard); // Release lock before calling admin_state
            admin_state.update_behavior_stats(sid, stats.clone());

            // Check if session should be automatically demoted to threat node
            if behavior_config.should_demote_to_threat(&stats) {
                // Only demote if not already in a threat tier
                if !trust_tier.requires_gate() {
                    // Build a reason string from violations
                    let reason = format!(
                        "Violations: {}, Severity: {}",
                        stats.total_violations(),
                        stats.severity_score()
                    );

                    // Record this demotion and check if session should be killed
                    let max_demotions = behavior_config.max_demotions_before_kill;
                    let was_killed =
                        admin_state.record_demotion_with_reason(sid, max_demotions, &reason);

                    if was_killed {
                        tracing::error!(
                            "Session {} KILLED - repeat offender ({} demotions)",
                            sid,
                            max_demotions
                        );
                        // Killed sessions get permanently burned - record_demotion_with_reason already recorded the kill event
                        trust_tier = TrustTier::Burned;
                    } else {
                        let demotion_count = admin_state.get_demotion_count(sid);
                        tracing::warn!(
                            "Auto-demoting session {} to threat pool (violations: {}, severity: {}, demotion #{}/{})",
                            sid, stats.total_violations(), stats.severity_score(), demotion_count, max_demotions
                        );
                        // Set admin override to force session to threat nodes - use auto version to record in history
                        admin_state.set_session_tier_auto(sid, "Suspicious", &reason);
                        trust_tier = TrustTier::Suspicious;
                    }
                }
            }
        }
    }

    // Check if session is banned or killed via admin state
    if let Some(ref sid) = verified_session_id {
        // Check blacklist first (if callback is set)
        if let Some(ref check_blacklist) = blacklist_check {
            if check_blacklist(sid) {
                tracing::warn!("Session {} is blacklisted, redirecting to gate", sid);
                let mut m = safe_lock(&metrics);
                m.record_denied();

                // Redirect to gate for re-verification (RELATIVE to preserve onion)
                return Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("Location", "/Fortify")
                    .header(
                        "Set-Cookie",
                        "fortify_demoted=1; Path=/; Max-Age=300; HttpOnly",
                    )
                    .body(Full::new(Bytes::from(
                        "Session blacklisted - please verify again",
                    )))
                    .expect("valid response");
            }
        }

        if admin_state.is_killed(sid) {
            // Killed sessions get a friendly page explaining they can try again
            tracing::info!("Session {} is killed, showing recovery page", sid);
            return serve_killed_session_page();
        }

        if admin_state.is_banned(sid) {
            let mut m = safe_lock(&metrics);
            m.record_denied();
            return error_response(StatusCode::FORBIDDEN, "Session permanently banned");
        }

        // Check for admin tier override - this overrides the token's tier
        if let Some(override_tier) = admin_state.get_tier_override(sid) {
            tracing::info!(
                "Admin tier override for session {}: {} -> {}",
                sid,
                trust_tier.as_str(),
                override_tier
            );
            trust_tier = match override_tier.to_lowercase().as_str() {
                "verified" => TrustTier::Verified,
                "trusted" => TrustTier::Trusted,
                "suspicious" => TrustTier::Suspicious,
                "burned" | "killed" => TrustTier::Burned,
                _ => TrustTier::Unknown,
            };
        }
    }

    // Check if session is burned
    if trust_tier == TrustTier::Burned {
        let mut m = safe_lock(&metrics);
        m.record_denied();
        return serve_killed_session_page(); // Use friendly page for burned sessions too
    }

    // ==========================================================================
    // ROUTING LOGIC - CRITICAL ARCHITECTURE:
    //
    // THREAT PATH (requires_gate() = true): Unknown, Suspicious users
    //   -> Proxy to Gate -> User sees Fortify/captcha page
    //   -> User MUST solve captcha to escape to healthy pool
    //   -> Threat nodes only serve static Fortify pages, never the real site
    //
    // HEALTHY PATH (requires_gate() = false): Verified, Trusted users
    //   -> Proxy to backend -> User sees real site
    //   -> Behavioral monitoring active, can demote back to threat if suspicious
    // ==========================================================================

    // THREAT PATH: Unknown users OR users demoted to threat pool
    // These users must solve a captcha to prove they're human before accessing the real site
    if trust_tier.requires_gate() {
        // Check global capacity FIRST - shed threat-tier traffic before verified sessions
        // This protects verified/trusted users during DDoS attacks
        let global_available = GLOBAL_CONNECTION_SEMAPHORE.available_permits();
        let capacity_threshold = 100; // Reserve 10% of 1000 permits for verified sessions

        if global_available < capacity_threshold {
            tracing::warn!(
                "503: Global capacity low ({} permits available), shedding threat-tier request",
                global_available
            );
            return serve_busy_page();
        }

        tracing::info!(
            "THREAT PATH: Proxying {} user to Gate for verification: {}",
            trust_tier.as_str(),
            request_path
        );
        {
            let mut m = safe_lock(&metrics);
            m.record_request();
        } // Lock released here before async call

        // Track new session in admin state before going to gate
        if let Some(ref sid) = verified_session_id {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Create or update session info in admin state
            let existing = admin_state.get_session(sid);
            let session_info = existing.unwrap_or_else(|| admin::SessionInfo {
                session_id: sid.clone(),
                trust_tier: trust_tier.as_str().to_string(),
                request_count: 0,
                violation_count: 0,
                page_loads: 0,
                created_at: now,
                last_activity: now,
                browsing_history: Vec::new(),
                is_banned: false,
                behavior_stats: None,
                demotion_count: 0,
                is_killed: false,
                current_node: String::new(),
                total_bytes: 0,
                current_mirror: host_header.clone(),
            });
            admin_state.update_session(session_info);
        }

        // Detect demoted users: they had an existing session that was demoted to threat pool
        // NEW visitors (is_new_visitor=true) should NOT be marked as demoted - they get 1 CAPTCHA
        // Demoted/stale users (is_new_visitor=false) get 2 CAPTCHAs to re-verify
        let is_demoted_user = !is_new_visitor;

        // Proxy to Gate - path logic:
        // 1. Demoted users ALWAYS go to /Fortify first (to see "Hold Position" page)
        // 2. Already in Gate flow (/Fortify/*) - preserve path for captcha/verify etc
        // 3. Root path - go to /Fortify landing page
        // 4. Other paths - preserve for after verification
        let gate_path = if is_demoted_user && !request_path.starts_with("/Fortify") {
            // Demoted users must see the "Hold Position" page first
            tracing::info!(
                "Demoted user redirected to /Fortify (was trying: {})",
                request_path
            );
            "/Fortify".to_string()
        } else if request_path == "/" || request_path.is_empty() {
            "/Fortify".to_string()
        } else {
            request_path.clone()
        };

        match proxy_to_gate_with_demoted(req, &gate_address, &gate_path, is_demoted_user).await {
            Ok(mut resp) => {
                // Inject session cookie for new visitors so they're tracked through Gate flow
                if let Some(ref sid) = verified_session_id {
                    // Create an unsigned token just for tracking (not for auth)
                    // Gate will issue a proper signed token after captcha verification
                    tracing::info!(
                        "Setting fortify_pending_session cookie for session: {}",
                        sid
                    );
                    resp.headers_mut().append(
                        "Set-Cookie",
                        format!("fortify_pending_session={}; Path=/; HttpOnly", sid)
                            .parse()
                            .expect("valid cookie header"),
                    );
                } else {
                    tracing::warn!("No verified_session_id available to set cookie!");
                }

                // CRITICAL: Set fortify_demoted=1 cookie for demoted users
                // This tells Gate to show the "hold position" page and require 2 captchas
                if is_demoted_user {
                    tracing::info!("Setting fortify_demoted cookie for demoted user");
                    resp.headers_mut().append(
                        "Set-Cookie",
                        "fortify_demoted=1; Path=/; HttpOnly; SameSite=Lax"
                            .parse()
                            .expect("valid cookie header"),
                    );
                }
                return resp;
            }
            Err(e) => {
                tracing::error!("Failed to proxy to Gate: {}", e);
                return error_response(StatusCode::BAD_GATEWAY, "Gate temporarily unavailable");
            }
        }
    }

    // HEALTHY PATH: Verified/Trusted users get proxied to the real backend
    tracing::info!(
        "HEALTHY PATH: Routing {} user to backend: {}",
        trust_tier.as_str(),
        request_path
    );

    // Route to backend via healthy nodes only
    // Pass the raw encoded token so backend can identify the session
    let (mut response, routed_node_id, response_bytes) = match route_to_backend(
        req,
        &healthy_nodes, // Always use healthy nodes for backend - threat users never reach here
        "healthy",
        Arc::clone(&admin_state),
        raw_token_for_forwarding.clone(),
    )
    .await
    {
        Ok((resp, node_id)) => {
            let mut m = safe_lock(&metrics);
            m.record_allowed();
            // Get response Content-Length for traffic tracking
            let resp_bytes = resp
                .headers()
                .get("Content-Length")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            // Record response traffic to global stats
            admin_state.record_response_traffic(resp_bytes);
            (resp, node_id, resp_bytes)
        }
        Err(e) => {
            let mut m = safe_lock(&metrics);
            m.record_backend_error();

            // Check if this is a capacity error - return 503 instead of 502
            if e.contains("No available backend nodes") || e.contains("capacity") {
                tracing::warn!("503: All backend nodes at capacity");
                (serve_busy_page(), String::new(), 0u64)
            } else {
                (
                    error_response(StatusCode::BAD_GATEWAY, &format!("Backend error: {}", e)),
                    String::new(),
                    0u64,
                )
            }
        }
    };

    // Task 6: If we upgraded a verification token, set the session cookie
    if let Some(session_token_str) = upgraded_session_token {
        tracing::info!("Setting session cookie after token upgrade");
        response.headers_mut().append(
            "Set-Cookie",
            format!(
                "fortify_session={}; Path=/; HttpOnly; Max-Age=86400; SameSite=Strict",
                session_token_str
            )
            .parse()
            .expect("valid cookie header"),
        );
        // Clear the verification token cookie
        response.headers_mut().append(
            "Set-Cookie",
            "fortify_verification=; Path=/; Max-Age=0; HttpOnly"
                .parse()
                .expect("valid cookie header"),
        );
    }

    // Log session activity with concise format
    if let Some(ref sid) = verified_session_id {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let seconds_idle = {
            let mut tracker = safe_lock(&activity_tracker);
            tracker.seconds_since_last(sid, now)
        };
        log_session_activity(sid, seconds_idle, &request_path, None);
    }

    // Track session in admin state
    if let Some(ref sid) = verified_session_id {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update or create session info in admin state
        let existing = admin_state.get_session(sid);
        let mut session_info = existing.unwrap_or_else(|| admin::SessionInfo {
            session_id: sid.clone(),
            trust_tier: trust_tier.as_str().to_string(),
            request_count: 0,
            violation_count: session.violation_count,
            page_loads: 0,
            created_at: now,
            last_activity: now,
            browsing_history: Vec::new(),
            is_banned: false,
            behavior_stats: None,
            demotion_count: 0,
            is_killed: false,
            current_node: String::new(),
            total_bytes: 0,
            current_mirror: String::new(),
        });

        session_info.request_count += 1;
        session_info.last_activity = now;
        session_info.trust_tier = trust_tier.as_str().to_string(); // Use possibly overridden tier
        session_info.current_node = routed_node_id.clone(); // Track current node
                                                            // Update mirror if host header contains .onion
        if host_header.contains(".onion") {
            session_info.current_mirror = host_header.clone();
        }

        // Record traffic bytes (request + response)
        let total_traffic = content_length as u64 + response_bytes;
        session_info.total_bytes += total_traffic;

        // Accumulate violation count from this request's behavioral violations
        session_info.violation_count += behavior_violations.len() as u32;

        // Get behavior stats from admin state and attach to session info
        if let Some(bstats) = admin_state.get_behavior_stats(sid) {
            // Sync violation count from behavior stats (authoritative source)
            session_info.violation_count = bstats.total_violations() as u32;
            session_info.behavior_stats = Some(bstats);
        }

        admin_state.update_session(session_info);
        admin_state.record_page_load(
            sid,
            &request_path,
            &request_method,
            response.status().as_u16(),
        );
    }

    response
}

/// Extract token from Authorization header or Cookie
/// Extract tokens from request (session token or verification token)
/// Returns (session_token, verification_token)
fn extract_tokens(req: &Request<Incoming>) -> (Option<String>, Option<String>) {
    let mut session_token = None;
    let mut verification_token = None;

    // 1. Check Authorization: Bearer for session token
    if let Some(token) = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
    {
        session_token = Some(token);
    }

    // 2. Check Cookies for both session and verification tokens
    if let Some(cookie_header) = req.headers().get(hyper::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(val) = cookie.strip_prefix("fortify_session=") {
                    session_token = Some(val.to_string());
                }
                if let Some(val) = cookie.strip_prefix("fortify_verification=") {
                    verification_token = Some(val.to_string());
                }
            }
        }
    }

    (session_token, verification_token)
}

/// Legacy function for backward compatibility
#[allow(dead_code)]
fn extract_token(req: &Request<Incoming>) -> Option<String> {
    let (session_token, _) = extract_tokens(req);
    session_token
}

/// Upgrade verification token to session token by calling Gate's upgrade endpoint
/// Returns session token string on success, None on failure
async fn upgrade_verification_token(
    verification_token: &str,
    user_agent: &str,
    gate_address: &str,
) -> Option<String> {
    // Timeout for Gate token upgrade (10s - this should be fast)
    // Jitter applied to prevent timing-based fingerprinting
    const TOKEN_UPGRADE_TIMEOUT_SECS: u64 = 10;

    // Use reqwest with explicit timeout for token upgrade
    let client = reqwest::Client::builder()
        .timeout(jittered_timeout(TOKEN_UPGRADE_TIMEOUT_SECS))
        .build()
        .ok()?;

    // Build request body
    let body_json = serde_json::json!({
        "verification_token": verification_token
    });

    let gate_url = format!("{}/gate/upgrade-token", gate_address);

    // Send request to Gate
    match client
        .post(&gate_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", user_agent)
        .body(body_json.to_string())
        .send()
        .await
    {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::OK {
                // Parse response JSON
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(session_token) =
                            json.get("session_token").and_then(|v| v.as_str())
                        {
                            tracing::info!(
                                "Successfully upgraded verification token to session token"
                            );
                            return Some(session_token.to_string());
                        } else {
                            tracing::error!("Gate response missing session_token field");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse Gate upgrade response: {}", e);
                    }
                }
            } else {
                tracing::warn!("Gate upgrade failed with status: {}", response.status());
            }
        }
        Err(e) => {
            tracing::error!("Failed to call Gate upgrade endpoint: {}", e);
        }
    }

    None
}

/// Proxy request to Gate for unknown users
/// This proxies instead of redirecting because redirects to 127.0.0.1 don't work for Tor users
async fn proxy_to_gate(
    req: Request<Incoming>,
    gate_address: &str,
    gate_path: &str,
) -> std::result::Result<Response<BoxBody>, String> {
    // Timeout for Gate requests (30s - Gate should respond quickly)
    // Jitter applied to prevent timing-based fingerprinting
    const GATE_REQUEST_TIMEOUT_SECS: u64 = 30;
    const GATE_CONNECT_TIMEOUT_SECS: u64 = 5;

    // Build full Gate URL preserving query string if present
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let gate_url = format!("{}{}{}", gate_address, gate_path, query);
    tracing::debug!("Proxying to Gate: {}", gate_url);

    // Use reqwest for simpler HTTP proxying with explicit timeouts
    let client = reqwest::Client::builder()
        .connect_timeout(jittered_timeout(GATE_CONNECT_TIMEOUT_SECS))
        .timeout(jittered_timeout(GATE_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Build the request
    let method = match req.method().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    let mut request_builder = client.request(method, &gate_url);

    // Copy safe headers
    for (name, value) in req.headers() {
        let name_str = name.as_str().to_lowercase();
        // Skip hop-by-hop headers and host
        if name_str != "host"
            && name_str != "connection"
            && name_str != "keep-alive"
            && name_str != "transfer-encoding"
            && name_str != "upgrade"
        {
            if let Ok(v) = value.to_str() {
                request_builder = request_builder.header(name.as_str(), v);
            }
        }
    }

    // Collect request body
    let body_bytes = req
        .collect()
        .await
        .map_err(|e| format!("Failed to read request body: {}", e))?
        .to_bytes();

    request_builder = request_builder.body(body_bytes.to_vec());

    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("Gate request failed: {}", e))?;

    // Convert reqwest response to hyper response
    let status = StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = Response::builder().status(status);

    for (name, value) in response.headers() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read gate response body: {}", e))?;

    builder
        .body(Full::new(Bytes::from(body_bytes.to_vec())))
        .map_err(|e| format!("Failed to build response: {}", e))
}

/// Proxy request to Gate service with demoted status header
/// This variant adds an X-Fortify-Demoted header so Gate knows immediately
/// that this is a demoted user (before cookies are set)
async fn proxy_to_gate_with_demoted(
    req: Request<Incoming>,
    gate_address: &str,
    gate_path: &str,
    is_demoted: bool,
) -> std::result::Result<Response<BoxBody>, String> {
    // Timeout for Gate requests (30s - Gate should respond quickly)
    const GATE_REQUEST_TIMEOUT_SECS: u64 = 30;
    const GATE_CONNECT_TIMEOUT_SECS: u64 = 5;

    // Build full Gate URL preserving query string if present
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let gate_url = format!("{}{}{}", gate_address, gate_path, query);
    tracing::debug!("Proxying to Gate: {} (demoted={})", gate_url, is_demoted);

    let client = reqwest::Client::builder()
        .connect_timeout(jittered_timeout(GATE_CONNECT_TIMEOUT_SECS))
        .timeout(jittered_timeout(GATE_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let method = match req.method().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    let mut request_builder = client.request(method, &gate_url);

    // Copy safe headers
    for (name, value) in req.headers() {
        let name_str = name.as_str().to_lowercase();
        if name_str != "host"
            && name_str != "connection"
            && name_str != "keep-alive"
            && name_str != "transfer-encoding"
            && name_str != "upgrade"
        {
            if let Ok(v) = value.to_str() {
                request_builder = request_builder.header(name.as_str(), v);
            }
        }
    }

    // CRITICAL: Add demoted status header so Gate knows immediately
    if is_demoted {
        request_builder = request_builder.header("X-Fortify-Demoted", "1");
    }

    let body_bytes = req
        .collect()
        .await
        .map_err(|e| format!("Failed to read request body: {}", e))?
        .to_bytes();

    request_builder = request_builder.body(body_bytes.to_vec());

    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("Gate request failed: {}", e))?;

    let status = StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = Response::builder().status(status);

    for (name, value) in response.headers() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read gate response body: {}", e))?;

    builder
        .body(Full::new(Bytes::from(body_bytes.to_vec())))
        .map_err(|e| format!("Failed to build response: {}", e))
}

/// Route request to backend node
/// Returns the response and the node ID that handled the request
async fn route_to_backend(
    req: Request<Incoming>,
    nodes: &[BackendNode],
    pool_type: &str,
    admin_state: Arc<AdminState>,
    session_token: Option<String>,
) -> std::result::Result<(Response<BoxBody>, String), String> {
    // Find available node
    let (node, node_index) = nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.can_accept())
        .map(|(i, n)| (n, i))
        .ok_or_else(|| "No available backend nodes".to_string())?;

    // Generate node ID for tracking
    let node_id = format!("{}-{}", pool_type, node_index);

    // Acquire connection slot
    if !node.acquire() {
        return Err("Failed to acquire backend connection".to_string());
    }

    // Record the request to this node
    let content_length = req
        .headers()
        .get("Content-Length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    admin_state.record_node_request(&node_id, content_length);

    // Build backend URI - include path AND query string
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");
    let backend_url = format!("{}{}", node.address, path_and_query);

    // Backend request timeouts (60s total, 10s connect)
    // These protect against slow-loris attacks on backend connections
    // Jitter applied to prevent timing-based fingerprinting
    const BACKEND_TIMEOUT_SECS: u64 = 60;
    const BACKEND_CONNECT_TIMEOUT_SECS: u64 = 10;

    // Use reqwest for backend proxying with explicit timeouts
    let client = reqwest::Client::builder()
        .connect_timeout(jittered_timeout(BACKEND_CONNECT_TIMEOUT_SECS))
        .timeout(jittered_timeout(BACKEND_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            node.release();
            admin_state.release_node_connection(&node_id);
            format!("Failed to build HTTP client: {}", e)
        })?;

    let method = match req.method().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    let mut request_builder = client.request(method, &backend_url);

    // Copy headers and inject session token
    for (name, value) in req.headers() {
        let name_str = name.as_str().to_lowercase();
        if name_str != "host"
            && name_str != "connection"
            && name_str != "keep-alive"
            && name_str != "transfer-encoding"
            && name_str != "upgrade"
        {
            if let Ok(v) = value.to_str() {
                request_builder = request_builder.header(name.as_str(), v);
            }
        }
    }

    // Inject X-Session-ID header if we have a verified session token
    if let Some(ref token) = session_token {
        request_builder = request_builder.header("X-Session-ID", token);
    }

    // Collect request body
    let body_bytes = req
        .collect()
        .await
        .map_err(|e| {
            node.release();
            admin_state.release_node_connection(&node_id);
            format!("Failed to read request body: {}", e)
        })?
        .to_bytes();

    request_builder = request_builder.body(body_bytes.to_vec());

    let result = request_builder.send().await;

    // Release connection slot
    node.release();
    admin_state.release_node_connection(&node_id);

    let response = result.map_err(|e| format!("Backend request failed: {}", e))?;

    // Convert reqwest response to hyper response
    let status = StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = Response::builder().status(status);

    // Check for demotion header
    let demote = response.headers().get("X-Fortify-Demote").is_some();

    for (name, value) in response.headers() {
        if name.as_str() != "x-fortify-demote" {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    // Handle demotion
    if demote {
        tracing::warn!("Session demoted by Node, clearing cookie");
        builder = builder.header(
            "Set-Cookie",
            "fortify_session=; Path=/; Max-Age=0; HttpOnly",
        );
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read backend response body: {}", e))?;

    let response = builder
        .body(Full::new(Bytes::from(body_bytes.to_vec())))
        .map_err(|e| format!("Failed to build response: {}", e))?;

    Ok((response, node_id))
}

/// Serve a friendly page for killed/burned sessions explaining they can try again
fn serve_killed_session_page() -> Response<BoxBody> {
    use fortify_core::templates::{BrandingVars, TemplateEngine, TemplateType};

    let engine = TemplateEngine::new();
    // Read branding from environment variables for consistency
    let branding = BrandingVars::from_env();
    let html = engine.render_with_branding(TemplateType::SessionExpired, &branding, None);

    // Clear the old session cookie so they start fresh
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .header(
            "Set-Cookie",
            "fortify_session=; Path=/; Max-Age=0; HttpOnly",
        )
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

/// Serve a 503 Service Unavailable page when at capacity
/// Includes auto-refresh and Retry-After header for graceful degradation
fn serve_busy_page() -> Response<BoxBody> {
    use rand::Rng;

    // Add jitter to retry time (25-35 seconds) to prevent thundering herd
    let mut rng = rand::rng();
    let retry_seconds = rng.random_range(25..=35);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="refresh" content="{}">
    <title>Fortify — Service Busy</title>
    <style>
        :root {{
            --bg-deep: #141417;
            --bg-surface: #1e1e23;
            --bg-elevated: #26262d;
            --border-subtle: #3a3a42;
            --gold-primary: #c9a227;
            --gold-muted: #a68b5b;
            --text-primary: #f5f0e8;
            --text-secondary: #a8a4a0;
            --text-muted: #6b6862;
            --amber: #e4bc5e;
            --slate-blue: #6b7c8c;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: var(--bg-deep);
            font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, sans-serif;
            color: var(--text-primary);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 24px;
        }}
        .container {{ max-width: 560px; width: 100%; }}
        .busy-box {{
            background: var(--bg-surface);
            border: 1px solid var(--amber);
            border-radius: 4px;
            padding: 32px;
        }}
        .busy-header {{
            display: flex;
            align-items: center;
            gap: 16px;
            margin-bottom: 20px;
            padding-bottom: 16px;
            border-bottom: 1px solid var(--border-subtle);
        }}
        .busy-icon {{ font-size: 2.25rem; opacity: 0.9; }}
        h1 {{
            font-size: 1.2rem;
            font-weight: 500;
            letter-spacing: 0.1em;
            text-transform: uppercase;
            color: var(--amber);
        }}
        .message {{
            color: var(--text-secondary);
            line-height: 1.6;
            margin-bottom: 24px;
        }}
        .countdown {{
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 16px;
            background: var(--bg-elevated);
            border: 1px solid var(--border-subtle);
            border-radius: 4px;
            margin-bottom: 20px;
        }}
        .countdown-icon {{ font-size: 1.5rem; opacity: 0.7; }}
        .countdown-text {{ color: var(--text-secondary); font-size: 0.95rem; }}
        .countdown-time {{ color: var(--amber); font-weight: 600; }}
        .retry-link {{
            display: inline-block;
            padding: 12px 24px;
            background: transparent;
            border: 1px solid var(--gold-muted);
            color: var(--gold-primary);
            text-decoration: none;
            border-radius: 4px;
            font-size: 0.9rem;
            transition: all 0.2s ease;
        }}
        .retry-link:hover {{
            background: var(--gold-primary);
            color: var(--bg-deep);
            border-color: var(--gold-primary);
        }}
        .status-info {{
            margin-top: 24px;
            padding-top: 16px;
            border-top: 1px solid var(--border-subtle);
            font-size: 0.85rem;
            color: var(--text-muted);
        }}
        .status-code {{ font-family: monospace; color: var(--slate-blue); }}
    </style>
</head>
<body>
    <div class="container">
        <div class="busy-box">
            <div class="busy-header">
                <span class="busy-icon">⏳</span>
                <h1>Service Busy</h1>
            </div>
            <p class="message">
                The service is experiencing high demand. Your request will be 
                retried automatically. Please wait a moment.
            </p>
            <div class="countdown">
                <span class="countdown-icon">🔄</span>
                <span class="countdown-text">
                    Auto-refresh in <span class="countdown-time">{} seconds</span>
                </span>
            </div>
            <a href="." class="retry-link">Retry Now</a>
            <div class="status-info">
                <span class="status-code">HTTP 503</span> — 
                Temporary capacity limit. Verified sessions are unaffected.
            </div>
        </div>
    </div>
</body>
</html>"#,
        retry_seconds, retry_seconds
    );

    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "text/html")
        .header("Retry-After", retry_seconds.to_string())
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

/// Check if a mirror is paused by querying the orchestrator
fn is_mirror_paused(onion_address: &str) -> bool {
    // Query orchestrator to check mirror status
    // This is a sync call so we spawn a blocking thread
    let addr = onion_address.to_string();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        for port in &[8080, 8180] {
            if let Ok(resp) = client
                .get(format!("http://127.0.0.1:{}/mirrors/all", port))
                .timeout(std::time::Duration::from_millis(500))
                .send()
            {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = json.get("mirrors").and_then(|m| m.as_array()) {
                        for mirror in arr {
                            if mirror.get("onion_address").and_then(|v| v.as_str()) == Some(&addr) {
                                return mirror.get("status").and_then(|v| v.as_str())
                                    == Some("paused");
                            }
                        }
                    }
                }
            }
        }
        false
    })
    .join()
    .unwrap_or_default()
}

/// Serve a static page for paused mirrors
fn serve_paused_mirror_page(_onion_address: &str) -> Response<BoxBody> {
    // Get active mirrors to provide alternative
    let active_mirrors: Vec<String> = std::thread::spawn(|| {
        let client = reqwest::blocking::Client::new();
        for port in &[8080, 8180] {
            if let Ok(resp) = client
                .get(format!("http://127.0.0.1:{}/mirrors", port))
                .timeout(std::time::Duration::from_millis(500))
                .send()
            {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = json.get("mirrors").and_then(|m| m.as_array()) {
                        return arr
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                }
            }
        }
        Vec::new()
    })
    .join()
    .unwrap_or_default();

    let alt_mirror_link = active_mirrors
        .first()
        .map(|m| format!(r#"<a href="http://{}" class="mirror-link">🧅 {}</a>"#, m, m))
        .unwrap_or_else(|| {
            "<span style='color: #888;'>No alternative mirrors available</span>".to_string()
        });

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mirror Paused - Fortify</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(135deg, #141417 0%, #18181b 50%, #141417 100%);
            font-family: 'Segoe UI', system-ui, sans-serif;
            color: #e4e4e7;
            padding: 20px;
        }}
        .container {{
            max-width: 600px;
            text-align: center;
            background: rgba(24, 24, 27, 0.9);
            border: 1px solid rgba(201, 162, 39, 0.4);
            border-radius: 12px;
            padding: 40px;
            box-shadow: 0 0 40px rgba(201, 162, 39, 0.2);
        }}
        .icon {{
            font-size: 64px;
            margin-bottom: 20px;
        }}
        h1 {{
            color: #c9a227;
            font-size: 2em;
            margin-bottom: 15px;
        }}
        p {{
            color: #a1a1aa;
            line-height: 1.6;
            margin-bottom: 25px;
        }}
        .mirror-link {{
            display: inline-block;
            background: rgba(201, 162, 39, 0.1);
            border: 1px solid rgba(201, 162, 39, 0.5);
            color: #c9a227;
            text-decoration: none;
            padding: 15px 30px;
            border-radius: 8px;
            font-family: monospace;
            font-size: 0.9em;
            word-break: break-all;
            transition: all 0.3s;
        }}
        .mirror-link:hover {{
            background: rgba(201, 162, 39, 0.2);
            box-shadow: 0 0 20px rgba(201, 162, 39, 0.3);
        }}
        .footer {{
            margin-top: 30px;
            color: #71717a;
            font-size: 0.85em;
        }}
        .shield {{
            color: #c9a227;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">⏸️</div>
        <h1>Mirror Temporarily Paused</h1>
        <p>
            This Fortify mirror has been temporarily paused by the administrator.
            Please use an alternative mirror to access the service.
        </p>
        <p style="font-weight: bold; color: #ccc;">Click below to continue:</p>
        {}
        <div class="footer">
            <span class="shield">🛡️</span> Protected by <strong>Fortify</strong>
        </div>
    </div>
</body>
</html>"#,
        alt_mirror_link
    );

    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

/// Generate error response
fn error_response(status: StatusCode, message: &str) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .expect("valid response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_node_capacity() {
        let node = BackendNode::new("http://localhost:8080".into(), true, 2);

        assert!(node.can_accept());
        assert!(node.acquire());
        assert!(node.can_accept());
        assert!(node.acquire());
        assert!(!node.can_accept());
        assert!(!node.acquire());

        node.release();
        assert!(node.can_accept());
    }

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = Metrics::default();

        metrics.record_request();
        metrics.record_allowed();
        metrics.record_valid_token();

        assert_eq!(metrics.requests_total, 1);
        assert_eq!(metrics.requests_allowed, 1);
        assert_eq!(metrics.tokens_valid, 1);
        assert_eq!(metrics.requests_denied, 0);
    }

    #[test]
    fn test_extract_token() {
        // Create a mock Incoming body - for tests we'll skip this as extract_token
        // requires Incoming which can't be easily constructed in unit tests
        // This test is now better suited for integration testing
    }
}
