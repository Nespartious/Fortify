use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub mod bitmap_stub;
pub mod detection;
pub mod mirror;
pub mod server;
pub mod tor;
use crate::tor::TorService;

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Mirror not found: {0}")]
    MirrorNotFound(String),
    #[error("Tor configuration error: {0}")]
    TorConfigError(String),
    #[error("Mirror already burned")]
    MirrorBurned,
    #[error("Mirror is not paused")]
    MirrorNotPaused,
    #[error("No healthy mirrors available")]
    NoHealthyMirrors,
    #[error("Failed to spawn replacement")]
    SpawnFailed,
    #[error("Tor control command timed out after {0}s")]
    TorTimeout(u64),
    #[error("Tor control connection failed: {0}")]
    TorConnectionFailed(String),
}

/// Extended mirror info for admin panel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorInfo {
    pub id: String,
    pub onion_address: String,
    pub status: String,
    pub pow_enabled: bool,
    pub is_standby: bool,
    pub file_based: bool,
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Mirror state lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirrorState {
    /// Spawning new mirror
    Spawning,
    /// Active and serving traffic
    Active,
    /// Paused by admin - serves redirect page
    Paused,
    /// Under suspicion of compromise
    Suspicious,
    /// Confirmed compromised, being burned
    Burning,
    /// Burned and disabled (legacy - prefer Retiring -> Dormant flow)
    Burned,
    /// Retiring: draining sessions, showing retirement page, still accepting connections optionally
    Retiring,
    /// Dormant: daemon running but rejecting all connections, awaiting resurrection evaluation
    Dormant,
    /// Restoring: in discovery period, gradually accepting more traffic (20% -> 50% -> 100%)
    Restoring,
}

/// Restoration phase during discovery period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestorationPhase {
    /// Phase 1: Accept 20% of users (first 30 minutes)
    Phase1,
    /// Phase 2: Accept 50% of users (30-60 minutes)
    Phase2,
    /// Phase 3: Accept 100% of users (60-120 minutes, then fully restored)
    Phase3,
}

impl RestorationPhase {
    /// Get the percentage of users that should see this mirror
    pub fn visibility_percent(&self) -> u8 {
        match self {
            RestorationPhase::Phase1 => 20,
            RestorationPhase::Phase2 => 50,
            RestorationPhase::Phase3 => 100,
        }
    }

    /// Get the next phase (if any)
    pub fn next(&self) -> Option<RestorationPhase> {
        match self {
            RestorationPhase::Phase1 => Some(RestorationPhase::Phase2),
            RestorationPhase::Phase2 => Some(RestorationPhase::Phase3),
            RestorationPhase::Phase3 => None,
        }
    }
}

impl MirrorState {
    pub fn can_serve_traffic(&self) -> bool {
        matches!(
            self,
            MirrorState::Active | MirrorState::Suspicious | MirrorState::Restoring
        )
    }

    /// Whether this mirror should be visible in the discovery bar
    pub fn visible_in_discovery(&self) -> bool {
        matches!(
            self,
            MirrorState::Active
                | MirrorState::Suspicious
                | MirrorState::Retiring
                | MirrorState::Restoring
        )
    }

    /// Whether this mirror is in a transitional state (not stable)
    pub fn is_transitional(&self) -> bool {
        matches!(
            self,
            MirrorState::Spawning
                | MirrorState::Burning
                | MirrorState::Retiring
                | MirrorState::Restoring
        )
    }

    pub fn should_replace(&self) -> bool {
        matches!(self, MirrorState::Burning | MirrorState::Burned)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MirrorState::Spawning => "spawning",
            MirrorState::Active => "active",
            MirrorState::Paused => "paused",
            MirrorState::Suspicious => "suspicious",
            MirrorState::Burning => "burning",
            MirrorState::Burned => "burned",
            MirrorState::Retiring => "retiring",
            MirrorState::Dormant => "dormant",
            MirrorState::Restoring => "restoring",
        }
    }
}

/// Compromise detection signals
#[derive(Debug, Clone)]
pub struct CompromiseSignal {
    pub signal_type: SignalType,
    pub severity: f32,
    pub timestamp: u64,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    UnusualTraffic,
    TimingAnomaly,
    RepeatedFailures,
    MemoryExhaustion,
    NetworkAnomaly,
}

impl CompromiseSignal {
    pub fn new(signal_type: SignalType, severity: f32, description: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            signal_type,
            severity,
            timestamp,
            description,
        }
    }
}

/// Mirror metrics and health tracking
#[derive(Debug, Clone)]
pub struct MirrorMetrics {
    pub requests_total: u64,
    pub requests_failed: u64,
    pub bytes_transferred: u64,
    pub uptime_seconds: u64,
    pub last_request_time: Option<u64>,
    pub average_response_time_ms: f64,
    pub compromise_score: f32,
}

impl Default for MirrorMetrics {
    fn default() -> Self {
        Self {
            requests_total: 0,
            requests_failed: 0,
            bytes_transferred: 0,
            uptime_seconds: 0,
            last_request_time: None,
            average_response_time_ms: 0.0,
            compromise_score: 0.0,
        }
    }
}

impl MirrorMetrics {
    pub fn record_request(&mut self, success: bool, response_time_ms: f64, bytes: u64) {
        self.requests_total += 1;
        if !success {
            self.requests_failed += 1;
        }
        self.bytes_transferred += bytes;
        self.last_request_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        // Update running average
        let n = self.requests_total as f64;
        self.average_response_time_ms =
            (self.average_response_time_ms * (n - 1.0) + response_time_ms) / n;
    }

    pub fn failure_rate(&self) -> f64 {
        if self.requests_total == 0 {
            0.0
        } else {
            self.requests_failed as f64 / self.requests_total as f64
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.failure_rate() < 0.1 && self.compromise_score < 0.5
    }
}

/// Retirement tracking for a mirror going through the burn process
#[derive(Debug, Clone)]
pub struct RetirementInfo {
    /// When retirement was initiated
    pub started_at: u64,
    /// When drain period ends (sessions should have migrated)
    pub drain_ends_at: u64,
    /// When retirement page expires (72 hours by default)
    pub page_expires_at: u64,
    /// Whether to allow new sessions during retirement (default: false)
    pub allow_new_sessions: bool,
    /// Reason for retirement
    pub reason: RetirementReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetirementReason {
    /// Proactive rotation (scheduled)
    Proactive,
    /// Suspicious activity detected
    Suspicious,
    /// Emergency burn (active attack)
    Emergency,
    /// Admin-initiated manual burn
    Manual,
    /// Confirmed compromise (keys leaked) - will be permanently destroyed
    Compromised,
}

impl RetirementInfo {
    pub fn new(reason: RetirementReason, drain_seconds: u64, page_hours: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            started_at: now,
            drain_ends_at: now + drain_seconds,
            page_expires_at: now + (page_hours * 3600),
            allow_new_sessions: false,
            reason,
        }
    }

    /// Check if we're still in the drain period
    pub fn is_draining(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now < self.drain_ends_at
    }

    /// Check if retirement page has expired (should go dormant)
    pub fn page_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.page_expires_at
    }
}

/// Resurrection tracking for dormant mirrors being evaluated
#[derive(Debug, Clone)]
pub struct ResurrectionInfo {
    /// When mirror entered dormant state
    pub dormant_since: u64,
    /// Connection attempts observed during current/last evaluation window
    pub connection_attempts: u64,
    /// Last evaluation timestamp
    pub last_evaluation_at: Option<u64>,
    /// Current restoration phase (if restoring)
    pub restoration_phase: Option<RestorationPhase>,
    /// When restoration started (if restoring)
    pub restoration_started_at: Option<u64>,
    /// Number of times this mirror has been re-dormanted during restoration attempts
    pub restoration_abort_count: u32,
}

impl ResurrectionInfo {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            dormant_since: now,
            connection_attempts: 0,
            last_evaluation_at: None,
            restoration_phase: None,
            restoration_started_at: None,
            restoration_abort_count: 0,
        }
    }

    /// Record a connection attempt (for silent evaluation)
    pub fn record_connection_attempt(&mut self) {
        self.connection_attempts += 1;
    }

    /// Reset connection counter for new evaluation window
    pub fn reset_evaluation(&mut self) {
        self.connection_attempts = 0;
        self.last_evaluation_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    /// Start restoration process
    pub fn start_restoration(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.restoration_phase = Some(RestorationPhase::Phase1);
        self.restoration_started_at = Some(now);
    }

    /// Abort restoration and return to dormant
    pub fn abort_restoration(&mut self) {
        self.restoration_phase = None;
        self.restoration_started_at = None;
        self.restoration_abort_count += 1;
        self.dormant_since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Advance to next restoration phase
    pub fn advance_phase(&mut self) -> bool {
        if let Some(current) = self.restoration_phase {
            if let Some(next) = current.next() {
                self.restoration_phase = Some(next);
                return true;
            }
        }
        false
    }

    /// Check if fully restored (Phase 3 complete)
    pub fn is_fully_restored(&self, phase3_duration_secs: u64) -> bool {
        if let (Some(RestorationPhase::Phase3), Some(started)) =
            (self.restoration_phase, self.restoration_started_at)
        {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            // Phase 3 is the last phase, check if we've been in it long enough
            // Phase 1: 0-30min, Phase 2: 30-60min, Phase 3: 60-120min
            // So Phase 3 starts at 60min and ends at 120min
            let elapsed = now - started;
            return elapsed >= phase3_duration_secs;
        }
        false
    }
}

impl Default for ResurrectionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Mirror instance
#[derive(Debug, Clone)]
pub struct Mirror {
    pub id: String,
    pub onion_address: Option<String>,
    pub state: MirrorState,
    pub created_at: u64,
    pub metrics: MirrorMetrics,
    pub signals: Vec<CompromiseSignal>,
    pub tor_data_dir: PathBuf,
    pub tor_service_id: Option<String>,
    /// Whether Tor PoW is enabled for this mirror
    pub pow_enabled: bool,
    /// Whether this is a file-based hidden service (vs ephemeral ADD_ONION)
    pub file_based: bool,
    /// Whether this mirror is a standby (created but paused for reserve)
    pub is_standby: bool,
    /// Retirement tracking (when retiring/dormant)
    pub retirement_info: Option<RetirementInfo>,
    /// Resurrection tracking (when dormant/restoring)
    pub resurrection_info: Option<ResurrectionInfo>,
    /// Whether connections should be denied at app layer (dormant mode)
    pub deny_connections: bool,
}

impl Mirror {
    pub fn new(id: String, tor_data_dir: PathBuf) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            onion_address: None,
            state: MirrorState::Spawning,
            created_at,
            metrics: MirrorMetrics::default(),
            signals: Vec::new(),
            tor_data_dir,
            tor_service_id: None,
            pow_enabled: false,
            file_based: false,
            is_standby: false,
            retirement_info: None,
            resurrection_info: None,
            deny_connections: false,
        }
    }

    pub fn activate(&mut self, onion_address: String) {
        self.onion_address = Some(onion_address);
        self.state = MirrorState::Active;
        self.deny_connections = false;
        self.save_metadata();
    }

    /// Activate as a standby mirror (paused but ready)
    pub fn activate_as_standby(&mut self, onion_address: String) {
        self.onion_address = Some(onion_address);
        self.state = MirrorState::Paused;
        self.is_standby = true;
        self.deny_connections = false;
        self.save_metadata();
    }

    /// Save mirror metadata to disk for persistence across restarts
    pub fn save_metadata(&self) {
        let metadata_path = self.tor_data_dir.join("metadata.json");

        #[derive(Serialize)]
        struct MirrorMetadata {
            is_standby: bool,
            state: String,
            created_at: u64,
            pow_enabled: bool,
        }

        let metadata = MirrorMetadata {
            is_standby: self.is_standby,
            state: self.state.as_str().to_string(),
            created_at: self.created_at,
            pow_enabled: self.pow_enabled,
        };

        if let Ok(data) = serde_json::to_string_pretty(&metadata) {
            if let Err(e) = std::fs::write(&metadata_path, data) {
                tracing::error!("Failed to save mirror metadata for {}: {}", self.id, e);
            }
        }
    }

    /// Load mirror metadata from disk
    pub fn load_metadata(&mut self) -> bool {
        let metadata_path = self.tor_data_dir.join("metadata.json");
        if !metadata_path.exists() {
            return false;
        }

        #[derive(Deserialize)]
        struct MirrorMetadata {
            is_standby: bool,
            state: String,
            #[serde(default)]
            created_at: u64,
            #[serde(default)]
            pow_enabled: bool,
        }

        match std::fs::read_to_string(&metadata_path) {
            Ok(data) => {
                match serde_json::from_str::<MirrorMetadata>(&data) {
                    Ok(metadata) => {
                        self.is_standby = metadata.is_standby;
                        self.pow_enabled = metadata.pow_enabled;
                        if metadata.created_at > 0 {
                            self.created_at = metadata.created_at;
                        }
                        // Restore state based on saved value
                        match metadata.state.as_str() {
                            "active" => self.state = MirrorState::Active,
                            "paused" => self.state = MirrorState::Paused,
                            "standby" => {
                                self.state = MirrorState::Paused;
                                self.is_standby = true;
                            }
                            "retiring" => self.state = MirrorState::Retiring,
                            "dormant" => self.state = MirrorState::Dormant,
                            _ => {} // Keep spawning state for unknown
                        }
                        tracing::debug!(
                            "Loaded metadata for {}: standby={}, state={}",
                            self.id,
                            self.is_standby,
                            metadata.state
                        );
                        true
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse metadata for {}: {}", self.id, e);
                        false
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read metadata for {}: {}", self.id, e);
                false
            }
        }
    }

    /// Begin retirement process (graceful burn)
    pub fn begin_retirement(
        &mut self,
        reason: RetirementReason,
        drain_seconds: u64,
        page_hours: u64,
    ) {
        self.state = MirrorState::Retiring;
        self.retirement_info = Some(RetirementInfo::new(reason, drain_seconds, page_hours));
        self.deny_connections = false; // Still serving during retirement
        tracing::info!(
            "Mirror {} entering retirement (reason: {:?})",
            self.id,
            reason
        );
    }

    /// Transition from retiring to dormant (after retirement page expires)
    pub fn go_dormant(&mut self) {
        self.state = MirrorState::Dormant;
        self.resurrection_info = Some(ResurrectionInfo::new());
        self.deny_connections = true; // Reject all connections at app layer
        tracing::info!("Mirror {} is now dormant (connections denied)", self.id);
    }

    /// Begin restoration process (from dormant to gradually accepting traffic)
    pub fn begin_restoration(&mut self) {
        if let Some(ref mut info) = self.resurrection_info {
            info.start_restoration();
        } else {
            let mut info = ResurrectionInfo::new();
            info.start_restoration();
            self.resurrection_info = Some(info);
        }
        self.state = MirrorState::Restoring;
        self.deny_connections = false; // Accept connections again
        tracing::info!(
            "Mirror {} beginning restoration (Phase 1: 20% visibility)",
            self.id
        );
    }

    /// Abort restoration and return to dormant
    pub fn abort_restoration(&mut self) {
        if let Some(ref mut info) = self.resurrection_info {
            info.abort_restoration();
        }
        self.state = MirrorState::Dormant;
        self.deny_connections = true;
        tracing::warn!(
            "Mirror {} restoration aborted, returning to dormant",
            self.id
        );
    }

    /// Complete restoration and return to active
    pub fn complete_restoration(&mut self) {
        self.state = MirrorState::Active;
        self.retirement_info = None;
        self.resurrection_info = None;
        self.deny_connections = false;
        tracing::info!("Mirror {} fully restored to active", self.id);
    }

    /// Permanently destroy mirror (for confirmed compromises)
    pub fn permanent_destroy(&mut self) {
        self.state = MirrorState::Burned;
        self.deny_connections = true;
        self.retirement_info = None;
        self.resurrection_info = None;
        tracing::warn!("Mirror {} permanently destroyed", self.id);
    }

    /// Check if this mirror should accept a given connection (based on restoration phase)
    pub fn should_accept_connection(&self, random_percent: u8) -> bool {
        if self.deny_connections {
            return false;
        }

        match self.state {
            MirrorState::Active | MirrorState::Suspicious => true,
            MirrorState::Retiring => {
                // Check if we allow new sessions during retirement
                if let Some(ref info) = self.retirement_info {
                    info.allow_new_sessions
                } else {
                    false
                }
            }
            MirrorState::Restoring => {
                // Check restoration phase visibility
                if let Some(ref info) = self.resurrection_info {
                    if let Some(phase) = info.restoration_phase {
                        random_percent <= phase.visibility_percent()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn add_signal(&mut self, signal: CompromiseSignal) {
        self.signals.push(signal);
        self.update_compromise_score();
    }

    fn update_compromise_score(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate weighted score from recent signals (last 5 minutes)
        let recent_signals: Vec<_> = self
            .signals
            .iter()
            .filter(|s| now - s.timestamp < 300)
            .collect();

        if recent_signals.is_empty() {
            self.metrics.compromise_score = 0.0;
            return;
        }

        let total_severity: f32 = recent_signals.iter().map(|s| s.severity).sum();
        self.metrics.compromise_score = (total_severity / recent_signals.len() as f32).min(1.0);

        // Update state based on score
        if self.metrics.compromise_score >= 0.8 && self.state == MirrorState::Active {
            self.state = MirrorState::Suspicious;
        }
    }

    pub fn burn(&mut self) {
        self.state = MirrorState::Burning;
        tracing::warn!("Mirror {} is being burned", self.id);
    }

    pub fn complete_burn(&mut self) {
        self.state = MirrorState::Burned;
        tracing::info!("Mirror {} has been burned", self.id);
    }

    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - self.created_at
    }
}

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub min_mirrors: usize,
    pub max_mirrors: usize,
    /// Number of standby mirrors to maintain (paused, ready to activate)
    pub standby_mirrors: usize,
    pub rotation_interval_seconds: u64,
    pub burn_threshold: f32,
    pub tor_data_dir: PathBuf,
    /// Base data directory for Fortify (for torrc path resolution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_data_dir: Option<PathBuf>,
    pub gate_address: String,
    pub public_bind_addr: String,
    pub proxy_port: u16,
    pub tor_control_addr: Option<String>,
    pub tor_cookie_path: Option<PathBuf>,
    /// Vanity address configuration for mirrors
    #[serde(default)]
    pub vanity_enabled: bool,
    #[serde(default)]
    pub vanity_prefix: String,
    #[serde(default = "default_vanity_timeout")]
    pub vanity_timeout: u64,
    /// Retirement configuration
    #[serde(default)]
    pub retirement: RetirementConfig,
    /// Resurrection configuration
    #[serde(default)]
    pub resurrection: ResurrectionConfig,
    /// Auto-scaling configuration
    #[serde(default)]
    pub auto_scaling: AutoScalingConfig,
    /// Self-cleaning configuration
    #[serde(default)]
    pub self_cleaning: SelfCleaningConfig,
    /// Multi-daemon architecture configuration
    #[serde(default)]
    pub multi_daemon: MultiDaemonConfig,
}

fn default_vanity_timeout() -> u64 {
    30
}

/// Configuration for mirror retirement process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementConfig {
    /// Enable proactive (scheduled) burns
    pub proactive_burn_enabled: bool,
    /// Minimum days between proactive burns
    pub burn_interval_days_min: u64,
    /// Maximum days between proactive burns
    pub burn_interval_days_max: u64,
    /// Drain period in seconds (sessions migrate away)
    pub drain_period_seconds: u64,
    /// How long to show retirement page (hours)
    pub retirement_page_hours: u64,
    /// Allow new sessions during retirement (default: false)
    pub allow_new_sessions_during_retirement: bool,
}

impl Default for RetirementConfig {
    fn default() -> Self {
        Self {
            proactive_burn_enabled: true,
            burn_interval_days_min: 60,  // 2 months
            burn_interval_days_max: 120, // 4 months
            drain_period_seconds: 3600,  // 1 hour
            retirement_page_hours: 72,   // 72 hours
            allow_new_sessions_during_retirement: false,
        }
    }
}

/// Configuration for mirror resurrection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResurrectionConfig {
    /// Enable resurrection system
    pub enabled: bool,
    /// Keep daemon running in dormant mode (deny at app layer)
    pub daemon_keep_running: bool,
    /// Wait time after burn before first evaluation (seconds)
    pub wait_after_burn_seconds: u64,
    /// Evaluation window duration (seconds)
    pub evaluation_window_seconds: u64,
    /// Connection attempts above this = attack ongoing
    pub threat_threshold_attempts: u64,
    /// Connection attempts below this = safe to restore
    pub safe_threshold_attempts: u64,
    /// Discovery period configuration
    #[serde(default)]
    pub discovery_period: DiscoveryPeriodConfig,
    /// Maximum days to keep dormant mirrors before permanent destruction
    pub max_dormant_days: u64,
}

impl Default for ResurrectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            daemon_keep_running: true,
            wait_after_burn_seconds: 900,   // 15 minutes
            evaluation_window_seconds: 300, // 5 minutes
            threat_threshold_attempts: 50,
            safe_threshold_attempts: 10,
            discovery_period: DiscoveryPeriodConfig::default(),
            max_dormant_days: 90,
        }
    }
}

/// Configuration for phased restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPeriodConfig {
    /// Total duration of discovery period (hours)
    pub total_duration_hours: u64,
    /// Phase 1 visibility percentage (first 30 min)
    pub phase1_percent: u8,
    /// Phase 2 visibility percentage (30-60 min)
    pub phase2_percent: u8,
    /// Phase 3 visibility percentage (60-120 min)
    pub phase3_percent: u8,
    /// Abort restoration if attack detected
    pub abort_on_threat: bool,
}

impl Default for DiscoveryPeriodConfig {
    fn default() -> Self {
        Self {
            total_duration_hours: 2,
            phase1_percent: 20,
            phase2_percent: 50,
            phase3_percent: 100,
            abort_on_threat: true,
        }
    }
}

/// Configuration for auto-scaling mirrors and standby pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingConfig {
    /// Enable auto-scaling
    pub enabled: bool,
    /// Minimum standby pool size
    pub min_standby: usize,
    /// Maximum standby pool size
    pub max_standby: usize,
    /// Target standby pool size (will try to maintain)
    pub target_standby: usize,
    /// Enable VPS resource awareness
    pub resource_aware: bool,
    /// Maximum CPU usage before refusing to spawn (percent)
    pub max_cpu_percent: f32,
    /// Maximum memory usage before refusing to spawn (percent)
    pub max_memory_percent: f32,
    /// Minimum available memory before refusing to spawn (MB)
    pub min_memory_available_mb: u64,
    /// Self-DDOS protection: max spawns per minute
    pub max_spawns_per_minute: u32,
    /// Self-DDOS protection: max activations per minute
    pub max_activations_per_minute: u32,
    /// Spawn cooldown after resource limit hit (seconds)
    pub spawn_cooldown_seconds: u64,
    /// Auto-replenish standby pool after activation
    pub auto_replenish_standby: bool,
}

impl Default for AutoScalingConfig {
    fn default() -> Self {
        Self {
            enabled: false, // DISABLED BY DEFAULT - Admin must explicitly enable
            min_standby: 1,
            max_standby: 5,
            target_standby: 2,
            resource_aware: true,
            max_cpu_percent: 80.0,
            max_memory_percent: 85.0,
            min_memory_available_mb: 512, // 512MB minimum
            max_spawns_per_minute: 5,
            max_activations_per_minute: 10,
            spawn_cooldown_seconds: 30,
            auto_replenish_standby: true,
        }
    }
}

/// Tracks spawn/activation rate for self-DDOS protection
#[derive(Debug, Clone)]
pub struct SpawnRateLimiter {
    /// Timestamps of recent spawns
    pub spawn_times: Vec<u64>,
    /// Timestamps of recent activations
    pub activation_times: Vec<u64>,
    /// Last time we hit resource limits
    pub last_resource_limit_hit: Option<u64>,
}

impl SpawnRateLimiter {
    pub fn new() -> Self {
        Self {
            spawn_times: Vec::new(),
            activation_times: Vec::new(),
            last_resource_limit_hit: None,
        }
    }

    /// Record a spawn
    pub fn record_spawn(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.spawn_times.push(now);
        // Keep only last 60 seconds
        self.spawn_times.retain(|&t| now - t < 60);
    }

    /// Record an activation
    pub fn record_activation(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.activation_times.push(now);
        self.activation_times.retain(|&t| now - t < 60);
    }

    /// Record resource limit hit
    pub fn record_resource_limit(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_resource_limit_hit = Some(now);
    }

    /// Count spawns in last minute
    pub fn spawns_last_minute(&self) -> u32 {
        self.spawn_times.len() as u32
    }

    /// Count activations in last minute
    pub fn activations_last_minute(&self) -> u32 {
        self.activation_times.len() as u32
    }

    /// Check if we're in cooldown after resource limit
    pub fn in_cooldown(&self, cooldown_seconds: u64) -> bool {
        if let Some(last_hit) = self.last_resource_limit_hit {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now - last_hit < cooldown_seconds
        } else {
            false
        }
    }

    /// Check if spawn is allowed
    pub fn can_spawn(&self, config: &AutoScalingConfig) -> bool {
        !self.in_cooldown(config.spawn_cooldown_seconds)
            && self.spawns_last_minute() < config.max_spawns_per_minute
    }

    /// Check if activation is allowed
    pub fn can_activate(&self, config: &AutoScalingConfig) -> bool {
        self.activations_last_minute() < config.max_activations_per_minute
    }
}

impl Default for SpawnRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Phase 4.7: Configuration for self-cleaning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfCleaningConfig {
    /// Enable self-cleaning
    pub enabled: bool,
    /// Clean expired sessions every N seconds
    pub cleanup_interval_seconds: u64,
    /// Session idle timeout (seconds before cleanup)
    pub session_idle_timeout_seconds: u64,
    /// Log rotation: max log file size in MB
    pub max_log_size_mb: u64,
    /// Log rotation: number of old logs to keep
    pub log_retention_count: u32,
    /// Memory high-water mark (MB) - trigger cleanup
    pub memory_high_water_mb: u64,
    /// Remove burned mirrors after N days
    pub burned_mirror_retention_days: u64,
    /// Remove destroyed mirrors' data after N days
    pub destroyed_data_retention_days: u64,
    /// Clean temporary files older than N hours
    pub temp_file_max_age_hours: u64,
}

impl Default for SelfCleaningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cleanup_interval_seconds: 300,      // 5 minutes
            session_idle_timeout_seconds: 3600, // 1 hour
            max_log_size_mb: 100,
            log_retention_count: 10,
            memory_high_water_mb: 4096, // 4GB - reasonable for dev VMs
            burned_mirror_retention_days: 7,
            destroyed_data_retention_days: 30,
            temp_file_max_age_hours: 24,
        }
    }
}

// ========== Phase 4.8: Multi-Daemon Architecture ==========

/// Configuration for multi-daemon Tor architecture (Phase 4.8)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDaemonConfig {
    /// Enable multi-daemon mode (one Tor per CPU core)
    pub enabled: bool,
    /// Number of daemons (0 = auto-detect from CPU cores)
    pub daemons_per_vps: usize,
    /// Enable CPU affinity pinning via taskset
    pub cpu_affinity: bool,
    /// Base SocksPort (each daemon gets base + daemon_id)
    pub base_socks_port: u16,
    /// Base ControlPort (each daemon gets base + daemon_id)
    pub base_control_port: u16,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    /// Max consecutive health failures before daemon restart
    pub max_health_failures: u32,
    /// Whether to auto-restart failed daemons
    pub auto_restart_daemons: bool,
    /// Flex Core configuration (Core 2)
    pub flex_core: FlexCoreConfig,
}

impl Default for MultiDaemonConfig {
    fn default() -> Self {
        Self {
            enabled: false,     // Disabled by default, opt-in feature
            daemons_per_vps: 0, // Auto-detect
            cpu_affinity: true,
            base_socks_port: 9050,
            base_control_port: 9051,
            health_check_interval_seconds: 30,
            max_health_failures: 3,
            auto_restart_daemons: true,
            flex_core: FlexCoreConfig::default(),
        }
    }
}

// ========== Phase 4.8: 4-Core Architecture Layout ==========

/// Core role assignment for the 4-core architecture
///
/// Layout:
/// - Core 0: Mirror A + Standby D + Healthy 0-4
/// - Core 1: Mirror B + Standby C + Healthy 5-9  
/// - Core 2: Flex Core (CAPTCHA pre-gen, overflow)
/// - Core 3: Threat Nodes 0-2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreLayoutConfig {
    /// Core assignments for primary services
    pub core_assignments: Vec<CoreAssignment>,
    /// Flex core configuration
    pub flex_core: FlexCoreConfig,
}

impl Default for CoreLayoutConfig {
    fn default() -> Self {
        Self {
            core_assignments: vec![
                CoreAssignment {
                    core_id: 0,
                    role: CoreRole::Primary {
                        mirror: "mirror-a".to_string(),
                        standby: Some("standby-d".to_string()),
                        healthy_nodes: vec![0, 1, 2, 3, 4],
                    },
                },
                CoreAssignment {
                    core_id: 1,
                    role: CoreRole::Primary {
                        mirror: "mirror-b".to_string(),
                        standby: Some("standby-c".to_string()),
                        healthy_nodes: vec![5, 6, 7, 8, 9],
                    },
                },
                CoreAssignment {
                    core_id: 2,
                    role: CoreRole::Flex,
                },
                CoreAssignment {
                    core_id: 3,
                    role: CoreRole::Threat {
                        threat_nodes: vec![0, 1, 2],
                    },
                },
            ],
            flex_core: FlexCoreConfig::default(),
        }
    }
}

/// Assignment of a specific CPU core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreAssignment {
    pub core_id: usize,
    pub role: CoreRole,
}

/// Role that a core can play
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreRole {
    /// Primary core: handles a mirror + standby + healthy nodes
    Primary {
        mirror: String,
        standby: Option<String>,
        healthy_nodes: Vec<usize>,
    },
    /// Threat core: handles threat/suspicious traffic
    Threat { threat_nodes: Vec<usize> },
    /// Flex core: dynamic role based on system state
    Flex,
}

/// Configuration for the Flex Core (Core 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexCoreConfig {
    /// Current mode of the flex core
    pub default_mode: FlexCoreMode,
    /// CPU threshold for switching to emergency mode (percent)
    pub emergency_cpu_threshold: f32,
    /// Healthy node capacity threshold for overflow mode (percent)
    pub healthy_overflow_threshold: f32,
    /// Threat node capacity threshold for overflow mode (percent)
    pub threat_overflow_threshold: f32,
    /// CAPTCHA pre-generation settings
    pub captcha_pregen: CaptchaPregenConfig,
}

impl Default for FlexCoreConfig {
    fn default() -> Self {
        Self {
            default_mode: FlexCoreMode::Standby,
            emergency_cpu_threshold: 80.0,
            healthy_overflow_threshold: 90.0,
            threat_overflow_threshold: 90.0,
            captcha_pregen: CaptchaPregenConfig::default(),
        }
    }
}

/// Flex Core operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlexCoreMode {
    /// Default: Background tasks, CAPTCHA pre-generation
    Standby,
    /// Emergency: Spawn additional mirror + healthy nodes
    EmergencyMirror,
    /// Overflow: Add healthy nodes 10-14 to the pool
    HealthyOverflow,
    /// Overflow: Add threat nodes 3-5 to the pool  
    ThreatOverflow,
}

/// CAPTCHA pre-generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaPregenConfig {
    /// Enable CAPTCHA pre-generation
    pub enabled: bool,
    /// Target pool size to maintain
    pub target_pool_size: usize,
    /// Minimum pool size before refill triggers
    pub min_pool_size: usize,
    /// Maximum pool size (hard cap)
    pub max_pool_size: usize,
    /// CPU threshold above which pre-generation pauses (percent)
    pub pause_cpu_threshold: f32,
    /// Number of CAPTCHAs to generate per batch
    pub batch_size: usize,
    /// Delay between batches in milliseconds (prevent self-DDOS)
    pub batch_delay_ms: u64,
    /// Rotation: percentage of pool to replace periodically
    pub rotation_percent: u8,
    /// Rotation interval in days
    pub rotation_interval_days: u64,
}

impl Default for CaptchaPregenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_pool_size: 500,      // Target 500 pre-generated CAPTCHAs
            min_pool_size: 100,         // Refill when below 100
            max_pool_size: 1000,        // Never exceed 1000
            pause_cpu_threshold: 70.0,  // Pause pre-gen if CPU > 70%
            batch_size: 10,             // Generate 10 at a time
            batch_delay_ms: 100,        // 100ms between batches
            rotation_percent: 25,       // Replace 25% of pool
            rotation_interval_days: 10, // Every 10 days
        }
    }
}

/// Pre-generated CAPTCHA ready for use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PregenCaptcha {
    /// Unique ID for this pre-generated captcha
    pub id: String,
    /// The answer text
    pub answer: String,
    /// Pre-rendered image data (BMP bytes)
    pub image_data: Vec<u8>,
    /// When this was generated
    pub generated_at: u64,
    /// Difficulty level
    pub difficulty: u8,
}

/// Manager for pre-generated CAPTCHA pool
pub struct CaptchaPoolManager {
    config: CaptchaPregenConfig,
    pool: Arc<Mutex<Vec<PregenCaptcha>>>,
    last_rotation: Arc<Mutex<u64>>,
    /// Directory for persisting CAPTCHA pool
    pool_dir: PathBuf,
    /// Total CAPTCHAs generated since startup
    total_generated: Arc<std::sync::atomic::AtomicU64>,
    /// Total CAPTCHAs served to clients
    total_served: Arc<std::sync::atomic::AtomicU64>,
    /// Total CAPTCHAs expired during rotation
    total_expired: Arc<std::sync::atomic::AtomicU64>,
}

impl CaptchaPoolManager {
    pub fn new(config: CaptchaPregenConfig, pool_dir: PathBuf) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create pool directory if needed
        if !pool_dir.exists() {
            let _ = std::fs::create_dir_all(&pool_dir);
        }

        let mut manager = Self {
            config,
            pool: Arc::new(Mutex::new(Vec::new())),
            last_rotation: Arc::new(Mutex::new(now)),
            pool_dir,
            total_generated: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_served: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_expired: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };

        // Load existing pool from disk
        manager.load_pool();

        manager
    }

    /// Load CAPTCHA pool from disk
    fn load_pool(&mut self) {
        let pool_file = self.pool_dir.join("captcha_pool.json");
        if !pool_file.exists() {
            tracing::info!("No existing CAPTCHA pool found, starting fresh");
            return;
        }

        match std::fs::read_to_string(&pool_file) {
            Ok(data) => {
                match serde_json::from_str::<Vec<PregenCaptcha>>(&data) {
                    Ok(captchas) => {
                        let count = captchas.len();
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        // Filter out expired CAPTCHAs (older than rotation interval)
                        let max_age = self.config.rotation_interval_days * 24 * 3600;
                        let valid: Vec<PregenCaptcha> = captchas
                            .into_iter()
                            .filter(|c| now - c.generated_at < max_age)
                            .collect();

                        let valid_count = valid.len();
                        *self.pool.lock().unwrap() = valid;

                        if valid_count < count {
                            tracing::info!(
                                "Loaded {} CAPTCHAs from disk ({} expired and removed)",
                                valid_count,
                                count - valid_count
                            );
                        } else {
                            tracing::info!("Loaded {} CAPTCHAs from disk", valid_count);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse CAPTCHA pool: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read CAPTCHA pool file: {}", e);
            }
        }

        // Load rotation timestamp
        let rotation_file = self.pool_dir.join("last_rotation.txt");
        if let Ok(data) = std::fs::read_to_string(&rotation_file) {
            if let Ok(ts) = data.trim().parse::<u64>() {
                *self.last_rotation.lock().unwrap() = ts;
                tracing::debug!("Loaded last rotation timestamp: {}", ts);
            }
        }
    }

    /// Save CAPTCHA pool to disk
    pub fn save_pool(&self) {
        let pool = self.pool.lock().unwrap();
        let pool_file = self.pool_dir.join("captcha_pool.json");

        match serde_json::to_string(&*pool) {
            Ok(data) => {
                if let Err(e) = std::fs::write(&pool_file, data) {
                    tracing::error!("Failed to save CAPTCHA pool: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize CAPTCHA pool: {}", e);
            }
        }

        // Save rotation timestamp
        let rotation_file = self.pool_dir.join("last_rotation.txt");
        let ts = *self.last_rotation.lock().unwrap();
        let _ = std::fs::write(&rotation_file, ts.to_string());
    }

    /// Get the current pool size
    pub fn pool_size(&self) -> usize {
        self.pool.lock().unwrap().len()
    }

    /// Check if pool needs refilling
    pub fn needs_refill(&self) -> bool {
        self.pool_size() < self.config.min_pool_size
    }

    /// Take a pre-generated CAPTCHA from the pool
    pub fn take_captcha(&self) -> Option<PregenCaptcha> {
        let mut pool = self.pool.lock().unwrap();
        if pool.is_empty() {
            None
        } else {
            // Take from the front (oldest first)
            self.total_served
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(pool.remove(0))
        }
    }

    /// Add a pre-generated CAPTCHA to the pool
    pub fn add_captcha(&self, captcha: PregenCaptcha) {
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.config.max_pool_size {
            self.total_generated
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            pool.push(captcha);

            // Save to disk every 50 CAPTCHAs for durability
            let total = self
                .total_generated
                .load(std::sync::atomic::Ordering::Relaxed);
            if total.is_multiple_of(50) {
                drop(pool); // Release lock before saving
                self.save_pool();
            }
        }
    }

    /// Generate a batch of CAPTCHAs (called from Flex Core)
    pub fn generate_batch(&self) -> Vec<PregenCaptcha> {
        let mut batch = Vec::with_capacity(self.config.batch_size);

        for _ in 0..self.config.batch_size {
            let captcha = self.generate_single();
            batch.push(captcha);
        }

        batch
    }

    fn generate_single(&self) -> PregenCaptcha {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Generate random text (6 chars)
        let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
        let answer: String = (0..6)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect();

        // Generate ID
        let id: String = (0..16)
            .map(|_| format!("{:02x}", rng.gen::<u8>()))
            .collect();

        // Generate image (this is the CPU-intensive part)
        let image_data = crate::bitmap_stub::generate_captcha_image(&answer);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        PregenCaptcha {
            id,
            answer,
            image_data,
            generated_at: now,
            difficulty: 2, // Medium
        }
    }

    /// Check if rotation is due
    pub fn rotation_due(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let last = *self.last_rotation.lock().unwrap();
        let rotation_interval_secs = self.config.rotation_interval_days * 24 * 3600;

        now - last >= rotation_interval_secs
    }

    /// Rotate the pool: remove oldest N% and regenerate
    pub fn rotate_pool(&self) {
        let mut pool = self.pool.lock().unwrap();
        let remove_count = (pool.len() * self.config.rotation_percent as usize) / 100;

        // Remove oldest (from front) and track as expired
        for _ in 0..remove_count {
            if !pool.is_empty() {
                pool.remove(0);
                self.total_expired
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        // Update rotation timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        *self.last_rotation.lock().unwrap() = now;

        tracing::info!(
            "CAPTCHA pool rotated: removed {} old CAPTCHAs, {} remaining",
            remove_count,
            pool.len()
        );
    }

    /// Get pool statistics
    pub fn stats(&self) -> CaptchaPoolStats {
        let pool = self.pool.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let oldest_age = pool.first().map(|c| now - c.generated_at).unwrap_or(0);
        let newest_age = pool.last().map(|c| now - c.generated_at).unwrap_or(0);

        CaptchaPoolStats {
            current_size: pool.len(),
            target_size: self.config.target_pool_size,
            min_size: self.config.min_pool_size,
            max_size: self.config.max_pool_size,
            oldest_age_seconds: oldest_age,
            newest_age_seconds: newest_age,
            needs_refill: pool.len() < self.config.min_pool_size,
            total_generated: self
                .total_generated
                .load(std::sync::atomic::Ordering::Relaxed),
            total_served: self.total_served.load(std::sync::atomic::Ordering::Relaxed),
            total_expired: self
                .total_expired
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Statistics about the CAPTCHA pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaPoolStats {
    pub current_size: usize,
    pub target_size: usize,
    pub min_size: usize,
    pub max_size: usize,
    pub oldest_age_seconds: u64,
    pub newest_age_seconds: u64,
    pub needs_refill: bool,
    /// Total CAPTCHAs generated since startup
    pub total_generated: u64,
    /// Total CAPTCHAs served to clients
    pub total_served: u64,
    /// Total CAPTCHAs expired during rotation
    pub total_expired: u64,
}

/// Represents a single Tor daemon in multi-daemon mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorDaemon {
    /// Unique daemon ID (0-indexed)
    pub id: usize,
    /// CPU core this daemon is pinned to (if affinity enabled)
    pub cpu_core: Option<usize>,
    /// SocksPort for this daemon
    pub socks_port: u16,
    /// ControlPort for this daemon  
    pub control_port: u16,
    /// Process ID of the Tor daemon
    pub pid: Option<u32>,
    /// Current health status
    pub health: DaemonHealth,
    /// Consecutive health check failures
    pub health_failures: u32,
    /// Mirrors assigned to this daemon
    pub assigned_mirrors: Vec<String>,
    /// Data directory for this daemon
    pub data_dir: PathBuf,
    /// When this daemon was started
    pub started_at: u64,
    /// Total requests processed
    pub total_requests: u64,
}

/// Health status of a Tor daemon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonHealth {
    /// Daemon is starting up
    Starting,
    /// Daemon is healthy and processing requests
    Healthy,
    /// Daemon is experiencing issues but still running
    Degraded,
    /// Daemon is unresponsive
    Unhealthy,
    /// Daemon has crashed or stopped
    Dead,
    /// Daemon is being restarted
    Restarting,
}

impl TorDaemon {
    pub fn new(
        id: usize,
        config: &MultiDaemonConfig,
        tor_data_dir: &Path,
        detected_cores: usize,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Use modulo to wrap daemon ID to valid core numbers
        // This ensures CPU affinity works on any hardware (2, 4, 8, 64+ cores)
        let core_id = if detected_cores > 0 {
            id % detected_cores
        } else {
            id
        };

        Self {
            id,
            cpu_core: if config.cpu_affinity {
                Some(core_id)
            } else {
                None
            },
            socks_port: config.base_socks_port + id as u16,
            control_port: config.base_control_port + id as u16,
            pid: None,
            health: DaemonHealth::Starting,
            health_failures: 0,
            assigned_mirrors: Vec::new(),
            data_dir: tor_data_dir.join(format!("daemon_{}", id)),
            started_at: now,
            total_requests: 0,
        }
    }

    /// Check if this daemon can accept more mirrors
    pub fn can_accept_mirrors(&self, max_per_daemon: usize) -> bool {
        self.health == DaemonHealth::Healthy && self.assigned_mirrors.len() < max_per_daemon
    }
}

/// Manager for multiple Tor daemons (Phase 4.8)
pub struct MultiDaemonManager {
    config: MultiDaemonConfig,
    daemons: Arc<Mutex<Vec<TorDaemon>>>,
    tor_data_dir: PathBuf,
    detected_cores: usize,
}

impl MultiDaemonManager {
    pub fn new(config: MultiDaemonConfig, tor_data_dir: PathBuf) -> Self {
        // Detect CPU cores
        let sys = sysinfo::System::new_all();
        let detected_cores = sys.cpus().len();

        Self {
            config,
            daemons: Arc::new(Mutex::new(Vec::new())),
            tor_data_dir,
            detected_cores,
        }
    }

    /// Get the number of daemons to spawn
    pub fn daemon_count(&self) -> usize {
        if self.config.daemons_per_vps == 0 {
            self.detected_cores
        } else {
            self.config.daemons_per_vps
        }
    }

    /// Initialize all daemons (does not start them)
    pub fn initialize_daemons(&self) {
        let count = self.daemon_count();
        let mut daemons = self.daemons.lock().unwrap();
        daemons.clear();

        for i in 0..count {
            let daemon = TorDaemon::new(i, &self.config, &self.tor_data_dir, self.detected_cores);
            tracing::info!(
                "Initialized Tor daemon {} on core {:?}, socks:{}, control:{}",
                daemon.id,
                daemon.cpu_core,
                daemon.socks_port,
                daemon.control_port
            );
            daemons.push(daemon);
        }
    }

    /// Start a specific daemon
    pub async fn start_daemon(&self, daemon_id: usize) -> anyhow::Result<u32> {
        let mut daemons = self.daemons.lock().unwrap();
        let daemon = daemons
            .get_mut(daemon_id)
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        // Create data directory
        std::fs::create_dir_all(&daemon.data_dir)?;

        // Build torrc content for this daemon
        let torrc_content = format!(
            r#"# Fortify Multi-Daemon Tor Configuration (daemon {})
SocksPort {}
ControlPort {}
DataDirectory {}
Log notice file {}/notice.log
CookieAuthentication 1
"#,
            daemon.id,
            daemon.socks_port,
            daemon.control_port,
            daemon.data_dir.display(),
            daemon.data_dir.display()
        );

        // Write torrc
        let torrc_path = daemon.data_dir.join("torrc");
        std::fs::write(&torrc_path, torrc_content)?;

        // Build command with optional CPU affinity
        let mut cmd = if self.config.cpu_affinity && daemon.cpu_core.is_some() {
            let core = daemon.cpu_core.unwrap();
            let mut c = std::process::Command::new("taskset");
            c.arg("-c").arg(core.to_string()).arg("tor");
            c
        } else {
            std::process::Command::new("tor")
        };

        cmd.arg("-f").arg(&torrc_path);

        // Start the daemon
        let child = cmd.spawn()?;
        let pid = child.id();
        daemon.pid = Some(pid);
        daemon.health = DaemonHealth::Starting;

        tracing::info!(
            "Started Tor daemon {} with PID {}, core {:?}",
            daemon_id,
            pid,
            daemon.cpu_core
        );

        Ok(pid)
    }

    /// Start all daemons
    pub async fn start_all(&self) -> anyhow::Result<()> {
        let count = self.daemon_count();
        for i in 0..count {
            if let Err(e) = self.start_daemon(i).await {
                tracing::error!("Failed to start daemon {}: {}", i, e);
            }
        }
        Ok(())
    }

    /// Check health of a specific daemon
    pub async fn check_daemon_health(&self, daemon_id: usize) -> DaemonHealth {
        // Get daemon info without holding lock across await
        let (pid, control_port, max_failures) = {
            let daemons = self.daemons.lock().unwrap();
            match daemons.get(daemon_id) {
                Some(d) => (d.pid, d.control_port, self.config.max_health_failures),
                None => return DaemonHealth::Dead,
            }
        };

        // Check if process is running
        if let Some(pid) = pid {
            // Try to connect to control port
            let connect_result =
                tokio::net::TcpStream::connect(format!("127.0.0.1:{}", control_port)).await;

            match connect_result {
                Ok(_) => {
                    // Can connect, daemon is healthy
                    let mut daemons = self.daemons.lock().unwrap();
                    if let Some(d) = daemons.get_mut(daemon_id) {
                        d.health = DaemonHealth::Healthy;
                        d.health_failures = 0;
                    }
                    DaemonHealth::Healthy
                }
                Err(_) => {
                    // Check if process still exists
                    let sys = sysinfo::System::new_all();
                    let process_exists = sys.process(sysinfo::Pid::from_u32(pid)).is_some();

                    let mut daemons = self.daemons.lock().unwrap();
                    if let Some(d) = daemons.get_mut(daemon_id) {
                        if process_exists {
                            d.health_failures += 1;
                            if d.health_failures >= max_failures {
                                d.health = DaemonHealth::Unhealthy;
                            } else {
                                d.health = DaemonHealth::Degraded;
                            }
                            d.health
                        } else {
                            d.health = DaemonHealth::Dead;
                            d.pid = None;
                            DaemonHealth::Dead
                        }
                    } else {
                        DaemonHealth::Dead
                    }
                }
            }
        } else {
            DaemonHealth::Dead
        }
    }

    /// Assign a mirror to the best available daemon
    pub fn assign_mirror(&self, mirror_id: &str) -> Option<usize> {
        let mut daemons = self.daemons.lock().unwrap();

        // Find daemon with fewest mirrors that's healthy
        let best = daemons
            .iter_mut()
            .filter(|d| d.health == DaemonHealth::Healthy)
            .min_by_key(|d| d.assigned_mirrors.len());

        if let Some(daemon) = best {
            daemon.assigned_mirrors.push(mirror_id.to_string());
            tracing::info!("Assigned mirror {} to daemon {}", mirror_id, daemon.id);
            Some(daemon.id)
        } else {
            tracing::warn!("No healthy daemon available for mirror {}", mirror_id);
            None
        }
    }

    /// Remove a mirror from its assigned daemon
    pub fn unassign_mirror(&self, mirror_id: &str) {
        let mut daemons = self.daemons.lock().unwrap();
        for daemon in daemons.iter_mut() {
            daemon.assigned_mirrors.retain(|m| m != mirror_id);
        }
    }

    /// Get daemon info for a specific mirror
    pub fn get_daemon_for_mirror(&self, mirror_id: &str) -> Option<TorDaemon> {
        let daemons = self.daemons.lock().unwrap();
        daemons
            .iter()
            .find(|d| d.assigned_mirrors.contains(&mirror_id.to_string()))
            .cloned()
    }

    /// Get all daemon statuses
    pub fn get_daemon_statuses(&self) -> Vec<TorDaemon> {
        self.daemons.lock().unwrap().clone()
    }

    /// Stop a specific daemon
    pub fn stop_daemon(&self, daemon_id: usize) -> anyhow::Result<()> {
        let mut daemons = self.daemons.lock().unwrap();
        let daemon = daemons
            .get_mut(daemon_id)
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        if let Some(pid) = daemon.pid {
            // Send SIGTERM to the process
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .status();
            }
            daemon.pid = None;
            daemon.health = DaemonHealth::Dead;
            tracing::info!("Stopped daemon {} (PID {})", daemon_id, pid);
        }

        Ok(())
    }

    /// Stop all daemons
    pub fn stop_all(&self) {
        let count = self.daemon_count();
        for i in 0..count {
            let _ = self.stop_daemon(i);
        }
    }
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        // Use persistent path if HOME is set
        let base_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path
        } else {
            PathBuf::from("/tmp/fortify")
        };

        Self {
            min_mirrors: 2,
            max_mirrors: 5,
            standby_mirrors: 2,              // 2 standby mirrors ready to activate
            rotation_interval_seconds: 3600, // 1 hour
            burn_threshold: 0.7,
            tor_data_dir: base_dir.join("tor").join("mirrors"),
            base_data_dir: Some(base_dir),
            gate_address: "http://127.0.0.1:8081".to_string(),
            public_bind_addr: "0.0.0.0:8080".to_string(),
            proxy_port: 8082,
            tor_control_addr: None,
            tor_cookie_path: None,
            vanity_enabled: false,
            vanity_prefix: String::new(),
            vanity_timeout: 30,
            retirement: RetirementConfig::default(),
            resurrection: ResurrectionConfig::default(),
            auto_scaling: AutoScalingConfig::default(),
            self_cleaning: SelfCleaningConfig::default(),
            multi_daemon: MultiDaemonConfig::default(),
        }
    }
}

/// Main orchestrator manager
pub struct Orchestrator {
    config: OrchestratorConfig,
    mirrors: Arc<Mutex<HashMap<String, Mirror>>>,
    active_count: Arc<Mutex<usize>>,
    tor_service: Arc<TorService>,
    /// Rate limiter for spawn/activation self-DDOS protection
    spawn_rate_limiter: Arc<Mutex<SpawnRateLimiter>>,
    /// Multi-daemon manager for Phase 4.8
    multi_daemon_manager: Option<Arc<MultiDaemonManager>>,
    /// CAPTCHA pre-generation pool manager for Flex Core
    captcha_pool: Arc<CaptchaPoolManager>,
    /// Reserved for future public API binding
    #[allow(dead_code)]
    public_socket: SocketAddr,
    /// Shutdown signal sender for background tasks
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let public_socket: SocketAddr = config
            .public_bind_addr
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:8080".parse().expect("valid default addr"));

        // Configure vanity for mirrors
        let vanity_config = tor::VanityConfig {
            enabled: config.vanity_enabled,
            prefix: config.vanity_prefix.clone(),
            timeout: config.vanity_timeout,
        };

        // Determine base data dir for torrc path
        let base_data_dir = config.base_data_dir.clone().unwrap_or_else(|| {
            if let Some(home) = std::env::var_os("HOME") {
                let mut path = PathBuf::from(home);
                path.push(".local");
                path.push("share");
                path.push("fortify");
                path
            } else {
                PathBuf::from("/tmp/fortify")
            }
        });

        let tor_service = Arc::new(
            TorService::new(
                config.tor_control_addr.clone(),
                config.tor_cookie_path.clone(),
            )
            .with_vanity(vanity_config)
            .with_base_data_dir(base_data_dir),
        );

        // Initialize multi-daemon manager if enabled
        let multi_daemon_manager = if config.multi_daemon.enabled {
            let manager =
                MultiDaemonManager::new(config.multi_daemon.clone(), config.tor_data_dir.clone());
            manager.initialize_daemons();
            Some(Arc::new(manager))
        } else {
            None
        };

        // Initialize CAPTCHA pre-generation pool with disk persistence
        let captcha_pool_dir = config.tor_data_dir.join("captcha_pool");
        let captcha_pool = Arc::new(CaptchaPoolManager::new(
            config.multi_daemon.flex_core.captcha_pregen.clone(),
            captcha_pool_dir,
        ));

        // Create shutdown broadcast channel for background tasks
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        Self {
            config,
            mirrors: Arc::new(Mutex::new(HashMap::new())),
            active_count: Arc::new(Mutex::new(0)),
            tor_service,
            spawn_rate_limiter: Arc::new(Mutex::new(SpawnRateLimiter::new())),
            multi_daemon_manager,
            captcha_pool,
            public_socket,
            shutdown_tx,
        }
    }

    /// Signal all background tasks to shutdown
    pub fn shutdown(&self) {
        tracing::info!("Orchestrator: signaling background tasks to shutdown");
        let _ = self.shutdown_tx.send(());

        // Save CAPTCHA pool to disk before shutdown
        self.captcha_pool.save_pool();
    }

    /// Start the orchestrator
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!(
            "Orchestrator starting: gate={}, proxy_port={}, min_mirrors={}",
            self.config.gate_address,
            self.config.proxy_port,
            self.config.min_mirrors
        );

        // Start multi-daemon Tor instances if enabled
        if let Some(ref manager) = self.multi_daemon_manager {
            tracing::info!(
                "Starting multi-daemon mode with {} daemons",
                manager.daemon_count()
            );
            manager.start_all().await?;
            self.start_daemon_health_task();
        }

        // Start Flex Core CAPTCHA pre-generation task
        if self.config.multi_daemon.flex_core.captcha_pregen.enabled {
            self.start_flex_core_task();
        }

        // Load existing mirrors from disk
        self.load_mirrors().await;

        // Spawn initial mirrors
        self.ensure_minimum_mirrors().await?;

        // Start background tasks
        self.start_rotation_task();
        self.start_monitoring_task();
        self.start_retirement_task();
        self.start_resurrection_task();
        self.start_auto_scaling_task();
        self.start_self_cleaning_task();

        Ok(())
    }

    /// Load existing mirrors from data directory
    async fn load_mirrors(&self) {
        if !self.config.tor_data_dir.exists() {
            tracing::info!(
                "Tor data dir {:?} does not exist, skipping load",
                self.config.tor_data_dir
            );
            return;
        }

        let entries = match std::fs::read_dir(&self.config.tor_data_dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to read tor data dir: {}", e);
                return;
            }
        };

        tracing::info!("Scanning {:?} for mirrors...", self.config.tor_data_dir);

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let mirror_id = path.file_name().unwrap().to_string_lossy().to_string();

            // Skip non-mirror directories (e.g., captcha_pool, temp, etc.)
            if !mirror_id.starts_with("mirror-") {
                tracing::debug!("Skipping non-mirror directory: {}", mirror_id);
                continue;
            }

            // Skip directories that are still being initialized (have .creating lock file)
            if path.join(".creating").exists() {
                tracing::debug!("Skipping mirror {} - still being created", mirror_id);
                continue;
            }

            // Check if valid mirror directory (has hostname and private_key)
            if !path.join("hostname").exists() || !path.join("private_key").exists() {
                // Only clean up directories older than 30 seconds to avoid race conditions
                let should_cleanup = match path.metadata() {
                    Ok(meta) => {
                        if let Ok(modified) = meta.modified() {
                            modified
                                .elapsed()
                                .map(|d| d.as_secs() > 30)
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    }
                    Err(_) => false,
                };

                if should_cleanup {
                    tracing::warn!(
                        "Removing invalid/incomplete mirror directory: {}",
                        mirror_id
                    );
                    // Clean up incomplete directories to prevent accumulation
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        tracing::error!("Failed to remove invalid mirror {}: {}", mirror_id, e);
                    }
                } else {
                    tracing::debug!(
                        "Skipping recent mirror {} - may still be initializing",
                        mirror_id
                    );
                }
                continue;
            }

            // Read hostname
            let onion_address = match std::fs::read_to_string(path.join("hostname")) {
                Ok(h) => h.trim().to_string(),
                Err(e) => {
                    tracing::warn!("Failed to read hostname for {}: {}", mirror_id, e);
                    continue;
                }
            };

            let mut mirror = Mirror::new(mirror_id.clone(), path.clone());

            // Load metadata to restore state (standby, active, etc.)
            let had_metadata = mirror.load_metadata();

            // Set onion address (metadata determines if standby or active)
            if mirror.is_standby {
                mirror.onion_address = Some(onion_address.clone());
                // State already set by load_metadata
                tracing::info!("Restoring standby mirror {} ({})", mirror_id, onion_address);
            } else {
                mirror.onion_address = Some(onion_address.clone());
                mirror.state = MirrorState::Active;
                tracing::info!("Restoring active mirror {} ({})", mirror_id, onion_address);
            }

            // If no metadata existed, save it now for future restarts
            if !had_metadata {
                mirror.save_metadata();
            }

            // Restore hidden service in Tor
            if let Err(e) = self
                .tor_service
                .restore_hidden_service(&mut mirror, self.config.proxy_port)
            {
                tracing::error!(
                    "Failed to restore hidden service for {}: {}. Cleaning up corrupted mirror.",
                    mirror_id,
                    e
                );
                // Clean up the invalid mirror directory so we don't accumulate junk
                if let Err(rm_err) = std::fs::remove_dir_all(&path) {
                    tracing::error!(
                        "Failed to remove invalid mirror dir {}: {}",
                        mirror_id,
                        rm_err
                    );
                }
                continue;
            }

            // Add to map
            {
                let mut mirrors = self.mirrors.lock().unwrap();

                // Only count as active if not standby
                if !mirror.is_standby && mirror.state == MirrorState::Active {
                    let mut count = self.active_count.lock().unwrap();
                    *count += 1;
                }

                mirrors.insert(mirror_id, mirror);
            }
        }
    }

    /// Ensure minimum number of healthy mirrors plus standby mirrors
    async fn ensure_minimum_mirrors(&self) -> anyhow::Result<()> {
        // Count mirrors from filesystem to handle multiple orchestrator instances
        // This prevents each orchestrator from spawning its own full set of standbys
        let (filesystem_active, filesystem_standby) = self.count_mirrors_on_disk();

        // First ensure active mirrors
        let active_needed = {
            let mirrors = self.mirrors.lock().unwrap();
            let in_memory_active = mirrors
                .values()
                .filter(|m| m.state == MirrorState::Active)
                .count();

            // Use max of in-memory and filesystem counts to be safe
            let total_active = std::cmp::max(in_memory_active, filesystem_active);
            self.config.min_mirrors.saturating_sub(total_active)
        };

        for _ in 0..active_needed {
            self.spawn_mirror().await?;
        }

        // Then ensure standby mirrors
        let standby_needed = {
            let mirrors = self.mirrors.lock().unwrap();
            let in_memory_standby = mirrors
                .values()
                .filter(|m| m.is_standby && m.state == MirrorState::Paused)
                .count();

            // Use max of in-memory and filesystem counts
            let total_standby = std::cmp::max(in_memory_standby, filesystem_standby);
            self.config.standby_mirrors.saturating_sub(total_standby)
        };

        for _ in 0..standby_needed {
            self.spawn_standby_mirror().await?;
        }

        Ok(())
    }

    /// Count mirrors directly from the filesystem (for coordination between multiple orchestrators)
    fn count_mirrors_on_disk(&self) -> (usize, usize) {
        let mut active = 0usize;
        let mut standby = 0usize;

        if let Ok(entries) = std::fs::read_dir(&self.config.tor_data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let mirror_id = path.file_name().unwrap().to_string_lossy().to_string();

                // Only count mirror-* directories
                if !mirror_id.starts_with("mirror-") {
                    continue;
                }

                // Must have hostname to be valid
                if !path.join("hostname").exists() {
                    continue;
                }

                // Check metadata to determine if standby
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                            if meta
                                .get("is_standby")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                            {
                                standby += 1;
                            } else {
                                active += 1;
                            }
                            continue;
                        }
                    }
                }

                // No metadata = assume active (legacy)
                active += 1;
            }
        }

        tracing::debug!(
            "Filesystem mirror count: {} active, {} standby",
            active,
            standby
        );
        (active, standby)
    }

    /// Spawn a new standby mirror (created but paused)
    pub async fn spawn_standby_mirror(&self) -> Result<String> {
        let mirror_id = self.generate_mirror_id();
        let mut mirror = Mirror::new(mirror_id.clone(), self.config.tor_data_dir.join(&mirror_id));

        tracing::info!("Spawning standby mirror: {}", mirror_id);

        let onion_address = self
            .tor_service
            .create_hidden_service(&mut mirror, self.config.proxy_port)?;
        mirror.activate_as_standby(onion_address.clone());

        // Store mirror
        {
            let mut mirrors = self.mirrors.lock().unwrap();
            mirrors.insert(mirror_id.clone(), mirror);
        }

        tracing::info!(
            "Standby mirror {} spawned: {} (paused, ready to activate)",
            mirror_id,
            onion_address
        );

        Ok(mirror_id)
    }

    /// Spawn a new mirror
    pub async fn spawn_mirror(&self) -> Result<String> {
        let mirror_id = self.generate_mirror_id();
        let mut mirror = Mirror::new(mirror_id.clone(), self.config.tor_data_dir.join(&mirror_id));

        tracing::info!("Spawning new mirror: {}", mirror_id);

        let onion_address = self
            .tor_service
            .create_hidden_service(&mut mirror, self.config.proxy_port)?;
        mirror.activate(onion_address);

        // Store mirror
        {
            let mut mirrors = self.mirrors.lock().unwrap();
            mirrors.insert(mirror_id.clone(), mirror);
        }

        {
            let mut count = self.active_count.lock().unwrap();
            *count += 1;
        }

        tracing::info!("Mirror {} spawned successfully", mirror_id);

        Ok(mirror_id)
    }

    /// Burn a compromised mirror
    pub async fn burn_mirror(&self, mirror_id: &str) -> Result<()> {
        // Scope lock to release before await
        {
            let mut mirrors = self.mirrors.lock().unwrap();
            let mirror = mirrors
                .get_mut(mirror_id)
                .ok_or_else(|| OrchestratorError::MirrorNotFound(mirror_id.to_string()))?;

            if mirror.state == MirrorState::Burned {
                return Err(OrchestratorError::MirrorBurned);
            }

            mirror.burn();
        }

        // Spawn replacement
        self.spawn_mirror().await?;

        // Complete burn process
        let mut mirrors = self.mirrors.lock().unwrap();
        if let Some(mirror) = mirrors.get_mut(mirror_id) {
            if let Err(err) = self.tor_service.remove_hidden_service(mirror) {
                tracing::error!("Failed to remove hidden service for {}: {}", mirror_id, err);
            }
            mirror.complete_burn();
        }

        Ok(())
    }

    /// Pause a mirror (stop accepting new traffic but don't destroy)
    pub async fn pause_mirror(&self, onion_address: &str) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();

        // Find mirror by onion address
        let mirror = mirrors
            .values_mut()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?;

        if mirror.state == MirrorState::Burned {
            return Err(OrchestratorError::MirrorBurned);
        }

        tracing::info!("Pausing mirror {} ({})", mirror.id, onion_address);
        mirror.state = MirrorState::Paused;

        Ok(())
    }

    /// Retire a mirror gracefully (drain -> retirement page -> dormant)
    pub async fn retire_mirror(&self, onion_address: &str, reason: RetirementReason) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();

        // Find mirror by onion address
        let mirror = mirrors
            .values_mut()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?;

        if mirror.state == MirrorState::Burned || mirror.state == MirrorState::Dormant {
            return Err(OrchestratorError::MirrorBurned);
        }

        // For compromised mirrors, permanent destruction
        if reason == RetirementReason::Compromised {
            tracing::warn!(
                "Mirror {} ({}) permanently destroyed due to compromise",
                mirror.id,
                onion_address
            );
            mirror.permanent_destroy();
            return Ok(());
        }

        // Begin normal retirement process
        let drain_secs = self.config.retirement.drain_period_seconds;
        let page_hours = self.config.retirement.retirement_page_hours;

        tracing::info!(
            "Retiring mirror {} ({}) - reason: {:?}, drain: {}s, page: {}h",
            mirror.id,
            onion_address,
            reason,
            drain_secs,
            page_hours
        );

        mirror.begin_retirement(reason, drain_secs, page_hours);

        // Set allow_new_sessions based on config
        if let Some(ref mut info) = mirror.retirement_info {
            info.allow_new_sessions = self.config.retirement.allow_new_sessions_during_retirement;
        }

        Ok(())
    }

    /// Force resurrect a dormant mirror (admin override)
    pub async fn force_resurrect_mirror(&self, onion_address: &str) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();

        let mirror = mirrors
            .values_mut()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?;

        if mirror.state != MirrorState::Dormant {
            return Err(OrchestratorError::MirrorNotPaused); // Reusing error for now
        }

        tracing::info!(
            "Force resurrecting dormant mirror {} ({})",
            mirror.id,
            onion_address
        );
        mirror.begin_restoration();

        Ok(())
    }

    /// Permanently destroy a dormant mirror (wipe keys forever)
    pub async fn permanently_destroy_mirror(&self, onion_address: &str) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();

        let mirror = mirrors
            .values_mut()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?;

        tracing::warn!(
            "Permanently destroying mirror {} ({})",
            mirror.id,
            onion_address
        );
        mirror.permanent_destroy();

        // Securely wipe Tor hidden service keys from disk
        let hs_dir = mirror.tor_data_dir.join("hidden_service");
        if let Err(e) = Self::wipe_mirror_keys(&hs_dir) {
            tracing::error!("Failed to wipe keys for mirror {}: {}", mirror.id, e);
            // Continue anyway - mirror is already marked destroyed
        } else {
            tracing::info!("Securely wiped keys for mirror {}", mirror.id);
        }

        Ok(())
    }

    /// Securely wipe Tor hidden service keys from disk
    /// Overwrites with zeros before deletion to prevent recovery
    fn wipe_mirror_keys(hs_dir: &std::path::Path) -> std::io::Result<()> {
        let key_files = [
            "hostname",
            "hs_ed25519_secret_key",
            "hs_ed25519_public_key",
            "private_key", // Legacy v2 onion (if present)
        ];

        for file in key_files {
            let path = hs_dir.join(file);
            if path.exists() {
                // Overwrite with zeros before deleting (secure wipe)
                let len = std::fs::metadata(&path)?.len() as usize;
                std::fs::write(&path, vec![0u8; len])?;
                std::fs::remove_file(&path)?;
                tracing::debug!("Wiped key file: {}", path.display());
            }
        }

        Ok(())
    }

    /// Get list of mirrors visible for discovery bar
    pub fn get_discovery_mirrors(&self) -> Vec<MirrorInfo> {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors
            .values()
            .filter(|m| m.state.visible_in_discovery())
            .filter(|m| m.onion_address.is_some())
            .map(|m| MirrorInfo {
                id: m.id.clone(),
                onion_address: m.onion_address.clone().unwrap_or_default(),
                status: m.state.as_str().to_string(),
                pow_enabled: m.pow_enabled,
                is_standby: m.is_standby,
                file_based: m.file_based,
            })
            .collect()
    }

    /// Get list of dormant mirrors (for admin panel)
    pub fn get_dormant_mirrors(&self) -> Vec<MirrorInfo> {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors
            .values()
            .filter(|m| m.state == MirrorState::Dormant)
            .filter(|m| m.onion_address.is_some())
            .map(|m| MirrorInfo {
                id: m.id.clone(),
                onion_address: m.onion_address.clone().unwrap_or_default(),
                status: m.state.as_str().to_string(),
                pow_enabled: m.pow_enabled,
                is_standby: m.is_standby,
                file_based: m.file_based,
            })
            .collect()
    }

    /// Resume a paused mirror
    pub async fn resume_mirror(&self, onion_address: &str) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();

        // Find mirror by onion address
        let mirror = mirrors
            .values_mut()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?;

        if mirror.state != MirrorState::Paused {
            return Err(OrchestratorError::MirrorNotPaused);
        }

        tracing::info!("Resuming mirror {} ({})", mirror.id, onion_address);
        mirror.state = MirrorState::Active;

        Ok(())
    }

    /// Activate a standby mirror (change from paused standby to active)
    pub async fn activate_standby(&self, onion_address: &str) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();

        // Find mirror by onion address
        let mirror = mirrors
            .values_mut()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?;

        if mirror.state == MirrorState::Burned {
            return Err(OrchestratorError::MirrorBurned);
        }

        if mirror.state != MirrorState::Paused {
            return Err(OrchestratorError::MirrorNotPaused);
        }

        tracing::info!(
            "Activating standby mirror {} ({})",
            mirror.id,
            onion_address
        );
        mirror.state = MirrorState::Active;
        mirror.is_standby = false; // No longer a standby

        Ok(())
    }

    /// Check if a mirror is paused by onion address
    pub fn is_mirror_paused(&self, onion_address: &str) -> bool {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors
            .values()
            .find(|m| m.onion_address.as_deref() == Some(onion_address))
            .map(|m| m.state == MirrorState::Paused)
            .unwrap_or(false)
    }

    /// Destroy a mirror permanently
    pub async fn destroy_mirror(&self, onion_address: &str) -> Result<()> {
        let mirror_id = {
            let mirrors = self.mirrors.lock().unwrap();
            mirrors
                .values()
                .find(|m| m.onion_address.as_deref() == Some(onion_address))
                .map(|m| m.id.clone())
                .ok_or_else(|| OrchestratorError::MirrorNotFound(onion_address.to_string()))?
        };

        tracing::warn!("Destroying mirror {} ({})", mirror_id, onion_address);

        // Remove the hidden service from Tor
        {
            let mut mirrors = self.mirrors.lock().unwrap();
            if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                if let Err(err) = self.tor_service.remove_hidden_service(mirror) {
                    tracing::error!("Failed to remove hidden service for {}: {}", mirror_id, err);
                }
                mirror.complete_burn();
            }
        }

        // Remove from data directory
        let mirror_path = self.config.tor_data_dir.join(&mirror_id);
        if mirror_path.exists() {
            if let Err(e) = std::fs::remove_dir_all(&mirror_path) {
                tracing::error!("Failed to remove mirror data dir {}: {}", mirror_id, e);
            }
        }

        // Remove from active map
        {
            let mut mirrors = self.mirrors.lock().unwrap();
            mirrors.remove(&mirror_id);

            let mut count = self.active_count.lock().unwrap();
            if *count > 0 {
                *count -= 1;
            }
        }

        // Spawn replacement to maintain minimum
        self.ensure_minimum_mirrors()
            .await
            .map_err(|_| OrchestratorError::SpawnFailed)?;

        Ok(())
    }

    /// Get list of active mirror addresses
    pub fn get_active_mirrors(&self) -> Vec<String> {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors
            .values()
            .filter(|m| m.state == MirrorState::Active)
            .filter_map(|m| m.onion_address.clone())
            .collect()
    }

    /// Get all mirrors with extended status info (for admin panel)
    pub fn get_all_mirrors_extended(&self) -> Vec<MirrorInfo> {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors
            .values()
            .filter(|m| m.onion_address.is_some() && m.state != MirrorState::Burned)
            .map(|m| MirrorInfo {
                id: m.id.clone(),
                onion_address: m.onion_address.clone().unwrap_or_default(),
                status: m.state.as_str().to_string(),
                pow_enabled: m.pow_enabled,
                is_standby: m.is_standby,
                file_based: m.file_based,
            })
            .collect()
    }

    /// Get all mirrors with their status (for admin panel) - legacy format
    pub fn get_all_mirrors(&self) -> Vec<(String, String, String)> {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors
            .values()
            .filter(|m| m.onion_address.is_some() && m.state != MirrorState::Burned)
            .map(|m| {
                (
                    m.id.clone(),
                    m.onion_address.clone().unwrap_or_default(),
                    m.state.as_str().to_string(),
                )
            })
            .collect()
    }

    /// Get a pre-generated CAPTCHA from the pool
    ///
    /// Returns a CAPTCHA if available, or None if pool is empty.
    /// The Gate should fall back to on-demand generation if None.
    pub fn take_pregen_captcha(&self) -> Option<PregenCaptcha> {
        self.captcha_pool.take_captcha()
    }

    /// Get CAPTCHA pool statistics
    ///
    /// Useful for monitoring and admin dashboard.
    pub fn captcha_pool_stats(&self) -> CaptchaPoolStats {
        self.captcha_pool.stats()
    }

    /// Check if CAPTCHA pool has available CAPTCHAs
    pub fn has_pregen_captchas(&self) -> bool {
        self.captcha_pool.stats().current_size > 0
    }

    /// Get mirror by ID
    pub fn get_mirror(&self, mirror_id: &str) -> Option<Mirror> {
        let mirrors = self.mirrors.lock().unwrap();
        mirrors.get(mirror_id).cloned()
    }

    /// Report compromise signal
    pub fn report_signal(&self, mirror_id: &str, signal: CompromiseSignal) -> Result<()> {
        let mut mirrors = self.mirrors.lock().unwrap();
        let mirror = mirrors
            .get_mut(mirror_id)
            .ok_or_else(|| OrchestratorError::MirrorNotFound(mirror_id.to_string()))?;

        mirror.add_signal(signal);

        // Auto-burn if threshold exceeded
        if mirror.metrics.compromise_score >= self.config.burn_threshold {
            tracing::warn!(
                "Mirror {} exceeded burn threshold ({}), scheduling burn",
                mirror_id,
                mirror.metrics.compromise_score
            );
            mirror.state = MirrorState::Burning;
        }

        Ok(())
    }

    fn start_rotation_task(&self) {
        let mirrors = Arc::clone(&self.mirrors);
        let rotation_interval = self.config.rotation_interval_seconds;
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(rotation_interval));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Rotation task shutting down");
                        break;
                    }
                    _ = interval.tick() => {}
                }

                // Find oldest active mirror
                let oldest = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Active)
                        .filter(|m| m.age_seconds() >= rotation_interval)
                        .max_by_key(|m| m.age_seconds())
                        .map(|m| m.id.clone())
                };

                if let Some(mirror_id) = oldest {
                    tracing::info!("Rotating mirror {} due to age", mirror_id);
                    // Would trigger rotation here
                }
            }
        });
    }

    fn start_monitoring_task(&self) {
        let mirrors = Arc::clone(&self.mirrors);
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Monitoring task shutting down");
                        break;
                    }
                    _ = interval.tick() => {}
                }

                let mirrors_to_burn: Vec<String> = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Burning)
                        .map(|m| m.id.clone())
                        .collect()
                };

                for mirror_id in mirrors_to_burn {
                    tracing::info!("Processing burn for mirror {}", mirror_id);
                    // Would complete burn process here
                }
            }
        });
    }

    /// Background task to manage retiring mirrors
    fn start_retirement_task(&self) {
        let mirrors = Arc::clone(&self.mirrors);
        let _config = self.config.retirement.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            // Check every 60 seconds
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Retirement task shutting down");
                        break;
                    }
                    _ = interval.tick() => {}
                }

                let _now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Find mirrors that should transition from Retiring -> Dormant
                let mirrors_to_dormant: Vec<String> = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Retiring)
                        .filter(|m| {
                            if let Some(ref info) = m.retirement_info {
                                info.page_expired()
                            } else {
                                false
                            }
                        })
                        .map(|m| m.id.clone())
                        .collect()
                };

                // Transition them to dormant
                {
                    let mut mirrors = mirrors.lock().unwrap();
                    for mirror_id in mirrors_to_dormant {
                        if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                            tracing::info!(
                                "Mirror {} retirement page expired, transitioning to dormant",
                                mirror_id
                            );
                            mirror.go_dormant();
                        }
                    }
                }

                // Log retirement status periodically
                let retiring_count = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Retiring)
                        .count()
                };
                if retiring_count > 0 {
                    tracing::debug!("{} mirrors currently retiring", retiring_count);
                }
            }
        });
    }

    /// Background task to manage dormant mirror resurrection
    fn start_resurrection_task(&self) {
        let mirrors = Arc::clone(&self.mirrors);
        let config = self.config.resurrection.clone();

        if !config.enabled {
            tracing::info!("Resurrection system disabled");
            return;
        }

        tokio::spawn(async move {
            // Check every 60 seconds
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Process dormant mirrors - check if evaluation window has passed
                let dormant_to_evaluate: Vec<String> = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Dormant)
                        .filter(|m| {
                            if let Some(ref info) = m.resurrection_info {
                                let dormant_duration = now - info.dormant_since;
                                // Wait for initial wait period before first evaluation
                                if dormant_duration < config.wait_after_burn_seconds {
                                    return false;
                                }
                                // Check if enough time has passed since last evaluation
                                if let Some(last_eval) = info.last_evaluation_at {
                                    // Wait at least 15 minutes between evaluations
                                    (now - last_eval) >= config.wait_after_burn_seconds
                                } else {
                                    true // First evaluation
                                }
                            } else {
                                false
                            }
                        })
                        .map(|m| m.id.clone())
                        .collect()
                };

                // Evaluate each dormant mirror
                for mirror_id in dormant_to_evaluate {
                    let should_restore = {
                        let mut mirrors = mirrors.lock().unwrap();
                        if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                            if let Some(ref mut info) = mirror.resurrection_info {
                                // Check connection attempts during evaluation window
                                let attempts = info.connection_attempts;
                                info.reset_evaluation();

                                if attempts < config.safe_threshold_attempts {
                                    tracing::info!(
                                        "Mirror {} evaluation: {} connection attempts (safe), beginning restoration",
                                        mirror_id, attempts
                                    );
                                    true
                                } else if attempts >= config.threat_threshold_attempts {
                                    tracing::info!(
                                        "Mirror {} evaluation: {} connection attempts (attack ongoing), remaining dormant",
                                        mirror_id, attempts
                                    );
                                    false
                                } else {
                                    tracing::info!(
                                        "Mirror {} evaluation: {} connection attempts (moderate), cautious restoration",
                                        mirror_id, attempts
                                    );
                                    true // Cautious restore for moderate traffic
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    };

                    if should_restore {
                        let mut mirrors = mirrors.lock().unwrap();
                        if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                            mirror.begin_restoration();
                        }
                    }
                }

                // Process restoring mirrors - advance phases or complete restoration
                let phase_duration_secs = (config.discovery_period.total_duration_hours * 3600) / 3;

                let restoring_updates: Vec<(String, bool, bool)> = {
                    // (mirror_id, should_advance, should_complete)
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Restoring)
                        .filter_map(|m| {
                            if let Some(ref info) = m.resurrection_info {
                                if let Some(started) = info.restoration_started_at {
                                    let elapsed = now - started;
                                    let current_phase = info.restoration_phase?;

                                    match current_phase {
                                        RestorationPhase::Phase1 => {
                                            if elapsed >= phase_duration_secs {
                                                Some((m.id.clone(), true, false))
                                            } else {
                                                None
                                            }
                                        }
                                        RestorationPhase::Phase2 => {
                                            if elapsed >= phase_duration_secs * 2 {
                                                Some((m.id.clone(), true, false))
                                            } else {
                                                None
                                            }
                                        }
                                        RestorationPhase::Phase3 => {
                                            if elapsed
                                                >= config.discovery_period.total_duration_hours
                                                    * 3600
                                            {
                                                Some((m.id.clone(), false, true))
                                            } else {
                                                None
                                            }
                                        }
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                // Apply updates
                {
                    let mut mirrors = mirrors.lock().unwrap();
                    for (mirror_id, should_advance, should_complete) in restoring_updates {
                        if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                            if should_complete {
                                mirror.complete_restoration();
                                tracing::info!(
                                    "Mirror {} restoration complete, now active",
                                    mirror_id
                                );
                            } else if should_advance {
                                if let Some(ref mut info) = mirror.resurrection_info {
                                    let old_phase = info.restoration_phase;
                                    info.advance_phase();
                                    let new_phase = info.restoration_phase;
                                    tracing::info!(
                                        "Mirror {} advancing from {:?} to {:?}",
                                        mirror_id,
                                        old_phase,
                                        new_phase
                                    );
                                }
                            }
                        }
                    }
                }

                // Check for mirrors that have been dormant too long (auto-destroy)
                let dormant_too_long: Vec<String> = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Dormant)
                        .filter(|m| {
                            if let Some(ref info) = m.resurrection_info {
                                let dormant_days = (now - info.dormant_since) / 86400;
                                dormant_days >= config.max_dormant_days
                            } else {
                                false
                            }
                        })
                        .map(|m| m.id.clone())
                        .collect()
                };

                {
                    let mut mirrors = mirrors.lock().unwrap();
                    for mirror_id in dormant_too_long {
                        if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                            tracing::warn!(
                                "Mirror {} has been dormant for {} days, permanently destroying",
                                mirror_id,
                                config.max_dormant_days
                            );
                            mirror.permanent_destroy();
                        }
                    }
                }
            }
        });
    }

    /// Background task to manage auto-scaling and standby pool
    fn start_auto_scaling_task(&self) {
        let mirrors = Arc::clone(&self.mirrors);
        let config = self.config.auto_scaling.clone();
        let tor_service = Arc::clone(&self.tor_service);
        let tor_data_dir = self.config.tor_data_dir.clone();
        let rate_limiter = Arc::clone(&self.spawn_rate_limiter);
        let min_mirrors = self.config.min_mirrors;
        let max_mirrors = self.config.max_mirrors;
        let _gate_address = self.config.gate_address.clone();
        let proxy_port = self.config.proxy_port;
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        if !config.enabled {
            tracing::info!("Auto-scaling system disabled");
            return;
        }

        tokio::spawn(async move {
            // Check every 30 seconds
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Auto-scaling task shutting down");
                        break;
                    }
                    _ = interval.tick() => {}
                }

                // Gather current state
                let (active_count, standby_count, total_count) = {
                    let mirrors = mirrors.lock().unwrap();
                    let active = mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Active)
                        .count();
                    let standby = mirrors
                        .values()
                        .filter(|m| m.is_standby && m.state == MirrorState::Paused)
                        .count();
                    let total = mirrors.len();
                    (active, standby, total)
                };

                // Check VPS resources if resource-aware
                let can_spawn = if config.resource_aware {
                    // Use sysinfo to check resources
                    let mut sys = sysinfo::System::new_all();
                    sys.refresh_all();

                    let cpu_usage = sys.global_cpu_info().cpu_usage();
                    let memory_total = sys.total_memory() / 1024 / 1024; // MB
                    let memory_used = sys.used_memory() / 1024 / 1024;
                    let memory_available = sys.available_memory() / 1024 / 1024;
                    let memory_percent = (memory_used as f32 / memory_total as f32) * 100.0;

                    if cpu_usage >= config.max_cpu_percent {
                        tracing::debug!(
                            "Auto-scaling: CPU usage {:.1}% exceeds limit {:.1}%",
                            cpu_usage,
                            config.max_cpu_percent
                        );
                        let mut rl = rate_limiter.lock().unwrap();
                        rl.record_resource_limit();
                        false
                    } else if memory_percent >= config.max_memory_percent {
                        tracing::debug!(
                            "Auto-scaling: Memory usage {:.1}% exceeds limit {:.1}%",
                            memory_percent,
                            config.max_memory_percent
                        );
                        let mut rl = rate_limiter.lock().unwrap();
                        rl.record_resource_limit();
                        false
                    } else if memory_available < config.min_memory_available_mb {
                        tracing::debug!(
                            "Auto-scaling: Available memory {}MB below minimum {}MB",
                            memory_available,
                            config.min_memory_available_mb
                        );
                        let mut rl = rate_limiter.lock().unwrap();
                        rl.record_resource_limit();
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };

                // Check rate limits (self-DDOS protection)
                let spawn_allowed = {
                    let rl = rate_limiter.lock().unwrap();
                    rl.can_spawn(&config)
                };

                // Determine if we need more standby mirrors
                let needs_standby = standby_count < config.target_standby
                    && standby_count < config.max_standby
                    && total_count < max_mirrors
                    && can_spawn
                    && spawn_allowed;

                if needs_standby {
                    tracing::debug!(
                        "Auto-scaling: standby mirrors {}/{}, spawning...",
                        standby_count,
                        config.target_standby
                    );

                    // Record the spawn attempt
                    {
                        let mut rl = rate_limiter.lock().unwrap();
                        rl.record_spawn();
                    }

                    // Generate a unique mirror ID
                    let mirror_id = {
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        let random: u32 = rng.gen();
                        format!("mirror-{:08x}", random)
                    };

                    // Create and spawn the standby mirror
                    let mirror_data_dir = tor_data_dir.join(&mirror_id);
                    let mut mirror = Mirror::new(mirror_id.clone(), mirror_data_dir.clone());

                    // Get onion address from Tor service (not async)
                    match tor_service.create_hidden_service(&mut mirror, proxy_port) {
                        Ok(onion_address) => {
                            // Mark as standby (paused but ready)
                            mirror.activate_as_standby(onion_address);

                            let mut mirrors = mirrors.lock().unwrap();
                            mirrors.insert(mirror_id.clone(), mirror);

                            tracing::info!("Standby mirror ready: {}", mirror_id);
                        }
                        Err(e) => {
                            tracing::error!("Auto-scaling: Failed to create standby mirror: {}", e);
                        }
                    }
                }

                // Check if we need fewer standby mirrors (scale down)
                if standby_count > config.max_standby {
                    tracing::debug!(
                        "Auto-scaling: Excess standbys ({} > {}), removing",
                        standby_count,
                        config.max_standby
                    );

                    let excess = standby_count - config.max_standby;
                    let mut removed = 0;

                    let mut mirrors = mirrors.lock().unwrap();
                    let standby_ids: Vec<String> = mirrors
                        .values()
                        .filter(|m| m.is_standby && m.state == MirrorState::Paused)
                        .map(|m| m.id.clone())
                        .collect();

                    for mirror_id in standby_ids.into_iter().take(excess) {
                        if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                            mirror.permanent_destroy();
                            removed += 1;
                            tracing::debug!("Removed excess standby: {}", mirror_id);
                        }
                    }

                    if removed > 0 {
                        tracing::info!("Auto-scaling: Removed {} excess standby mirrors", removed);
                    }
                }

                // Ensure minimum active mirrors
                if active_count < min_mirrors {
                    tracing::info!(
                        "Auto-scaling: Active mirrors ({}) below minimum ({}), activating standby",
                        active_count,
                        min_mirrors
                    );

                    // Try to activate a standby
                    let standby_to_activate: Option<String> = {
                        let mirrors = mirrors.lock().unwrap();
                        mirrors
                            .values()
                            .filter(|m| m.is_standby && m.state == MirrorState::Paused)
                            .map(|m| m.id.clone())
                            .next()
                    };

                    if let Some(mirror_id) = standby_to_activate {
                        // Check activation rate limit
                        let activate_allowed = {
                            let rl = rate_limiter.lock().unwrap();
                            rl.can_activate(&config)
                        };

                        if activate_allowed {
                            let mut mirrors = mirrors.lock().unwrap();
                            if let Some(mirror) = mirrors.get_mut(&mirror_id) {
                                mirror.is_standby = false;
                                mirror.state = MirrorState::Active;

                                let mut rl = rate_limiter.lock().unwrap();
                                rl.record_activation();

                                tracing::info!(
                                    "Auto-scaling: Activated standby mirror {} (now: {} active)",
                                    mirror_id,
                                    active_count + 1
                                );
                            }
                        } else {
                            tracing::debug!("Auto-scaling: Activation rate limited");
                        }
                    } else {
                        tracing::warn!(
                            "Auto-scaling: No standby mirrors available to activate, need to spawn new"
                        );
                        // We'll spawn on next iteration if resources allow
                    }
                }
            }
        });
    }

    /// Background task for self-cleaning (Phase 4.7)
    fn start_self_cleaning_task(&self) {
        let mirrors = Arc::clone(&self.mirrors);
        let config = self.config.self_cleaning.clone();
        let tor_data_dir = self.config.tor_data_dir.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        if !config.enabled {
            tracing::info!("Self-cleaning system disabled");
            return;
        }

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                config.cleanup_interval_seconds,
            ));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Self-cleaning task shutting down");
                        break;
                    }
                    _ = interval.tick() => {}
                }

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // 1. Clean up burned mirrors that have exceeded retention
                let burned_to_remove: Vec<String> = {
                    let mirrors = mirrors.lock().unwrap();
                    mirrors
                        .values()
                        .filter(|m| m.state == MirrorState::Burned)
                        .filter(|m| {
                            if let Some(last_time) = m.metrics.last_request_time {
                                let burned_days = (now - last_time) / 86400;
                                burned_days >= config.burned_mirror_retention_days
                            } else {
                                // No last request time, use creation time fallback
                                true
                            }
                        })
                        .map(|m| m.id.clone())
                        .collect()
                };

                if !burned_to_remove.is_empty() {
                    let mut mirrors = mirrors.lock().unwrap();
                    for mirror_id in burned_to_remove {
                        tracing::info!(
                            "Self-cleaning: Removing burned mirror {} (retention exceeded)",
                            mirror_id
                        );
                        mirrors.remove(&mirror_id);
                    }
                }

                // 2. Clean up destroyed mirror data directories
                if let Ok(entries) = std::fs::read_dir(&tor_data_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }

                        // Check if this directory belongs to a known mirror
                        let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
                        let is_known = {
                            let mirrors = mirrors.lock().unwrap();
                            mirrors.contains_key(&dir_name)
                        };

                        if !is_known {
                            // Check if directory is old enough to remove
                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(duration) = SystemTime::now().duration_since(modified)
                                    {
                                        let days_old = duration.as_secs() / 86400;
                                        if days_old >= config.destroyed_data_retention_days {
                                            tracing::info!(
                                                "Self-cleaning: Removing orphaned directory {:?} ({} days old)",
                                                path, days_old
                                            );
                                            if let Err(e) = std::fs::remove_dir_all(&path) {
                                                tracing::warn!(
                                                    "Failed to remove orphaned directory {:?}: {}",
                                                    path,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Clean temporary files
                let temp_dirs = [PathBuf::from("/tmp/fortify"), tor_data_dir.join("tmp")];

                let max_age_secs = config.temp_file_max_age_hours * 3600;

                for temp_dir in &temp_dirs {
                    if !temp_dir.exists() {
                        continue;
                    }

                    if let Ok(entries) = std::fs::read_dir(temp_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();

                            if let Ok(metadata) = path.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(duration) = SystemTime::now().duration_since(modified)
                                    {
                                        if duration.as_secs() > max_age_secs {
                                            tracing::debug!(
                                                "Self-cleaning: Removing old temp file {:?}",
                                                path
                                            );
                                            let _ = if path.is_dir() {
                                                std::fs::remove_dir_all(&path)
                                            } else {
                                                std::fs::remove_file(&path)
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 4. Check memory usage
                let mut sys = sysinfo::System::new_all();
                sys.refresh_memory();
                let memory_used_mb = sys.used_memory() / 1024 / 1024;

                if memory_used_mb > config.memory_high_water_mb {
                    tracing::warn!(
                        "Self-cleaning: Memory usage {}MB exceeds high-water mark {}MB",
                        memory_used_mb,
                        config.memory_high_water_mb
                    );

                    // Trigger garbage collection hints
                    // In a real system, we'd free caches, compact data structures, etc.
                    // For now, just log the warning
                }

                // 5. Log cleanup stats periodically
                let mirror_count = mirrors.lock().unwrap().len();
                let active_count = mirrors
                    .lock()
                    .unwrap()
                    .values()
                    .filter(|m| m.state == MirrorState::Active)
                    .count();
                let burned_count = mirrors
                    .lock()
                    .unwrap()
                    .values()
                    .filter(|m| m.state == MirrorState::Burned)
                    .count();

                tracing::debug!(
                    "Self-cleaning cycle complete: {} mirrors ({} active, {} burned), {}MB memory",
                    mirror_count,
                    active_count,
                    burned_count,
                    memory_used_mb
                );
            }
        });
    }

    /// Background task for daemon health monitoring (Phase 4.8)
    fn start_daemon_health_task(&self) {
        let manager = match &self.multi_daemon_manager {
            Some(m) => Arc::clone(m),
            None => return,
        };
        let config = self.config.multi_daemon.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                config.health_check_interval_seconds,
            ));

            loop {
                interval.tick().await;

                let daemon_count = manager.daemon_count();
                let mut unhealthy_daemons = Vec::new();

                // Check each daemon's health
                for i in 0..daemon_count {
                    let health = manager.check_daemon_health(i).await;

                    match health {
                        DaemonHealth::Healthy => {
                            // All good
                        }
                        DaemonHealth::Degraded => {
                            tracing::warn!("Daemon {} is degraded", i);
                        }
                        DaemonHealth::Unhealthy => {
                            tracing::error!("Daemon {} is unhealthy", i);
                            unhealthy_daemons.push(i);
                        }
                        DaemonHealth::Dead => {
                            tracing::error!("Daemon {} is dead", i);
                            unhealthy_daemons.push(i);
                        }
                        _ => {}
                    }
                }

                // Restart unhealthy daemons if auto-restart enabled
                if config.auto_restart_daemons {
                    for daemon_id in unhealthy_daemons {
                        tracing::info!("Auto-restarting daemon {}", daemon_id);

                        // Stop existing daemon if running
                        let _ = manager.stop_daemon(daemon_id);

                        // Wait a moment before restart
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                        // Start daemon again
                        match manager.start_daemon(daemon_id).await {
                            Ok(pid) => {
                                tracing::info!("Restarted daemon {} with PID {}", daemon_id, pid);
                            }
                            Err(e) => {
                                tracing::error!("Failed to restart daemon {}: {}", daemon_id, e);
                            }
                        }
                    }
                }

                // Log daemon status summary
                let statuses = manager.get_daemon_statuses();
                let healthy = statuses
                    .iter()
                    .filter(|d| d.health == DaemonHealth::Healthy)
                    .count();
                let total = statuses.len();
                tracing::debug!("Daemon health: {}/{} healthy", healthy, total);
            }
        });
    }

    /// Flex Core background task for CAPTCHA pre-generation
    ///
    /// This task runs on Core 2 (Flex Core) and manages the CAPTCHA pool:
    /// - Generates CAPTCHAs during low CPU usage periods
    /// - Pauses generation when CPU exceeds threshold
    /// - Rotates 25% of pool every 10 days for freshness
    /// - Monitors pool levels and logs statistics
    fn start_flex_core_task(&self) {
        let pool_manager = Arc::clone(&self.captcha_pool);
        let config = self.config.multi_daemon.flex_core.captcha_pregen.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tracing::info!(
            "Starting Flex Core CAPTCHA pre-generation task (target: {}, rotation: {}% every {} days)",
            config.target_pool_size,
            config.rotation_percent,
            config.rotation_interval_days
        );

        tokio::spawn(async move {
            // Track rotation timing
            let mut last_rotation = std::time::Instant::now();
            let rotation_interval =
                std::time::Duration::from_secs(config.rotation_interval_days * 24 * 60 * 60);

            // Track last logged count to avoid spam
            let mut last_logged_generated: u64 = 0;

            // Main generation loop - check every 5 seconds
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Flex Core task received shutdown signal, saving pool and exiting");
                        pool_manager.save_pool();
                        break;
                    }
                    _ = interval.tick() => {
                        // Continue with normal processing
                    }
                }

                // Check if rotation is due
                if last_rotation.elapsed() >= rotation_interval {
                    tracing::info!(
                        "CAPTCHA pool rotation triggered ({}% refresh)",
                        config.rotation_percent
                    );
                    pool_manager.rotate_pool();
                    pool_manager.save_pool(); // Persist after rotation
                    last_rotation = std::time::Instant::now();
                }

                // Get current pool stats
                let stats = pool_manager.stats();

                // Skip generation if pool is full
                if stats.current_size >= config.max_pool_size {
                    tracing::debug!(
                        "CAPTCHA pool at max capacity ({}), skipping generation",
                        stats.current_size
                    );
                    continue;
                }

                // Check CPU usage before generating (simulated - would use sys-info crate in production)
                let cpu_usage = Self::get_cpu_usage().await;

                if cpu_usage > config.pause_cpu_threshold {
                    tracing::debug!(
                        "CPU usage {:.1}% exceeds threshold {:.1}%, pausing CAPTCHA generation",
                        cpu_usage,
                        config.pause_cpu_threshold
                    );
                    continue;
                }

                // Generate a batch if below target
                if stats.current_size < config.target_pool_size {
                    let batch = pool_manager.generate_batch();
                    let batch_count = batch.len();

                    for captcha in batch {
                        pool_manager.add_captcha(captcha);
                    }

                    let new_stats = pool_manager.stats();
                    tracing::debug!(
                        "Generated {} CAPTCHAs, pool now at {}/{} (target: {})",
                        batch_count,
                        new_stats.current_size,
                        config.max_pool_size,
                        config.target_pool_size
                    );

                    // Save when we reach target to persist
                    if new_stats.current_size >= config.target_pool_size
                        && stats.current_size < config.target_pool_size
                    {
                        tracing::info!("CAPTCHA pool reached target, persisting to disk");
                        pool_manager.save_pool();
                    }

                    // Brief delay between batches to avoid CPU spike
                    tokio::time::sleep(std::time::Duration::from_millis(config.batch_delay_ms))
                        .await;
                }

                // Log stats periodically (only when total_generated increases by 500)
                if stats.total_generated >= last_logged_generated + 500 {
                    tracing::info!(
                        "CAPTCHA pool: size={}/{}, served={}, expired={}",
                        stats.current_size,
                        config.target_pool_size,
                        stats.total_served,
                        stats.total_expired
                    );
                    last_logged_generated = stats.total_generated;
                }
            }
        });
    }

    /// Get current CPU usage percentage using sysinfo crate
    async fn get_cpu_usage() -> f32 {
        use sysinfo::System;

        // Create a static System instance for efficiency
        // sysinfo needs two measurements to calculate CPU usage
        static CPU_SYSTEM: std::sync::OnceLock<std::sync::Mutex<System>> =
            std::sync::OnceLock::new();

        let system = CPU_SYSTEM.get_or_init(|| {
            let mut sys = System::new();
            sys.refresh_cpu_usage();
            std::sync::Mutex::new(sys)
        });

        // Lock and refresh CPU usage
        if let Ok(mut sys) = system.lock() {
            sys.refresh_cpu_usage();
            sys.global_cpu_info().cpu_usage()
        } else {
            // Fallback if lock fails
            tracing::warn!("Failed to lock CPU monitor, returning estimate");
            25.0
        }
    }

    fn generate_mirror_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random: u32 = rng.gen();
        format!("mirror-{:08x}", random)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror_state_transitions() {
        let mut mirror = Mirror::new("test-1".into(), PathBuf::from("/tmp"));

        assert_eq!(mirror.state, MirrorState::Spawning);
        assert!(!mirror.state.can_serve_traffic());

        mirror.activate("test.onion".into());
        assert_eq!(mirror.state, MirrorState::Active);
        assert!(mirror.state.can_serve_traffic());

        mirror.burn();
        assert_eq!(mirror.state, MirrorState::Burning);
        assert!(!mirror.state.can_serve_traffic());

        mirror.complete_burn();
        assert_eq!(mirror.state, MirrorState::Burned);
    }

    #[test]
    fn test_compromise_score_calculation() {
        let mut mirror = Mirror::new("test-2".into(), PathBuf::from("/tmp"));
        mirror.activate("test.onion".into());

        // Add signals
        mirror.add_signal(CompromiseSignal::new(
            SignalType::UnusualTraffic,
            0.3,
            "High traffic".into(),
        ));

        assert!(mirror.metrics.compromise_score > 0.0);
        assert_eq!(mirror.state, MirrorState::Active);

        // Add high severity signal - score becomes (0.3 + 0.9) / 2 = 0.6
        // Still below 0.8 threshold, so stays Active
        mirror.add_signal(CompromiseSignal::new(
            SignalType::RepeatedFailures,
            0.9,
            "Many failures".into(),
        ));

        assert!(mirror.metrics.compromise_score > 0.5);
        // Score 0.6 is below 0.8 threshold, so still Active
        assert_eq!(mirror.state, MirrorState::Active);

        // Add critical signal - score becomes (0.3 + 0.9 + 1.0) / 3 = 0.73
        // Still below 0.8, add another
        mirror.add_signal(CompromiseSignal::new(
            SignalType::RepeatedFailures,
            1.0,
            "Critical failures".into(),
        ));

        // Add one more to push over threshold
        mirror.add_signal(CompromiseSignal::new(
            SignalType::RepeatedFailures,
            1.0,
            "More failures".into(),
        ));

        // Score now (0.3 + 0.9 + 1.0 + 1.0) / 4 = 0.8 - at threshold
        assert!(mirror.metrics.compromise_score >= 0.8);
        assert_eq!(mirror.state, MirrorState::Suspicious);
    }

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = MirrorMetrics::default();

        metrics.record_request(true, 100.0, 1024);
        assert_eq!(metrics.requests_total, 1);
        assert_eq!(metrics.requests_failed, 0);
        assert_eq!(metrics.bytes_transferred, 1024);

        metrics.record_request(false, 200.0, 512);
        assert_eq!(metrics.requests_total, 2);
        assert_eq!(metrics.requests_failed, 1);
        assert_eq!(metrics.failure_rate(), 0.5);
    }

    #[tokio::test]
    async fn test_orchestrator_spawn_mirror() {
        let config = OrchestratorConfig::default();
        let orch = Orchestrator::new(config);

        let mirror_id = orch.spawn_mirror().await.unwrap();
        assert!(!mirror_id.is_empty());

        let mirror = orch.get_mirror(&mirror_id).unwrap();
        assert_eq!(mirror.state, MirrorState::Active);
        assert!(mirror.onion_address.is_some());
    }

    #[tokio::test]
    async fn test_orchestrator_burn_and_replace() {
        let config = OrchestratorConfig::default();
        let orch = Orchestrator::new(config);

        let mirror_id = orch.spawn_mirror().await.unwrap();
        let initial_count = orch.get_active_mirrors().len();

        orch.burn_mirror(&mirror_id).await.unwrap();

        // Should spawn replacement
        let final_count = orch.get_active_mirrors().len();
        assert_eq!(final_count, initial_count);

        // Original should be burned
        let mirror = orch.get_mirror(&mirror_id).unwrap();
        assert_eq!(mirror.state, MirrorState::Burned);
    }

    #[test]
    fn test_compromise_signal_severity() {
        let signal =
            CompromiseSignal::new(SignalType::TimingAnomaly, 0.75, "Suspicious timing".into());

        assert_eq!(signal.signal_type, SignalType::TimingAnomaly);
        assert_eq!(signal.severity, 0.75);
        assert!(!signal.description.is_empty());
    }
}
