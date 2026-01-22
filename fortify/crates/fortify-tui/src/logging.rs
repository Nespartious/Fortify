//! Log entry types and buffer management

use chrono::{DateTime, Utc};
use ratatui::style::Color;
use std::collections::VecDeque;

// ============================================================================
// Status Dashboard Types
// ============================================================================

/// Status of a system component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentStatus {
    #[default]
    Pending,
    Starting,
    Running,
    Warning,
    Error,
    Stopped,
}

impl ComponentStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            ComponentStatus::Pending => "○",
            ComponentStatus::Starting => "◐",
            ComponentStatus::Running => "●",
            ComponentStatus::Warning => "◐",
            ComponentStatus::Error => "✗",
            ComponentStatus::Stopped => "○",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ComponentStatus::Pending => Color::DarkGray,
            ComponentStatus::Starting => Color::Yellow,
            ComponentStatus::Running => Color::Green,
            ComponentStatus::Warning => Color::Rgb(255, 165, 0), // Orange
            ComponentStatus::Error => Color::Red,
            ComponentStatus::Stopped => Color::DarkGray,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ComponentStatus::Pending => "PENDING",
            ComponentStatus::Starting => "STARTING",
            ComponentStatus::Running => "RUNNING",
            ComponentStatus::Warning => "WARNING",
            ComponentStatus::Error => "ERROR",
            ComponentStatus::Stopped => "STOPPED",
        }
    }
}

/// System-wide status for the status dashboard
#[derive(Debug, Clone, Default)]
pub struct SystemStatus {
    /// Tor daemon status
    pub tor_daemon: ComponentStatus,
    /// Gate service status
    pub gate: ComponentStatus,
    /// Controller status
    pub controller: ComponentStatus,
    /// Orchestrator counts: (active, target)
    pub orchestrators: (usize, usize),
    /// Orchestrator overall status
    pub orchestrator_status: ComponentStatus,
    /// Mirror counts: (live, standby, total)
    pub mirrors: (usize, usize, usize),
    /// Mirror overall status
    pub mirror_status: ComponentStatus,
    /// CAPTCHA pool: (current, target)
    pub captcha_pool: (usize, usize),
    /// CAPTCHA pool status
    pub captcha_status: ComponentStatus,
    /// Node counts: (healthy, threat)
    pub nodes: (usize, usize),
    /// Current deployment step: (current, total, description)
    pub deploy_step: Option<(usize, usize, String)>,
    /// Last status update timestamp
    pub last_update: Option<std::time::Instant>,
}

impl SystemStatus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update component status with automatic timestamp
    pub fn touch(&mut self) {
        self.last_update = Some(std::time::Instant::now());
    }
}

// ============================================================================
// Security Status (Attack Detection)
// ============================================================================

/// Security threat level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityLevel {
    /// No threats, normal traffic
    #[default]
    Clear,
    /// Some pending sessions, no unusual patterns
    Normal,
    /// Above-baseline new sessions
    Elevated,
    /// High unverified rate, possible probing
    Suspicious,
    /// Attack patterns detected, system coping
    Warning,
    /// Confirmed attack, high volume
    Attack,
}

impl SecurityLevel {
    pub fn color(&self) -> Color {
        match self {
            SecurityLevel::Clear => Color::Green,
            SecurityLevel::Normal => Color::Rgb(144, 238, 144), // Pale green
            SecurityLevel::Elevated => Color::Yellow,
            SecurityLevel::Suspicious => Color::Rgb(255, 165, 0), // Orange
            SecurityLevel::Warning => Color::Rgb(255, 100, 100),  // Pale red
            SecurityLevel::Attack => Color::Red,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SecurityLevel::Clear => "Clear",
            SecurityLevel::Normal => "Normal",
            SecurityLevel::Elevated => "Elevated",
            SecurityLevel::Suspicious => "Suspicious",
            SecurityLevel::Warning => "Warning",
            SecurityLevel::Attack => "ATTACK",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            SecurityLevel::Clear => "●",
            SecurityLevel::Normal => "●",
            SecurityLevel::Elevated => "◐",
            SecurityLevel::Suspicious => "◐",
            SecurityLevel::Warning => "⚠",
            SecurityLevel::Attack => "🔴",
        }
    }

    /// Returns true if security level is elevated or higher
    pub fn is_elevated(&self) -> bool {
        matches!(
            self,
            SecurityLevel::Elevated
                | SecurityLevel::Suspicious
                | SecurityLevel::Warning
                | SecurityLevel::Attack
        )
    }
}

/// Security status tracking with rolling counters
/// Uses bucket-based counting for O(1) updates
#[derive(Debug, Clone)]
pub struct SecurityStatus {
    /// Current threat level
    pub level: SecurityLevel,
    /// New sessions in current 30-second bucket
    pub new_sessions_current: u32,
    /// New sessions in previous 30-second bucket
    pub new_sessions_previous: u32,
    /// Unverified requests in current 30-second bucket
    pub unverified_requests_current: u32,
    /// Unverified requests in previous 30-second bucket
    pub unverified_requests_previous: u32,
    /// Sessions that solved CAPTCHA (resolved)
    pub resolved_sessions: u32,
    /// Failed CAPTCHA attempts
    pub failed_captcha_attempts: u32,
    /// Last bucket swap time
    pub last_bucket_swap: std::time::Instant,
    /// Suspicious pattern flags detected
    pub suspicious_flags: Vec<String>,
}

impl Default for SecurityStatus {
    fn default() -> Self {
        Self {
            level: SecurityLevel::Clear,
            new_sessions_current: 0,
            new_sessions_previous: 0,
            unverified_requests_current: 0,
            unverified_requests_previous: 0,
            resolved_sessions: 0,
            failed_captcha_attempts: 0,
            last_bucket_swap: std::time::Instant::now(),
            suspicious_flags: Vec::new(),
        }
    }
}

impl SecurityStatus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Swap buckets if 30 seconds have passed
    pub fn maybe_swap_buckets(&mut self) {
        let elapsed = self.last_bucket_swap.elapsed();
        if elapsed.as_secs() >= 30 {
            // Move current to previous, reset current
            self.new_sessions_previous = self.new_sessions_current;
            self.new_sessions_current = 0;
            self.unverified_requests_previous = self.unverified_requests_current;
            self.unverified_requests_current = 0;
            self.last_bucket_swap = std::time::Instant::now();
            // Clear old suspicious flags
            self.suspicious_flags.clear();
            // Decay resolved count slowly
            self.resolved_sessions = self.resolved_sessions.saturating_sub(5);
            self.failed_captcha_attempts = self.failed_captcha_attempts.saturating_sub(2);
        }
    }

    /// Record a new session
    pub fn record_new_session(&mut self) {
        self.maybe_swap_buckets();
        self.new_sessions_current = self.new_sessions_current.saturating_add(1);
    }

    /// Record an unverified request
    pub fn record_unverified_request(&mut self) {
        self.maybe_swap_buckets();
        self.unverified_requests_current = self.unverified_requests_current.saturating_add(1);
    }

    /// Record a session that solved CAPTCHA
    pub fn record_session_resolved(&mut self) {
        self.resolved_sessions = self.resolved_sessions.saturating_add(1);
    }

    /// Record a failed CAPTCHA attempt
    pub fn record_failed_captcha(&mut self) {
        self.failed_captcha_attempts = self.failed_captcha_attempts.saturating_add(1);
    }

    /// Add a suspicious pattern flag
    pub fn add_suspicious_flag(&mut self, flag: &str) {
        if self.suspicious_flags.len() < 10 {
            self.suspicious_flags.push(flag.to_string());
        }
    }

    /// Get new sessions per minute (estimated from buckets)
    pub fn new_sessions_per_minute(&self) -> u32 {
        // Sum both buckets (represents ~1 minute of data)
        self.new_sessions_current
            .saturating_add(self.new_sessions_previous)
    }

    /// Get unverified requests per minute (estimated from buckets)
    pub fn unverified_requests_per_minute(&self) -> u32 {
        self.unverified_requests_current
            .saturating_add(self.unverified_requests_previous)
    }

    /// Compute the security level based on current metrics
    pub fn compute_level(&mut self) {
        self.maybe_swap_buckets();

        let sessions_per_min = self.new_sessions_per_minute();
        let unverified_per_min = self.unverified_requests_per_minute();
        let failed_captcha = self.failed_captcha_attempts;
        let has_suspicious_flags = !self.suspicious_flags.is_empty();

        // Thresholds (these could be configurable)
        self.level = if sessions_per_min > 100 || unverified_per_min > 500 || failed_captcha > 20 {
            SecurityLevel::Attack
        } else if sessions_per_min > 60
            || unverified_per_min > 300
            || failed_captcha > 10
            || has_suspicious_flags
        {
            SecurityLevel::Warning
        } else if sessions_per_min > 30 || unverified_per_min > 100 || failed_captcha > 5 {
            SecurityLevel::Suspicious
        } else if sessions_per_min > 10 || unverified_per_min > 30 {
            SecurityLevel::Elevated
        } else if sessions_per_min > 0 || unverified_per_min > 0 {
            SecurityLevel::Normal
        } else {
            SecurityLevel::Clear
        };
    }
}

// ============================================================================
// Network Traffic Types
// ============================================================================

/// HTTP request method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Other,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            _ => HttpMethod::Other,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DEL",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPT",
            HttpMethod::Other => "???",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            HttpMethod::Get => Color::Green,
            HttpMethod::Post => Color::Cyan,
            HttpMethod::Put => Color::Yellow,
            HttpMethod::Delete => Color::Red,
            _ => Color::DarkGray,
        }
    }
}

/// HTTP response status category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Success,     // 2xx
    Redirect,    // 3xx
    ClientError, // 4xx
    ServerError, // 5xx
    Pending,     // In progress
}

impl ResponseStatus {
    pub fn from_code(code: u16) -> Self {
        match code {
            200..=299 => ResponseStatus::Success,
            300..=399 => ResponseStatus::Redirect,
            400..=499 => ResponseStatus::ClientError,
            500..=599 => ResponseStatus::ServerError,
            _ => ResponseStatus::Pending,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ResponseStatus::Success => Color::Green,
            ResponseStatus::Redirect => Color::Cyan,
            ResponseStatus::ClientError => Color::Yellow,
            ResponseStatus::ServerError => Color::Red,
            ResponseStatus::Pending => Color::DarkGray,
        }
    }
}

/// Session trust level for traffic categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionTrust {
    #[default]
    Unknown, // Not yet determined
    Verified, // Passed verification, trusted
    Threat,   // Failed verification or suspicious
}

/// Session entry with trust level and last seen time
#[derive(Debug, Clone)]
pub struct SessionEntry {
    /// Trust level for this session
    pub trust: SessionTrust,
    /// Last time this session was seen
    pub last_seen: std::time::Instant,
}

impl SessionEntry {
    pub fn new(trust: SessionTrust) -> Self {
        Self {
            trust,
            last_seen: std::time::Instant::now(),
        }
    }

    pub fn update_trust(&mut self, trust: SessionTrust) {
        self.trust = trust;
        self.last_seen = std::time::Instant::now();
    }

    pub fn touch(&mut self) {
        self.last_seen = std::time::Instant::now();
    }
}

/// A single network traffic event (or aggregated asset bundle)
#[derive(Debug, Clone)]
pub struct NetworkEvent {
    /// Timestamp of request
    pub timestamp: DateTime<Utc>,
    /// Session ID (short hash)
    pub session_id: String,
    /// HTTP method
    pub method: HttpMethod,
    /// Request path (or aggregated description like "[5 assets]")
    pub path: String,
    /// Response status code
    pub status_code: Option<u16>,
    /// Response status category
    pub status: ResponseStatus,
    /// Request duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Response size in bytes
    pub size_bytes: Option<usize>,
    /// Source mirror (truncated onion address)
    pub mirror: Option<String>,
    /// If this is an aggregated asset bundle
    pub is_asset_bundle: bool,
    /// Number of assets in bundle (if aggregated)
    pub asset_count: usize,
    /// Session trust level
    pub trust: SessionTrust,
}

/// Static asset file extensions to aggregate
const ASSET_EXTENSIONS: &[&str] = &[
    ".webp", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".bmp", ".avif", ".woff", ".woff2",
    ".ttf", ".eot", ".otf", ".css", ".map",
];

impl NetworkEvent {
    pub fn new(session_id: &str, method: HttpMethod, path: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            session_id: session_id.to_string(),
            method,
            path: path.to_string(),
            status_code: None,
            status: ResponseStatus::Pending,
            duration_ms: None,
            size_bytes: None,
            mirror: None,
            is_asset_bundle: false,
            asset_count: 1,
            trust: SessionTrust::Unknown,
        }
    }

    /// Check if this event is for a static asset that should be bundled
    pub fn is_static_asset(&self) -> bool {
        let path_lower = self.path.to_lowercase();
        ASSET_EXTENSIONS.iter().any(|ext| path_lower.ends_with(ext))
    }

    /// Merge another asset event into this bundle
    pub fn merge_asset(&mut self, other: &NetworkEvent) {
        self.is_asset_bundle = true;
        self.asset_count += other.asset_count;

        // Sum sizes
        match (self.size_bytes, other.size_bytes) {
            (Some(a), Some(b)) => self.size_bytes = Some(a + b),
            (None, Some(b)) => self.size_bytes = Some(b),
            _ => {}
        }

        // Take max duration (parallel loads)
        match (self.duration_ms, other.duration_ms) {
            (Some(a), Some(b)) => self.duration_ms = Some(a.max(b)),
            (None, Some(b)) => self.duration_ms = Some(b),
            _ => {}
        }

        // Update path to show bundle info
        self.path = format!("[{} assets]", self.asset_count);

        // Use most recent timestamp
        if other.timestamp > self.timestamp {
            self.timestamp = other.timestamp;
        }
    }

    /// Format path for display (truncate if too long)
    pub fn display_path(&self, max_len: usize) -> String {
        if self.path.len() <= max_len {
            self.path.clone()
        } else {
            format!("{}…", &self.path[..max_len - 1])
        }
    }

    /// Format session ID for display (first 8 chars)
    pub fn display_session(&self) -> &str {
        if self.session_id.len() > 8 {
            &self.session_id[..8]
        } else {
            &self.session_id
        }
    }

    /// Format duration for display
    pub fn display_duration(&self) -> String {
        match self.duration_ms {
            Some(ms) if ms >= 1000 => format!("{:.1}s", ms as f64 / 1000.0),
            Some(ms) => format!("{}ms", ms),
            None => "---".to_string(),
        }
    }

    /// Format size for display
    pub fn display_size(&self) -> String {
        match self.size_bytes {
            Some(bytes) if bytes >= 1024 * 1024 => {
                format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
            }
            Some(bytes) if bytes >= 1024 => format!("{:.1}KB", bytes as f64 / 1024.0),
            Some(bytes) => format!("{}B", bytes),
            None => "---".to_string(),
        }
    }
}

/// Buffer for network events (similar to LogBuffer)
#[derive(Debug)]
pub struct NetworkEventBuffer {
    events: VecDeque<NetworkEvent>,
    capacity: usize,
}

impl NetworkEventBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, event: NetworkEvent) {
        // Deduplicate: skip if we have same (session_id, path) within last 2 seconds
        // This prevents duplicate entries from session activity + routing logs
        let dominated = self.events.iter().rev().take(10).any(|e| {
            e.session_id == event.session_id
                && e.path == event.path
                && (event.timestamp - e.timestamp).num_seconds().abs() < 2
        });
        if dominated {
            return; // Skip duplicate
        }

        // Check if this is a static asset that should be bundled with the previous event
        if event.is_static_asset() {
            // Look for an existing asset bundle from the same session in recent events
            // (check last 5 events to allow for some interleaving)
            let bundle_idx = self.events.iter().rev().take(5).position(|e| {
                e.session_id == event.session_id && (e.is_asset_bundle || e.is_static_asset())
            });

            if let Some(rev_idx) = bundle_idx {
                // Convert reverse index to forward index
                let idx = self.events.len() - 1 - rev_idx;
                if let Some(existing) = self.events.get_mut(idx) {
                    existing.merge_asset(&event);
                    return; // Merged, don't add new event
                }
            }
        }

        // Not an asset or no bundle to merge with - add as new event
        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get recent events (most recent first, limited)
    pub fn recent(&self, count: usize) -> Vec<&NetworkEvent> {
        self.events.iter().rev().take(count).collect()
    }

    /// Get all events (oldest first)
    pub fn all(&self) -> impl Iterator<Item = &NetworkEvent> {
        self.events.iter()
    }
}

// ============================================================================
// Log Entry Types (existing)
// ============================================================================

/// Log severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn symbol(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRC",
            LogLevel::Debug => "DBG",
            LogLevel::Info => "INF",
            LogLevel::Warn => "WRN",
            LogLevel::Error => "ERR",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            LogLevel::Trace => Color::DarkGray,
            LogLevel::Debug => Color::Gray,
            LogLevel::Info => Color::Green,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// A single log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, source: &str, message: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            source: source.to_string(),
            message: message.to_string(),
        }
    }

    pub fn trace(message: &str) -> Self {
        Self::new(LogLevel::Trace, "tui", message)
    }

    pub fn debug(message: &str) -> Self {
        Self::new(LogLevel::Debug, "tui", message)
    }

    pub fn info(message: &str) -> Self {
        Self::new(LogLevel::Info, "tui", message)
    }

    pub fn warn(message: &str) -> Self {
        Self::new(LogLevel::Warn, "tui", message)
    }

    pub fn error(message: &str) -> Self {
        Self::new(LogLevel::Error, "tui", message)
    }

    pub fn from_source(level: LogLevel, source: &str, message: &str) -> Self {
        Self::new(level, source, message)
    }

    /// Format as terminal-style log line
    pub fn format(&self) -> String {
        format!(
            "{} {} [{}] {}",
            self.timestamp.format("%H:%M:%S%.3f"),
            self.level.symbol(),
            self.source,
            self.message
        )
    }
}

/// Circular buffer for log entries
#[derive(Debug)]
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entries filtered by level
    pub fn filtered(&self, min_level: LogLevel) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level >= min_level)
            .collect()
    }

    /// Get last N entries
    pub fn tail(&self, n: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(n).rev().collect()
    }

    /// Get entries with scroll offset
    pub fn scroll(&self, offset: usize, count: usize, min_level: LogLevel) -> Vec<&LogEntry> {
        let filtered: Vec<_> = self.filtered(min_level);
        let len = filtered.len();

        if len == 0 || offset >= len {
            return vec![];
        }

        let start = len.saturating_sub(offset + count);
        let end = len.saturating_sub(offset);

        filtered[start..end].to_vec()
    }

    /// Iterate all entries
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // Match ANSI escape sequences: ESC[...m for colors/styles
    // Pattern: \x1b (or \u001b) followed by [ and ending with m
    // Also handles bare [0m, [2m, [32m etc. that may appear without ESC
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' || c == '\u{001b}' {
            // Skip escape sequence: ESC [ ... m
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until 'm' or end of string
                while let Some(c) = chars.next() {
                    if c == 'm' {
                        break;
                    }
                }
            }
        } else if c == '[' {
            // Check if this is a bare ANSI code like [0m or [32m
            let mut is_ansi = true;
            let mut peek_chars: Vec<char> = Vec::new();

            // Peek ahead to check pattern: digits followed by 'm'
            loop {
                match chars.peek() {
                    Some(&d) if d.is_ascii_digit() => {
                        peek_chars.push(*chars.peek().unwrap());
                        chars.next();
                    }
                    Some(&';') => {
                        // Multiple codes like [0;32m
                        peek_chars.push(*chars.peek().unwrap());
                        chars.next();
                    }
                    Some(&'m') if !peek_chars.is_empty() => {
                        // Found [NNm pattern - skip the 'm' and continue
                        chars.next();
                        break;
                    }
                    _ => {
                        // Not an ANSI code, put chars back
                        is_ansi = false;
                        break;
                    }
                }
            }

            if !is_ansi {
                // Not ANSI, output the '[' and any peeked chars
                result.push('[');
                for ch in peek_chars {
                    result.push(ch);
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse log lines from child process stdout
pub fn parse_log_line(line: &str) -> Option<LogEntry> {
    // Try to parse structured log format: "2026-01-16T16:49:29.506004Z  INFO fortify_orchestrator: Message"
    // Also handles: "Jan 16 18:25:26.605 [notice] Bootstrapped 100%"

    // First strip any ANSI escape codes
    let line = strip_ansi_codes(line);
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Skip very short lines that are just fragments from ANSI formatting
    // These are typically bare values like "true", "sigilll", "30"
    if line.len() < 10 {
        // Allow Tor percentage lines like "10%" and status messages
        if !line.ends_with('%') && !line.contains("OK") && !line.contains("Done") {
            return None;
        }
    }

    // Look for level indicators
    let level = if line.contains(" ERROR ") || line.contains("ERR") || line.contains("[err]") {
        LogLevel::Error
    } else if line.contains(" WARN ") || line.contains("WRN") || line.contains("[warn]") {
        LogLevel::Warn
    } else if line.contains(" INFO ") || line.contains("INF") || line.contains("[notice]") {
        LogLevel::Info
    } else if line.contains(" DEBUG ") || line.contains("DBG") || line.contains("[debug]") {
        LogLevel::Debug
    } else if line.contains(" TRACE ") || line.contains("TRC") {
        LogLevel::Trace
    } else {
        LogLevel::Info
    };

    // Skip DEBUG and TRACE level logs to reduce noise in TUI
    if level == LogLevel::Debug || level == LogLevel::Trace {
        return None;
    }

    // Skip noisy patterns that don't provide useful info
    let noisy_patterns = [
        "Found binary",
        "target/release/",
        "target/debug/",
        "enabled=false, prefix=''",
        "OrchestratorConfig {",
        "resource-usage", // Filter out periodic resource monitoring logs
    ];
    for pattern in noisy_patterns {
        if line.contains(pattern) {
            return None;
        }
    }

    // Try to extract source
    let source = if let Some(start) = line.find("fortify_") {
        let end = line[start..].find(':').unwrap_or(20);
        &line[start..start + end.min(30)]
    } else if line.contains("Tor") || line.contains("[notice]") || line.contains("Bootstrapped") {
        "tor"
    } else {
        "system"
    };

    // Extract message, stripping timestamp and level info
    // Format 1: "2026-01-16T16:49:29.506004Z  INFO fortify_orchestrator: Message"
    // Format 2: "Jan 16 18:25:26.605 [notice] Message"
    let message = extract_message(line);

    Some(LogEntry::from_source(level, source, &message))
}

/// Extract just the message part, stripping timestamps and log level indicators
fn extract_message(line: &str) -> String {
    // Try to find message after "fortify_*: "
    if let Some(idx) = line.find("fortify_") {
        if let Some(colon_idx) = line[idx..].find(": ") {
            return line[idx + colon_idx + 2..].to_string();
        }
    }

    // Try to find message after "[notice] ", "[warn] ", etc.
    for marker in &["[notice] ", "[warn] ", "[err] ", "[debug] "] {
        if let Some(idx) = line.find(marker) {
            return line[idx + marker.len()..].to_string();
        }
    }

    // Try to find message after " INFO ", " WARN ", etc.
    for marker in &[" INFO ", " WARN ", " ERROR ", " DEBUG ", " TRACE "] {
        if let Some(idx) = line.find(marker) {
            // Skip to after the source: part
            let rest = &line[idx + marker.len()..];
            if let Some(colon_idx) = rest.find(": ") {
                return rest[colon_idx + 2..].to_string();
            }
            return rest.to_string();
        }
    }

    // If line starts with timestamp pattern, try to strip it
    // Pattern: "Jan 16 18:25:26.605" or "2026-01-16T18:25:26"
    if line.len() > 24 {
        // Check for ISO timestamp
        if line.chars().nth(4) == Some('-') && line.chars().nth(10) == Some('T') {
            // ISO format: skip past "2026-01-16T16:49:29.506004Z  "
            if let Some(z_idx) = line.find('Z') {
                let rest = line[z_idx + 1..].trim_start();
                if !rest.is_empty() {
                    return rest.to_string();
                }
            }
        }
        // Check for "Mon DD HH:MM:SS" format
        let first_word: String = line.chars().take_while(|c| c.is_alphabetic()).collect();
        if matches!(
            first_word.as_str(),
            "Jan"
                | "Feb"
                | "Mar"
                | "Apr"
                | "May"
                | "Jun"
                | "Jul"
                | "Aug"
                | "Sep"
                | "Oct"
                | "Nov"
                | "Dec"
        ) {
            // Find the space after the time (after .NNN or after SS)
            // "Jan 16 18:25:26.605 [notice] message"
            if let Some(bracket_idx) = line.find('[') {
                if let Some(close_idx) = line[bracket_idx..].find(']') {
                    let after_bracket = &line[bracket_idx + close_idx + 1..].trim_start();
                    if !after_bracket.is_empty() {
                        return after_bracket.to_string();
                    }
                }
            }
            // No brackets, just take everything after the time
            if line.len() > 20 {
                return line[20..].trim_start().to_string();
            }
        }
    }

    // Fallback: return the original line
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer() {
        let mut buf = LogBuffer::new(3);
        buf.push(LogEntry::info("one"));
        buf.push(LogEntry::info("two"));
        buf.push(LogEntry::info("three"));
        buf.push(LogEntry::info("four"));

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.entries.front().unwrap().message, "two");
    }

    #[test]
    fn test_parse_log() {
        let line = "2026-01-16T16:49:29.506004Z  INFO fortify_orchestrator: Starting";
        let entry = parse_log_line(line).unwrap();
        assert_eq!(entry.level, LogLevel::Info);
    }
}
