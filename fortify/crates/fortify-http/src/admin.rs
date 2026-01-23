//! Admin Control Panel
//!
//! Secret admin panel for managing Fortify services.
//! All pages are pure HTML with forms - no JavaScript required.
//!
//! Theme: Retro Synthwave / Outrun with Fortification hints

use bytes::Bytes;
use fortify_core::{safe_read, safe_write, BehaviorConfig, BehaviorStats, KNOWN_ATTACK_PATHS};
use fortify_gate::{CaptchaConfig, CaptchaType};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{header, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Type alias for the response body type used throughout
type BoxBody = Full<Bytes>;

/// Secret admin panel path - random 32 char string
pub const ADMIN_PATH: &str = "/ctrl_8f7k3m9x2n4p1q6w5v0b8c";

/// Admin password (in production, use environment variable and proper hashing)
const ADMIN_PASSWORD: &str = "pleaseletmein123";

/// Authentication token header for orchestrator API calls
pub const AUTH_TOKEN_HEADER: &str = "X-Fortify-Admin-Token";

/// Generate authentication token from password
fn generate_auth_token(password: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// The authentication token used for internal API calls
pub fn get_auth_token() -> String {
    generate_auth_token(ADMIN_PASSWORD)
}

/// Admin state shared across all components
#[derive(Debug, Clone)]
pub struct AdminState {
    inner: Arc<RwLock<AdminStateInner>>,
}

#[derive(Debug, Clone, Default)]
struct AdminStateInner {
    sessions: HashMap<String, SessionInfo>,
    nodes: HashMap<String, NodeInfo>,
    mirrors: HashMap<String, MirrorInfo>,
    banned_sessions: Vec<String>,
    /// Override tiers set by admin - session_id -> forced tier
    tier_overrides: HashMap<String, String>,
    /// Behavioral analysis configuration
    behavior_config: BehaviorConfig,
    /// Captcha system configuration
    captcha_config: CaptchaConfig,
    /// CAPTCHA pool configuration
    captcha_pool_config: CaptchaPoolConfig,
    /// Branding configuration
    branding_config: BrandingConfig,
    /// Per-type CAPTCHA settings
    captcha_type_settings: Vec<CaptchaTypeSettings>,
    /// Per-session behavioral stats
    behavior_stats: HashMap<String, BehaviorStats>,
    /// Total traffic (bytes) through the system
    total_traffic_bytes: u64,
    /// Total requests through the system
    total_requests: u64,
    /// Time-series request log for aggregation (timestamp, bytes, node_id)
    request_log: Vec<(u64, u64, String)>,
    /// Users currently viewing the Gate/Fortify page (session_id -> timestamp)
    gate_queue: HashMap<String, u64>,
    /// Admin authenticated sessions (cookie -> timestamp)
    admin_sessions: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub trust_tier: String,
    pub request_count: u64,
    pub violation_count: u32,
    pub page_loads: u64,
    pub created_at: u64,
    pub last_activity: u64,
    pub browsing_history: Vec<HistoryEntry>,
    pub is_banned: bool,
    /// Behavioral analysis stats for this session
    #[serde(default)]
    pub behavior_stats: Option<BehaviorStats>,
    /// Number of times this session has been demoted and re-verified
    #[serde(default)]
    pub demotion_count: u32,
    /// Session marked as "killed" - repeat offender, permanently orphaned
    #[serde(default)]
    pub is_killed: bool,
    /// Current node this session is routed to
    #[serde(default)]
    pub current_node: String,
    /// Total bytes transferred for this session
    #[serde(default)]
    pub total_bytes: u64,
    /// Current mirror/onion address the user came through
    #[serde(default)]
    pub current_mirror: String,
}

/// Types of history events
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum HistoryEventType {
    /// Standard page request/navigation
    #[default]
    PageRequest,
    /// Admin manually changed session tier
    AdminTierChange,
    /// System auto-demoted session due to behavioral violations
    AutoDemotion,
    /// Session was banned
    SessionBanned,
    /// Session was unbanned
    SessionUnbanned,
    /// Session was killed (repeat offender)
    SessionKilled,
    /// Session passed captcha/verification
    CaptchaVerified,
    /// Behavioral violation detected
    ViolationDetected,
}

impl HistoryEventType {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::PageRequest => "📄",
            Self::AdminTierChange => "👮",
            Self::AutoDemotion => "⚠️",
            Self::SessionBanned => "🚫",
            Self::SessionUnbanned => "✅",
            Self::SessionKilled => "💀",
            Self::CaptchaVerified => "🔓",
            Self::ViolationDetected => "🚨",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::PageRequest => "",
            Self::AdminTierChange => "event-admin",
            Self::AutoDemotion => "event-warning",
            Self::SessionBanned => "event-danger",
            Self::SessionUnbanned => "event-success",
            Self::SessionKilled => "event-danger",
            Self::CaptchaVerified => "event-success",
            Self::ViolationDetected => "event-warning",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: u64,
    /// Event type determines how this entry is displayed
    #[serde(default)]
    pub event_type: HistoryEventType,
    /// For PageRequest: the requested path. For events: short description
    pub path: String,
    /// For PageRequest: HTTP method. For events: source (admin/system)
    pub method: String,
    /// For PageRequest: HTTP status code. For events: 0
    pub status_code: u16,
    /// Optional detailed reason/explanation for the event
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub bind_addr: String,
    pub onion_address: Option<String>, // Optional Tor onion address for this node
    pub mode: String,                  // "healthy" or "threat"
    pub status: String,
    pub created_at: u64,
    pub total_requests: u64,
    pub active_connections: usize,
    pub violations_detected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorInfo {
    pub id: String,
    pub onion_address: String,
    pub status: String,
    pub created_at: u64,
    pub total_requests: u64,
}

/// Branding configuration for the protected service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Display name/title for the service
    pub service_name: String,
    /// Short description
    pub description: String,
    /// Welcome message on CAPTCHA page
    pub welcome_message: String,
    /// Primary brand color (hex format: #RRGGBB)
    pub primary_color: String,
    /// Secondary/accent color (hex format: #RRGGBB)
    pub secondary_color: String,
    /// Tertiary/subtle accent color (hex format: #RRGGBB)
    pub tertiary_color: String,
    /// Custom CSS for gate pages (optional)
    pub custom_css: Option<String>,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            service_name: "Fortify".to_string(),
            description: "Protected Gateway".to_string(),
            welcome_message: "Complete verification to enter".to_string(),
            primary_color: "#c9a227".to_string(),
            secondary_color: "#a68b5b".to_string(),
            tertiary_color: "#8b7355".to_string(),
            custom_css: None,
        }
    }
}

/// CAPTCHA pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaPoolConfig {
    /// Target pool size
    pub pool_size: usize,
    /// Minimum pool before emergency generation
    pub min_pool_size: usize,
    /// Maximum pool size
    pub max_pool_size: usize,
    /// CAPTCHA difficulty (1-10)
    pub difficulty: u8,
    /// Time limit to solve in seconds
    pub timeout_seconds: u64,
    /// Maximum solve attempts
    pub max_attempts: u32,
    /// Rotate pool percentage
    pub rotation_percent: u8,
    /// Rotation interval in days
    pub rotation_interval_days: u32,
}

impl Default for CaptchaPoolConfig {
    fn default() -> Self {
        Self {
            pool_size: 500,
            min_pool_size: 100,
            max_pool_size: 1000,
            difficulty: 5,
            timeout_seconds: 120,
            max_attempts: 3,
            rotation_percent: 25,
            rotation_interval_days: 10,
        }
    }
}

/// Per-type CAPTCHA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaTypeSettings {
    /// CAPTCHA type name (e.g., "BmpText", "Emoji")
    pub type_name: String,
    /// Whether this type is enabled
    pub enabled: bool,
    /// Number of options to display (for selection-based)
    pub option_count: usize,
    /// Difficulty level (1-3)
    pub difficulty: u8,
    /// Minimum pool size for this type
    pub min_pool_size: usize,
}

impl CaptchaTypeSettings {
    pub fn new(type_name: &str) -> Self {
        let (option_count, difficulty) = match type_name {
            "BmpText" => (0, 2),
            "Emoji" => (6, 2),
            "Direction" => (4, 1),
            "Sequence" => (4, 2),
            "WordUnscramble" => (0, 2),
            "ImageRotation" => (4, 2),
            "Silhouette" => (4, 2),
            _ => (4, 2),
        };
        Self {
            type_name: type_name.to_string(),
            enabled: true,
            option_count,
            difficulty,
            min_pool_size: 50,
        }
    }

    pub fn all_types() -> Vec<Self> {
        vec![
            Self::new("BmpText"),
            Self::new("Emoji"),
            Self::new("Direction"),
            Self::new("Sequence"),
            Self::new("WordUnscramble"),
            Self::new("ImageRotation"),
            Self::new("Silhouette"),
        ]
    }
}

/// Exportable admin configuration for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfigExport {
    pub branding: BrandingConfig,
    pub captcha_pool: CaptchaPoolConfig,
    pub behavior: BehaviorConfig,
    pub captcha_type_settings: Vec<CaptchaTypeSettings>,
}

impl Default for AdminState {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminState {
    pub fn new() -> Self {
        // Initialize with per-type CAPTCHA settings
        let inner = AdminStateInner {
            captcha_type_settings: CaptchaTypeSettings::all_types(),
            ..Default::default()
        };
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    // Session management
    pub fn update_session(&self, info: SessionInfo) {
        let mut inner = safe_write(&self.inner);
        inner.sessions.insert(info.session_id.clone(), info);
    }

    pub fn get_sessions(&self) -> Vec<SessionInfo> {
        let inner = safe_read(&self.inner);
        inner.sessions.values().cloned().collect()
    }

    pub fn get_session(&self, id: &str) -> Option<SessionInfo> {
        let inner = safe_read(&self.inner);
        inner.sessions.get(id).cloned()
    }

    pub fn record_page_load(&self, session_id: &str, path: &str, method: &str, status: u16) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.page_loads += 1;
            session.last_activity = now();
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::PageRequest,
                path: path.to_string(),
                method: method.to_string(),
                status_code: status,
                reason: None,
            });
            // Keep last 100 entries
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
    }

    /// Record a session event (non-page-load) in history
    pub fn record_session_event(
        &self,
        session_id: &str,
        event_type: HistoryEventType,
        description: &str,
        source: &str,
        reason: Option<&str>,
    ) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.last_activity = now();
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type,
                path: description.to_string(),
                method: source.to_string(),
                status_code: 0,
                reason: reason.map(String::from),
            });
            // Keep last 100 entries
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
    }

    pub fn set_session_tier(&self, session_id: &str, tier: &str) {
        let mut inner = safe_write(&self.inner);
        let old_tier = inner.sessions.get(session_id).map(|s| s.trust_tier.clone());
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.trust_tier = tier.to_string();
        }
        // Store admin override so we can enforce it on subsequent requests
        inner
            .tier_overrides
            .insert(session_id.to_string(), tier.to_string());
        tracing::info!(
            "Admin override: session {} tier set to {}",
            session_id,
            tier
        );

        // Record in history (this is called from admin actions and auto-demotion)
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::AdminTierChange,
                path: format!("Tier changed to {}", tier),
                method: "Admin".to_string(),
                status_code: 0,
                reason: old_tier.map(|o| format!("Changed from {}", o)),
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
    }

    /// Set session tier from automatic system detection (not admin action)
    pub fn set_session_tier_auto(&self, session_id: &str, tier: &str, reason: &str) {
        let mut inner = safe_write(&self.inner);
        let old_tier = inner.sessions.get(session_id).map(|s| s.trust_tier.clone());
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.trust_tier = tier.to_string();
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::AutoDemotion,
                path: format!("Auto-demoted to {}", tier),
                method: "System".to_string(),
                status_code: 0,
                reason: Some(format!(
                    "{} (was: {})",
                    reason,
                    old_tier.unwrap_or_default()
                )),
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
        // Store override so we can enforce it on subsequent requests
        inner
            .tier_overrides
            .insert(session_id.to_string(), tier.to_string());
        tracing::info!(
            "Auto demotion: session {} tier set to {} - {}",
            session_id,
            tier,
            reason
        );
    }

    /// Check if session has an admin-forced tier override
    pub fn get_tier_override(&self, session_id: &str) -> Option<String> {
        let inner = safe_read(&self.inner);
        inner.tier_overrides.get(session_id).cloned()
    }

    /// Clear tier override for a session
    pub fn clear_tier_override(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.tier_overrides.remove(session_id);
    }

    pub fn ban_session(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.is_banned = true;
            session.trust_tier = "Burned".to_string();
            // Record event in history
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::SessionBanned,
                path: "Session BANNED".to_string(),
                method: "Admin".to_string(),
                status_code: 0,
                reason: Some("Manually banned by administrator".to_string()),
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
        if !inner.banned_sessions.contains(&session_id.to_string()) {
            inner.banned_sessions.push(session_id.to_string());
        }
    }

    pub fn unban_session(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.is_banned = false;
            // Record event in history
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::SessionUnbanned,
                path: "Session UNBANNED".to_string(),
                method: "Admin".to_string(),
                status_code: 0,
                reason: Some("Ban lifted by administrator".to_string()),
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
        inner.banned_sessions.retain(|s| s != session_id);
    }

    pub fn is_banned(&self, session_id: &str) -> bool {
        let inner = safe_read(&self.inner);
        inner.banned_sessions.contains(&session_id.to_string())
    }

    /// Increment demotion count for a session and check if it should be killed
    /// Returns true if session was killed (exceeded max demotions)
    pub fn record_demotion(&self, session_id: &str, max_demotions: u32) -> bool {
        self.record_demotion_with_reason(
            session_id,
            max_demotions,
            "Behavioral violations exceeded threshold",
        )
    }

    /// Record demotion with a specific reason for the history
    pub fn record_demotion_with_reason(
        &self,
        session_id: &str,
        max_demotions: u32,
        reason: &str,
    ) -> bool {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.demotion_count += 1;
            tracing::info!(
                "Session {} demotion count: {} / {}",
                session_id,
                session.demotion_count,
                max_demotions
            );

            // Record demotion event in history
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::AutoDemotion,
                path: format!("Demotion #{}/{}", session.demotion_count, max_demotions),
                method: "System".to_string(),
                status_code: 0,
                reason: Some(reason.to_string()),
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }

            if session.demotion_count >= max_demotions {
                session.is_killed = true;
                session.trust_tier = "Killed".to_string();
                // Record kill event
                session.browsing_history.push(HistoryEntry {
                    timestamp: now(),
                    event_type: HistoryEventType::SessionKilled,
                    path: "Session KILLED - Repeat Offender".to_string(),
                    method: "System".to_string(),
                    status_code: 0,
                    reason: Some(format!("Exceeded max demotions limit ({})", max_demotions)),
                });
                if session.browsing_history.len() > 100 {
                    session.browsing_history.remove(0);
                }
                tracing::warn!(
                    "Session {} KILLED - exceeded max demotions ({})",
                    session_id,
                    max_demotions
                );
                return true;
            }
        }
        false
    }

    /// Record a behavioral violation in the session history
    pub fn record_violation(
        &self,
        session_id: &str,
        violation_type: &str,
        details: &str,
        severity: u8,
    ) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::ViolationDetected,
                path: format!("[SEV-{}] {}", severity, violation_type),
                method: "System".to_string(),
                status_code: 0,
                reason: Some(details.to_string()),
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
    }

    /// Record successful captcha verification in session history
    pub fn record_captcha_verified(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.browsing_history.push(HistoryEntry {
                timestamp: now(),
                event_type: HistoryEventType::CaptchaVerified,
                path: "Captcha verification passed".to_string(),
                method: "Gate".to_string(),
                status_code: 0,
                reason: None,
            });
            if session.browsing_history.len() > 100 {
                session.browsing_history.remove(0);
            }
        }
    }

    /// Check if a session is killed (repeat offender)
    pub fn is_killed(&self, session_id: &str) -> bool {
        let inner = safe_read(&self.inner);
        inner
            .sessions
            .get(session_id)
            .map(|s| s.is_killed)
            .unwrap_or(false)
    }

    /// Get demotion count for a session
    pub fn get_demotion_count(&self, session_id: &str) -> u32 {
        let inner = safe_read(&self.inner);
        inner
            .sessions
            .get(session_id)
            .map(|s| s.demotion_count)
            .unwrap_or(0)
    }

    pub fn delete_session(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.sessions.remove(session_id);
    }

    // Node management
    pub fn update_node(&self, info: NodeInfo) {
        let mut inner = safe_write(&self.inner);
        inner.nodes.insert(info.id.clone(), info);
    }

    pub fn get_nodes(&self) -> Vec<NodeInfo> {
        let inner = safe_read(&self.inner);
        inner.nodes.values().cloned().collect()
    }

    pub fn get_node(&self, id: &str) -> Option<NodeInfo> {
        let inner = safe_read(&self.inner);
        inner.nodes.get(id).cloned()
    }

    pub fn set_node_mode(&self, id: &str, mode: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(node) = inner.nodes.get_mut(id) {
            node.mode = mode.to_string();
        }
    }

    pub fn remove_node(&self, id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.nodes.remove(id);
    }

    /// Record a request to a specific node (updates node stats and total traffic)
    pub fn record_node_request(&self, node_id: &str, bytes: u64) {
        let mut inner = safe_write(&self.inner);
        let timestamp = now();
        if let Some(node) = inner.nodes.get_mut(node_id) {
            node.total_requests += 1;
            node.active_connections = node.active_connections.saturating_add(1);
        }
        inner.total_requests += 1;
        inner.total_traffic_bytes += bytes;
        // Log for time-series aggregation
        inner
            .request_log
            .push((timestamp, bytes, node_id.to_string()));
        // Clean up old entries (keep last 7 days worth)
        let cutoff = timestamp.saturating_sub(7 * 24 * 60 * 60);
        inner.request_log.retain(|(ts, _, _)| *ts >= cutoff);
    }

    /// Record response traffic (adds bytes to total without incrementing request count)
    pub fn record_response_traffic(&self, bytes: u64) {
        let mut inner = safe_write(&self.inner);
        inner.total_traffic_bytes += bytes;
        // Add to time-series as well (with empty node_id to indicate response traffic)
        let timestamp = now();
        inner
            .request_log
            .push((timestamp, bytes, "_response".to_string()));
    }

    /// Release a connection from a node
    pub fn release_node_connection(&self, node_id: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(node) = inner.nodes.get_mut(node_id) {
            node.active_connections = node.active_connections.saturating_sub(1);
        }
    }

    /// Track a user entering the gate queue (viewing /Fortify page)
    pub fn enter_gate_queue(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.gate_queue.insert(session_id.to_string(), now());
        // Clean up stale entries (anyone not active in last 5 minutes)
        let cutoff = now().saturating_sub(300);
        inner.gate_queue.retain(|_, ts| *ts >= cutoff);
    }

    /// Remove a user from the gate queue (they completed verification or left)
    pub fn exit_gate_queue(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.gate_queue.remove(session_id);
    }

    /// Create admin authentication session
    pub fn create_admin_session(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.admin_sessions.insert(session_id.to_string(), now());
        // Clean old sessions (> 24 hours)
        let cutoff = now().saturating_sub(86400);
        inner.admin_sessions.retain(|_, ts| *ts >= cutoff);
    }

    /// Check if admin session is valid
    pub fn is_admin_session_valid(&self, session_id: &str) -> bool {
        let inner = safe_read(&self.inner);
        if let Some(&timestamp) = inner.admin_sessions.get(session_id) {
            // Session valid for 24 hours
            let cutoff = now().saturating_sub(86400);
            timestamp >= cutoff
        } else {
            false
        }
    }

    /// Remove admin session (logout)
    pub fn remove_admin_session(&self, session_id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.admin_sessions.remove(session_id);
    }

    /// Get current gate queue count (users viewing Fortify page)
    pub fn get_gate_queue_count(&self) -> usize {
        let mut inner = safe_write(&self.inner);
        // Clean up stale entries first
        let cutoff = now().saturating_sub(300);
        inner.gate_queue.retain(|_, ts| *ts >= cutoff);
        inner.gate_queue.len()
    }

    /// Get time-based statistics for nodes
    pub fn get_time_based_stats(&self) -> TimeBasedStats {
        let inner = safe_read(&self.inner);
        let current_time = now();

        // Define time windows in seconds
        let windows = [
            ("15min", 15 * 60),
            ("1hour", 60 * 60),
            ("4hours", 4 * 60 * 60),
            ("1day", 24 * 60 * 60),
            ("1week", 7 * 24 * 60 * 60),
            ("1month", 30 * 24 * 60 * 60),
        ];

        let mut result = TimeBasedStats {
            per_node: HashMap::new(),
            totals: HashMap::new(),
            gate_queue: inner.gate_queue.len(),
        };

        // Initialize totals for each window
        for (name, _) in &windows {
            result
                .totals
                .insert(name.to_string(), WindowStats::default());
        }

        // Initialize per-node stats
        for node in inner.nodes.values() {
            let mut node_windows = HashMap::new();
            for (name, _) in &windows {
                node_windows.insert(name.to_string(), WindowStats::default());
            }
            result.per_node.insert(
                node.id.clone(),
                NodeTimeStats {
                    node_id: node.id.clone(),
                    mode: node.mode.clone(),
                    status: node.status.clone(),
                    total_requests: node.total_requests,
                    active_connections: node.active_connections,
                    windows: node_windows,
                },
            );
        }

        // Aggregate from request log
        for (timestamp, bytes, node_id) in &inner.request_log {
            let age = current_time.saturating_sub(*timestamp);

            for (name, window_secs) in &windows {
                if age <= *window_secs {
                    // Update totals
                    if let Some(total) = result.totals.get_mut(*name) {
                        total.requests += 1;
                        total.bytes += bytes;
                    }

                    // Update per-node
                    if let Some(node_stats) = result.per_node.get_mut(node_id) {
                        if let Some(window) = node_stats.windows.get_mut(*name) {
                            window.requests += 1;
                            window.bytes += bytes;
                        }
                    }
                }
            }
        }

        // Count sessions by window (using last_activity)
        for session in inner.sessions.values() {
            let age = current_time.saturating_sub(session.last_activity);
            for (name, window_secs) in &windows {
                if age <= *window_secs {
                    if let Some(total) = result.totals.get_mut(*name) {
                        total.sessions += 1;
                    }
                }
            }
        }

        result
    }

    /// Update session's current node
    pub fn set_session_node(&self, session_id: &str, node_id: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.current_node = node_id.to_string();
        }
    }

    /// Update session's current mirror (onion address)
    pub fn set_session_mirror(&self, session_id: &str, mirror: &str) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.current_mirror = mirror.to_string();
        }
    }

    /// Record bytes transferred for a session
    pub fn record_session_traffic(&self, session_id: &str, bytes: u64) {
        let mut inner = safe_write(&self.inner);
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.total_bytes += bytes;
        }
    }

    /// Get next suggested node name and port for a given pool
    pub fn get_next_node_suggestion(&self, mode: &str) -> (String, String) {
        let inner = safe_read(&self.inner);

        // Count existing nodes in this pool
        let pool_count = inner.nodes.values().filter(|n| n.mode == mode).count();

        // Find highest port in use (default base ports: healthy=9100, threat=8081)
        let base_port = if mode == "healthy" { 9100 } else { 8081 };
        let max_port = inner
            .nodes
            .values()
            .filter(|n| n.mode == mode)
            .filter_map(|n| {
                n.bind_addr
                    .split(':')
                    .next_back()
                    .and_then(|p| p.parse::<u16>().ok())
            })
            .max()
            .unwrap_or(base_port - 1);

        let suggested_name = format!("{}-{}", mode, pool_count);
        let suggested_port = format!("127.0.0.1:{}", max_port + 1);

        (suggested_name, suggested_port)
    }

    /// Get traffic statistics
    pub fn get_traffic_stats(&self) -> TrafficStats {
        let inner = safe_read(&self.inner);

        let mut per_node: HashMap<String, NodeTrafficStats> = HashMap::new();
        for node in inner.nodes.values() {
            per_node.insert(
                node.id.clone(),
                NodeTrafficStats {
                    node_id: node.id.clone(),
                    mode: node.mode.clone(),
                    total_requests: node.total_requests,
                    active_connections: node.active_connections,
                },
            );
        }

        let mut per_session: Vec<SessionTrafficStats> = inner
            .sessions
            .values()
            .map(|s| SessionTrafficStats {
                session_id: s.session_id.clone(),
                trust_tier: s.trust_tier.clone(),
                request_count: s.request_count,
                total_bytes: s.total_bytes,
                current_node: s.current_node.clone(),
            })
            .collect();

        // Sort by request count descending
        per_session.sort_by(|a, b| b.request_count.cmp(&a.request_count));

        TrafficStats {
            total_requests: inner.total_requests,
            total_bytes: inner.total_traffic_bytes,
            per_node,
            per_session,
        }
    }

    // Mirror management
    pub fn update_mirror(&self, info: MirrorInfo) {
        let mut inner = safe_write(&self.inner);
        inner.mirrors.insert(info.id.clone(), info);
    }

    pub fn get_mirrors(&self) -> Vec<MirrorInfo> {
        let inner = safe_read(&self.inner);
        inner.mirrors.values().cloned().collect()
    }

    /// Get mirror by onion address
    pub fn get_mirror_by_onion(&self, onion_address: &str) -> Option<MirrorInfo> {
        let inner = safe_read(&self.inner);
        inner
            .mirrors
            .values()
            .find(|m| m.onion_address == onion_address)
            .cloned()
    }

    /// Record a request through a specific mirror (by onion address)
    /// Creates the mirror entry if it doesn't exist
    pub fn record_mirror_request(&self, onion_address: &str) {
        let mut inner = safe_write(&self.inner);
        // Try to find existing mirror by onion address
        let mirror_id = inner
            .mirrors
            .iter()
            .find(|(_, m)| m.onion_address == onion_address)
            .map(|(id, _)| id.clone());

        if let Some(id) = mirror_id {
            if let Some(mirror) = inner.mirrors.get_mut(&id) {
                mirror.total_requests += 1;
            }
        } else {
            // Create a new mirror entry with a generated ID
            let id = format!(
                "mirror-{}",
                onion_address.chars().take(8).collect::<String>()
            );
            inner.mirrors.insert(
                id.clone(),
                MirrorInfo {
                    id,
                    onion_address: onion_address.to_string(),
                    status: "active".to_string(),
                    created_at: now(),
                    total_requests: 1,
                },
            );
        }
    }

    pub fn remove_mirror(&self, id: &str) {
        let mut inner = safe_write(&self.inner);
        inner.mirrors.remove(id);
    }

    // Stats
    pub fn get_stats(&self) -> AdminStats {
        let inner = safe_read(&self.inner);
        let sessions = &inner.sessions;

        AdminStats {
            total_sessions: sessions.len(),
            active_sessions: sessions
                .values()
                .filter(|s| now() - s.last_activity < 300)
                .count(),
            banned_sessions: inner.banned_sessions.len(),
            healthy_nodes: inner.nodes.values().filter(|n| n.mode == "healthy").count(),
            threat_nodes: inner.nodes.values().filter(|n| n.mode == "threat").count(),
            active_mirrors: inner.mirrors.len(),
            total_requests: sessions.values().map(|s| s.request_count).sum(),
            total_violations: sessions.values().map(|s| s.violation_count as u64).sum(),
        }
    }

    // =========================================================================
    // BEHAVIORAL ANALYSIS METHODS
    // =========================================================================

    /// Get current behavioral config
    pub fn get_behavior_config(&self) -> BehaviorConfig {
        let inner = safe_read(&self.inner);
        inner.behavior_config.clone()
    }

    /// Update behavioral config
    pub fn update_behavior_config(&self, config: BehaviorConfig) {
        let mut inner = safe_write(&self.inner);
        inner.behavior_config = config;
        tracing::info!("Behavioral analysis config updated");
    }

    // =========================================================================
    // CAPTCHA CONFIGURATION METHODS
    // =========================================================================

    /// Get current captcha config
    pub fn get_captcha_config(&self) -> CaptchaConfig {
        let inner = safe_read(&self.inner);
        inner.captcha_config.clone()
    }

    /// Update captcha config
    pub fn update_captcha_config(&self, config: CaptchaConfig) {
        let mut inner = safe_write(&self.inner);
        inner.captcha_config = config;
        tracing::info!("Captcha config updated");
    }

    // =========================================================================
    // BRANDING CONFIGURATION METHODS
    // =========================================================================

    /// Get current branding config
    pub fn get_branding_config(&self) -> BrandingConfig {
        let inner = safe_read(&self.inner);
        inner.branding_config.clone()
    }

    /// Update branding config
    pub fn update_branding_config(&self, config: BrandingConfig) {
        let mut inner = safe_write(&self.inner);
        inner.branding_config = config;
        tracing::info!("Branding config updated");
    }

    // =========================================================================
    // CAPTCHA POOL CONFIGURATION METHODS
    // =========================================================================

    /// Get current captcha pool config
    pub fn get_captcha_pool_config(&self) -> CaptchaPoolConfig {
        let inner = safe_read(&self.inner);
        inner.captcha_pool_config.clone()
    }

    /// Update captcha pool config
    pub fn update_captcha_pool_config(&self, config: CaptchaPoolConfig) {
        let mut inner = safe_write(&self.inner);
        inner.captcha_pool_config = config;
        tracing::info!("Captcha pool config updated");
    }

    // =========================================================================
    // PER-TYPE CAPTCHA CONFIGURATION METHODS
    // =========================================================================

    /// Get all per-type CAPTCHA settings
    pub fn get_captcha_type_settings(&self) -> Vec<CaptchaTypeSettings> {
        let inner = safe_read(&self.inner);
        inner.captcha_type_settings.clone()
    }

    /// Update a specific CAPTCHA type's settings
    pub fn update_captcha_type_setting(
        &self,
        type_name: &str,
        enabled: bool,
        option_count: usize,
        difficulty: u8,
        min_pool_size: usize,
    ) {
        let mut inner = safe_write(&self.inner);
        if let Some(setting) = inner
            .captcha_type_settings
            .iter_mut()
            .find(|s| s.type_name == type_name)
        {
            setting.enabled = enabled;
            setting.option_count = option_count;
            setting.difficulty = difficulty.clamp(1, 3);
            setting.min_pool_size = min_pool_size;
            tracing::info!(
                "CAPTCHA type {} settings updated: enabled={}, difficulty={}",
                type_name,
                enabled,
                difficulty
            );
        }
    }

    // =========================================================================
    // CONFIGURATION PERSISTENCE METHODS
    // =========================================================================

    /// Save current admin state to a JSON file for persistence
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let inner = safe_read(&self.inner);
        let export = AdminConfigExport {
            branding: inner.branding_config.clone(),
            captcha_pool: inner.captcha_pool_config.clone(),
            behavior: inner.behavior_config.clone(),
            captcha_type_settings: inner.captcha_type_settings.clone(),
        };
        let json = serde_json::to_string_pretty(&export).map_err(std::io::Error::other)?;
        std::fs::write(path, json)?;
        tracing::info!("Admin config saved to {:?}", path);
        Ok(())
    }

    /// Load admin state from a JSON file
    pub fn load_from_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let export: AdminConfigExport =
            serde_json::from_str(&json).map_err(std::io::Error::other)?;

        let mut inner = safe_write(&self.inner);
        inner.branding_config = export.branding;
        inner.captcha_pool_config = export.captcha_pool;
        inner.behavior_config = export.behavior;
        inner.captcha_type_settings = export.captcha_type_settings;
        tracing::info!("Admin config loaded from {:?}", path);
        Ok(())
    }

    /// Reload configuration from the default path
    pub fn reload_config(&self) -> Result<(), std::io::Error> {
        let path = Self::default_config_path();
        if path.exists() {
            self.load_from_file(&path)
        } else {
            tracing::warn!("Config file not found at {:?}, using defaults", path);
            Ok(())
        }
    }

    /// Get default config file path
    pub fn default_config_path() -> std::path::PathBuf {
        std::path::PathBuf::from("/etc/fortify/admin-state.json")
    }

    /// Toggle a specific behavioral feature
    pub fn toggle_behavior_feature(&self, feature: &str, enabled: bool) {
        let mut inner = safe_write(&self.inner);
        match feature {
            "ua_analysis" => inner.behavior_config.ua_analysis_enabled = enabled,
            "referer_analysis" => inner.behavior_config.referer_analysis_enabled = enabled,
            "path_analysis" => inner.behavior_config.path_analysis_enabled = enabled,
            "enumeration_detection" => {
                inner.behavior_config.enumeration_detection_enabled = enabled
            }
            "form_tracking" => inner.behavior_config.form_tracking_enabled = enabled,
            "payload_analysis" => inner.behavior_config.payload_analysis_enabled = enabled,
            _ => {
                tracing::warn!("Unknown behavior feature: {}", feature);
            }
        }
        tracing::info!("Behavioral feature '{}' set to {}", feature, enabled);
    }

    /// Update behavioral stats for a session
    pub fn update_behavior_stats(&self, session_id: &str, stats: BehaviorStats) {
        let mut inner = safe_write(&self.inner);
        inner
            .behavior_stats
            .insert(session_id.to_string(), stats.clone());

        // Also update in session info if present
        if let Some(session) = inner.sessions.get_mut(session_id) {
            session.behavior_stats = Some(stats);
        }
    }

    /// Get behavioral stats for a session
    pub fn get_behavior_stats(&self, session_id: &str) -> Option<BehaviorStats> {
        let inner = safe_read(&self.inner);
        inner.behavior_stats.get(session_id).cloned()
    }

    /// Get aggregate behavioral stats across all sessions
    pub fn get_aggregate_behavior_stats(&self) -> AggregateBehaviorStats {
        let inner = safe_read(&self.inner);
        let mut agg = AggregateBehaviorStats::default();

        for stats in inner.behavior_stats.values() {
            agg.total_requests_analyzed += stats.requests_analyzed;
            agg.total_violations += stats.total_violations();

            for (vtype, count) in &stats.violations_by_type {
                *agg.violations_by_type.entry(vtype.clone()).or_insert(0) += count;
            }

            if stats.suspicious_ua_detected {
                agg.sessions_with_suspicious_ua += 1;
            }
        }

        agg.sessions_analyzed = inner.behavior_stats.len();
        agg
    }

    /// Check if behavioral analysis is enabled
    pub fn is_behavior_enabled(&self) -> bool {
        let inner = safe_read(&self.inner);
        // At least one feature must be enabled
        inner.behavior_config.ua_analysis_enabled
            || inner.behavior_config.referer_analysis_enabled
            || inner.behavior_config.path_analysis_enabled
            || inner.behavior_config.enumeration_detection_enabled
            || inner.behavior_config.form_tracking_enabled
            || inner.behavior_config.payload_analysis_enabled
    }
}

/// Aggregate behavioral stats across all sessions
#[derive(Debug, Clone, Default, Serialize)]
pub struct AggregateBehaviorStats {
    pub sessions_analyzed: usize,
    pub total_requests_analyzed: u64,
    pub total_violations: u64,
    pub violations_by_type: HashMap<String, u64>,
    pub sessions_with_suspicious_ua: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub banned_sessions: usize,
    pub healthy_nodes: usize,
    pub threat_nodes: usize,
    pub active_mirrors: usize,
    pub total_requests: u64,
    pub total_violations: u64,
}

/// Traffic statistics per node
#[derive(Debug, Clone, Serialize)]
pub struct NodeTrafficStats {
    pub node_id: String,
    pub mode: String,
    pub total_requests: u64,
    pub active_connections: usize,
}

/// Traffic statistics per session
#[derive(Debug, Clone, Serialize)]
pub struct SessionTrafficStats {
    pub session_id: String,
    pub trust_tier: String,
    pub request_count: u64,
    pub total_bytes: u64,
    pub current_node: String,
}

/// Overall traffic statistics
#[derive(Debug, Clone, Serialize)]
pub struct TrafficStats {
    pub total_requests: u64,
    pub total_bytes: u64,
    pub per_node: HashMap<String, NodeTrafficStats>,
    pub per_session: Vec<SessionTrafficStats>,
}

/// Stats for a time window
#[derive(Debug, Clone, Default, Serialize)]
pub struct WindowStats {
    pub requests: u64,
    pub bytes: u64,
    pub sessions: usize,
}

/// Per-node time-based stats
#[derive(Debug, Clone, Serialize)]
pub struct NodeTimeStats {
    pub node_id: String,
    pub mode: String,
    pub status: String,
    pub total_requests: u64,
    pub active_connections: usize,
    pub windows: HashMap<String, WindowStats>,
}

/// Time-based statistics for the whole system
#[derive(Debug, Clone, Serialize)]
pub struct TimeBasedStats {
    pub per_node: HashMap<String, NodeTimeStats>,
    pub totals: HashMap<String, WindowStats>,
    pub gate_queue: usize,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Check if request is for admin panel
pub fn is_admin_request(path: &str) -> bool {
    path.starts_with(ADMIN_PATH)
}

/// Check if request has valid authentication
fn is_authenticated(req: &Request<Incoming>, admin_state: &AdminState) -> bool {
    // Check for auth cookie
    if let Some(cookie_header) = req.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("fortify_admin_session=") {
                    let session_id = cookie.strip_prefix("fortify_admin_session=").unwrap_or("");
                    return admin_state.is_admin_session_valid(session_id);
                }
            }
        }
    }
    false
}

/// Handle admin panel request
pub async fn handle_admin_request(
    req: Request<Incoming>,
    admin_state: Arc<AdminState>,
) -> Response<BoxBody> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Route admin requests
    let sub_path = path.strip_prefix(ADMIN_PATH).unwrap_or("");

    // Public routes (no auth required)
    match (method.clone(), sub_path) {
        (Method::GET, "/login") => return render_login_page(None),
        (Method::POST, "/login") => return handle_login(req, admin_state).await,
        (Method::POST, "/logout") => return handle_logout(req, admin_state).await,
        _ => {}
    }

    // All other routes require authentication
    if !is_authenticated(&req, &admin_state) {
        return render_login_page(Some("Please log in to access the admin panel"));
    }

    match (method, sub_path) {
        // Dashboard
        (Method::GET, "" | "/") => render_dashboard(&admin_state),

        // Sessions
        (Method::GET, "/sessions") => render_sessions(&admin_state, req.uri()),
        (Method::GET, p) if p.starts_with("/session/") => {
            let id = p.strip_prefix("/session/").unwrap_or("");
            render_session_detail(&admin_state, id)
        }
        (Method::POST, "/session/action") => handle_session_action(req, admin_state).await,

        // Nodes
        (Method::GET, "/nodes") => render_nodes(&admin_state),
        (Method::POST, "/node/action") => handle_node_action(req, admin_state).await,

        // Mirrors
        (Method::GET, "/mirrors") => render_mirrors(&admin_state),
        (Method::POST, "/mirror/action") => handle_mirror_action(req, admin_state).await,

        // Behavioral Analysis Settings
        (Method::GET, "/settings") => render_settings(&admin_state),
        (Method::POST, "/settings/behavior") => handle_behavior_settings(req, admin_state).await,
        (Method::POST, "/settings/branding") => handle_branding_settings(req, admin_state).await,
        (Method::POST, "/settings/captcha") => handle_captcha_settings(req, admin_state).await,
        (Method::POST, "/settings/captcha-pool") => {
            handle_captcha_pool_settings(req, admin_state).await
        }
        (Method::POST, "/settings/captcha-type") => {
            handle_captcha_type_settings(req, admin_state).await
        }
        (Method::POST, "/config/save") => handle_config_save(admin_state).await,
        (Method::POST, "/config/reload") => handle_config_reload(admin_state).await,

        // Tutorial / Documentation
        (Method::GET, "/tutorial") => render_tutorial(),

        _ => not_found(),
    }
}

// ============================================================================
// AUTHENTICATION
// ============================================================================

fn render_login_page(error: Option<&str>) -> Response<BoxBody> {
    let error_html = error.map(|msg| format!(r#"<div style="background: var(--crimson); padding: 12px; border-radius: 4px; margin-bottom: 20px; color: white;">{}</div>"#, msg)).unwrap_or_default();

    let html = format!(
        r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Login - Fortify Control Panel</title>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        :root {{
            --bg-deep: #141417;
            --gold-primary: #c9a227;
            --gold-muted: #a68b5b;
            --text-primary: #f5f0e8;
            --text-secondary: #a8a4a0;
            --crimson: #c96969;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: 'Inter', -apple-system, sans-serif;
            background: var(--bg-deep);
            color: var(--text-primary);
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 20px;
        }}
        .login-container {{
            max-width: 400px;
            width: 100%;
            background: #1e1e23;
            padding: 40px;
            border-radius: 8px;
            border: 1px solid #3a3a42;
        }}
        h1 {{
            color: var(--gold-primary);
            margin-bottom: 10px;
            font-size: 24px;
        }}
        p {{
            color: var(--text-secondary);
            margin-bottom: 30px;
        }}
        label {{
            display: block;
            color: var(--text-secondary);
            margin-bottom: 8px;
            font-size: 14px;
        }}
        input {{
            width: 100%;
            padding: 12px;
            background: var(--bg-deep);
            border: 1px solid #3a3a42;
            border-radius: 4px;
            color: var(--text-primary);
            font-size: 14px;
            margin-bottom: 20px;
        }}
        input:focus {{
            outline: none;
            border-color: var(--gold-primary);
        }}
        button {{
            width: 100%;
            padding: 12px;
            background: var(--gold-primary);
            border: none;
            border-radius: 4px;
            color: var(--bg-deep);
            font-weight: 600;
            font-size: 14px;
            cursor: pointer;
            transition: background 0.2s;
        }}
        button:hover {{
            background: var(--gold-muted);
        }}
        .shield {{
            font-size: 48px;
            text-align: center;
            margin-bottom: 20px;
        }}
    </style>
</head>
<body>
    <div class="login-container">
        <div class="shield">🛡️</div>
        <h1>Fortify Control Panel</h1>
        <p>Enter password to access administrative functions</p>
        {error_html}
        <form method="POST" action="{ADMIN_PATH}/login">
            <label>Password</label>
            <input type="password" name="password" required autofocus>
            <button type="submit">Login</button>
        </form>
    </div>
</body>
</html>"##
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

async fn handle_login(req: Request<Incoming>, state: Arc<AdminState>) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    let password = params.get("password").map(|s| s.as_str()).unwrap_or("");

    if password == ADMIN_PASSWORD {
        // Generate session ID
        let session_id = uuid_v4();
        state.create_admin_session(&session_id);

        tracing::info!("✅ Admin login successful from control panel");

        // Set cookie and redirect to dashboard
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(header::LOCATION, ADMIN_PATH)
            .header(
                header::SET_COOKIE,
                format!(
                    "fortify_admin_session={}; Path={}; HttpOnly; Max-Age=86400",
                    session_id, ADMIN_PATH
                ),
            )
            .body(Full::new(Bytes::new()))
            .unwrap()
    } else {
        tracing::warn!("❌ Failed admin login attempt from control panel");
        render_login_page(Some("Invalid password"))
    }
}

async fn handle_logout(req: Request<Incoming>, state: Arc<AdminState>) -> Response<BoxBody> {
    // Remove session
    if let Some(cookie_header) = req.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("fortify_admin_session=") {
                    let session_id = cookie.strip_prefix("fortify_admin_session=").unwrap_or("");
                    state.remove_admin_session(session_id);
                }
            }
        }
    }

    tracing::info!("Admin logged out");

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, format!("{}/login", ADMIN_PATH))
        .header(
            header::SET_COOKIE,
            format!("fortify_admin_session=; Path={}; Max-Age=0", ADMIN_PATH),
        )
        .body(Full::new(Bytes::new()))
        .unwrap()
}

// ============================================================================
// HTML TEMPLATES
// ============================================================================

fn html_page(title: &str, content: &str) -> Response<BoxBody> {
    let html = format!(
        r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title} - Fortify Control Panel</title>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');

        :root {{
            --bg-deep: #141417;
            --bg-surface: #1e1e23;
            --bg-elevated: #26262d;
            --border-subtle: #3a3a42;
            --border-accent: #4a4a55;
            --gold-primary: #c9a227;
            --gold-muted: #a68b5b;
            --gold-light: #d4b85a;
            --bronze: #8b7355;
            --text-primary: #f5f0e8;
            --text-secondary: #a8a4a0;
            --text-muted: #6b6862;
            --sage: #9ab893;
            --amber: #e4bc5e;
            --crimson: #c96969;
            --slate-blue: #6b7c8c;
        }}

        * {{ box-sizing: border-box; margin: 0; padding: 0; }}

        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            background: var(--bg-deep);
            color: var(--text-primary);
            min-height: 100vh;
            overflow-x: hidden;
        }}

        body::before {{
            content: '';
            position: fixed;
            inset: 0;
            background:
                radial-gradient(ellipse at 50% 0%, rgba(201, 162, 39, 0.02) 0%, transparent 50%);
            pointer-events: none;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 24px;
            position: relative;
            z-index: 1;
        }}

        header {{
            text-align: center;
            padding: 32px 0;
            margin-bottom: 32px;
            border-bottom: 1px solid var(--border-subtle);
            position: relative;
        }}

        header::after {{
            content: '';
            position: absolute;
            bottom: -1px;
            left: 50%;
            transform: translateX(-50%);
            width: 120px;
            height: 1px;
            background: linear-gradient(90deg, transparent, var(--gold-primary), transparent);
        }}

        .logo {{
            font-size: 2rem;
            font-weight: 300;
            color: var(--text-primary);
            letter-spacing: 0.25em;
            text-transform: uppercase;
            margin-bottom: 8px;
        }}

        .logo-sub {{
            font-size: 0.75rem;
            color: var(--gold-muted);
            letter-spacing: 0.2em;
            text-transform: uppercase;
        }}

        .castle-icon {{
            font-size: 1.2em;
            margin: 0 12px;
            opacity: 0.8;
        }}

        nav {{
            margin-top: 28px;
            display: flex;
            justify-content: center;
            gap: 8px;
            flex-wrap: wrap;
        }}

        nav a {{
            color: var(--text-secondary);
            text-decoration: none;
            padding: 10px 20px;
            border: 1px solid var(--border-subtle);
            border-radius: 3px;
            background: var(--bg-surface);
            text-transform: uppercase;
            letter-spacing: 0.1em;
            font-size: 0.75rem;
            font-weight: 500;
            transition: all 0.2s ease;
        }}

        nav a:hover {{
            background: var(--bg-elevated);
            border-color: var(--gold-muted);
            color: var(--text-primary);
        }}

        nav a.active {{
            background: var(--gold-primary);
            border-color: var(--gold-primary);
            color: var(--bg-deep);
        }}

        h1 {{ font-size: 1.5rem; font-weight: 400; margin-bottom: 12px; color: var(--text-primary); letter-spacing: 0.05em; }}
        h2 {{
            font-size: 1rem;
            font-weight: 500;
            margin: 28px 0 18px;
            color: var(--gold-primary);
            text-transform: uppercase;
            letter-spacing: 0.1em;
            display: flex;
            align-items: center;
            gap: 12px;
        }}
        h2::before {{
            content: '';
            width: 3px;
            height: 16px;
            background: var(--gold-primary);
            border-radius: 1px;
        }}
        h3 {{
            font-size: 0.9rem;
            font-weight: 500;
            margin: 18px 0 14px;
            color: var(--gold-muted);
            letter-spacing: 0.05em;
        }}

        .card {{
            background: var(--bg-surface);
            border: 1px solid var(--border-subtle);
            border-radius: 4px;
            padding: 24px;
            margin-bottom: 24px;
            position: relative;
        }}

        .card::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 2px;
            background: linear-gradient(90deg, var(--gold-muted), var(--gold-primary), var(--gold-muted));
            border-radius: 4px 4px 0 0;
            opacity: 0.6;
        }}

        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
            gap: 16px;
            margin-bottom: 24px;
        }}

        .stat-box {{
            background: var(--bg-elevated);
            border: 1px solid var(--border-subtle);
            border-radius: 4px;
            padding: 18px;
            text-align: center;
        }}

        .stat-value {{
            font-size: 1.5rem;
            font-weight: 600;
            color: var(--text-primary);
        }}

        .stat-label {{
            font-size: 0.7rem;
            color: var(--gold-muted);
            margin-top: 6px;
            text-transform: uppercase;
            letter-spacing: 0.1em;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            background: var(--bg-surface);
            border-radius: 4px;
            overflow: hidden;
        }}

        th, td {{
            text-align: left;
            padding: 14px 16px;
            border-bottom: 1px solid var(--border-subtle);
        }}

        th {{
            background: var(--bg-elevated);
            color: var(--gold-muted);
            font-size: 0.7rem;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.1em;
        }}

        tr:hover {{ background: var(--bg-elevated); }}

        .btn {{
            font-family: 'Inter', sans-serif;
            background: transparent;
            color: var(--gold-primary);
            border: 1px solid var(--gold-primary);
            border-radius: 3px;
            padding: 8px 16px;
            cursor: pointer;
            font-size: 0.75rem;
            font-weight: 500;
            margin: 3px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            transition: all 0.2s ease;
            text-decoration: none;
            display: inline-block;
        }}

        .btn:hover {{
            background: var(--gold-primary);
            color: var(--bg-deep);
        }}

        .btn-danger {{ border-color: var(--crimson); color: var(--crimson); }}
        .btn-danger:hover {{ background: var(--crimson); color: var(--text-primary); }}

        .btn-warning {{ border-color: var(--amber); color: var(--amber); }}
        .btn-warning:hover {{ background: var(--amber); color: var(--bg-deep); }}

        .btn-success {{ border-color: var(--sage); color: var(--sage); }}
        .btn-success:hover {{ background: var(--sage); color: var(--bg-deep); }}

        input, select {{
            font-family: 'Inter', sans-serif;
            background: var(--bg-deep);
            border: 1px solid var(--border-subtle);
            border-radius: 3px;
            color: var(--text-primary);
            padding: 10px 14px;
            font-size: 0.9rem;
            margin: 5px;
            transition: all 0.2s ease;
        }}

        input:focus, select:focus {{
            border-color: var(--gold-muted);
            outline: none;
        }}

        .tier-verified {{ color: var(--sage); }}
        .tier-trusted {{ color: var(--gold-primary); }}
        .tier-suspicious {{ color: var(--amber); }}
        .tier-burned {{ color: var(--crimson); }}
        .tier-unknown {{ color: var(--text-muted); }}

        .status-online {{ color: var(--sage); }}
        .status-offline {{ color: var(--crimson); }}

        .mode-healthy {{ color: var(--sage); }}
        .mode-threat {{ color: var(--amber); }}

        .history-list {{
            max-height: 400px;
            overflow-y: auto;
            font-size: 0.85rem;
            background: var(--bg-deep);
            border: 1px solid var(--border-subtle);
            border-radius: 4px;
            padding: 12px;
        }}

        .history-item {{
            padding: 10px 12px;
            border-bottom: 1px solid var(--border-subtle);
            font-family: 'Inter', sans-serif;
        }}

        /* Event type styles for history entries */
        .history-item.event-admin {{
            background: rgba(201, 162, 39, 0.08);
            border-left: 3px solid var(--gold-primary);
        }}
        .history-item.event-warning {{
            background: rgba(212, 168, 75, 0.08);
            border-left: 3px solid var(--amber);
        }}
        .history-item.event-danger {{
            background: rgba(168, 84, 84, 0.08);
            border-left: 3px solid var(--crimson);
        }}
        .history-item.event-success {{
            background: rgba(125, 154, 120, 0.08);
            border-left: 3px solid var(--sage);
        }}

        .timestamp {{ color: var(--text-muted); font-size: 0.8rem; margin-right: 10px; }}

        form {{ display: inline; }}

        .section {{ margin-bottom: 40px; }}

        .actions {{ margin-top: 20px; }}

        pre {{
            background: var(--bg-deep);
            padding: 16px;
            overflow-x: auto;
            border: 1px solid var(--border-subtle);
            border-radius: 4px;
            color: var(--text-secondary);
            font-size: 0.85rem;
        }}

        code {{
            font-family: 'SF Mono', 'Monaco', 'Consolas', monospace;
            color: var(--gold-muted);
            background: var(--bg-elevated);
            padding: 2px 6px;
            border-radius: 2px;
        }}

        a {{ color: var(--gold-primary); text-decoration: none; }}
        a:hover {{ color: var(--gold-light); }}

        /* Scrollbar styling */
        ::-webkit-scrollbar {{
            width: 6px;
            height: 6px;
        }}
        ::-webkit-scrollbar-track {{
            background: var(--bg-deep);
        }}
        ::-webkit-scrollbar-thumb {{
            background: var(--border-accent);
            border-radius: 3px;
        }}
        ::-webkit-scrollbar-thumb:hover {{
            background: var(--gold-muted);
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo">FORTIFY</div>
            <div class="logo-sub">Control Citadel</div>
            <nav>
                <a href="{ADMIN_PATH}">Dashboard</a>
                <a href="{ADMIN_PATH}/sessions">Sessions</a>
                <a href="{ADMIN_PATH}/nodes">Nodes</a>
                <a href="{ADMIN_PATH}/mirrors">Mirrors</a>
                <a href="{ADMIN_PATH}/settings">Settings</a>
            </nav>
        </header>
        <main>
            {content}
        </main>
    </div>
</body>
</html>"##
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

fn render_dashboard(state: &AdminState) -> Response<BoxBody> {
    let stats = state.get_stats();
    let traffic_stats = state.get_traffic_stats();
    let time_stats = state.get_time_based_stats();
    let sessions = state.get_sessions();
    let recent_sessions: Vec<_> = sessions.iter().take(5).collect();

    let mut recent_html = String::new();
    for s in recent_sessions {
        let tier_class = format!("tier-{}", s.trust_tier.to_lowercase());
        recent_html.push_str(&format!(
            r#"<tr>
                <td><a href="{}/session/{}">{}</a></td>
                <td class="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
            ADMIN_PATH,
            s.session_id,
            &s.session_id[..8.min(s.session_id.len())],
            tier_class,
            s.trust_tier,
            s.page_loads,
            format_time_ago(s.last_activity)
        ));
    }

    // Build time-based stats table
    let windows = ["15min", "1hour", "4hours", "1day", "1week", "1month"];
    let mut time_stats_html = String::new();
    for window in &windows {
        if let Some(ws) = time_stats.totals.get(*window) {
            let label = match *window {
                "15min" => "15 Min",
                "1hour" => "1 Hour",
                "4hours" => "4 Hours",
                "1day" => "1 Day",
                "1week" => "1 Week",
                "1month" => "1 Month",
                _ => *window,
            };
            time_stats_html.push_str(&format!(
                r#"<tr>
                    <td>{}</td>
                    <td style="color: var(--amber);">{}</td>
                    <td>{}</td>
                    <td style="color: var(--gold-primary);">{}</td>
                </tr>"#,
                label,
                ws.requests,
                format_bytes(ws.bytes),
                ws.sessions
            ));
        }
    }

    let content = format!(
        r#"
        <h2>System Overview</h2>
        <div class="stats-grid">
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Sessions</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Active (5min)</div>
            </div>
            <div class="stat-box" style="border-color: var(--gold-muted);">
                <div class="stat-value" style="color: var(--gold-muted);">🚪 {}</div>
                <div class="stat-label">Gate Queue</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Banned</div>
            </div>
            <div class="stat-box">
                <div class="stat-value mode-healthy">{}</div>
                <div class="stat-label">Healthy Nodes</div>
            </div>
            <div class="stat-box">
                <div class="stat-value mode-threat">{}</div>
                <div class="stat-label">Threat Nodes</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Active Mirrors</div>
            </div>
            <div class="stat-box">
                <div class="stat-value" style="color: var(--amber);">{}</div>
                <div class="stat-label">Total Requests</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Traffic</div>
            </div>
            <div class="stat-box">
                <div class="stat-value" style="color: var(--crimson);">{}</div>
                <div class="stat-label">Violations</div>
            </div>
        </div>

        <div class="section">
            <h2>Traffic Over Time</h2>
            <table>
                <thead>
                    <tr>
                        <th>Period</th>
                        <th>Requests</th>
                        <th>Traffic</th>
                        <th>Active Sessions</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>

        <div class="section">
            <h2>Recent Sessions</h2>
            <table>
                <thead>
                    <tr>
                        <th>Session ID</th>
                        <th>Trust Tier</th>
                        <th>Page Loads</th>
                        <th>Last Active</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
            <a href="{}/sessions" class="btn">View All Sessions →</a>
        </div>
    "#,
        stats.total_sessions,
        stats.active_sessions,
        time_stats.gate_queue,
        stats.banned_sessions,
        stats.healthy_nodes,
        stats.threat_nodes,
        stats.active_mirrors,
        stats.total_requests,
        format_bytes(traffic_stats.total_bytes),
        stats.total_violations,
        time_stats_html,
        recent_html,
        ADMIN_PATH
    );

    html_page("Dashboard", &content)
}

/// Parse page number from query string (e.g., "?page=2")
fn parse_page_from_query(uri: &hyper::Uri) -> usize {
    uri.query()
        .and_then(|q| {
            q.split('&').find_map(|pair| {
                let mut parts = pair.split('=');
                if parts.next() == Some("page") {
                    parts.next()?.parse().ok()
                } else {
                    None
                }
            })
        })
        .unwrap_or(1)
        .max(1) // Minimum page 1
}

fn render_sessions(state: &AdminState, uri: &hyper::Uri) -> Response<BoxBody> {
    let mut sessions = state.get_sessions();
    let traffic_stats = state.get_traffic_stats();

    // Sort by last_activity descending (most recent first)
    sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

    // Pagination (50 per page)
    let per_page = 50;
    let total_pages = sessions.len().div_ceil(per_page);
    let page = parse_page_from_query(uri).min(total_pages.max(1)); // Clamp to valid range
    let start = (page - 1) * per_page;
    let end = start + per_page;
    let page_sessions = &sessions[start..end.min(sessions.len())];

    let mut rows = String::new();
    for s in page_sessions {
        let tier_class = format!("tier-{}", s.trust_tier.to_lowercase());
        let status_marker = if s.is_killed {
            "💀 "
        } else if s.is_banned {
            "🚫 "
        } else {
            ""
        };
        let demotion_display = if s.demotion_count > 0 {
            format!(
                "<span style='color: var(--amber);' title='Demotion cycles'>↻{}</span>",
                s.demotion_count
            )
        } else {
            "-".to_string()
        };
        let node_display = if s.current_node.is_empty() {
            "<span style='color: var(--text-muted);'>-</span>".to_string()
        } else {
            let node_class = if s.current_node.starts_with("healthy") {
                "mode-healthy"
            } else {
                "mode-threat"
            };
            format!("<span class='{}'>{}</span>", node_class, s.current_node)
        };
        let traffic_display = format_bytes(s.total_bytes);

        rows.push_str(&format!(
            r#"<tr>
                <td><a href="{}/session/{}">{}{}</a></td>
                <td class="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>
                    <form method="POST" action="{}/session/action">
                        <input type="hidden" name="session_id" value="{}">
                        <button type="submit" name="action" value="to_threat" class="btn btn-warning" title="Move to Threat Pool">⚠</button>
                        <button type="submit" name="action" value="to_healthy" class="btn btn-success" title="Move to Healthy Pool">✓</button>
                        <button type="submit" name="action" value="ban" class="btn btn-danger" title="Ban Session">🚫</button>
                    </form>
                </td>
            </tr>"#,
            ADMIN_PATH, s.session_id,
            status_marker, &s.session_id[..12.min(s.session_id.len())],
            tier_class, s.trust_tier,
            node_display,
            s.page_loads,
            s.violation_count,
            demotion_display,
            traffic_display,
            format_time_ago(s.last_activity),
            ADMIN_PATH,
            s.session_id
        ));
    }

    let content = format!(
        r#"
        <h2>Session Management</h2>

        <div class="stats-grid">
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Sessions</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Requests</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Traffic</div>
            </div>
        </div>

        <div style="margin: 20px 0; color: var(--text-secondary);">
            Showing {} - {} of {} sessions (sorted by most recent activity)
        </div>

        <table>
            <thead>
                <tr>
                    <th>Session ID</th>
                    <th>Trust Tier</th>
                    <th>Current Node</th>
                    <th>Page Loads</th>
                    <th>Violations</th>
                    <th>Demotions</th>
                    <th>Traffic</th>
                    <th>Last Active</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    "#,
        sessions.len(),
        traffic_stats.total_requests,
        format_bytes(traffic_stats.total_bytes),
        start + 1,
        end.min(sessions.len()),
        sessions.len(),
        rows
    );

    html_page("Sessions", &content)
}

fn render_session_detail(state: &AdminState, session_id: &str) -> Response<BoxBody> {
    let session = match state.get_session(session_id) {
        Some(s) => s,
        None => return not_found(),
    };

    let tier_class = format!("tier-{}", session.trust_tier.to_lowercase());

    let mut history_html = String::new();
    for entry in session.browsing_history.iter().rev().take(50) {
        // Render based on event type
        match entry.event_type {
            HistoryEventType::PageRequest => {
                let status_class = if entry.status_code < 400 {
                    "status-online"
                } else {
                    "status-offline"
                };
                history_html.push_str(&format!(
                    r#"<div class="history-item">
                        <span class="timestamp">{}</span>
                        <span class="{}">{}</span>
                        <strong>{}</strong> {}
                    </div>"#,
                    format_timestamp(entry.timestamp),
                    status_class,
                    entry.status_code,
                    entry.method,
                    entry.path
                ));
            }
            _ => {
                // Event entries (non-page requests)
                let event_class = entry.event_type.css_class();
                let icon = entry.event_type.icon();
                let reason_html = if let Some(ref reason) = entry.reason {
                    format!("<div style='color: var(--text-muted); font-size: 0.85em; margin-left: 32px;'>↳ {}</div>", reason)
                } else {
                    String::new()
                };
                history_html.push_str(&format!(
                    r#"<div class="history-item {}">
                        <span class="timestamp">{}</span>
                        <span style="font-size: 1.2em;">{}</span>
                        <strong>[{}]</strong> {}
                    </div>{}"#,
                    event_class,
                    format_timestamp(entry.timestamp),
                    icon,
                    entry.method,
                    entry.path,
                    reason_html
                ));
            }
        }
    }

    if history_html.is_empty() {
        history_html = "<p>No history recorded.</p>".to_string();
    }

    // Build behavioral analysis section
    let behavior_html = if let Some(ref bstats) = session.behavior_stats {
        let mut violations_rows = String::new();
        for (vtype, count) in &bstats.violations_by_type {
            let severity_class = if *count > 5 {
                "status-offline"
            } else if *count > 2 {
                "btn-warning"
            } else {
                ""
            };
            violations_rows.push_str(&format!(
                r#"<tr><td>{}</td><td class="{}">{}</td></tr>"#,
                vtype, severity_class, count
            ));
        }
        if violations_rows.is_empty() {
            violations_rows = r#"<tr><td colspan="2" style="color: var(--sage);">No behavioral violations detected ✓</td></tr>"#.to_string();
        }

        let mut recent_violations_html = String::new();
        for v in bstats.recent_violations.iter().rev().take(10) {
            let severity_class = match v.severity {
                3 => "status-offline",
                2 => "btn-warning",
                _ => "",
            };
            recent_violations_html.push_str(&format!(
                r#"<div class="history-item">
                    <span class="timestamp">{}</span>
                    <span class="{}">SEV-{}</span>
                    <strong>{}</strong>
                    <span style="color: var(--text-muted);">{}</span>
                </div>"#,
                format_timestamp(v.timestamp),
                severity_class,
                v.severity,
                v.violation_type.as_str(),
                v.details
            ));
        }
        if recent_violations_html.is_empty() {
            recent_violations_html =
                "<p style=\"color: var(--sage);\">No recent violations ✓</p>".to_string();
        }

        format!(
            r#"
        <div class="card" style="border-color: var(--amber);">
            <h3>Behavioral Analysis</h3>

            <div class="stats-grid">
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Requests Analyzed</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value {}">{}</div>
                    <div class="stat-label">Total Violations</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Unique Paths</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Form Submissions</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value {}">{}</div>
                    <div class="stat-label">Suspicious UA</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Severity Score</div>
                </div>
            </div>

            <h3>Violations by Type</h3>
            <table>
                <thead><tr><th>Violation Type</th><th>Count</th></tr></thead>
                <tbody>{}</tbody>
            </table>

            <h3>Recent Violations (Last 10)</h3>
            <div class="history-list" style="max-height: 250px;">
                {}
            </div>
        </div>
        "#,
            bstats.requests_analyzed,
            if bstats.total_violations() > 5 {
                "status-offline"
            } else if bstats.total_violations() > 0 {
                "btn-warning"
            } else {
                "status-online"
            },
            bstats.total_violations(),
            bstats.unique_paths_count,
            bstats.form_submissions,
            if bstats.suspicious_ua_detected {
                "status-offline"
            } else {
                "status-online"
            },
            if bstats.suspicious_ua_detected {
                "YES"
            } else {
                "NO"
            },
            bstats.severity_score(),
            violations_rows,
            recent_violations_html
        )
    } else {
        r#"
        <div class="card" style="border-color: var(--border-subtle); opacity: 0.7;">
            <h3>Behavioral Analysis</h3>
            <p style="color: var(--text-muted);">No behavioral data collected for this session yet.</p>
            <p style="color: var(--text-muted); font-size: 0.9em;">Behavioral analysis runs on each request. Data will appear as the session makes requests.</p>
        </div>
        "#.to_string()
    };

    // Format current node display
    let node_display = if session.current_node.is_empty() {
        "<span style='color: var(--text-muted);'>Not yet routed</span>".to_string()
    } else {
        let node_class = if session.current_node.starts_with("healthy") {
            "mode-healthy"
        } else {
            "mode-threat"
        };
        format!(
            "<span class='{}'>{}</span>",
            node_class, session.current_node
        )
    };

    // Format current mirror display (name + first 5 chars of onion address)
    let mirror_display = if session.current_mirror.is_empty() {
        "<span style='color: var(--text-muted);'>Direct / Local</span>".to_string()
    } else {
        // Extract first 5 chars of the onion address for display
        let onion_prefix = if session.current_mirror.len() >= 5 {
            &session.current_mirror[..5]
        } else {
            &session.current_mirror
        };
        format!("<span style='color: var(--gold-primary);'>🧅 {}</span> <code style='font-size: 0.9em;'>{}...</code>",
            "Mirror", onion_prefix)
    };

    let content = format!(
        r#"
        <h2>Session: {}</h2>

        <div class="card">
            <h3>Session Info</h3>
            <table>
                <tr><td>Session ID</td><td><code>{}</code></td></tr>
                <tr><td>Trust Tier</td><td class="{}">{}</td></tr>
                <tr><td>Current Node</td><td>{}</td></tr>
                <tr><td>Current Mirror</td><td>{}</td></tr>
                <tr><td>Page Loads</td><td>{}</td></tr>
                <tr><td>Request Count</td><td>{}</td></tr>
                <tr><td>Total Traffic</td><td>{}</td></tr>
                <tr><td>Violations</td><td>{}</td></tr>
                <tr><td>Demotion Count</td><td>{}</td></tr>
                <tr><td>Created</td><td>{}</td></tr>
                <tr><td>Last Activity</td><td>{}</td></tr>
                <tr><td>Banned</td><td>{}</td></tr>
                <tr><td>Killed</td><td>{}</td></tr>
            </table>

            <div class="actions">
                <form method="POST" action="{}/session/action">
                    <input type="hidden" name="session_id" value="{}">
                    <button type="submit" name="action" value="to_threat" class="btn btn-warning">Move to Threat Pool</button>
                    <button type="submit" name="action" value="to_healthy" class="btn btn-success">Move to Healthy Pool</button>
                    <button type="submit" name="action" value="ban" class="btn btn-danger">Ban Session</button>
                    <button type="submit" name="action" value="unban" class="btn">Unban</button>
                    <button type="submit" name="action" value="delete" class="btn btn-danger">Delete Session</button>
                </form>
            </div>
        </div>

        {}

        <div class="card">
            <h3>📜 Session History (Last 50)</h3>
            <p style="color: var(--text-muted); font-size: 0.85em; margin-bottom: 15px;">Page requests, tier changes, demotions, bans, and system events</p>
            <div class="history-list">
                {}
            </div>
        </div>

        <a href="{}/sessions" class="btn">← Back to Sessions</a>
    "#,
        &session.session_id[..12.min(session.session_id.len())],
        session.session_id,
        tier_class,
        session.trust_tier,
        node_display,
        mirror_display,
        session.page_loads,
        session.request_count,
        format_bytes(session.total_bytes),
        session.violation_count,
        session.demotion_count,
        format_timestamp(session.created_at),
        format_timestamp(session.last_activity),
        if session.is_banned { "Yes" } else { "No" },
        if session.is_killed {
            "<span style='color: var(--crimson);'>YES - REPEAT OFFENDER</span>"
        } else {
            "No"
        },
        ADMIN_PATH,
        session.session_id,
        behavior_html,
        history_html,
        ADMIN_PATH
    );

    html_page("Session Detail", &content)
}

fn render_nodes(state: &AdminState) -> Response<BoxBody> {
    let mut nodes = state.get_nodes();
    let traffic_stats = state.get_traffic_stats();
    let time_stats = state.get_time_based_stats();

    // Sort nodes by total_requests descending (busiest first)
    nodes.sort_by(|a, b| b.total_requests.cmp(&a.total_requests));

    let healthy: Vec<_> = nodes.iter().filter(|n| n.mode == "healthy").collect();
    let threat: Vec<_> = nodes.iter().filter(|n| n.mode == "threat").collect();

    // Calculate total requests across all nodes
    let total_node_requests: u64 = nodes.iter().map(|n| n.total_requests).sum();
    let total_connections: usize = nodes.iter().map(|n| n.active_connections).sum();

    // Build time-based stats summary at the top
    let windows = ["15min", "1hour", "4hours", "1day", "1week", "1month"];
    let mut time_summary_html = String::new();
    for (node_id, node_stats) in &time_stats.per_node {
        let mut window_cells = String::new();
        for window in &windows {
            if let Some(ws) = node_stats.windows.get(*window) {
                window_cells.push_str(&format!(
                    "<td>{} / {}</td>",
                    ws.requests,
                    format_bytes(ws.bytes)
                ));
            } else {
                window_cells.push_str("<td>-</td>");
            }
        }
        let mode_class = if node_stats.mode == "healthy" {
            "mode-healthy"
        } else {
            "mode-threat"
        };
        time_summary_html.push_str(&format!(
            r#"<tr>
                <td><strong class="{}">{}</strong></td>
                <td class="{}">{}</td>
                {}
            </tr>"#,
            mode_class, node_id, mode_class, node_stats.mode, window_cells
        ));
    }

    // System totals row
    let mut total_window_cells = String::new();
    for window in &windows {
        if let Some(ws) = time_stats.totals.get(*window) {
            total_window_cells.push_str(&format!(
                "<td><strong>{} / {}</strong></td>",
                ws.requests,
                format_bytes(ws.bytes)
            ));
        } else {
            total_window_cells.push_str("<td>-</td>");
        }
    }

    let render_node_table = |nodes: &[&NodeInfo], pool_name: &str| -> String {
        let mut rows = String::new();
        for n in nodes {
            let status_class = if n.status == "online" {
                "status-online"
            } else {
                "status-offline"
            };
            let age = format_time_ago(n.created_at);

            // Build address display with both local and onion if available
            let local_url = format!(
                "<code style='color: var(--gold-primary); background: var(--bg-elevated); padding: 2px 6px;'>{}</code>",
                n.bind_addr
            );
            let url_display = if let Some(ref onion) = n.onion_address {
                format!(
                    r#"{}<br>
                    <code style='color: var(--gold-muted); background: var(--bg-elevated); padding: 2px 6px; font-size: 0.85em;'>🧅 {}</code>"#,
                    local_url, onion
                )
            } else {
                local_url
            };

            rows.push_str(&format!(
                r#"<tr>
                    <td><strong style="color: var(--gold-primary);">{}</strong></td>
                    <td>{}</td>
                    <td class="{}">{}</td>
                    <td>{}</td>
                    <td style="color: var(--amber); font-weight: bold;">{}</td>
                    <td style="color: var(--sage);">{}</td>
                    <td>
                        <form method="POST" action="{}/node/action">
                            <input type="hidden" name="node_id" value="{}">
                            <select name="action">
                                <option value="">Actions...</option>
                                <option value="to_healthy">→ Healthy</option>
                                <option value="to_threat">→ Threat</option>
                                <option value="remove">Remove</option>
                            </select>
                            <button type="submit" class="btn">Go</button>
                        </form>
                    </td>
                </tr>"#,
                n.id,
                url_display,
                status_class,
                n.status,
                age,
                n.total_requests,
                n.active_connections,
                ADMIN_PATH,
                n.id
            ));
        }

        // Calculate pool stats
        let pool_requests: u64 = nodes.iter().map(|n| n.total_requests).sum();
        let pool_connections: usize = nodes.iter().map(|n| n.active_connections).sum();

        format!(
            r#"
            <h3>{} Pool ({} nodes) - {} requests, {} active</h3>
            <table>
                <thead>
                    <tr>
                        <th>Node ID</th>
                        <th>Addresses (Local / Onion)</th>
                        <th>Status</th>
                        <th>Age</th>
                        <th>Total Requests</th>
                        <th>Active Conn.</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        "#,
            pool_name,
            nodes.len(),
            pool_requests,
            pool_connections,
            rows
        )
    };

    // Get auto-suggestions for new nodes
    let (suggested_healthy_name, suggested_healthy_url) = state.get_next_node_suggestion("healthy");
    let (suggested_threat_name, suggested_threat_url) = state.get_next_node_suggestion("threat");

    let content = format!(
        r#"
        <h2>Node Management</h2>

        <div class="stats-grid">
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Nodes</div>
            </div>
            <div class="stat-box">
                <div class="stat-value mode-healthy">{}</div>
                <div class="stat-label">Healthy Pool</div>
            </div>
            <div class="stat-box">
                <div class="stat-value mode-threat">{}</div>
                <div class="stat-label">Threat Pool</div>
            </div>
            <div class="stat-box">
                <div class="stat-value" style="color: var(--amber);">{}</div>
                <div class="stat-label">Total Requests</div>
            </div>
            <div class="stat-box">
                <div class="stat-value" style="color: var(--sage);">{}</div>
                <div class="stat-label">Active Conn.</div>
            </div>
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">System Traffic</div>
            </div>
        </div>

        <div class="card">
            <h3>+ Add New Node</h3>
            <p style="color: var(--text-muted); margin-bottom: 15px;">
                Add a new node to handle traffic. Fields are pre-filled with suggested values.
            </p>
            <form method="POST" action="{}/node/action" id="addNodeForm">
                <input type="hidden" name="action" value="add">
                <div style="display: grid; grid-template-columns: 150px 200px 250px 150px auto; gap: 10px; align-items: center;">
                    <input type="text" name="node_name" id="nodeName" value="{}" placeholder="Node Name" style="width: 100%;">
                    <input type="text" name="bind_addr" id="nodeAddr" value="{}" placeholder="127.0.0.1:9102" required style="width: 100%;">
                    <input type="text" name="onion_address" id="nodeOnion" placeholder="abc123...xyz.onion (optional)" style="width: 100%;">
                    <select name="mode" id="nodeMode" onchange="updateSuggestion()">
                        <option value="healthy">Healthy Pool</option>
                        <option value="threat">Threat Pool</option>
                    </select>
                    <button type="submit" class="btn btn-success">Add Node</button>
                </div>
            </form>
            <p style="color: var(--text-muted); font-size: 0.8em; margin-top: 10px;">
                Suggested next: <strong>Healthy</strong> = <code>{}</code> @ <code>{}</code> |
                <strong>Threat</strong> = <code>{}</code> @ <code>{}</code>
            </p>
        </div>

        <div class="section">
            {}
        </div>

        <div class="section">
            {}
        </div>

        <div class="card" style="margin-top: 20px;">
            <h3>Node Traffic by Time Window</h3>
            <table class="data-table" style="width: 100%; margin-top: 10px;">
                <thead>
                    <tr>
                        <th>Node</th>
                        <th>Mode</th>
                        <th>15 min</th>
                        <th>1 hour</th>
                        <th>4 hours</th>
                        <th>1 day</th>
                        <th>1 week</th>
                        <th>1 month</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                    <tr style="font-weight: bold; background: rgba(201,162,39,0.1);">
                        <td colspan="2">TOTAL</td>
                        {}
                    </tr>
                </tbody>
            </table>
            <p style="color: var(--text-muted); font-size: 0.8em; margin-top: 10px;">
                Format: requests / traffic. Updated on page refresh.
            </p>
        </div>
    "#,
        nodes.len(),
        healthy.len(),
        threat.len(),
        total_node_requests,
        total_connections,
        format_bytes(traffic_stats.total_bytes),
        ADMIN_PATH,
        suggested_healthy_name,
        suggested_healthy_url,
        suggested_healthy_name,
        suggested_healthy_url,
        suggested_threat_name,
        suggested_threat_url,
        render_node_table(&healthy, "Healthy"),
        render_node_table(&threat, "Threat"),
        time_summary_html,
        total_window_cells
    );

    html_page("Nodes", &content)
}

fn render_mirrors(state: &AdminState) -> Response<BoxBody> {
    // Fetch ALL mirrors with extended info (PoW, standby status) from orchestrator
    #[derive(Debug)]
    struct OrchestratorMirror {
        id: String,
        onion_address: String,
        status: String,
        pow_enabled: bool,
        is_standby: bool,
    }

    let orchestrator_mirrors: Vec<OrchestratorMirror> = std::thread::spawn(|| {
        let client = reqwest::blocking::Client::new();
        // Try both orchestrator ports - use /mirrors/extended for full info
        for port in &[8080, 8180] {
            if let Ok(resp) = client
                .get(format!("http://127.0.0.1:{}/mirrors/extended", port))
                .timeout(std::time::Duration::from_secs(2))
                .send()
            {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = json.get("mirrors").and_then(|m| m.as_array()) {
                        return arr
                            .iter()
                            .filter_map(|v| {
                                let id = v.get("id")?.as_str()?.to_string();
                                let onion_address = v.get("onion_address")?.as_str()?.to_string();
                                let status = v.get("status")?.as_str()?.to_string();
                                let pow_enabled = v
                                    .get("pow_enabled")
                                    .and_then(|p| p.as_bool())
                                    .unwrap_or(false);
                                let is_standby = v
                                    .get("is_standby")
                                    .and_then(|s| s.as_bool())
                                    .unwrap_or(false);
                                Some(OrchestratorMirror {
                                    id,
                                    onion_address,
                                    status,
                                    pow_enabled,
                                    is_standby,
                                })
                            })
                            .collect();
                    }
                }
            }
        }
        Vec::new()
    })
    .join()
    .unwrap_or_default();

    // Get local mirror stats (created_at, total_requests)
    let local_mirrors = state.get_mirrors();

    let active_count = orchestrator_mirrors
        .iter()
        .filter(|m| m.status == "active")
        .count();
    let _paused_count = orchestrator_mirrors
        .iter()
        .filter(|m| m.status == "paused" && !m.is_standby)
        .count();
    let standby_count = orchestrator_mirrors.iter().filter(|m| m.is_standby).count();
    let pow_count = orchestrator_mirrors
        .iter()
        .filter(|m| m.pow_enabled)
        .count();

    let current_time = now();

    let mut rows = String::new();
    for mirror in orchestrator_mirrors.iter() {
        // Find local stats for this mirror by onion address
        let local_stats = local_mirrors
            .iter()
            .find(|lm| lm.onion_address == mirror.onion_address);

        // Format age (time since first seen or created)
        let age_display = if let Some(stats) = local_stats {
            let age_secs = current_time.saturating_sub(stats.created_at);
            format_duration(age_secs)
        } else {
            "-".to_string()
        };

        // Format request count
        let requests_display = if let Some(stats) = local_stats {
            format_number(stats.total_requests)
        } else {
            "0".to_string()
        };

        // PoW badge
        let pow_badge = if mirror.pow_enabled {
            r#"<span style="background: rgba(125,154,120,0.2); color: var(--sage); padding: 2px 6px; font-size: 0.65em; border-radius: 3px; margin-left: 5px;">PoW</span>"#
        } else {
            ""
        };

        // Standby badge
        let standby_badge = if mirror.is_standby {
            r#"<span style="background: rgba(212,168,75,0.2); color: var(--amber); padding: 2px 6px; font-size: 0.65em; border-radius: 3px; margin-left: 5px;">STANDBY</span>"#
        } else {
            ""
        };

        let (status_class, status_text, action_buttons) = match (
            mirror.status.as_str(),
            mirror.is_standby,
        ) {
            ("active", _) => (
                "status-online",
                format!("🟢 Active{}{}", pow_badge, standby_badge),
                format!(
                    r#"<a href="http://{}" target="_blank" class="btn" style="font-size: 0.8em;">Visit</a>
                    <form method="POST" action="{}/mirror/action" style="display: inline; margin: 0;">
                        <input type="hidden" name="action" value="pause">
                        <input type="hidden" name="mirror_id" value="{}">
                        <input type="hidden" name="onion_address" value="{}">
                        <button type="submit" class="btn" style="background: rgba(212,168,75,0.2); border-color: var(--amber); color: var(--amber); font-size: 0.8em;" onclick="return confirm('PAUSE this mirror? Users will be redirected to other mirrors.');">⏸ Pause</button>
                    </form>
                    <form method="POST" action="{}/mirror/action" style="display: inline; margin: 0;">
                        <input type="hidden" name="action" value="destroy">
                        <input type="hidden" name="mirror_id" value="{}">
                        <input type="hidden" name="onion_address" value="{}">
                        <button type="submit" class="btn btn-danger" style="font-size: 0.8em;" onclick="return confirm('DESTROY this mirror? This will permanently remove it.');">🔥 Destroy</button>
                    </form>"#,
                    mirror.onion_address,
                    ADMIN_PATH,
                    mirror.id,
                    mirror.onion_address,
                    ADMIN_PATH,
                    mirror.id,
                    mirror.onion_address
                ),
            ),
            ("paused", true) => (
                "status-threat",
                format!("⏸️ Standby{}{}", pow_badge, standby_badge),
                format!(
                    r#"<form method="POST" action="{}/mirror/action" style="display: inline; margin: 0;">
                        <input type="hidden" name="action" value="activate">
                        <input type="hidden" name="mirror_id" value="{}">
                        <input type="hidden" name="onion_address" value="{}">
                        <button type="submit" class="btn btn-success" style="font-size: 0.8em;">🚀 Activate</button>
                    </form>
                    <form method="POST" action="{}/mirror/action" style="display: inline; margin: 0;">
                        <input type="hidden" name="action" value="destroy">
                        <input type="hidden" name="mirror_id" value="{}">
                        <input type="hidden" name="onion_address" value="{}">
                        <button type="submit" class="btn btn-danger" style="font-size: 0.8em;" onclick="return confirm('DESTROY this standby mirror?');">🔥 Destroy</button>
                    </form>"#,
                    ADMIN_PATH,
                    mirror.id,
                    mirror.onion_address,
                    ADMIN_PATH,
                    mirror.id,
                    mirror.onion_address
                ),
            ),
            ("paused", false) => (
                "status-threat",
                format!("⏸️ Paused{}", pow_badge),
                format!(
                    r#"<form method="POST" action="{}/mirror/action" style="display: inline; margin: 0;">
                        <input type="hidden" name="action" value="resume">
                        <input type="hidden" name="mirror_id" value="{}">
                        <input type="hidden" name="onion_address" value="{}">
                        <button type="submit" class="btn btn-success" style="font-size: 0.8em;">▶️ Resume</button>
                    </form>
                    <form method="POST" action="{}/mirror/action" style="display: inline; margin: 0;">
                        <input type="hidden" name="action" value="destroy">
                        <input type="hidden" name="mirror_id" value="{}">
                        <input type="hidden" name="onion_address" value="{}">
                        <button type="submit" class="btn btn-danger" style="font-size: 0.8em;" onclick="return confirm('DESTROY this mirror? This will permanently remove it.');">🔥 Destroy</button>
                    </form>"#,
                    ADMIN_PATH,
                    mirror.id,
                    mirror.onion_address,
                    ADMIN_PATH,
                    mirror.id,
                    mirror.onion_address
                ),
            ),
            _ => ("status-unknown", mirror.status.clone(), String::new()),
        };

        rows.push_str(&format!(
            r#"<tr>
                <td>{}</td>
                <td><code>{}</code></td>
                <td class="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td style="display: flex; gap: 5px; flex-wrap: wrap;">{}</td>
            </tr>"#,
            mirror.id,
            mirror.onion_address,
            status_class,
            status_text,
            age_display,
            requests_display,
            action_buttons
        ));
    }

    if rows.is_empty() {
        rows = r#"<tr><td colspan="6" style="text-align: center; color: var(--text-muted);">No mirrors found. Orchestrator may still be starting up...</td></tr>"#.to_string();
    }

    let content = format!(
        r#"
        <h2>Mirror Management</h2>

        <div class="stats-grid">
            <div class="stat-box">
                <div class="stat-value">{}</div>
                <div class="stat-label">Active Mirrors</div>
            </div>
            <div class="stat-box" style="border-color: var(--amber);">
                <div class="stat-value" style="color: var(--amber);">{}</div>
                <div class="stat-label">Standby Mirrors</div>
            </div>
            <div class="stat-box" style="border-color: var(--sage);">
                <div class="stat-value" style="color: var(--sage);">{}</div>
                <div class="stat-label">PoW Enabled</div>
            </div>
            <div class="stat-box" style="border-color: var(--gold-muted);">
                <div class="stat-value" style="color: var(--gold-muted);">{}</div>
                <div class="stat-label">Total Mirrors</div>
            </div>
        </div>

        <div class="card" style="background: var(--bg-elevated); border-color: var(--gold-muted);">
            <h3>How Mirrors Work</h3>
            <p style="color: var(--text-secondary); line-height: 1.6;">
                Mirrors are managed automatically by the <strong>Orchestrator</strong> service.
                The orchestrator maintains a pool of rotating .onion addresses that point to this Fortify instance.
            </p>
            <p style="color: var(--text-secondary); line-height: 1.6; margin-top: 10px;">
                <strong>Pausing</strong> a mirror shows users a redirect page directing them to active mirrors.
                <strong>Destroying</strong> a mirror permanently removes it from Tor.
            </p>
            <p style="margin-top: 15px;">
                <a href="http://127.0.0.1:8080/status" target="_blank" class="btn">View Orchestrator Status</a>
            </p>
        </div>

        <div class="card">
            <h3>+ Create New Mirror</h3>
            <p style="color: var(--text-muted); margin-bottom: 15px;">
                Request the orchestrator to spawn a new .onion mirror. This creates a new Tor hidden service.
            </p>
            <div style="display: flex; gap: 10px; flex-wrap: wrap;">
                <form method="POST" action="{}/mirror/action">
                    <input type="hidden" name="action" value="create">
                    <button type="submit" class="btn btn-success">🧅 Create Active Mirror</button>
                </form>
                <form method="POST" action="{}/mirror/action">
                    <input type="hidden" name="action" value="create_standby">
                    <button type="submit" class="btn" style="background: rgba(212,168,75,0.1); border-color: var(--amber); color: var(--amber);">🛡️ Create Standby Mirror</button>
                </form>
            </div>
            <p style="color: var(--text-muted); font-size: 0.85em; margin-top: 10px;">
                <strong>Active</strong> mirrors serve traffic immediately. <strong>Standby</strong> mirrors are paused but ready for instant activation.
            </p>
        </div>

        <div class="section">
            <h3>All Mirrors</h3>
            <p style="color: var(--text-muted); margin-bottom: 15px;">These .onion addresses route through Tor to this Fortify instance:</p>
            <table>
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Onion Address</th>
                        <th>Status</th>
                        <th>Age</th>
                        <th>Requests</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    "#,
        active_count,
        standby_count,
        pow_count,
        orchestrator_mirrors.len(),
        ADMIN_PATH,
        ADMIN_PATH,
        rows
    );

    html_page("Mirrors", &content)
}

fn render_settings(state: &AdminState) -> Response<BoxBody> {
    let config = state.get_behavior_config();
    let captcha_config = state.get_captcha_config();
    let captcha_pool_config = state.get_captcha_pool_config();
    let branding_config = state.get_branding_config();
    let captcha_type_settings = state.get_captcha_type_settings();
    let agg_stats = state.get_aggregate_behavior_stats();

    let checkbox = |name: &str, label: &str, enabled: bool| -> String {
        let checked = if enabled { "checked" } else { "" };
        format!(
            r#"
            <div style="display: flex; align-items: center; gap: 15px; padding: 12px; border-bottom: 1px solid var(--border-subtle);">
                <input type="checkbox" name="{}" value="1" {} id="{}" style="width: 20px; height: 20px; accent-color: var(--gold-primary);">
                <label for="{}" style="cursor: pointer; flex: 1;">{}</label>
                <span style="color: {};">{}</span>
            </div>
        "#,
            name,
            checked,
            name,
            name,
            label,
            if enabled {
                "var(--sage)"
            } else {
                "var(--crimson)"
            },
            if enabled { "ACTIVE" } else { "DISABLED" }
        )
    };

    let mut violations_breakdown = String::new();
    for (vtype, count) in &agg_stats.violations_by_type {
        violations_breakdown.push_str(&format!(r#"<tr><td>{}</td><td>{}</td></tr>"#, vtype, count));
    }
    if violations_breakdown.is_empty() {
        violations_breakdown = r#"<tr><td colspan="2" style="color: var(--text-muted);">No violations detected yet</td></tr>"#.to_string();
    }

    // Build per-type CAPTCHA settings forms
    let mut captcha_type_forms = String::new();
    for type_setting in &captcha_type_settings {
        let enabled_checked = if type_setting.enabled { "checked" } else { "" };
        let status_color = if type_setting.enabled {
            "var(--sage)"
        } else {
            "var(--crimson)"
        };
        let status_text = if type_setting.enabled {
            "ENABLED"
        } else {
            "DISABLED"
        };

        captcha_type_forms.push_str(&format!(
            r#"
            <form method="POST" action="{}/settings/captcha-type" style="border: 1px solid var(--border-subtle); padding: 15px; margin-bottom: 10px; border-radius: 8px;">
                <input type="hidden" name="type_name" value="{}">
                <div style="display: flex; align-items: center; gap: 15px; margin-bottom: 10px;">
                    <input type="checkbox" name="enabled" value="1" {} style="width: 20px; height: 20px; accent-color: var(--gold-primary);">
                    <strong style="flex: 1; color: var(--text-primary);">{}</strong>
                    <span style="color: {}; font-size: 0.85em;">{}</span>
                </div>
                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px;">
                    <div>
                        <label style="display: block; color: var(--text-muted); font-size: 0.75em;">Options</label>
                        <input type="number" name="option_count" value="{}" min="0" max="10" style="width: 100%; padding: 5px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                    </div>
                    <div>
                        <label style="display: block; color: var(--text-muted); font-size: 0.75em;">Difficulty (1-3)</label>
                        <input type="number" name="difficulty" value="{}" min="1" max="3" style="width: 100%; padding: 5px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                    </div>
                    <div>
                        <label style="display: block; color: var(--text-muted); font-size: 0.75em;">Min Pool</label>
                        <input type="number" name="min_pool_size" value="{}" min="0" max="500" style="width: 100%; padding: 5px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                    </div>
                </div>
                <button type="submit" class="btn btn-success" style="margin-top: 10px; padding: 5px 15px; font-size: 0.85em;">Save</button>
            </form>
            "#,
            ADMIN_PATH,
            type_setting.type_name,
            enabled_checked,
            type_setting.type_name,
            status_color,
            status_text,
            type_setting.option_count,
            type_setting.difficulty,
            type_setting.min_pool_size
        ));
    }

    // Build attack path toggles - grouped by category
    let mut attack_path_rows = String::new();
    let categories = [
        ("traversal", "Path Traversal"),
        ("config", "Config Files"),
        ("vcs", "Version Control"),
        ("cms", "CMS Probes"),
        ("admin", "Admin Panels"),
        ("sensitive", "Sensitive Files"),
        ("debug", "Debug Endpoints"),
        ("exploit", "Exploit Attempts"),
    ];

    for (cat_id, cat_label) in &categories {
        let paths_in_cat: Vec<_> = KNOWN_ATTACK_PATHS
            .iter()
            .filter(|(_, _, cat)| cat == cat_id)
            .collect();

        if !paths_in_cat.is_empty() {
            attack_path_rows.push_str(&format!(
                r#"<div style="margin-top: 15px; margin-bottom: 5px; color: var(--gold-primary); font-weight: bold;">{}</div>"#,
                cat_label
            ));

            for (pattern, desc, _) in paths_in_cat {
                let is_enabled = !config.disabled_attack_paths.contains(*pattern);
                let checked = if is_enabled { "checked" } else { "" };
                let field_name = pattern.replace(['/', '.', '\\'], "_");
                attack_path_rows.push_str(&format!(r#"
                    <div style="display: flex; align-items: center; gap: 10px; padding: 6px 10px; border-bottom: 1px solid var(--border-subtle);">
                        <input type="checkbox" name="attack_path_{}" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <code style="color: var(--amber); background: var(--bg-elevated); padding: 2px 6px;">{}</code>
                        <span style="color: var(--text-muted); font-size: 0.9em;">{}</span>
                        <span style="margin-left: auto; color: {}; font-size: 0.8em;">{}</span>
                    </div>
                "#,
                    field_name,
                    checked,
                    html_escape(pattern),
                    desc,
                    if is_enabled { "var(--sage)" } else { "var(--crimson)" },
                    if is_enabled { "DETECTING" } else { "DISABLED" }
                ));
            }
        }
    }

    // Custom whitelist paths textarea
    let custom_whitelist_str = config.custom_whitelist_paths.join("\n");

    // Build violation type thresholds rows
    let mut threshold_rows = String::new();
    let violation_types = [
        "Attack Path Access",
        "Suspicious User-Agent",
        "Path Enumeration",
        "Resource Enumeration",
        "Form Submission Flood",
        "Automated Behavior",
        "Suspicious Referer",
        "Oversized Payload",
        "Undersized Payload",
    ];
    for vtype in &violation_types {
        let threshold = config
            .violation_type_thresholds
            .get(*vtype)
            .copied()
            .unwrap_or(5);
        let field_name = vtype.to_lowercase().replace([' ', '-'], "_");
        threshold_rows.push_str(&format!(r#"
            <div style="display: flex; justify-content: space-between; align-items: center; padding: 8px 0; border-bottom: 1px solid var(--border-subtle);">
                <span style="color: var(--text-secondary);">{}</span>
                <input type="number" name="threshold_{}" value="{}" min="1" max="100" style="width: 80px; text-align: center;">
            </div>
        "#, vtype, field_name, threshold));
    }

    let content = format!(
        r#"
        <h2>Settings</h2>

        <div class="card" style="background: var(--bg-elevated); border-color: var(--gold-primary); margin-bottom: 20px;">
            <h3>Need Help?</h3>
            <p style="color: var(--text-secondary); margin-bottom: 15px;">
                New to Fortify? Check out our comprehensive tutorial that explains every feature and setting.
            </p>
            <a href="{}/tutorial" class="btn" style="background: var(--gold-primary); color: var(--bg-deep); font-weight: bold;">
                View Tutorial &amp; Documentation
            </a>
        </div>

        <div class="card" style="border-color: var(--amber);">
            <h3>Behavioral Analysis Engine</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Toggle individual detection modules on/off. Changes take effect immediately for new requests.
            </p>

            <form method="POST" action="{}/settings/behavior">
                <div style="background: var(--bg-deep); border: 1px solid var(--border-subtle); margin-bottom: 20px;">
                    {}
                    {}
                    {}
                    {}
                    {}
                    {}
                </div>

                <h3>Detection Thresholds</h3>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Max Unique Paths/Min</label>
                        <input type="number" name="max_unique_paths_per_minute" value="{}" style="width: 100%;">
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Max Form Submissions/Min</label>
                        <input type="number" name="max_form_submissions_per_minute" value="{}" style="width: 100%;">
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Max Payload Size (bytes)</label>
                        <input type="number" name="max_payload_size" value="{}" style="width: 100%;">
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Sequential Path Threshold</label>
                        <input type="number" name="sequential_path_threshold" value="{}" style="width: 100%;">
                    </div>
                </div>

                <h3>Threat Node Demotion Thresholds</h3>
                <p style="color: var(--text-muted); margin-bottom: 15px;">
                    Configure when sessions get automatically demoted to the threat node pool.
                </p>
                <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--crimson); margin-bottom: 5px;">Total Violations Threshold</label>
                        <input type="number" name="threat_demotion_threshold" value="{}" min="1" style="width: 100%;">
                        <small style="color: var(--text-muted);">Demote after this many total violations</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--crimson); margin-bottom: 5px;">Severity Score Threshold</label>
                        <input type="number" name="threat_severity_threshold" value="{}" min="1" style="width: 100%;">
                        <small style="color: var(--text-muted);">Demote when severity score reaches this</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--crimson); margin-bottom: 5px;">Max Demotions Before Kill</label>
                        <input type="number" name="max_demotions_before_kill" value="{}" min="1" style="width: 100%;">
                        <small style="color: var(--text-muted);">Kill session after N demotion cycles</small>
                    </div>
                </div>

                <h4 style="color: var(--amber); margin-top: 20px;">Per-Violation Type Thresholds</h4>
                <p style="color: var(--text-muted); margin-bottom: 10px;">
                    Demote to threat node when a specific violation type count reaches these limits:
                </p>
                <div style="background: var(--bg-deep); border: 1px solid var(--border-subtle); padding: 15px; margin-bottom: 20px;">
                    {}
                </div>

                <h3>Attack Path Detection</h3>
                <p style="color: var(--text-muted); margin-bottom: 10px;">
                    Toggle detection for each attack path pattern. Disable patterns that may conflict with your hidden service's legitimate paths.
                </p>
                <div style="background: var(--bg-deep); border: 1px solid var(--border-subtle); padding: 15px; margin-bottom: 20px; max-height: 400px; overflow-y: auto;">
                    {}
                </div>

                <h3>Custom Path Whitelist</h3>
                <p style="color: var(--text-muted); margin-bottom: 10px;">
                    Additional paths to whitelist. One per line. Use * for prefix matching (e.g., /my-app/*).
                </p>
                <textarea name="custom_whitelist_paths" rows="4" style="width: 100%; font-family: monospace; margin-bottom: 20px;" placeholder="/my-custom-path&#10;/my-api/*">{}</textarea>

                <button type="submit" class="btn btn-success">Save All Settings</button>
            </form>
        </div>

        <div class="card">
            <h3>Aggregate Behavioral Stats</h3>
            <div class="stats-grid">
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Sessions Analyzed</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Requests Analyzed</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Total Violations</div>
                </div>
                <div class="stat-box">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Suspicious UAs</div>
                </div>
            </div>

            <h3>Violations by Type (All Sessions)</h3>
            <table>
                <thead><tr><th>Violation Type</th><th>Count</th></tr></thead>
                <tbody>{}</tbody>
            </table>
        </div>

        <div class="card" style="background: var(--bg-elevated); border-color: var(--gold-muted);">
            <h3>About Behavioral Analysis</h3>
            <p style="color: var(--text-secondary); line-height: 1.6;">
                The Behavioral Analysis Engine monitors request patterns to detect automated threats,
                scrapers, and attack attempts <strong>without using JavaScript</strong>.
            </p>
            <p style="color: var(--text-secondary); line-height: 1.6; margin-top: 10px;">
                <strong>Detection modules:</strong>
            </p>
            <ul style="color: var(--text-secondary); margin: 10px 0 10px 20px; line-height: 1.8;">
                <li><strong>User-Agent Analysis</strong> - Detects non-Tor browser and bot User-Agents</li>
                <li><strong>Referer Analysis</strong> - Flags suspicious or impossible referer headers</li>
                <li><strong>Path Analysis</strong> - Detects attack paths (../, .env, admin probes)</li>
                <li><strong>Enumeration Detection</strong> - Catches rapid unique path scanning</li>
                <li><strong>Form Tracking</strong> - Monitors form submission flood attempts</li>
                <li><strong>Payload Analysis</strong> - Flags abnormal request body sizes</li>
            </ul>
            <p style="color: var(--text-muted); font-size: 0.9em; margin-top: 15px;">
                All detection is Tor Browser "safest mode" compatible. Missing referers and standardized UAs are treated as normal.
            </p>
        </div>

        <!-- BRANDING CONFIGURATION -->
        <div class="card" style="border-color: var(--gold-primary); background: var(--bg-elevated);">
            <h3>🏰 Branding Configuration</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Customize the appearance of your Gate and CAPTCHA pages.
            </p>

            <form method="POST" action="{}/settings/branding">
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px; font-weight: bold;">Service Name</label>
                        <input type="text" name="service_name" value="{}" maxlength="100" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Displayed as the main title</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px; font-weight: bold;">Description</label>
                        <input type="text" name="description" value="{}" maxlength="200" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Shown below the title</small>
                    </div>
                </div>

                <div style="margin-bottom: 20px;">
                    <label style="display: block; color: var(--gold-primary); margin-bottom: 5px; font-weight: bold;">Welcome Message</label>
                    <textarea name="welcome_message" rows="2" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">{}</textarea>
                    <small style="color: var(--text-muted);">Instructions shown on the CAPTCHA page</small>
                </div>

                <h4 style="color: var(--amber); margin-bottom: 15px;">Color Scheme</h4>
                <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Primary Color</label>
                        <div style="display: flex; gap: 10px; align-items: center;">
                            <input type="color" name="primary_color_picker" value="{}" style="width: 50px; height: 35px; border: none; cursor: pointer;">
                            <input type="text" name="primary_color" value="{}" maxlength="7" style="flex: 1; padding: 8px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary); font-family: monospace;">
                        </div>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Secondary Color</label>
                        <div style="display: flex; gap: 10px; align-items: center;">
                            <input type="color" name="secondary_color_picker" value="{}" style="width: 50px; height: 35px; border: none; cursor: pointer;">
                            <input type="text" name="secondary_color" value="{}" maxlength="7" style="flex: 1; padding: 8px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary); font-family: monospace;">
                        </div>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Tertiary Color</label>
                        <div style="display: flex; gap: 10px; align-items: center;">
                            <input type="color" name="tertiary_color_picker" value="{}" style="width: 50px; height: 35px; border: none; cursor: pointer;">
                            <input type="text" name="tertiary_color" value="{}" maxlength="7" style="flex: 1; padding: 8px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary); font-family: monospace;">
                        </div>
                    </div>
                </div>

                <div style="margin-bottom: 20px;">
                    <label style="display: block; color: var(--gold-primary); margin-bottom: 5px; font-weight: bold;">Custom CSS (Advanced)</label>
                    <textarea name="custom_css" rows="4" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary); font-family: monospace; font-size: 0.9em;" placeholder="/* Additional CSS rules */">{}</textarea>
                    <small style="color: var(--text-muted);">Optional custom styles injected into Gate/CAPTCHA pages</small>
                </div>

                <button type="submit" class="btn btn-success">Save Branding Settings</button>
            </form>
        </div>

        <!-- CAPTCHA CONFIGURATION -->
        <div class="card" style="border-color: var(--sage); background: var(--bg-elevated);">
            <h3>Captcha Configuration</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Configure captcha types used at the Gate (new visitors) and for Threat verification (demoted sessions).
            </p>

            <form method="POST" action="{}/settings/captcha">
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 25px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 10px; font-weight: bold;">Gate Captcha Type</label>
                        <p style="color: var(--text-muted); font-size: 0.85em; margin-bottom: 10px;">Captcha shown to new visitors at the gate.</p>
                        <select name="gate_captcha_type" style="width: 100%; padding: 12px; background: var(--bg-deep); border: 1px solid var(--gold-primary); color: var(--sage);">
                            {}
                        </select>
                    </div>
                    <div>
                        <label style="display: block; color: var(--crimson); margin-bottom: 10px; font-weight: bold;">Threat Captcha Type</label>
                        <p style="color: var(--text-muted); font-size: 0.85em; margin-bottom: 10px;">Captcha shown when users are demoted to threat mode.</p>
                        <select name="threat_captcha_type" style="width: 100%; padding: 12px; background: var(--bg-deep); border: 1px solid var(--crimson); color: var(--sage);">
                            {}
                        </select>
                    </div>
                </div>

                <div style="background: var(--bg-deep); border: 1px solid var(--border-subtle); padding: 15px; margin-bottom: 20px;">
                    <div style="display: flex; align-items: center; gap: 15px; padding: 12px; border-bottom: 1px solid var(--border-subtle);">
                        <input type="checkbox" name="threat_captcha_enabled" value="1" {} id="threat_captcha_enabled" style="width: 20px; height: 20px; accent-color: var(--gold-primary);">
                        <label for="threat_captcha_enabled" style="cursor: pointer; flex: 1;">Enable separate Threat captcha type</label>
                        <span style="color: {};">{}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 15px; padding: 12px;">
                        <input type="checkbox" name="random_cycling" value="1" {} id="random_cycling" style="width: 20px; height: 20px; accent-color: var(--gold-primary);">
                        <label for="random_cycling" style="cursor: pointer; flex: 1;">Enable random captcha cycling</label>
                        <span style="color: {};">{}</span>
                    </div>
                </div>

                <h4 style="color: var(--amber); margin-bottom: 15px;">Captcha Types for Cycling</h4>
                <p style="color: var(--text-muted); font-size: 0.85em; margin-bottom: 15px;">When random cycling is enabled, select which captcha types to cycle through:</p>
                <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 10px; margin-bottom: 20px;">
                    <div style="background: var(--bg-deep); padding: 10px; border: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 10px;">
                        <input type="checkbox" name="cycle_BmpText" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <div>
                            <strong style="color: var(--gold-primary);">Text Image</strong>
                            <p style="color: var(--text-muted); font-size: 0.75em; margin: 0;">Type characters from BMP</p>
                        </div>
                    </div>
                    <div style="background: var(--bg-deep); padding: 10px; border: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 10px;">
                        <input type="checkbox" name="cycle_Emoji" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <div>
                            <strong style="color: var(--gold-primary);">Emoji</strong>
                            <p style="color: var(--text-muted); font-size: 0.75em; margin: 0;">Click matching emoji</p>
                        </div>
                    </div>
                    <div style="background: var(--bg-deep); padding: 10px; border: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 10px;">
                        <input type="checkbox" name="cycle_Direction" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <div>
                            <strong style="color: var(--gold-primary);">Direction</strong>
                            <p style="color: var(--text-muted); font-size: 0.75em; margin: 0;">Click correct arrow</p>
                        </div>
                    </div>
                    <div style="background: var(--bg-deep); padding: 10px; border: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 10px;">
                        <input type="checkbox" name="cycle_Sequence" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <div>
                            <strong style="color: var(--gold-primary);">Sequence</strong>
                            <p style="color: var(--text-muted); font-size: 0.75em; margin: 0;">Complete the pattern</p>
                        </div>
                    </div>
                    <div style="background: var(--bg-deep); padding: 10px; border: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 10px;">
                        <input type="checkbox" name="cycle_WordUnscramble" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <div>
                            <strong style="color: var(--gold-primary);">Word</strong>
                            <p style="color: var(--text-muted); font-size: 0.75em; margin: 0;">Unscramble letters</p>
                        </div>
                    </div>
                    <div style="background: var(--bg-deep); padding: 10px; border: 1px solid var(--border-subtle); display: flex; align-items: center; gap: 10px;">
                        <input type="checkbox" name="cycle_Silhouette" value="1" {} style="width: 18px; height: 18px; accent-color: var(--gold-primary);">
                        <div>
                            <strong style="color: var(--gold-primary);">Silhouette</strong>
                            <p style="color: var(--text-muted); font-size: 0.75em; margin: 0;">Identify category</p>
                        </div>
                    </div>
                </div>

                <button type="submit" class="btn btn-success">Save Captcha Settings</button>
            </form>
        </div>

        <!-- CAPTCHA POOL CONFIGURATION -->
        <div class="card" style="border-color: var(--amber); background: var(--bg-elevated);">
            <h3>Captcha Pool Settings</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Configure the CAPTCHA pool size, difficulty, and behavior settings.
            </p>

            <form method="POST" action="{}/settings/captcha-pool">
                <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Target Pool Size</label>
                        <input type="number" name="pool_size" value="{}" min="50" max="5000" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Target CAPTCHAs to maintain</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Min Pool Size</label>
                        <input type="number" name="min_pool_size" value="{}" min="10" max="1000" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Emergency generation trigger</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Max Pool Size</label>
                        <input type="number" name="max_pool_size" value="{}" min="100" max="10000" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Maximum pool capacity</small>
                    </div>
                </div>

                <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Difficulty (1-10)</label>
                        <input type="number" name="difficulty" value="{}" min="1" max="10" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Visual complexity</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Timeout (seconds)</label>
                        <input type="number" name="timeout_seconds" value="{}" min="30" max="600" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Time to solve CAPTCHA</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Max Attempts</label>
                        <input type="number" name="max_attempts" value="{}" min="1" max="10" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Attempts before failure</small>
                    </div>
                </div>

                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Rotation Percent</label>
                        <input type="number" name="rotation_percent" value="{}" min="0" max="100" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">% of pool to refresh each cycle</small>
                    </div>
                    <div>
                        <label style="display: block; color: var(--gold-primary); margin-bottom: 5px;">Rotation Interval (days)</label>
                        <input type="number" name="rotation_interval_days" value="{}" min="1" max="90" style="width: 100%; padding: 10px; background: var(--bg-deep); border: 1px solid var(--border-subtle); color: var(--text-primary);">
                        <small style="color: var(--text-muted);">Days between pool rotations</small>
                    </div>
                </div>

                <button type="submit" class="btn btn-success">Save Pool Settings</button>
            </form>
        </div>

        <!-- PER-TYPE CAPTCHA CONFIGURATION -->
        <div class="card" style="border-color: var(--amber); background: var(--bg-elevated);">
            <h3>Per-Type CAPTCHA Settings</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Configure individual CAPTCHA types - enable/disable, difficulty, and pool allocation.
            </p>

            {}
        </div>

        <!-- CONFIGURATION MANAGEMENT -->
        <div class="card" style="border-color: var(--sage); background: var(--bg-elevated);">
            <h3>Configuration Management</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Save settings to disk or reload from the configuration file.
            </p>
            <div style="display: flex; gap: 15px;">
                <form method="POST" action="{}/config/save" style="margin: 0;">
                    <button type="submit" class="btn btn-success">💾 Save Config to Disk</button>
                </form>
                <form method="POST" action="{}/config/reload" style="margin: 0;">
                    <button type="submit" class="btn" style="background: var(--amber); color: var(--bg-deep);">🔄 Reload Config from Disk</button>
                </form>
            </div>
            <p style="color: var(--text-muted); font-size: 0.85em; margin-top: 15px;">
                <strong>Precedence:</strong> TUI wizard → config file → runtime changes (Control Panel)
            </p>
        </div>
    "#,
        ADMIN_PATH,
        ADMIN_PATH,
        checkbox(
            "ua_analysis_enabled",
            "User-Agent Analysis (detect bots, non-Tor browsers)",
            config.ua_analysis_enabled
        ),
        checkbox(
            "referer_analysis_enabled",
            "Referer Analysis (detect suspicious external referers)",
            config.referer_analysis_enabled
        ),
        checkbox(
            "path_analysis_enabled",
            "Path Analysis (detect attack paths like ../, .env)",
            config.path_analysis_enabled
        ),
        checkbox(
            "enumeration_detection_enabled",
            "Enumeration Detection (detect rapid path scanning)",
            config.enumeration_detection_enabled
        ),
        checkbox(
            "form_tracking_enabled",
            "Form Submission Tracking (detect form floods)",
            config.form_tracking_enabled
        ),
        checkbox(
            "payload_analysis_enabled",
            "Payload Size Analysis (detect oversized requests)",
            config.payload_analysis_enabled
        ),
        config.max_unique_paths_per_minute,
        config.max_form_submissions_per_minute,
        config.max_payload_size,
        config.sequential_path_threshold,
        config.threat_demotion_threshold,
        config.threat_severity_threshold,
        config.max_demotions_before_kill,
        threshold_rows,
        attack_path_rows,
        custom_whitelist_str,
        agg_stats.sessions_analyzed,
        agg_stats.total_requests_analyzed,
        agg_stats.total_violations,
        agg_stats.sessions_with_suspicious_ua,
        violations_breakdown,
        // Branding config section
        ADMIN_PATH,
        html_escape(&branding_config.service_name),
        html_escape(&branding_config.description),
        html_escape(&branding_config.welcome_message),
        &branding_config.primary_color,
        &branding_config.primary_color,
        &branding_config.secondary_color,
        &branding_config.secondary_color,
        &branding_config.tertiary_color,
        &branding_config.tertiary_color,
        branding_config.custom_css.as_deref().unwrap_or(""),
        // Captcha config section
        ADMIN_PATH,
        render_captcha_type_options(captcha_config.gate_captcha_type),
        render_captcha_type_options(captcha_config.threat_captcha_type),
        if captcha_config.threat_captcha_enabled {
            "checked"
        } else {
            ""
        },
        if captcha_config.threat_captcha_enabled {
            "var(--sage)"
        } else {
            "var(--crimson)"
        },
        if captcha_config.threat_captcha_enabled {
            "ENABLED"
        } else {
            "DISABLED"
        },
        if captcha_config.random_cycling {
            "checked"
        } else {
            ""
        },
        if captcha_config.random_cycling {
            "var(--sage)"
        } else {
            "var(--crimson)"
        },
        if captcha_config.random_cycling {
            "ENABLED"
        } else {
            "DISABLED"
        },
        // Cycling types checkboxes
        if captcha_config.cycling_types.contains(&CaptchaType::BmpText) {
            "checked"
        } else {
            ""
        },
        if captcha_config.cycling_types.contains(&CaptchaType::Emoji) {
            "checked"
        } else {
            ""
        },
        if captcha_config
            .cycling_types
            .contains(&CaptchaType::Direction)
        {
            "checked"
        } else {
            ""
        },
        if captcha_config
            .cycling_types
            .contains(&CaptchaType::Sequence)
        {
            "checked"
        } else {
            ""
        },
        if captcha_config
            .cycling_types
            .contains(&CaptchaType::WordUnscramble)
        {
            "checked"
        } else {
            ""
        },
        if captcha_config
            .cycling_types
            .contains(&CaptchaType::Silhouette)
        {
            "checked"
        } else {
            ""
        },
        // Captcha pool config section
        ADMIN_PATH,
        captcha_pool_config.pool_size,
        captcha_pool_config.min_pool_size,
        captcha_pool_config.max_pool_size,
        captcha_pool_config.difficulty,
        captcha_pool_config.timeout_seconds,
        captcha_pool_config.max_attempts,
        captcha_pool_config.rotation_percent,
        captcha_pool_config.rotation_interval_days,
        // Per-type CAPTCHA settings
        captcha_type_forms,
        // Config management buttons
        ADMIN_PATH,
        ADMIN_PATH,
    );

    html_page("Settings", &content)
}

fn render_tutorial() -> Response<BoxBody> {
    let content = format!(
        r#"
        <h2>Fortify System Tutorial</h2>

        <!-- Quick Intro Section -->
        <div class="card" style="background: var(--bg-elevated); border: 2px solid var(--gold-primary);">
            <h3 style="font-size: 1.5em; margin-bottom: 15px;">Welcome to Fortify</h3>
            <p style="color: var(--text-secondary); font-size: 1.05em; line-height: 1.8;">
                <strong>Fortify</strong> is a military-grade protection layer for Tor hidden services (.onion sites).
                It acts as an intelligent reverse proxy that shields your service from automated attacks, scrapers,
                and malicious actors — all while respecting the privacy-first nature of Tor.
            </p>
            <div style="margin-top: 20px; padding: 15px; background: var(--bg-deep); border-radius: 4px;">
                <p style="color: var(--gold-primary); font-weight: 500; font-size: 1.1em; margin-bottom: 10px;">
                    Key Features at a Glance
                </p>
                <ul style="color: var(--text-secondary); margin-left: 20px; line-height: 2;">
                    <li><strong>JavaScript-Free CAPTCHA</strong> - Works with Tor Browser's "Safest" mode</li>
                    <li><strong>Trust-Based Routing</strong> - Verified users get fast healthy nodes; threats get isolated</li>
                    <li><strong>Behavioral Analysis</strong> - Detects bots without fingerprinting</li>
                    <li><strong>Mirror Management</strong> - Rotating .onion addresses for DDoS resilience</li>
                    <li><strong>Real-Time Monitoring</strong> - Full visibility into traffic and threats</li>
                </ul>
            </div>
        </div>

        <!-- What It Protects Against -->
        <div class="card">
            <h3>What Fortify Protects Against</h3>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 15px; margin-top: 15px;">
                <div style="background: rgba(168,84,84,0.08); border: 1px solid var(--crimson); padding: 15px; border-radius: 4px;">
                    <h4 style="color: var(--crimson);">Automated Scraping</h4>
                    <p style="color: var(--text-muted); font-size: 0.9em;">Bots trying to mirror your entire site or harvest data</p>
                </div>
                <div style="background: rgba(168,84,84,0.08); border: 1px solid var(--crimson); padding: 15px; border-radius: 4px;">
                    <h4 style="color: var(--crimson);">Exploit Probes</h4>
                    <p style="color: var(--text-muted); font-size: 0.9em;">Attackers scanning for .env files, admin panels, vulnerabilities</p>
                </div>
                <div style="background: rgba(168,84,84,0.08); border: 1px solid var(--crimson); padding: 15px; border-radius: 4px;">
                    <h4 style="color: var(--crimson);">DDoS Attacks</h4>
                    <p style="color: var(--text-muted); font-size: 0.9em;">Distributed denial-of-service through request flooding</p>
                </div>
                <div style="background: rgba(168,84,84,0.08); border: 1px solid var(--crimson); padding: 15px; border-radius: 4px;">
                    <h4 style="color: var(--crimson);">Path Enumeration</h4>
                    <p style="color: var(--text-muted); font-size: 0.9em;">Directory busting and resource discovery attacks</p>
                </div>
                <div style="background: rgba(168,84,84,0.08); border: 1px solid var(--crimson); padding: 15px; border-radius: 4px;">
                    <h4 style="color: var(--crimson);">Form Spam</h4>
                    <p style="color: var(--text-muted); font-size: 0.9em;">Automated submission floods targeting login/signup forms</p>
                </div>
                <div style="background: rgba(168,84,84,0.08); border: 1px solid var(--crimson); padding: 15px; border-radius: 4px;">
                    <h4 style="color: var(--crimson);">Deanonymization</h4>
                    <p style="color: var(--text-muted); font-size: 0.9em;">Timing attacks and traffic correlation analysis</p>
                </div>
            </div>
        </div>

        <!-- How Routing Works -->
        <div class="card" style="border-color: var(--gold-muted);">
            <h3>How User Routing Works</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Fortify uses a trust-based tier system to route users through appropriate backend nodes:
            </p>
            <div style="background: var(--bg-deep); padding: 20px; border-radius: 4px; font-family: monospace;">
                <div style="display: flex; align-items: center; margin-bottom: 15px;">
                    <div style="width: 120px; color: var(--gold-primary);">NEW USER</div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="flex: 1; padding: 10px; background: rgba(212,168,75,0.1); border: 1px solid var(--amber); margin: 0 10px; text-align: center; color: var(--amber);">
                        GATE (CAPTCHA)
                    </div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="padding: 10px; color: var(--sage);">✓ Verified</div>
                </div>
                <div style="display: flex; align-items: center; margin-bottom: 15px;">
                    <div style="width: 120px; color: var(--sage);">VERIFIED</div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="flex: 1; padding: 10px; background: rgba(125,154,120,0.1); border: 1px solid var(--sage); margin: 0 10px; text-align: center; color: var(--sage);">
                        HEALTHY NODES
                    </div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="padding: 10px; color: var(--sage);">Fast Service</div>
                </div>
                <div style="display: flex; align-items: center; margin-bottom: 15px;">
                    <div style="width: 120px; color: var(--amber);">SUSPICIOUS</div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="flex: 1; padding: 10px; background: rgba(212,168,75,0.1); border: 1px solid var(--amber); margin: 0 10px; text-align: center; color: var(--amber);">
                        THREAT NODES
                    </div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="padding: 10px; color: var(--amber);">Isolated/Monitored</div>
                </div>
                <div style="display: flex; align-items: center;">
                    <div style="width: 120px; color: var(--crimson);">BURNED</div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="flex: 1; padding: 10px; background: rgba(168,84,84,0.1); border: 1px solid var(--crimson); margin: 0 10px; text-align: center; color: var(--crimson);">
                        BLOCKED
                    </div>
                    <div style="color: var(--text-muted);">→</div>
                    <div style="padding: 10px; color: var(--crimson);">Access Denied</div>
                </div>
            </div>
            <p style="color: var(--text-muted); font-size: 0.9em; margin-top: 15px;">
                Users automatically move between tiers based on their behavior. Legitimate users stay verified;
                attackers get demoted and eventually burned.
            </p>
        </div>

        <!-- Settings Explanations -->
        <div class="card" style="border-color: var(--amber);">
            <h3>Settings Reference</h3>
            <p style="color: var(--text-muted); margin-bottom: 20px;">
                Click each setting to expand detailed explanations.
            </p>

            <!-- Detection Modules -->
            <details style="margin-bottom: 15px; background: var(--bg-deep); border: 1px solid var(--border-subtle); padding: 15px; border-radius: 4px;">
                <summary style="cursor: pointer; color: var(--gold-primary); font-weight: 500; font-size: 1.05em;">
                    User-Agent Analysis
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border-subtle);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Detects non-browser and suspicious User-Agent strings that indicate bots or automated tools.
                    </p>
                    <details style="margin-top: 10px; background: var(--bg-elevated); padding: 10px; border-radius: 3px;">
                        <summary style="color: var(--amber); cursor: pointer;">Advanced Details</summary>
                        <div style="margin-top: 10px; color: var(--text-muted); font-size: 0.9em; line-height: 1.6;">
                            <p>The UA analyzer checks for:</p>
                            <ul style="margin-left: 20px; margin-top: 5px;">
                                <li>Known bot signatures (curl, wget, python-requests, scrapy, etc.)</li>
                                <li>Missing or malformed User-Agent headers</li>
                                <li>Non-browser UA patterns that don't match Tor Browser</li>
                                <li>Tor Browser's standard UA is <strong>always</strong> allowed</li>
                            </ul>
                            <p style="margin-top: 10px; color: var(--gold-primary);">
                                Note: Tor Browser in "Safest" mode uses a standardized UA, which is whitelisted.
                            </p>
                        </div>
                    </details>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-deep); border: 1px solid var(--border-subtle); padding: 15px; border-radius: 4px;">
                <summary style="cursor: pointer; color: var(--gold-primary); font-weight: 500; font-size: 1.05em;">
                    Referer Analysis
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border-subtle);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Flags requests with suspicious or impossible Referer headers from external clearnet sites.
                    </p>
                    <details style="margin-top: 10px; background: var(--bg-elevated); padding: 10px; border-radius: 3px;">
                        <summary style="color: var(--amber); cursor: pointer;">Advanced Details</summary>
                        <div style="margin-top: 10px; color: var(--text-muted); font-size: 0.9em; line-height: 1.6;">
                            <p>Checks for:</p>
                            <ul style="margin-left: 20px; margin-top: 5px;">
                                <li>Referers from clearnet domains (impossible for direct .onion access)</li>
                                <li>Mismatched referer patterns that indicate scanning tools</li>
                                <li>Empty/missing referers are <strong>normal</strong> for Tor and allowed</li>
                            </ul>
                            <p style="margin-top: 10px; color: var(--gold-primary);">
                                Tor Browser often strips referers for privacy, so missing referers are not penalized.
                            </p>
                        </div>
                    </details>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-deep); border: 1px solid var(--border-subtle); padding: 15px; border-radius: 4px;">
                <summary style="cursor: pointer; color: var(--gold-primary); font-weight: 500; font-size: 1.05em;">
                    Path Analysis
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border-subtle);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Detects requests to known attack paths like .env files, admin panels, and config files.
                    </p>
                    <details style="margin-top: 10px; background: var(--bg-elevated); padding: 10px; border-radius: 3px;">
                        <summary style="color: var(--amber); cursor: pointer;">Advanced Details</summary>
                        <div style="margin-top: 10px; color: var(--text-muted); font-size: 0.9em; line-height: 1.6;">
                            <p>Monitors requests for:</p>
                            <ul style="margin-left: 20px; margin-top: 5px;">
                                <li><code>../</code> path traversal attempts</li>
                                <li><code>.env</code>, <code>.git</code>, <code>.htaccess</code> config exposure</li>
                                <li><code>/wp-admin</code>, <code>/phpmyadmin</code> admin probes</li>
                                <li><code>/debug</code>, <code>/actuator</code> debug endpoints</li>
                                <li>SQL injection patterns, shell commands in URLs</li>
                            </ul>
                            <p style="margin-top: 10px; color: var(--gold-muted);">
                                You can disable specific patterns in Settings if they conflict with your app's legitimate paths.
                            </p>
                        </div>
                    </details>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-surface); border: 1px solid var(--border-accent); padding: 15px; border-radius: 5px;">
                <summary style="cursor: pointer; color: var(--gold-primary); font-weight: bold; font-size: 1.1em;">
                    Enumeration Detection
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border-subtle);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Catches rapid scanning of unique paths (directory busting attacks).
                    </p>
                    <details style="margin-top: 10px; background: var(--bg-deep); padding: 10px; border-radius: 3px;">
                        <summary style="color: var(--gold-muted); cursor: pointer;">Advanced Details</summary>
                        <div style="margin-top: 10px; color: var(--text-muted); font-size: 0.9em; line-height: 1.6;">
                            <p>Tracks per-session:</p>
                            <ul style="margin-left: 20px; margin-top: 5px;">
                                <li>Number of unique paths accessed per minute</li>
                                <li>Sequential number patterns (file1, file2, file3...)</li>
                                <li>Resource ID enumeration (/user/1, /user/2...)</li>
                            </ul>
                            <p style="margin-top: 10px;">
                                <strong>Threshold:</strong> "Max Unique Paths/Min" - triggers violation if exceeded
                            </p>
                        </div>
                    </details>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-surface); border: 1px solid var(--border-accent); padding: 15px; border-radius: 5px;">
                <summary style="cursor: pointer; color: var(--gold-primary); font-weight: bold; font-size: 1.1em;">
                    Form Submission Tracking
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border-subtle);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Monitors POST request frequency to detect form spam and brute-force attempts.
                    </p>
                    <details style="margin-top: 10px; background: var(--bg-deep); padding: 10px; border-radius: 3px;">
                        <summary style="color: var(--gold-muted); cursor: pointer;">Advanced Details</summary>
                        <div style="margin-top: 10px; color: var(--text-muted); font-size: 0.9em; line-height: 1.6;">
                            <p>Counts:</p>
                            <ul style="margin-left: 20px; margin-top: 5px;">
                                <li>POST requests per session per minute</li>
                                <li>Login/signup endpoint hits</li>
                                <li>Rapid submission patterns indicating automation</li>
                            </ul>
                            <p style="margin-top: 10px;">
                                <strong>Threshold:</strong> "Max Form Submissions/Min" - triggers violation if exceeded
                            </p>
                        </div>
                    </details>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-surface); border: 1px solid var(--border-accent); padding: 15px; border-radius: 5px;">
                <summary style="cursor: pointer; color: var(--gold-primary); font-weight: bold; font-size: 1.1em;">
                    Payload Size Analysis
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border-subtle);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Flags requests with abnormally large payloads that could indicate attacks.
                    </p>
                    <details style="margin-top: 10px; background: var(--bg-deep); padding: 10px; border-radius: 3px;">
                        <summary style="color: var(--gold-muted); cursor: pointer;">Advanced Details</summary>
                        <div style="margin-top: 10px; color: var(--text-muted); font-size: 0.9em; line-height: 1.6;">
                            <p>Monitors:</p>
                            <ul style="margin-left: 20px; margin-top: 5px;">
                                <li>Request body Content-Length</li>
                                <li>Oversized payloads (potential buffer overflow attempts)</li>
                                <li>Suspiciously small payloads on endpoints expecting data</li>
                            </ul>
                            <p style="margin-top: 10px;">
                                <strong>Threshold:</strong> "Max Payload Size" in bytes
                            </p>
                        </div>
                    </details>
                </div>
            </details>

            <!-- Demotion Thresholds -->
            <h4 style="color: var(--crimson); margin: 25px 0 15px 0;">Demotion Thresholds</h4>

            <details style="margin-bottom: 15px; background: var(--bg-surface); border: 1px solid var(--crimson); padding: 15px; border-radius: 5px;">
                <summary style="cursor: pointer; color: var(--crimson); font-weight: bold; font-size: 1.1em;">
                    Total Violations Threshold
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid rgba(168,84,84,0.3);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Demote user to threat pool after this many total violations.
                    </p>
                    <p style="color: var(--text-muted); font-size: 0.9em;">
                        <strong>Example:</strong> If set to 10, a user accumulating 10 violations of any type gets moved to threat nodes.
                    </p>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-surface); border: 1px solid var(--crimson); padding: 15px; border-radius: 5px;">
                <summary style="cursor: pointer; color: var(--crimson); font-weight: bold; font-size: 1.1em;">
                    Severity Score Threshold
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid rgba(168,84,84,0.3);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Demote when cumulative severity score reaches this value.
                    </p>
                    <p style="color: var(--text-muted); font-size: 0.9em;">
                        Some violations (like exploit attempts) carry higher severity weight than others.
                        This threshold triggers on weighted severity, not just count.
                    </p>
                </div>
            </details>

            <details style="margin-bottom: 15px; background: var(--bg-surface); border: 1px solid var(--crimson); padding: 15px; border-radius: 5px;">
                <summary style="cursor: pointer; color: var(--crimson); font-weight: bold; font-size: 1.1em;">
                    Max Demotions Before Kill
                </summary>
                <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid rgba(168,84,84,0.3);">
                    <p style="color: var(--text-secondary); margin-bottom: 10px;">
                        <strong>Simple:</strong> Permanently burn a session after being demoted this many times.
                    </p>
                    <p style="color: var(--text-muted); font-size: 0.9em;">
                        Users can be demoted → re-verify → demoted again. After N cycles, they're burned forever.
                        This prevents persistent attackers from repeatedly re-verifying.
                    </p>
                </div>
            </details>
        </div>

        <!-- Back to Settings -->
        <div style="text-align: center; margin-top: 30px;">
            <a href="{}/settings" class="btn" style="padding: 15px 40px; font-size: 1.1em;">
                ← Back to Settings
            </a>
        </div>
    "#,
        ADMIN_PATH
    );

    html_page("Tutorial", &content)
}

// ============================================================================
// ACTION HANDLERS
// ============================================================================

async fn handle_session_action(
    req: Request<Incoming>,
    state: Arc<AdminState>,
) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    let session_id = params.get("session_id").map(|s| s.as_str()).unwrap_or("");
    let action = params.get("action").map(|s| s.as_str()).unwrap_or("");

    match action {
        "to_threat" => {
            state.set_session_tier(session_id, "Suspicious");
            tracing::info!("Admin: Session {} moved to threat pool", session_id);
        }
        "to_healthy" => {
            state.set_session_tier(session_id, "Verified");
            state.unban_session(session_id);
            tracing::info!("Admin: Session {} moved to healthy pool", session_id);
        }
        "ban" => {
            state.ban_session(session_id);
            tracing::warn!("Admin: Session {} banned", session_id);
        }
        "unban" => {
            state.unban_session(session_id);
            tracing::info!("Admin: Session {} unbanned", session_id);
        }
        "delete" => {
            state.delete_session(session_id);
            tracing::info!("Admin: Session {} deleted", session_id);
        }
        _ => {}
    }

    redirect(&format!("{}/sessions", ADMIN_PATH))
}

async fn handle_node_action(req: Request<Incoming>, state: Arc<AdminState>) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    let node_id = params.get("node_id").map(|s| s.as_str()).unwrap_or("");
    let action = params.get("action").map(|s| s.as_str()).unwrap_or("");

    match action {
        "to_healthy" => {
            state.set_node_mode(node_id, "healthy");
            // Also update status to online when moving to a pool
            if let Some(mut node) = state.get_node(node_id) {
                node.status = "online".to_string();
                state.update_node(node);
            }
            tracing::info!("Admin: Node {} set to healthy mode", node_id);
        }
        "to_threat" => {
            state.set_node_mode(node_id, "threat");
            // Also update status to online when moving to a pool
            if let Some(mut node) = state.get_node(node_id) {
                node.status = "online".to_string();
                state.update_node(node);
            }

            // Demote all sessions currently using this node
            // They'll be forced through the Gate/captcha flow on their next request
            let sessions = state.get_sessions();
            let mut demoted_count = 0;
            for session in sessions {
                if session.current_node == node_id {
                    // Set tier to Suspicious so requires_gate() returns true
                    state.set_session_tier(&session.session_id, "Suspicious");
                    demoted_count += 1;
                    tracing::info!(
                        "Demoted session {} (was on node {})",
                        session.session_id,
                        node_id
                    );
                }
            }

            tracing::info!(
                "Admin: Node {} set to threat mode, demoted {} sessions",
                node_id,
                demoted_count
            );
        }
        "remove" => {
            state.remove_node(node_id);
            tracing::warn!("Admin: Node {} removed", node_id);
        }
        "activate" => {
            // Manually activate a pending node
            if let Some(mut node) = state.get_node(node_id) {
                node.status = "online".to_string();
                state.update_node(node);
                tracing::info!("Admin: Node {} activated", node_id);
            }
        }
        "add" => {
            let bind_addr = params.get("bind_addr").map(|s| s.as_str()).unwrap_or("");
            let onion_address = params
                .get("onion_address")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let mode = params.get("mode").map(|s| s.as_str()).unwrap_or("healthy");
            let custom_name = params.get("node_name").map(|s| s.trim()).unwrap_or("");

            if !bind_addr.is_empty() {
                // Generate node ID: use custom name if provided, otherwise auto-generate
                let new_id = if !custom_name.is_empty() {
                    custom_name.to_string()
                } else {
                    // Auto-generate based on pool and existing count
                    let existing_nodes = state.get_nodes();
                    let pool_count = existing_nodes.iter().filter(|n| n.mode == mode).count();
                    format!("{}-{}", mode, pool_count)
                };

                state.update_node(NodeInfo {
                    id: new_id.clone(),
                    bind_addr: bind_addr.to_string(),
                    onion_address,
                    mode: mode.to_string(),
                    // Mark as online immediately - user can deactivate if needed
                    status: "online".to_string(),
                    created_at: now(),
                    total_requests: 0,
                    active_connections: 0,
                    violations_detected: 0,
                });
                tracing::info!(
                    "Admin: Node {} added at {} (mode: {})",
                    new_id,
                    bind_addr,
                    mode
                );
            }
        }
        _ => {}
    }

    redirect(&format!("{}/nodes", ADMIN_PATH))
}

async fn handle_mirror_action(req: Request<Incoming>, state: Arc<AdminState>) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    let mirror_id = params.get("mirror_id").map(|s| s.as_str()).unwrap_or("");
    let action = params.get("action").map(|s| s.as_str()).unwrap_or("");

    let onion_address = params
        .get("onion_address")
        .map(|s| s.as_str())
        .unwrap_or("");

    // Get auth token for orchestrator API calls
    let auth_token = get_auth_token();

    match action {
        "pause" => {
            // Call orchestrator to pause/standdown this mirror
            for port in &[8080, 8180] {
                let addr = onion_address.to_string();
                let token = auth_token.clone();
                if let Ok(Ok(_)) = std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    client
                        .post(format!("http://127.0.0.1:{}/mirror/pause", port))
                        .header(AUTH_TOKEN_HEADER, token)
                        .json(&serde_json::json!({"onion_address": addr}))
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                })
                .join()
                {
                    tracing::info!("Admin: Mirror {} paused via orchestrator", mirror_id);
                    break;
                }
            }
        }
        "resume" => {
            // Call orchestrator to resume a paused mirror
            for port in &[8080, 8180] {
                let addr = onion_address.to_string();
                let token = auth_token.clone();
                if let Ok(Ok(_)) = std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    client
                        .post(format!("http://127.0.0.1:{}/mirror/resume", port))
                        .header(AUTH_TOKEN_HEADER, token)
                        .json(&serde_json::json!({"onion_address": addr}))
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                })
                .join()
                {
                    tracing::info!("Admin: Mirror {} resumed via orchestrator", mirror_id);
                    break;
                }
            }
        }
        "destroy" => {
            // Call orchestrator to destroy/burn this mirror permanently
            for port in &[8080, 8180] {
                let addr = onion_address.to_string();
                let token = auth_token.clone();
                if let Ok(Ok(_)) = std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    client
                        .post(format!("http://127.0.0.1:{}/mirror/destroy", port))
                        .header(AUTH_TOKEN_HEADER, token)
                        .json(&serde_json::json!({"onion_address": addr}))
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                })
                .join()
                {
                    tracing::warn!("Admin: Mirror {} destroyed via orchestrator", mirror_id);
                    break;
                }
            }
        }
        "remove" => {
            state.remove_mirror(mirror_id);
            tracing::warn!("Admin: Mirror {} removed", mirror_id);
        }
        "create" => {
            // Call orchestrator to create a new mirror
            // Try both orchestrator ports
            let mut created = false;
            for port in &[8080, 8180] {
                let token = auth_token.clone();
                match std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    client
                        .post(format!("http://127.0.0.1:{}/mirror/create", port))
                        .header(AUTH_TOKEN_HEADER, token)
                        .timeout(std::time::Duration::from_secs(30))
                        .send()
                })
                .join()
                {
                    Ok(Ok(resp)) if resp.status().is_success() => {
                        tracing::info!(
                            "✅ Admin: Mirror creation triggered via orchestrator (port {})",
                            port
                        );
                        created = true;
                        break;
                    }
                    Ok(Ok(resp)) => {
                        tracing::warn!("Admin: Orchestrator responded with {}", resp.status());
                    }
                    Ok(Err(e)) => {
                        tracing::debug!(
                            "Admin: Could not reach orchestrator on port {}: {}",
                            port,
                            e
                        );
                    }
                    Err(_) => {
                        tracing::debug!("Admin: Thread panicked trying port {}", port);
                    }
                }
            }

            if !created {
                // Fallback: create placeholder if orchestrator is unreachable
                let new_id = format!("mirror-{}", &uuid_v4()[..8]);
                state.update_mirror(MirrorInfo {
                    id: new_id.clone(),
                    onion_address: format!("pending-{}.onion", &uuid_v4()[..16]),
                    status: "creating".to_string(),
                    created_at: now(),
                    total_requests: 0,
                });
                tracing::warn!("Admin: Mirror creation requested but orchestrator unreachable - placeholder created ({})", new_id);
            }
        }
        "activate" => {
            // Activate a standby mirror to make it active
            for port in &[8080, 8180] {
                let addr = onion_address.to_string();
                match std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    client
                        .post(format!("http://127.0.0.1:{}/mirror/activate", port))
                        .json(&serde_json::json!({"onion_address": addr}))
                        .timeout(std::time::Duration::from_secs(15))
                        .send()
                })
                .join()
                {
                    Ok(Ok(resp)) if resp.status().is_success() => {
                        tracing::info!(
                            "Admin: Standby mirror {} activated via orchestrator",
                            mirror_id
                        );
                        break;
                    }
                    Ok(Ok(resp)) => {
                        tracing::warn!(
                            "Admin: Orchestrator returned {} when activating mirror",
                            resp.status()
                        );
                    }
                    Ok(Err(e)) => {
                        tracing::debug!(
                            "Admin: Could not reach orchestrator on port {}: {}",
                            port,
                            e
                        );
                    }
                    Err(_) => {
                        tracing::debug!("Admin: Thread panicked trying port {}", port);
                    }
                }
            }
        }
        "create_standby" => {
            // Create a new standby mirror
            for port in &[8080, 8180] {
                match std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    client
                        .post(format!("http://127.0.0.1:{}/mirror/create-standby", port))
                        .timeout(std::time::Duration::from_secs(30))
                        .send()
                })
                .join()
                {
                    Ok(Ok(resp)) if resp.status().is_success() => {
                        tracing::info!(
                            "Admin: Standby mirror creation triggered via orchestrator (port {})",
                            port
                        );
                        break;
                    }
                    Ok(Ok(resp)) => {
                        tracing::warn!(
                            "Admin: Orchestrator responded with {} when creating standby",
                            resp.status()
                        );
                    }
                    Ok(Err(e)) => {
                        tracing::debug!(
                            "Admin: Could not reach orchestrator on port {}: {}",
                            port,
                            e
                        );
                    }
                    Err(_) => {
                        tracing::debug!("Admin: Thread panicked trying port {}", port);
                    }
                }
            }
        }
        _ => {}
    }

    redirect(&format!("{}/mirrors", ADMIN_PATH))
}

async fn handle_behavior_settings(
    req: Request<Incoming>,
    state: Arc<AdminState>,
) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    // Get current config and update with form values
    let mut config = state.get_behavior_config();

    // Checkboxes: present in form = enabled, absent = disabled
    config.ua_analysis_enabled = params.contains_key("ua_analysis_enabled");
    config.referer_analysis_enabled = params.contains_key("referer_analysis_enabled");
    config.path_analysis_enabled = params.contains_key("path_analysis_enabled");
    config.enumeration_detection_enabled = params.contains_key("enumeration_detection_enabled");
    config.form_tracking_enabled = params.contains_key("form_tracking_enabled");
    config.payload_analysis_enabled = params.contains_key("payload_analysis_enabled");

    // Numeric thresholds
    if let Some(val) = params.get("max_unique_paths_per_minute") {
        if let Ok(v) = val.parse::<u32>() {
            config.max_unique_paths_per_minute = v;
        }
    }
    if let Some(val) = params.get("max_form_submissions_per_minute") {
        if let Ok(v) = val.parse::<u32>() {
            config.max_form_submissions_per_minute = v;
        }
    }
    if let Some(val) = params.get("max_payload_size") {
        if let Ok(v) = val.parse::<usize>() {
            config.max_payload_size = v;
        }
    }
    if let Some(val) = params.get("sequential_path_threshold") {
        if let Ok(v) = val.parse::<u32>() {
            config.sequential_path_threshold = v;
        }
    }

    // Threat demotion thresholds
    if let Some(val) = params.get("threat_demotion_threshold") {
        if let Ok(v) = val.parse::<u32>() {
            config.threat_demotion_threshold = v;
        }
    }
    if let Some(val) = params.get("threat_severity_threshold") {
        if let Ok(v) = val.parse::<u32>() {
            config.threat_severity_threshold = v;
        }
    }
    if let Some(val) = params.get("max_demotions_before_kill") {
        if let Ok(v) = val.parse::<u32>() {
            config.max_demotions_before_kill = v;
        }
    }

    // Per-violation type thresholds
    let violation_type_mappings = [
        ("threshold_attack_path_access", "Attack Path Access"),
        ("threshold_suspicious_user_agent", "Suspicious User-Agent"),
        ("threshold_path_enumeration", "Path Enumeration"),
        ("threshold_resource_enumeration", "Resource Enumeration"),
        ("threshold_form_submission_flood", "Form Submission Flood"),
        ("threshold_automated_behavior", "Automated Behavior"),
        ("threshold_suspicious_referer", "Suspicious Referer"),
        ("threshold_oversized_payload", "Oversized Payload"),
        ("threshold_undersized_payload", "Undersized Payload"),
    ];

    for (field_name, violation_type) in &violation_type_mappings {
        if let Some(val) = params.get(*field_name) {
            if let Ok(v) = val.parse::<u32>() {
                config
                    .violation_type_thresholds
                    .insert(violation_type.to_string(), v);
            }
        }
    }

    // Attack path toggles - rebuild the disabled set based on which checkboxes are checked
    let mut disabled_paths = HashSet::new();
    for (pattern, _, _) in KNOWN_ATTACK_PATHS {
        let field_name = format!("attack_path_{}", pattern.replace(['/', '.', '\\'], "_"));
        // If checkbox is NOT in params, the path is disabled
        if !params.contains_key(&field_name) {
            disabled_paths.insert(pattern.to_string());
        }
    }
    config.disabled_attack_paths = disabled_paths;

    // Custom whitelist paths (one per line in textarea)
    if let Some(val) = params.get("custom_whitelist_paths") {
        config.custom_whitelist_paths = val
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    state.update_behavior_config(config);
    tracing::info!("Admin: Behavioral analysis settings updated");

    redirect(&format!("{}/settings", ADMIN_PATH))
}

/// Render HTML options for captcha type select
fn render_captcha_type_options(selected: CaptchaType) -> String {
    let captcha_types = [
        (
            CaptchaType::BmpText,
            "Text Image - Type characters from BMP image",
        ),
        (
            CaptchaType::Emoji,
            "Emoji Selection - Click emoji matching description",
        ),
        (
            CaptchaType::Direction,
            "Arrow Direction - Click the arrow pointing correctly",
        ),
        (
            CaptchaType::Sequence,
            "Sequence Pattern - Complete the pattern (A,B,C,?)",
        ),
        (
            CaptchaType::WordUnscramble,
            "Word Unscramble - Unscramble letters to form word",
        ),
        (
            CaptchaType::ImageRotation,
            "Image Rotation - Select correctly oriented image",
        ),
        (
            CaptchaType::Silhouette,
            "Silhouette ID - Identify the silhouette category",
        ),
    ];

    captcha_types
        .iter()
        .map(|(ctype, desc)| {
            let selected_attr = if *ctype == selected { "selected" } else { "" };
            let value = match ctype {
                CaptchaType::BmpText => "BmpText",
                CaptchaType::Emoji => "Emoji",
                CaptchaType::Direction => "Direction",
                CaptchaType::Sequence => "Sequence",
                CaptchaType::WordUnscramble => "WordUnscramble",
                CaptchaType::ImageRotation => "ImageRotation",
                CaptchaType::Silhouette => "Silhouette",
            };
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                value, selected_attr, desc
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse CaptchaType from string
fn parse_captcha_type(s: &str) -> CaptchaType {
    match s {
        "Emoji" => CaptchaType::Emoji,
        "Direction" => CaptchaType::Direction,
        "Sequence" => CaptchaType::Sequence,
        "WordUnscramble" => CaptchaType::WordUnscramble,
        "ImageRotation" => CaptchaType::ImageRotation,
        "Silhouette" => CaptchaType::Silhouette,
        _ => CaptchaType::BmpText, // Default
    }
}

async fn handle_branding_settings(
    req: Request<Incoming>,
    state: Arc<AdminState>,
) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    // Get current config and update with form values
    let mut config = state.get_branding_config();

    if let Some(val) = params.get("service_name") {
        config.service_name = val.clone();
    }
    if let Some(val) = params.get("description") {
        config.description = val.clone();
    }
    if let Some(val) = params.get("welcome_message") {
        config.welcome_message = val.clone();
    }
    if let Some(val) = params.get("primary_color") {
        // Validate hex color format
        if val.starts_with('#') && val.len() == 7 {
            config.primary_color = val.clone();
        }
    }
    if let Some(val) = params.get("secondary_color") {
        if val.starts_with('#') && val.len() == 7 {
            config.secondary_color = val.clone();
        }
    }
    if let Some(val) = params.get("tertiary_color") {
        if val.starts_with('#') && val.len() == 7 {
            config.tertiary_color = val.clone();
        }
    }
    if let Some(val) = params.get("custom_css") {
        config.custom_css = if val.trim().is_empty() {
            None
        } else {
            Some(val.clone())
        };
    }

    state.update_branding_config(config.clone());
    tracing::info!(
        "Admin: Branding settings updated - service_name={}, primary_color={}",
        config.service_name,
        config.primary_color
    );

    redirect(&format!("{}/settings", ADMIN_PATH))
}

async fn handle_captcha_settings(
    req: Request<Incoming>,
    state: Arc<AdminState>,
) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    // Get current config and update with form values
    let mut config = state.get_captcha_config();

    // Captcha type selections
    if let Some(val) = params.get("gate_captcha_type") {
        config.gate_captcha_type = parse_captcha_type(val);
    }
    if let Some(val) = params.get("threat_captcha_type") {
        config.threat_captcha_type = parse_captcha_type(val);
    }

    // Toggle checkboxes
    config.threat_captcha_enabled = params.contains_key("threat_captcha_enabled");
    config.random_cycling = params.contains_key("random_cycling");

    // Parse cycling types (checkboxes for each captcha type)
    let mut cycling_types = Vec::new();
    if params.contains_key("cycle_BmpText") {
        cycling_types.push(CaptchaType::BmpText);
    }
    if params.contains_key("cycle_Emoji") {
        cycling_types.push(CaptchaType::Emoji);
    }
    if params.contains_key("cycle_Direction") {
        cycling_types.push(CaptchaType::Direction);
    }
    if params.contains_key("cycle_Sequence") {
        cycling_types.push(CaptchaType::Sequence);
    }
    if params.contains_key("cycle_WordUnscramble") {
        cycling_types.push(CaptchaType::WordUnscramble);
    }
    if params.contains_key("cycle_ImageRotation") {
        cycling_types.push(CaptchaType::ImageRotation);
    }
    if params.contains_key("cycle_Silhouette") {
        cycling_types.push(CaptchaType::Silhouette);
    }
    if !cycling_types.is_empty() {
        config.cycling_types = cycling_types;
    }

    state.update_captcha_config(config.clone());
    tracing::info!(
        "Admin: Captcha settings updated - random_cycling={}, threat_captcha_enabled={}",
        config.random_cycling,
        config.threat_captcha_enabled
    );

    // Push config to Gate server via HTTP API
    // Use default gate address if not configured
    let gate_address =
        std::env::var("GATE_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());

    match sync_captcha_config_to_gate(&gate_address, &config).await {
        Ok(_) => tracing::info!("Admin: Captcha config synced to Gate"),
        Err(e) => tracing::warn!("Admin: Failed to sync captcha config to Gate: {}", e),
    }

    redirect(&format!("{}/settings", ADMIN_PATH))
}

async fn handle_captcha_pool_settings(
    req: Request<Incoming>,
    state: Arc<AdminState>,
) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    // Get current config and update with form values
    let mut config = state.get_captcha_pool_config();

    if let Some(val) = params.get("pool_size") {
        if let Ok(v) = val.parse::<usize>() {
            config.pool_size = v;
        }
    }
    if let Some(val) = params.get("min_pool_size") {
        if let Ok(v) = val.parse::<usize>() {
            config.min_pool_size = v;
        }
    }
    if let Some(val) = params.get("max_pool_size") {
        if let Ok(v) = val.parse::<usize>() {
            config.max_pool_size = v;
        }
    }
    if let Some(val) = params.get("difficulty") {
        if let Ok(v) = val.parse::<u8>() {
            config.difficulty = v.clamp(1, 10);
        }
    }
    if let Some(val) = params.get("timeout_seconds") {
        if let Ok(v) = val.parse::<u64>() {
            config.timeout_seconds = v;
        }
    }
    if let Some(val) = params.get("max_attempts") {
        if let Ok(v) = val.parse::<u32>() {
            config.max_attempts = v;
        }
    }
    if let Some(val) = params.get("rotation_percent") {
        if let Ok(v) = val.parse::<u8>() {
            config.rotation_percent = v.min(100);
        }
    }
    if let Some(val) = params.get("rotation_interval_days") {
        if let Ok(v) = val.parse::<u32>() {
            config.rotation_interval_days = v;
        }
    }

    state.update_captcha_pool_config(config.clone());
    tracing::info!(
        "Admin: CAPTCHA Pool settings updated - pool_size={}, difficulty={}, timeout={}s",
        config.pool_size,
        config.difficulty,
        config.timeout_seconds
    );

    redirect(&format!("{}/settings", ADMIN_PATH))
}

async fn handle_captcha_type_settings(
    req: Request<Incoming>,
    state: Arc<AdminState>,
) -> Response<BoxBody> {
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let params = parse_form_data(&body_bytes);

    // Get type name from form
    let type_name = match params.get("type_name") {
        Some(name) => name.clone(),
        None => {
            tracing::warn!("Admin: CAPTCHA type settings missing type_name");
            return redirect(&format!("{}/settings", ADMIN_PATH));
        }
    };

    // Parse settings
    let enabled = params.contains_key("enabled");
    let option_count = params
        .get("option_count")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4);
    let difficulty = params
        .get("difficulty")
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(2);
    let min_pool_size = params
        .get("min_pool_size")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50);

    state.update_captcha_type_setting(&type_name, enabled, option_count, difficulty, min_pool_size);
    tracing::info!(
        "Admin: CAPTCHA type {} settings updated - enabled={}, difficulty={}, min_pool={}",
        type_name,
        enabled,
        difficulty,
        min_pool_size
    );

    redirect(&format!("{}/settings", ADMIN_PATH))
}

async fn handle_config_save(state: Arc<AdminState>) -> Response<BoxBody> {
    let path = AdminState::default_config_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match state.save_to_file(&path) {
        Ok(_) => {
            tracing::info!("Admin: Configuration saved to {:?}", path);
        }
        Err(e) => {
            tracing::error!("Admin: Failed to save config: {}", e);
        }
    }

    redirect(&format!("{}/settings", ADMIN_PATH))
}

async fn handle_config_reload(state: Arc<AdminState>) -> Response<BoxBody> {
    match state.reload_config() {
        Ok(_) => {
            tracing::info!("Admin: Configuration reloaded");
        }
        Err(e) => {
            tracing::error!("Admin: Failed to reload config: {}", e);
        }
    }

    redirect(&format!("{}/settings", ADMIN_PATH))
}

/// Sync captcha configuration to Gate server
async fn sync_captcha_config_to_gate(
    gate_address: &str,
    config: &CaptchaConfig,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("{}/gate/admin/captcha-config", gate_address);

    let response = client
        .post(&url)
        .json(config)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Gate returned error: {}", response.status()))
    }
}

// ============================================================================
// UTILITIES
// ============================================================================

fn parse_form_data(body: &[u8]) -> HashMap<String, String> {
    let body_str = String::from_utf8_lossy(body);
    let mut params = HashMap::new();

    for pair in body_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = urlencoding::decode(key).unwrap_or_default().to_string();
            let value = urlencoding::decode(value).unwrap_or_default().to_string();
            params.insert(key, value);
        }
    }

    params
}

fn redirect(location: &str) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Location", location)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

fn not_found() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        return "Never".to_string();
    }
    // Simple formatting - would use chrono in production
    let now = now();
    let diff = now.saturating_sub(ts);

    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

fn format_time_ago(ts: u64) -> String {
    format_timestamp(ts)
}

/// Format a duration in seconds as human-readable (e.g., "2h", "3d", "1w")
fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else if secs < 604800 {
        format!("{}d", secs / 86400)
    } else {
        format!("{}w", secs / 604800)
    }
}

/// Format a number with K/M suffix for readability
fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }

    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn uuid_v4() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);

    format!("{:016x}{:016x}", hasher.finish(), now())
}
