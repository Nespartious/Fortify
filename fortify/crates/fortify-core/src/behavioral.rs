//! Behavioral Analysis Engine for Fortify
//!
//! Detects suspicious request patterns without JavaScript or fingerprinting.
//! Designed for Tor Browser "safest" mode compatibility.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

/// All known attack path patterns that can be detected
/// Each tuple is (pattern, description, category)
pub const KNOWN_ATTACK_PATHS: &[(&str, &str, &str)] = &[
    ("../", "Path traversal attempt", "traversal"),
    ("..\\", "Path traversal attempt (Windows)", "traversal"),
    ("/.env", "Environment file access", "config"),
    ("/.git", "Git directory access", "vcs"),
    ("/.svn", "SVN directory access", "vcs"),
    ("/.htaccess", "Htaccess access", "config"),
    ("/.htpasswd", "Htpasswd access", "config"),
    ("/wp-admin", "WordPress admin probe", "cms"),
    ("/wp-login", "WordPress login probe", "cms"),
    ("/wp-content", "WordPress content probe", "cms"),
    ("/phpmyadmin", "phpMyAdmin probe", "admin"),
    ("/admin", "Admin panel probe", "admin"),
    ("/administrator", "Admin panel probe", "admin"),
    ("/config.", "Config file probe", "config"),
    ("/backup", "Backup file probe", "sensitive"),
    ("/.sql", "SQL file probe", "sensitive"),
    ("/dump", "Dump file probe", "sensitive"),
    ("/debug", "Debug endpoint probe", "debug"),
    ("/test", "Test endpoint probe", "debug"),
    ("/phpinfo", "PHP info probe", "debug"),
    ("/server-status", "Server status probe", "debug"),
    ("/shell", "Shell access probe", "exploit"),
    ("/cmd", "Command execution probe", "exploit"),
    ("/eval", "Eval probe", "exploit"),
    ("/exec", "Exec probe", "exploit"),
];

/// Types of behavioral violations detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationType {
    /// Non-Tor or bot User-Agent detected
    SuspiciousUserAgent,
    /// Unusual referer header (external/suspicious source)
    SuspiciousReferer,
    /// Sequential path scanning detected
    PathEnumeration,
    /// Known attack path accessed (../,  /.env, /admin, etc)
    AttackPathAccess,
    /// Rapid unique path requests (resource enumeration)
    ResourceEnumeration,
    /// Excessive form submissions in short time
    FormSubmissionFlood,
    /// Abnormally large payload size
    OversizedPayload,
    /// Abnormally small payload for endpoint type
    UndersizedPayload,
    /// Session exhibiting automated behavior patterns
    AutomatedBehavior,
}

impl ViolationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViolationType::SuspiciousUserAgent => "Suspicious User-Agent",
            ViolationType::SuspiciousReferer => "Suspicious Referer",
            ViolationType::PathEnumeration => "Path Enumeration",
            ViolationType::AttackPathAccess => "Attack Path Access",
            ViolationType::ResourceEnumeration => "Resource Enumeration",
            ViolationType::FormSubmissionFlood => "Form Submission Flood",
            ViolationType::OversizedPayload => "Oversized Payload",
            ViolationType::UndersizedPayload => "Undersized Payload",
            ViolationType::AutomatedBehavior => "Automated Behavior",
        }
    }

    pub fn severity(&self) -> u8 {
        match self {
            ViolationType::AttackPathAccess | ViolationType::AutomatedBehavior => 3,
            ViolationType::SuspiciousUserAgent
            | ViolationType::PathEnumeration
            | ViolationType::ResourceEnumeration
            | ViolationType::FormSubmissionFlood => 2,
            ViolationType::SuspiciousReferer
            | ViolationType::OversizedPayload
            | ViolationType::UndersizedPayload => 1,
        }
    }
}

/// A recorded behavioral violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorViolation {
    pub violation_type: ViolationType,
    pub timestamp: u64,
    pub details: String,
    pub severity: u8,
}

impl BehaviorViolation {
    pub fn new(violation_type: ViolationType, details: String) -> Self {
        Self {
            violation_type,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            details,
            severity: violation_type.severity(),
        }
    }
}

/// Per-session behavioral statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorStats {
    /// Total requests analyzed
    pub requests_analyzed: u64,
    /// Violations by type with counts
    pub violations_by_type: HashMap<String, u64>,
    /// Recent violations (last 50)
    pub recent_violations: VecDeque<BehaviorViolation>,
    /// Unique paths accessed this session
    pub unique_paths_count: u64,
    /// Form submissions count
    pub form_submissions: u64,
    /// Total payload bytes received
    pub total_payload_bytes: u64,
    /// Suspicious UA detected
    pub suspicious_ua_detected: bool,
    /// Last activity timestamp
    pub last_activity: u64,
}

impl BehaviorStats {
    pub fn new() -> Self {
        Self {
            recent_violations: VecDeque::with_capacity(50),
            ..Default::default()
        }
    }

    pub fn record_violation(&mut self, violation: BehaviorViolation) {
        let type_key = violation.violation_type.as_str().to_string();
        *self.violations_by_type.entry(type_key).or_insert(0) += 1;

        self.recent_violations.push_back(violation);
        while self.recent_violations.len() > 50 {
            self.recent_violations.pop_front();
        }
    }

    pub fn total_violations(&self) -> u64 {
        self.violations_by_type.values().sum()
    }

    pub fn severity_score(&self) -> u64 {
        self.recent_violations
            .iter()
            .map(|v| u64::from(v.severity))
            .sum()
    }
}

/// Configuration for behavioral analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct BehaviorConfig {
    /// Enable User-Agent analysis
    pub ua_analysis_enabled: bool,
    /// Enable Referer analysis
    pub referer_analysis_enabled: bool,
    /// Enable path pattern detection
    pub path_analysis_enabled: bool,
    /// Enable resource enumeration detection
    pub enumeration_detection_enabled: bool,
    /// Enable form submission tracking
    pub form_tracking_enabled: bool,
    /// Enable payload size analysis
    pub payload_analysis_enabled: bool,

    /// Maximum unique paths before flagging enumeration
    pub max_unique_paths_per_minute: u32,
    /// Maximum form submissions per minute
    pub max_form_submissions_per_minute: u32,
    /// Maximum payload size in bytes (default 10MB)
    pub max_payload_size: usize,
    /// Minimum expected payload for POST (bytes)
    pub min_post_payload_size: usize,
    /// Sequential path threshold (paths in numeric sequence)
    pub sequential_path_threshold: u32,

    /// Whitelisted paths that won't trigger attack path violations
    /// Supports exact matches and prefix matches (ending with *)
    /// DEPRECATED: Use disabled_attack_paths and custom_whitelist_paths instead
    pub whitelisted_paths: Vec<String>,

    /// Attack path patterns that are DISABLED (won't trigger violations)
    /// Contains patterns from KNOWN_ATTACK_PATHS that the admin has turned off
    pub disabled_attack_paths: HashSet<String>,

    /// Custom whitelist paths added by admin (prefix matching with *)
    pub custom_whitelist_paths: Vec<String>,

    /// Violation threshold to trigger threat node demotion (total violations)
    pub threat_demotion_threshold: u32,
    /// Severity score threshold to trigger threat node demotion
    pub threat_severity_threshold: u32,
    /// Individual violation type thresholds (type name -> threshold)
    /// When a specific violation type reaches this count, demote to threat
    pub violation_type_thresholds: HashMap<String, u32>,
    /// Maximum number of demotions before session is "killed" (orphaned permanently)
    /// When a session exceeds this, they're marked as a repeat offender
    pub max_demotions_before_kill: u32,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        let mut violation_type_thresholds = HashMap::new();
        // Default thresholds for each violation type
        violation_type_thresholds.insert("Attack Path Access".to_string(), 3);
        violation_type_thresholds.insert("Suspicious User-Agent".to_string(), 5);
        violation_type_thresholds.insert("Path Enumeration".to_string(), 3);
        violation_type_thresholds.insert("Resource Enumeration".to_string(), 3);
        violation_type_thresholds.insert("Form Submission Flood".to_string(), 3);
        violation_type_thresholds.insert("Automated Behavior".to_string(), 2);
        violation_type_thresholds.insert("Suspicious Referer".to_string(), 10);
        violation_type_thresholds.insert("Oversized Payload".to_string(), 5);
        violation_type_thresholds.insert("Undersized Payload".to_string(), 10);

        Self {
            ua_analysis_enabled: true,
            referer_analysis_enabled: true,
            path_analysis_enabled: true,
            enumeration_detection_enabled: true,
            form_tracking_enabled: true,
            payload_analysis_enabled: true,
            max_unique_paths_per_minute: 60,
            max_form_submissions_per_minute: 10,
            max_payload_size: 10 * 1024 * 1024, // 10MB
            min_post_payload_size: 1,
            sequential_path_threshold: 5,
            whitelisted_paths: vec![], // Deprecated - use disabled_attack_paths
            disabled_attack_paths: HashSet::new(), // All attack paths enabled by default
            custom_whitelist_paths: vec![
                "/api/*".to_string(),    // API prefix
                "/static/*".to_string(), // Static files
            ],
            threat_demotion_threshold: 10, // 10 total violations triggers demotion
            threat_severity_threshold: 15, // severity score of 15 triggers demotion
            violation_type_thresholds,
            max_demotions_before_kill: 3, // Kill session after 3 demotion cycles
        }
    }
}

impl BehaviorConfig {
    /// Check if an attack path pattern is enabled (not disabled by admin)
    pub fn is_attack_path_enabled(&self, pattern: &str) -> bool {
        !self.disabled_attack_paths.contains(pattern)
    }

    /// Check if a path matches custom whitelist patterns
    pub fn is_custom_whitelisted(&self, path: &str) -> bool {
        for pattern in &self.custom_whitelist_paths {
            if pattern.ends_with('*') {
                // Prefix match
                let prefix = &pattern[..pattern.len() - 1];
                if path.starts_with(prefix) {
                    return true;
                }
            } else {
                // Exact match
                if path == pattern {
                    return true;
                }
            }
        }
        // Also check legacy whitelisted_paths for backward compatibility
        for pattern in &self.whitelisted_paths {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if path.starts_with(prefix) {
                    return true;
                }
            } else if path == pattern {
                return true;
            }
        }
        false
    }

    /// Check if a path is whitelisted (legacy - use is_custom_whitelisted instead)
    pub fn is_path_whitelisted(&self, path: &str) -> bool {
        self.is_custom_whitelisted(path)
    }

    /// Check if a session should be demoted based on their stats
    pub fn should_demote_to_threat(&self, stats: &BehaviorStats) -> bool {
        // Check total violations threshold
        if stats.total_violations() >= u64::from(self.threat_demotion_threshold) {
            return true;
        }

        // Check severity score threshold
        if stats.severity_score() >= u64::from(self.threat_severity_threshold) {
            return true;
        }

        // Check individual violation type thresholds
        for (vtype, count) in &stats.violations_by_type {
            if let Some(&threshold) = self.violation_type_thresholds.get(vtype) {
                if *count >= u64::from(threshold) {
                    return true;
                }
            }
        }

        false
    }
}

/// Request metadata for behavioral analysis
#[derive(Debug, Clone)]
pub struct RequestMeta {
    pub path: String,
    pub method: String,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub content_length: usize,
    pub timestamp: u64,
}

impl RequestMeta {
    pub fn new(
        path: String,
        method: String,
        user_agent: Option<String>,
        referer: Option<String>,
        content_length: usize,
    ) -> Self {
        Self {
            path,
            method,
            user_agent,
            referer,
            content_length,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Per-session behavior analyzer
#[derive(Debug, Clone)]
pub struct SessionBehavior {
    /// Session ID
    pub session_id: String,
    /// Configuration
    config: BehaviorConfig,
    /// Statistics for this session
    pub stats: BehaviorStats,
    /// Paths accessed with timestamps
    path_history: VecDeque<(String, u64)>,
    /// Form submission timestamps
    form_timestamps: VecDeque<u64>,
    /// Unique paths seen
    unique_paths: HashSet<String>,
    /// Path timestamps for rate calculation
    path_timestamps: VecDeque<u64>,
    /// Early behavioral analysis (first 5 minutes)
    pub early_analysis: EarlyBehaviorAnalysis,
}

/// Phase 4.4: Early Behavioral Analysis
/// Tracks session behavior during the first 5 minutes with soft scoring (+1/-1)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarlyBehaviorAnalysis {
    /// Session creation timestamp
    pub created_at: u64,
    /// Analysis window duration (default: 5 minutes = 300 seconds)
    pub window_seconds: u64,
    /// Soft score: positive = good behavior, negative = suspicious
    /// Range: -100 to +100
    pub soft_score: i32,
    /// Number of good signals (human-like behavior)
    pub good_signals: u32,
    /// Number of bad signals (bot-like behavior)
    pub bad_signals: u32,
    /// Whether early analysis period has ended
    pub window_closed: bool,
    /// Final recommendation after window closes
    pub recommendation: Option<EarlyRecommendation>,
    /// Individual signal events
    pub signals: Vec<EarlySignal>,
}

/// Signal types for early behavioral analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EarlySignalType {
    // Positive signals (+1 each)
    ReasonablePacing,    // Requests spaced naturally (not instant)
    HumanLikeNavigation, // Clicks around like a human
    ValidUserAgent,      // Has a real browser UA
    HasReferer,          // Proper navigation flow
    PostWithContent,     // Forms with real content
    ReadingTime,         // Stayed on page reasonable time
    InteractivePattern,  // Mouse/click patterns suggest human

    // Negative signals (-1 each)
    InstantSequence,    // Requests too fast (<100ms)
    NoUserAgent,        // Missing UA header
    BotUserAgent,       // Known bot UA patterns
    LinearNavigation,   // Sequential paths (scraping)
    EmptyPosts,         // POST with no/tiny content
    NoReferer,          // Direct access patterns
    EnumerationPattern, // ID/path enumeration
}

/// An individual early signal event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlySignal {
    pub signal_type: EarlySignalType,
    pub timestamp: u64,
    pub score_delta: i32, // +1 for good, -1 for bad
    pub context: Option<String>,
}

/// Recommendation after early analysis window closes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarlyRecommendation {
    /// High confidence human - fast-track to Verified
    PromoteToVerified,
    /// Normal behavior - standard progression
    StandardFlow,
    /// Suspicious but not conclusive - increase monitoring
    IncreasedMonitoring,
    /// Likely bot - route to threat node for captcha
    RouteToThreat,
    /// Definitely malicious - burn session
    BurnSession,
}

impl EarlyBehaviorAnalysis {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            created_at: now,
            window_seconds: 300, // 5 minutes
            soft_score: 0,
            good_signals: 0,
            bad_signals: 0,
            window_closed: false,
            recommendation: None,
            signals: Vec::with_capacity(50),
        }
    }

    /// Check if still in early analysis window
    pub fn in_window(&self, now: u64) -> bool {
        !self.window_closed && (now - self.created_at) < self.window_seconds
    }

    /// Record a positive signal (+1)
    pub fn record_good_signal(&mut self, signal_type: EarlySignalType, context: Option<String>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if !self.in_window(now) {
            return;
        }

        self.soft_score = (self.soft_score + 1).min(100);
        self.good_signals += 1;
        self.signals.push(EarlySignal {
            signal_type,
            timestamp: now,
            score_delta: 1,
            context,
        });
    }

    /// Record a negative signal (-1)
    pub fn record_bad_signal(&mut self, signal_type: EarlySignalType, context: Option<String>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if !self.in_window(now) {
            return;
        }

        self.soft_score = (self.soft_score - 1).max(-100);
        self.bad_signals += 1;
        self.signals.push(EarlySignal {
            signal_type,
            timestamp: now,
            score_delta: -1,
            context,
        });
    }

    /// Close the window and determine recommendation
    pub fn close_window(&mut self) -> EarlyRecommendation {
        if self.window_closed {
            return self
                .recommendation
                .unwrap_or(EarlyRecommendation::StandardFlow);
        }

        self.window_closed = true;

        // Determine recommendation based on soft score
        let recommendation = if self.soft_score >= 10 {
            // Strong positive signals - likely human
            EarlyRecommendation::PromoteToVerified
        } else if self.soft_score >= 0 {
            // Neutral to slightly positive - normal flow
            EarlyRecommendation::StandardFlow
        } else if self.soft_score >= -10 {
            // Slightly negative - watch closely
            EarlyRecommendation::IncreasedMonitoring
        } else if self.soft_score >= -30 {
            // Definitely suspicious - needs challenge
            EarlyRecommendation::RouteToThreat
        } else {
            // Very negative - likely malicious
            EarlyRecommendation::BurnSession
        };

        self.recommendation = Some(recommendation);
        recommendation
    }

    /// Get time remaining in window (seconds)
    pub fn time_remaining(&self) -> u64 {
        if self.window_closed {
            return 0;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let elapsed = now - self.created_at;
        self.window_seconds.saturating_sub(elapsed)
    }
}

impl SessionBehavior {
    pub fn new(session_id: String, config: BehaviorConfig) -> Self {
        Self {
            session_id,
            config,
            stats: BehaviorStats::new(),
            path_history: VecDeque::with_capacity(100),
            form_timestamps: VecDeque::with_capacity(100),
            unique_paths: HashSet::new(),
            path_timestamps: VecDeque::with_capacity(100),
            early_analysis: EarlyBehaviorAnalysis::new(),
        }
    }

    /// Analyze a request and return any violations
    pub fn analyze(&mut self, req: &RequestMeta) -> Vec<BehaviorViolation> {
        let mut violations = Vec::new();
        self.stats.requests_analyzed += 1;
        self.stats.last_activity = req.timestamp;

        // Phase 4.4: Early behavioral analysis during first 5 minutes
        self.analyze_early_behavior(req);

        // Track path
        self.path_history
            .push_back((req.path.clone(), req.timestamp));
        while self.path_history.len() > 100 {
            self.path_history.pop_front();
        }

        // Track unique paths
        let is_new_path = self.unique_paths.insert(req.path.clone());
        if is_new_path {
            self.stats.unique_paths_count += 1;
            self.path_timestamps.push_back(req.timestamp);
        }

        // User-Agent Analysis
        if self.config.ua_analysis_enabled {
            if let Some(v) = self.analyze_user_agent(req.user_agent.as_ref()) {
                violations.push(v);
            }
        }

        // Referer Analysis
        if self.config.referer_analysis_enabled {
            if let Some(v) = Self::analyze_referer(req.referer.as_ref(), &req.path) {
                violations.push(v);
            }
        }

        // Path Analysis
        if self.config.path_analysis_enabled {
            violations.extend(self.analyze_path(&req.path));
        }

        // Resource Enumeration Detection
        if self.config.enumeration_detection_enabled {
            if let Some(v) = self.detect_enumeration(req.timestamp) {
                violations.push(v);
            }
        }

        // Form Submission Tracking
        if self.config.form_tracking_enabled && req.method == "POST" {
            self.stats.form_submissions += 1;
            self.form_timestamps.push_back(req.timestamp);
            if let Some(v) = self.check_form_flood(req.timestamp) {
                violations.push(v);
            }
        }

        // Payload Analysis
        if self.config.payload_analysis_enabled {
            self.stats.total_payload_bytes += req.content_length as u64;
            violations.extend(self.analyze_payload(&req.method, req.content_length));
        }

        // Record all violations in stats
        for v in &violations {
            self.stats.record_violation(v.clone());
        }

        violations
    }

    /// Analyze User-Agent for non-Tor patterns
    fn analyze_user_agent(&mut self, ua: Option<&String>) -> Option<BehaviorViolation> {
        let ua_str = match ua {
            Some(s) => s.to_lowercase(),
            None => {
                // Missing UA is actually common in Tor safest mode, not suspicious
                return None;
            }
        };

        // Known bot/scraper patterns
        let bot_patterns = [
            "curl",
            "wget",
            "python-requests",
            "python-urllib",
            "httpie",
            "scrapy",
            "bot",
            "crawler",
            "spider",
            "scraper",
            "googlebot",
            "bingbot",
            "yandex",
            "baidu",
            "duckduck",
            "facebookexternalhit",
            "twitterbot",
            "linkedinbot",
            "slurp",
            "msnbot",
            "teoma",
            "gigabot",
            "java/",
            "perl",
            "ruby",
            "go-http-client",
            "axios",
            "node-fetch",
            "undici",
            "libwww",
            "lwp-",
            "mechanize",
            "httpclient",
            "okhttp",
            "apache-httpclient",
        ];

        for pattern in &bot_patterns {
            if ua_str.contains(pattern) {
                self.stats.suspicious_ua_detected = true;
                return Some(BehaviorViolation::new(
                    ViolationType::SuspiciousUserAgent,
                    format!("Bot pattern detected: {pattern}"),
                ));
            }
        }

        // Tor Browser typically identifies as Firefox ESR on Windows
        // Very short or unusual UA strings are suspicious
        if ua_str.len() < 20 {
            self.stats.suspicious_ua_detected = true;
            return Some(BehaviorViolation::new(
                ViolationType::SuspiciousUserAgent,
                "Abnormally short User-Agent".to_string(),
            ));
        }

        None
    }

    /// Analyze Referer header
    fn analyze_referer(referer: Option<&String>, _current_path: &str) -> Option<BehaviorViolation> {
        let Some(referer_str) = referer else {
            // Missing referer is NORMAL for Tor safest mode
            return None;
        };

        // Suspicious external referers that shouldn't link to an onion
        let suspicious_referers = [
            "google.com",
            "bing.com",
            "yahoo.com",
            "yandex.",
            "baidu.com",
            "duckduckgo.com", // Search engines shouldn't have onion in referer
            "facebook.com",
            "twitter.com",
            "reddit.com", // Social media as referer is sus
        ];

        let referer_lower = referer_str.to_lowercase();
        for pattern in &suspicious_referers {
            if referer_lower.contains(pattern) {
                return Some(BehaviorViolation::new(
                    ViolationType::SuspiciousReferer,
                    format!("External referer from: {pattern}"),
                ));
            }
        }

        // Check for referer injection attempts
        if referer_lower.contains("<script")
            || referer_lower.contains("javascript:")
            || referer_lower.contains("data:")
        {
            return Some(BehaviorViolation::new(
                ViolationType::SuspiciousReferer,
                "Potential injection in referer".to_string(),
            ));
        }

        None
    }

    /// Analyze path for attack patterns
    fn analyze_path(&self, path: &str) -> Vec<BehaviorViolation> {
        let mut violations = Vec::new();
        let path_lower = path.to_lowercase();

        // Check custom whitelist first - whitelisted paths don't trigger attack violations
        if self.config.is_custom_whitelisted(path) || self.config.is_custom_whitelisted(&path_lower)
        {
            // Still check for sequential enumeration even on whitelisted paths
            if self.path_history.len() >= self.config.sequential_path_threshold as usize
                && self.detect_sequential_paths()
            {
                violations.push(BehaviorViolation::new(
                    ViolationType::PathEnumeration,
                    "Sequential path scanning detected".to_string(),
                ));
            }
            return violations;
        }

        // Check known attack paths - only if enabled in config
        for (pattern, desc, _category) in KNOWN_ATTACK_PATHS {
            // Skip disabled patterns
            if !self.config.is_attack_path_enabled(pattern) {
                continue;
            }

            if path_lower.contains(pattern) {
                violations.push(BehaviorViolation::new(
                    ViolationType::AttackPathAccess,
                    format!("{desc}: {path}"),
                ));
                break; // One violation per request
            }
        }

        // Check for sequential path enumeration
        if self.path_history.len() >= self.config.sequential_path_threshold as usize
            && self.detect_sequential_paths()
        {
            violations.push(BehaviorViolation::new(
                ViolationType::PathEnumeration,
                "Sequential path scanning detected".to_string(),
            ));
        }

        violations
    }

    /// Detect sequential numeric path patterns (e.g., /page1, /page2, /page3)
    fn detect_sequential_paths(&self) -> bool {
        if self.path_history.len() < 3 {
            return false;
        }

        // Extract numeric suffixes from recent paths
        let recent: Vec<_> = self.path_history.iter().rev().take(10).collect();
        let mut numbers: Vec<i64> = Vec::new();

        for (path, _) in &recent {
            // Try to extract trailing number
            if let Some(num) = extract_trailing_number(path) {
                numbers.push(num);
            }
        }

        if numbers.len() < 3 {
            return false;
        }

        // Check for sequential pattern
        let mut sequential_count = 0;
        for window in numbers.windows(2) {
            if (window[0] - window[1]).abs() == 1 {
                sequential_count += 1;
            }
        }

        sequential_count >= (self.config.sequential_path_threshold - 1) as usize
    }

    /// Detect rapid resource enumeration
    #[allow(clippy::cast_possible_truncation)]
    fn detect_enumeration(&mut self, current_time: u64) -> Option<BehaviorViolation> {
        // Clean old timestamps (keep last minute)
        while let Some(&ts) = self.path_timestamps.front() {
            if current_time - ts > 60 {
                self.path_timestamps.pop_front();
            } else {
                break;
            }
        }

        let paths_per_minute = self.path_timestamps.len() as u32;
        if paths_per_minute > self.config.max_unique_paths_per_minute {
            return Some(BehaviorViolation::new(
                ViolationType::ResourceEnumeration,
                format!(
                    "{paths_per_minute} unique paths in last minute (limit: {})",
                    self.config.max_unique_paths_per_minute
                ),
            ));
        }

        None
    }

    /// Check for form submission flooding
    #[allow(clippy::cast_possible_truncation)]
    fn check_form_flood(&mut self, current_time: u64) -> Option<BehaviorViolation> {
        // Clean old timestamps
        while let Some(&ts) = self.form_timestamps.front() {
            if current_time - ts > 60 {
                self.form_timestamps.pop_front();
            } else {
                break;
            }
        }

        let submissions_per_minute = self.form_timestamps.len() as u32;
        if submissions_per_minute > self.config.max_form_submissions_per_minute {
            return Some(BehaviorViolation::new(
                ViolationType::FormSubmissionFlood,
                format!(
                    "{submissions_per_minute} form submissions in last minute (limit: {})",
                    self.config.max_form_submissions_per_minute
                ),
            ));
        }

        None
    }

    /// Analyze payload size
    fn analyze_payload(&self, method: &str, size: usize) -> Vec<BehaviorViolation> {
        let mut violations = Vec::new();

        // Check for oversized payloads
        if size > self.config.max_payload_size {
            violations.push(BehaviorViolation::new(
                ViolationType::OversizedPayload,
                format!(
                    "Payload size {} bytes exceeds limit {}",
                    size, self.config.max_payload_size
                ),
            ));
        }

        // Check for suspiciously small POST payloads (might be probing)
        if method == "POST" && size < self.config.min_post_payload_size && size == 0 {
            violations.push(BehaviorViolation::new(
                ViolationType::UndersizedPayload,
                "Empty POST body".to_string(),
            ));
        }

        violations
    }

    /// Phase 4.4: Analyze early behavior during first 5 minutes
    /// Records soft signals (+1/-1) to determine session trustworthiness
    fn analyze_early_behavior(&mut self, req: &RequestMeta) {
        // Skip if window already closed
        if !self.early_analysis.in_window(req.timestamp) {
            // Check if we need to close the window
            if !self.early_analysis.window_closed {
                self.early_analysis.close_window();
            }
            return;
        }

        // Check request pacing (time since last request)
        if let Some((_, last_timestamp)) = self.path_history.back() {
            let gap_ms = (req.timestamp - last_timestamp) * 1000;

            if gap_ms < 100 {
                // Instant sequence - bot-like
                self.early_analysis.record_bad_signal(
                    EarlySignalType::InstantSequence,
                    Some(format!("{gap_ms}ms between requests")),
                );
            } else if gap_ms > 500 && gap_ms < 30000 {
                // Reasonable pacing (0.5s to 30s) - human-like
                self.early_analysis
                    .record_good_signal(EarlySignalType::ReasonablePacing, None);
            }
        }

        // Analyze User-Agent
        match &req.user_agent {
            Some(ua) if ua.len() >= 50 => {
                // Reasonable UA length - likely real browser
                self.early_analysis
                    .record_good_signal(EarlySignalType::ValidUserAgent, None);
            }
            Some(ua) => {
                // Check for bot patterns
                let ua_lower = ua.to_lowercase();
                let bot_patterns = ["curl", "wget", "python", "bot", "crawler", "spider"];
                if bot_patterns.iter().any(|p| ua_lower.contains(p)) {
                    self.early_analysis
                        .record_bad_signal(EarlySignalType::BotUserAgent, Some(ua.clone()));
                }
            }
            None => {
                // Missing UA - slight negative but not conclusive (Tor safest mode)
                self.early_analysis
                    .record_bad_signal(EarlySignalType::NoUserAgent, None);
            }
        }

        // Analyze Referer
        if req.referer.is_some() {
            // Has proper navigation flow
            self.early_analysis
                .record_good_signal(EarlySignalType::HasReferer, None);
        } else if self.stats.requests_analyzed > 2 {
            // No referer after multiple requests - slightly suspicious
            self.early_analysis
                .record_bad_signal(EarlySignalType::NoReferer, None);
        }

        // Analyze POST requests
        if req.method == "POST" {
            if req.content_length > 50 {
                // POST with real content
                self.early_analysis.record_good_signal(
                    EarlySignalType::PostWithContent,
                    Some(format!("{} bytes", req.content_length)),
                );
            } else if req.content_length == 0 {
                // Empty POST - probing
                self.early_analysis
                    .record_bad_signal(EarlySignalType::EmptyPosts, None);
            }
        }

        // Check for enumeration patterns (sequential paths)
        if self.path_history.len() >= 3 {
            let recent_paths: Vec<&str> = self
                .path_history
                .iter()
                .rev()
                .take(5)
                .map(|(p, _)| p.as_str())
                .collect();

            // Check if paths look like enumeration (e.g., /user/1, /user/2, /user/3)
            if Self::looks_like_enumeration(&recent_paths) {
                self.early_analysis.record_bad_signal(
                    EarlySignalType::EnumerationPattern,
                    Some(recent_paths.join(" -> ")),
                );
            } else if recent_paths.len() >= 3 {
                // Natural navigation pattern
                self.early_analysis
                    .record_good_signal(EarlySignalType::HumanLikeNavigation, None);
            }
        }
    }

    /// Check if a series of paths looks like enumeration
    fn looks_like_enumeration(paths: &[&str]) -> bool {
        if paths.len() < 3 {
            return false;
        }

        // Extract numbers from paths
        let numbers: Vec<i64> = paths
            .iter()
            .filter_map(|p| extract_trailing_number(p))
            .collect();

        if numbers.len() < 3 {
            return false;
        }

        // Check for sequential pattern
        let diffs: Vec<i64> = numbers.windows(2).map(|w| (w[0] - w[1]).abs()).collect();
        diffs.iter().all(|&d| d == 1) // All consecutive
    }

    /// Get early analysis recommendation (closes window if still open)
    pub fn get_early_recommendation(&mut self) -> EarlyRecommendation {
        if self.early_analysis.window_closed {
            self.early_analysis
                .recommendation
                .unwrap_or(EarlyRecommendation::StandardFlow)
        } else {
            self.early_analysis.close_window()
        }
    }

    /// Get current behavior statistics
    pub fn get_stats(&self) -> &BehaviorStats {
        &self.stats
    }

    /// Check if session should be flagged as automated
    pub fn is_likely_automated(&self) -> bool {
        let total_violations = self.stats.total_violations();
        let severity_score = self.stats.severity_score();

        // Multiple high-severity violations suggest automation
        total_violations >= 5 || severity_score >= 10
    }

    /// Update configuration
    pub fn update_config(&mut self, config: BehaviorConfig) {
        self.config = config;
    }

    /// Get a snapshot of config for display
    pub fn get_config(&self) -> &BehaviorConfig {
        &self.config
    }
}

/// Extract trailing number from path (e.g., "/page123" -> 123)
fn extract_trailing_number(path: &str) -> Option<i64> {
    let path = path.trim_end_matches('/');
    let mut num_str = String::new();

    for c in path.chars().rev() {
        if c.is_ascii_digit() {
            num_str.insert(0, c);
        } else {
            break;
        }
    }

    if num_str.is_empty() {
        None
    } else {
        num_str.parse().ok()
    }
}

/// Global behavior analyzer managing all sessions
#[derive(Debug)]
pub struct BehaviorAnalyzer {
    /// Global configuration (can be toggled)
    pub config: BehaviorConfig,
    /// Per-session behavior tracking
    sessions: HashMap<String, SessionBehavior>,
    /// Global stats
    pub global_stats: GlobalBehaviorStats,
}

/// Global statistics across all sessions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalBehaviorStats {
    pub total_requests_analyzed: u64,
    pub total_violations: u64,
    pub violations_by_type: HashMap<String, u64>,
    pub sessions_flagged_automated: u64,
}

impl BehaviorAnalyzer {
    pub fn new(config: BehaviorConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            global_stats: GlobalBehaviorStats::default(),
        }
    }

    /// Get or create session behavior tracker
    pub fn get_or_create_session(&mut self, session_id: &str) -> &mut SessionBehavior {
        if !self.sessions.contains_key(session_id) {
            let session = SessionBehavior::new(session_id.to_string(), self.config.clone());
            self.sessions.insert(session_id.to_string(), session);
        }
        self.sessions.get_mut(session_id).unwrap()
    }

    /// Analyze a request for a session
    pub fn analyze(&mut self, session_id: &str, req: &RequestMeta) -> Vec<BehaviorViolation> {
        // First, get or create the session and run analysis
        if !self.sessions.contains_key(session_id) {
            let session = SessionBehavior::new(session_id.to_string(), self.config.clone());
            self.sessions.insert(session_id.to_string(), session);
        }

        let session = self.sessions.get_mut(session_id).unwrap();
        let violations = session.analyze(req);
        let is_automated = session.is_likely_automated();

        // Now update global stats (no longer borrowing session)
        self.global_stats.total_requests_analyzed += 1;
        for v in &violations {
            self.global_stats.total_violations += 1;
            let type_key = v.violation_type.as_str().to_string();
            *self
                .global_stats
                .violations_by_type
                .entry(type_key)
                .or_insert(0) += 1;
        }

        if is_automated {
            self.global_stats.sessions_flagged_automated += 1;
        }

        violations
    }

    /// Get session behavior stats
    pub fn get_session_stats(&self, session_id: &str) -> Option<&BehaviorStats> {
        self.sessions.get(session_id).map(|s| &s.stats)
    }

    /// Get session behavior
    pub fn get_session(&self, session_id: &str) -> Option<&SessionBehavior> {
        self.sessions.get(session_id)
    }

    /// Remove session
    pub fn remove_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Clean up old sessions
    pub fn cleanup(&mut self, max_idle_seconds: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.sessions
            .retain(|_, session| now - session.stats.last_activity < max_idle_seconds);
    }

    /// Update global config and propagate to sessions
    pub fn update_config(&mut self, config: &BehaviorConfig) {
        self.config = config.clone();
        for session in self.sessions.values_mut() {
            session.update_config(config.clone());
        }
    }

    /// Get global stats
    pub fn get_global_stats(&self) -> &GlobalBehaviorStats {
        &self.global_stats
    }

    /// Get all session IDs with their violation counts
    pub fn get_session_summary(&self) -> Vec<(String, u64)> {
        self.sessions
            .iter()
            .map(|(id, s)| (id.clone(), s.stats.total_violations()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_user_agent_detection() {
        let config = BehaviorConfig::default();
        let mut session = SessionBehavior::new("test".to_string(), config);

        let req = RequestMeta::new(
            "/".to_string(),
            "GET".to_string(),
            Some("python-requests/2.28.0".to_string()),
            None,
            0,
        );

        let violations = session.analyze(&req);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::SuspiciousUserAgent));
    }

    #[test]
    fn test_attack_path_detection() {
        let config = BehaviorConfig::default();
        let mut session = SessionBehavior::new("test".to_string(), config);

        let req = RequestMeta::new(
            "/../../../etc/passwd".to_string(),
            "GET".to_string(),
            None,
            None,
            0,
        );

        let violations = session.analyze(&req);
        assert!(violations
            .iter()
            .any(|v| v.violation_type == ViolationType::AttackPathAccess));
    }

    #[test]
    fn test_form_flood_detection() {
        let mut config = BehaviorConfig::default();
        config.max_form_submissions_per_minute = 3;
        let mut session = SessionBehavior::new("test".to_string(), config);

        // Submit forms rapidly
        for i in 0..5 {
            let req = RequestMeta::new("/submit".to_string(), "POST".to_string(), None, None, 100);
            let violations = session.analyze(&req);
            if i >= 3 {
                assert!(violations
                    .iter()
                    .any(|v| v.violation_type == ViolationType::FormSubmissionFlood));
            }
        }
    }

    #[test]
    fn test_normal_tor_browser_passes() {
        let config = BehaviorConfig::default();
        let mut session = SessionBehavior::new("test".to_string(), config);

        // Normal Tor Browser request
        let req = RequestMeta::new(
            "/page".to_string(),
            "GET".to_string(),
            Some(
                "Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0".to_string(),
            ),
            None, // No referer is normal for Tor
            0,
        );

        let violations = session.analyze(&req);
        assert!(
            violations.is_empty(),
            "Normal Tor request should not trigger violations"
        );
    }

    #[test]
    fn test_trailing_number_extraction() {
        assert_eq!(extract_trailing_number("/page123"), Some(123));
        assert_eq!(extract_trailing_number("/user/42"), Some(42));
        assert_eq!(extract_trailing_number("/about"), None);
        assert_eq!(extract_trailing_number("/item/123/"), Some(123));
    }

    #[test]
    fn test_resource_enumeration() {
        let mut config = BehaviorConfig::default();
        config.max_unique_paths_per_minute = 5;
        let mut session = SessionBehavior::new("test".to_string(), config);

        // Access many unique paths
        for i in 0..10 {
            let req = RequestMeta::new(format!("/path{}", i), "GET".to_string(), None, None, 0);
            let violations = session.analyze(&req);
            if i >= 5 {
                assert!(violations
                    .iter()
                    .any(|v| v.violation_type == ViolationType::ResourceEnumeration));
            }
        }
    }
}
