//! Main application state and loop

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::{CaptchaType, ChangeManager, FortifyConfig};
use crate::deployment::DeploymentManager;
use crate::logging::{LogBuffer, LogEntry, LogLevel};
use crate::ui;

/// Parse yes/no/true/false input to boolean
fn parse_yes_no(value: &str, default: bool) -> bool {
    match value.to_lowercase().trim() {
        "yes" | "y" | "true" | "1" | "on" => true,
        "no" | "n" | "false" | "0" | "off" => false,
        _ => default,
    }
}

/// Active panel focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Menu,
    Settings,
    Logs,
    Dialog,
}

/// Main menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Deploy,
    JoinNetwork,
    Settings,
    Status,
    Destroy,
    Quit,
}

impl MenuItem {
    pub fn all() -> &'static [MenuItem] {
        &[
            MenuItem::Deploy,
            MenuItem::JoinNetwork,
            MenuItem::Settings,
            MenuItem::Status,
            MenuItem::Destroy,
            MenuItem::Quit,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            MenuItem::Deploy => "Deploy",
            MenuItem::JoinNetwork => "Join Community Network",
            MenuItem::Settings => "Settings",
            MenuItem::Status => "System Status",
            MenuItem::Destroy => "Destroy Instance",
            MenuItem::Quit => "Quit",
        }
    }

    pub fn hotkey(&self) -> char {
        match self {
            MenuItem::Deploy => 'D',
            MenuItem::JoinNetwork => 'J',
            MenuItem::Settings => 'S',
            MenuItem::Status => 'T',
            MenuItem::Destroy => 'X',
            MenuItem::Quit => 'Q',
        }
    }

    pub fn color(&self) -> Color {
        match self {
            MenuItem::Destroy => Color::Red,
            _ => Color::White,
        }
    }
}

/// Settings tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    TrafficTier,
    Branding,
    Captcha,
    Thresholds,
    Network,
    Mirrors,
    Vanity,
}

impl SettingsTab {
    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::TrafficTier,
            SettingsTab::Branding,
            SettingsTab::Captcha,
            SettingsTab::Thresholds,
            SettingsTab::Network,
            SettingsTab::Mirrors,
            SettingsTab::Vanity,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsTab::TrafficTier => "Tier",
            SettingsTab::Branding => "Branding",
            SettingsTab::Captcha => "CAPTCHA",
            SettingsTab::Thresholds => "Thresholds",
            SettingsTab::Network => "Network",
            SettingsTab::Mirrors => "Mirrors",
            SettingsTab::Vanity => "Vanity",
        }
    }
}

/// Mirror verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorStatusState {
    /// Tor daemon starting, not yet announced
    Pending,
    /// Announced to network, self-verification in progress
    Verifying,
    /// Self-verified and accessible
    Live,
    /// Failed to create or publish
    Failed,
    /// Vanity address being generated
    Generating,
}

impl MirrorStatusState {
    pub fn color(&self) -> Color {
        match self {
            MirrorStatusState::Pending => Color::Yellow,
            MirrorStatusState::Verifying => Color::Rgb(255, 165, 0), // Orange
            MirrorStatusState::Live => Color::Green,
            MirrorStatusState::Failed => Color::Red,
            MirrorStatusState::Generating => Color::Magenta,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            MirrorStatusState::Live => "●",
            MirrorStatusState::Pending
            | MirrorStatusState::Verifying
            | MirrorStatusState::Generating => "◐",
            MirrorStatusState::Failed => "○",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MirrorStatusState::Pending => "PENDING",
            MirrorStatusState::Verifying => "VERIFYING",
            MirrorStatusState::Live => "LIVE",
            MirrorStatusState::Failed => "FAILED",
            MirrorStatusState::Generating => "GENERATING",
        }
    }
}

/// Status of a single mirror
#[derive(Debug, Clone)]
pub struct MirrorStatus {
    /// Mirror index
    pub index: usize,
    /// Onion address (without .onion suffix)
    pub address: String,
    /// Current status
    pub state: MirrorStatusState,
    /// Is this a standby mirror
    pub is_standby: bool,
    /// Vanity prefix used (if any)
    pub vanity_prefix: Option<String>,
    /// Last verification time
    pub last_verified: Option<std::time::Instant>,
}

/// Backend health state with degraded levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendHealthState {
    /// Not yet checked
    Unknown,
    /// Currently checking
    Checking,
    /// 3/3 checks passed - fully connected
    Connected,
    /// 2/3 checks passed - mostly connected
    Degraded2of3,
    /// 1/3 checks passed - degraded
    Degraded1of3,
    /// 0/3 checks passed - disconnected
    Disconnected,
}

impl BackendHealthState {
    pub fn color(&self) -> Color {
        match self {
            BackendHealthState::Unknown => Color::DarkGray,
            BackendHealthState::Checking => Color::Yellow,
            BackendHealthState::Connected => Color::Green,
            BackendHealthState::Degraded2of3 => Color::Rgb(144, 238, 144), // Light green
            BackendHealthState::Degraded1of3 => Color::Yellow,
            BackendHealthState::Disconnected => Color::Red,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            BackendHealthState::Unknown => "○",
            BackendHealthState::Checking => "◐",
            BackendHealthState::Connected => "●",
            BackendHealthState::Degraded2of3 => "◐",
            BackendHealthState::Degraded1of3 => "◐",
            BackendHealthState::Disconnected => "✗",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BackendHealthState::Unknown => "UNKNOWN",
            BackendHealthState::Checking => "CHECKING",
            BackendHealthState::Connected => "CONNECTED",
            BackendHealthState::Degraded2of3 => "DEGRADED (2/3)",
            BackendHealthState::Degraded1of3 => "DEGRADED (1/3)",
            BackendHealthState::Disconnected => "DISCONNECTED",
        }
    }

    /// Calculate state from recent check history
    pub fn from_recent_checks(checks: &[BackendHealthCheck]) -> Self {
        if checks.is_empty() {
            return BackendHealthState::Unknown;
        }

        // Look at last 3 checks
        let recent: Vec<_> = checks.iter().rev().take(3).collect();
        let success_count = recent.iter().filter(|c| c.success).count();

        match success_count {
            3 => BackendHealthState::Connected,
            2 => BackendHealthState::Degraded2of3,
            1 => BackendHealthState::Degraded1of3,
            _ => BackendHealthState::Disconnected,
        }
    }
}

/// Backend health check entry
#[derive(Debug, Clone)]
pub struct BackendHealthCheck {
    /// Timestamp of check
    pub timestamp: std::time::Instant,
    /// Result of check
    pub success: bool,
    /// Duration of check
    pub duration: std::time::Duration,
    /// Error message if failed
    pub error: Option<String>,
}

/// Application view/screen
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// Main menu
    Home,
    /// Deploy new instance wizard
    DeployWizard { step: usize },
    /// Resume selection
    ResumeSelect,
    /// Settings configuration
    Settings {
        tab: SettingsTab,
        field_index: usize,
    },
    /// Running/deployed view
    Running,
    /// Join community network
    JoinNetwork,
    /// System status
    Status,
}

/// Dialog types
#[derive(Debug, Clone)]
pub enum Dialog {
    None,
    /// Confirm action
    Confirm {
        title: String,
        message: String,
        on_confirm: DialogAction,
    },
    /// Apply changes now or store for later
    ApplyChanges {
        changes: Vec<String>,
    },
    /// Text input
    Input {
        title: String,
        value: String,
        field: String,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Info message
    Info {
        title: String,
        message: String,
    },
    /// Dependency check progress dialog
    DependencyCheck {
        /// Status of each dependency
        statuses: Vec<DependencyStatus>,
        /// Current phase
        phase: DependencyCheckPhase,
        /// Time when all checks completed (for auto-dismiss)
        completed_at: Option<std::time::Instant>,
    },
}

/// Status of a single dependency during check
#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub state: DependencyState,
}

/// State of a dependency check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyState {
    Pending,
    Checking,
    Installing,
    Ok,
    Failed(String),
    Skipped,
}

/// Phase of the dependency check process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyCheckPhase {
    Checking,
    Installing,
    Complete,
    Failed,
}

#[derive(Debug, Clone)]
pub enum DialogAction {
    Deploy,
    QuickDeploy,
    Stop,
    Quit,
    ApplyNow,
    StoreLater,
    DestroyConfirm1,
    DestroyConfirm2,
    InstallDeps,
    None,
}

/// Main application state
pub struct App {
    /// Current view
    pub view: View,
    /// Focus panel
    pub focus: Focus,
    /// Selected menu item
    pub menu_index: usize,
    /// Configuration
    pub config: FortifyConfig,
    /// Change manager for hot-reload
    pub changes: ChangeManager,
    /// Deployment manager
    pub deployment: DeploymentManager,
    /// Log buffer
    pub logs: LogBuffer,
    /// Log receiver
    pub log_rx: mpsc::Receiver<LogEntry>,
    /// Log sender (for passing to components)
    pub log_tx: mpsc::Sender<LogEntry>,
    /// Active dialog
    pub dialog: Dialog,
    /// Log filter level
    pub log_filter: LogLevel,
    /// Logs paused
    pub logs_paused: bool,
    /// Log scroll offset (from bottom)
    pub log_scroll: usize,
    /// Log selection mode (for copying)
    pub log_select_mode: bool,
    /// Selected log line index (relative to visible area)
    pub log_selected_line: usize,
    /// Settings scroll position
    pub settings_scroll: usize,
    /// Should quit
    pub should_quit: bool,
    /// Status message
    pub status_message: Option<(String, std::time::Instant)>,
    /// Existing deployments (for resume)
    pub existing_deployments: Vec<(String, std::path::PathBuf)>,
    /// Selected deployment for resume
    pub resume_index: usize,
    /// Text input buffer
    pub input_buffer: String,
    /// Editing field
    pub editing_field: Option<String>,
    /// Mirror statuses (populated when deployed)
    pub mirror_statuses: Vec<MirrorStatus>,
    /// Vanity generation progress (current prefix being attempted)
    pub vanity_current_prefix: Option<String>,
    /// Last time we polled for mirror status
    last_mirror_poll: Option<std::time::Instant>,
    /// Log file for persistent log output
    log_file: Option<File>,
    /// Attack events log file
    attack_log_file: Option<File>,
    /// Stats log file
    stats_log_file: Option<File>,
    /// Logs directory path (for future log rotation)
    #[allow(dead_code)]
    logs_dir: std::path::PathBuf,
    /// Last stats log time
    last_stats_log: std::time::Instant,
    /// Last session cleanup time
    last_session_cleanup: std::time::Instant,
    /// Last security level (for detecting changes)
    last_security_level: crate::logging::SecurityLevel,
    /// Last attack log update time (for periodic updates during attacks)
    last_attack_log_update: std::time::Instant,
    /// Backend health state
    pub backend_health: BackendHealthState,
    /// Last backend health check time
    pub backend_last_check: Option<std::time::Instant>,
    /// Backend health check history (last 20 checks)
    pub backend_check_history: Vec<BackendHealthCheck>,
    /// Current backend check interval (in seconds)
    pub backend_check_interval: u64,
    /// Mirror health checks (address -> list of recent checks)
    pub mirror_health_checks: std::collections::HashMap<String, Vec<BackendHealthCheck>>,
    /// System status for status dashboard
    pub system_status: crate::logging::SystemStatus,
    /// Security status for attack detection
    pub security_status: crate::logging::SecurityStatus,
    /// Network events buffer for verified/trusted traffic stream
    pub network_events: crate::logging::NetworkEventBuffer,
    /// Network events buffer for threat/unverified traffic stream
    pub threat_events: crate::logging::NetworkEventBuffer,
    /// Session trust level tracking (session_id -> session entry with trust and last_seen)
    pub session_trust: std::collections::HashMap<String, crate::logging::SessionEntry>,
}

impl App {
    /// Create new application
    pub async fn new() -> Result<Self> {
        let (log_tx, log_rx) = mpsc::channel(1000);

        // Ensure config directory exists (use persistent location)
        let config_dir = FortifyConfig::default_path()
            .parent()
            .unwrap()
            .to_path_buf();
        std::fs::create_dir_all(&config_dir)?;

        // Create log directory and open log file for persistent logging
        let logs_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path.push("logs");
            path
        } else {
            std::path::PathBuf::from("/tmp/fortify/logs")
        };
        std::fs::create_dir_all(&logs_dir)?;

        // Perform log rotation (daily, keep 90 days)
        Self::rotate_logs(&logs_dir);

        // Open log files with date-based naming
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs_dir.join(format!("deployment-{}.log", today)))
            .ok();

        let attack_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs_dir.join("attacks.log"))
            .ok();

        let stats_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs_dir.join("stats.log"))
            .ok();

        // Try to load existing config or use default
        let config_path = FortifyConfig::default_path();
        let config = if config_path.exists() {
            match FortifyConfig::load(&config_path) {
                Ok(cfg) => cfg,
                Err(_) => {
                    // Load failed, use default but keep the path
                    FortifyConfig {
                        config_path: Some(config_path),
                        ..Default::default()
                    }
                }
            }
        } else {
            FortifyConfig {
                config_path: Some(config_path),
                ..Default::default()
            }
        };

        // Load existing deployments
        let existing_deployments = FortifyConfig::list_deployments().unwrap_or_default();

        // Check if there's an existing session
        let deployment = DeploymentManager::new(log_tx.clone());
        // Get captcha pool size before config is moved
        let captcha_pool_target = config.captcha.pool_size;

        Ok(Self {
            view: View::Home,
            focus: Focus::Menu,
            menu_index: 0,
            config,
            changes: ChangeManager::new(),
            deployment,
            logs: LogBuffer::new(5000),
            log_rx,
            log_tx,
            dialog: Dialog::None,
            log_filter: LogLevel::Debug,
            logs_paused: false,
            log_scroll: 0,
            log_select_mode: false,
            log_selected_line: 0,
            settings_scroll: 0,
            should_quit: false,
            status_message: None,
            existing_deployments,
            resume_index: 0,
            input_buffer: String::new(),
            editing_field: None,
            mirror_statuses: Vec::new(),
            vanity_current_prefix: None,
            last_mirror_poll: None,
            log_file,
            attack_log_file,
            stats_log_file,
            logs_dir,
            last_stats_log: std::time::Instant::now(),
            last_session_cleanup: std::time::Instant::now(),
            last_security_level: crate::logging::SecurityLevel::Clear,
            last_attack_log_update: std::time::Instant::now(),
            backend_health: BackendHealthState::Unknown,
            backend_last_check: None,
            backend_check_history: Vec::new(),
            backend_check_interval: 15, // Start with 15 second checks
            mirror_health_checks: std::collections::HashMap::new(),
            system_status: {
                let mut status = crate::logging::SystemStatus::new();
                // Initialize CAPTCHA target from config
                status.captcha_pool.1 = captcha_pool_target;
                status
            },
            security_status: crate::logging::SecurityStatus::new(),
            network_events: crate::logging::NetworkEventBuffer::new(500),
            threat_events: crate::logging::NetworkEventBuffer::new(500),
            session_trust: std::collections::HashMap::new(),
        })
    }

    /// Main run loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        // Add initial log
        self.log_tx
            .send(LogEntry::info("Fortify TUI started"))
            .await
            .ok();

        loop {
            // Draw UI
            terminal.draw(|frame| ui::draw(frame, self))?;

            // Poll for events with timeout
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await?;
                }
            }

            // Receive logs
            while let Ok(entry) = self.log_rx.try_recv() {
                // Write to log file for persistence
                if let Some(ref mut file) = self.log_file {
                    let _ = writeln!(file, "{}", entry.format());
                }

                // Parse backend health check logs
                self.parse_backend_health_log(&entry);

                // Parse mirror health check logs
                self.parse_mirror_health_log(&entry);

                // Parse system status updates for dashboard
                self.parse_system_status_log(&entry);

                // Parse network traffic for traffic stream
                self.parse_network_traffic_log(&entry);

                if !self.logs_paused {
                    self.logs.push(entry);
                }
            }

            // Poll for mirror status when running (every 2 seconds)
            if self.deployment.is_running() {
                let should_poll = self
                    .last_mirror_poll
                    .map(|t| t.elapsed() > Duration::from_secs(2))
                    .unwrap_or(true);

                if should_poll {
                    self.update_mirror_status().await;
                    self.last_mirror_poll = Some(std::time::Instant::now());
                }
            }

            // Clear old status messages
            if let Some((_, time)) = &self.status_message {
                if time.elapsed() > Duration::from_secs(5) {
                    self.status_message = None;
                }
            }

            // Handle dependency check dialog auto-dismiss and deployment start
            if let Dialog::DependencyCheck {
                phase,
                completed_at,
                ..
            } = &self.dialog
            {
                if *phase == DependencyCheckPhase::Complete {
                    if let Some(completed_time) = completed_at {
                        if completed_time.elapsed() >= Duration::from_secs(2) {
                            // Auto-dismiss and start actual deployment
                            self.dialog = Dialog::None;
                            self.do_actual_deployment().await?;
                        }
                    }
                }
            }

            // Periodic stats logging (every 60 seconds)
            if self.deployment.is_running()
                && self.last_stats_log.elapsed() >= Duration::from_secs(60)
            {
                self.log_stats();
                self.last_stats_log = std::time::Instant::now();
            }

            // Periodic session cleanup (every 5 minutes)
            if self.last_session_cleanup.elapsed() >= Duration::from_secs(300) {
                self.cleanup_sessions();
                self.last_session_cleanup = std::time::Instant::now();
            }

            // Tick security status for bucket swaps, decay, and level computation
            // This ensures status degrades properly even without new events
            self.security_status.tick();

            // Check for security level changes and log attack events
            let current_level = self.security_status.level;
            if current_level != self.last_security_level {
                self.log_attack_event("STATE_CHANGE");
                self.last_security_level = current_level;
                self.last_attack_log_update = std::time::Instant::now();
            } else if current_level.is_elevated()
                && self.last_attack_log_update.elapsed() >= Duration::from_secs(60)
            {
                // Periodic update during elevated states
                self.log_attack_event("PERIODIC");
                self.last_attack_log_update = std::time::Instant::now();
            }

            // Check if should quit
            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Rotate log files, deleting logs older than 90 days
    fn rotate_logs(logs_dir: &std::path::Path) {
        let retention_days = 90;
        let now = std::time::SystemTime::now();

        if let Ok(entries) = std::fs::read_dir(logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Only process deployment-*.log files
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with("deployment-") || !name.ends_with(".log") {
                        continue;
                    }
                }

                // Check file age
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            let age_days = age.as_secs() / 86400;
                            if age_days > retention_days {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Log system stats (every 60 seconds)
    fn log_stats(&mut self) {
        if let Some(ref mut file) = self.stats_log_file {
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let status = &self.system_status;
            let security = &self.security_status;
            let (orch_cur, orch_tgt) = status.orchestrators;
            let (live, standby, total) = status.mirrors;
            let (captcha_cur, captcha_tgt) = status.captcha_pool;
            let sessions = self.session_trust.len();

            // Get rate from security status
            let rate = security.new_sessions_per_minute();

            let _ = writeln!(
                file,
                "{} | sessions={} rate={}/min | orch={}/{} mir={}/{}/{} cap={}/{} | backend={:?} security={}",
                now,
                sessions,
                rate,
                orch_cur, orch_tgt,
                live, standby, total,
                captcha_cur, captcha_tgt,
                self.backend_health,
                security.level.label()
            );
            let _ = file.flush();
        }
    }

    /// Log attack events (state changes and periodic updates)
    fn log_attack_event(&mut self, event_type: &str) {
        if let Some(ref mut file) = self.attack_log_file {
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let security = &self.security_status;
            let sessions = self.session_trust.len();

            // Calculate current rates using helper methods
            let new_rate = security.new_sessions_per_minute();
            let unverified = security.unverified_requests_per_minute();
            let failed_captcha = security.failed_captcha_attempts;

            let _ = writeln!(
                file,
                "{} {} level={} sessions={} new_rate={}/min unverified={} failed_captcha={}",
                now,
                event_type,
                security.level.label(),
                sessions,
                new_rate,
                unverified,
                failed_captcha
            );
            let _ = file.flush();
        }
    }

    /// Cleanup stale sessions (older than 30 minutes with no activity)
    fn cleanup_sessions(&mut self) {
        let max_age = Duration::from_secs(30 * 60); // 30 minutes
        let now = std::time::Instant::now();

        self.session_trust
            .retain(|_, trust| now.duration_since(trust.last_seen) < max_age);
    }

    /// Parse backend health check logs and update state
    fn parse_backend_health_log(&mut self, entry: &LogEntry) {
        let msg = &entry.message;

        // Only parse logs from controller
        if !entry.source.contains("controller") {
            return;
        }

        // Parse "Backend is now REACHABLE (took 234ms) - scaling down check frequency" or "Backend check: REACHABLE (123ms)"
        if msg.contains("Backend is now REACHABLE") || msg.contains("Backend check: REACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: true,
                    duration: Duration::from_millis(duration_ms),
                    error: None,
                });
                self.trim_health_history();
                // Calculate state from last 3 checks
                self.backend_health =
                    BackendHealthState::from_recent_checks(&self.backend_check_history);
            }
        }
        // Parse "Backend became UNREACHABLE (456ms) - increasing check frequency"
        else if msg.contains("Backend became UNREACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: false,
                    duration: Duration::from_millis(duration_ms),
                    error: Some("Connection timeout or circuit not ready".to_string()),
                });
                self.trim_health_history();
                // Calculate state from last 3 checks
                self.backend_health =
                    BackendHealthState::from_recent_checks(&self.backend_check_history);
            }
        }
        // Parse "Backend check: UNREACHABLE (789ms) - circuits may still be building..."
        else if msg.contains("Backend check: UNREACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: false,
                    duration: Duration::from_millis(duration_ms),
                    error: Some("Circuits still building".to_string()),
                });
                self.trim_health_history();
                // Calculate state from last 3 checks
                self.backend_health =
                    BackendHealthState::from_recent_checks(&self.backend_check_history);
            }
        }
        // Parse "Check interval adjusted to 30s"
        else if msg.contains("Check interval adjusted to") {
            if let Some(interval) = Self::extract_interval_seconds(msg) {
                self.backend_check_interval = interval;
            }
        }
        // Parse "Backend health checker started"
        else if msg.contains("Backend health checker started") {
            self.backend_health = BackendHealthState::Checking;
        }
    }

    /// Extract duration in milliseconds from log message like "(123ms)"
    fn extract_duration_ms(msg: &str) -> Option<u64> {
        // Look for pattern like "(123ms)" or "took 123ms"
        let re = regex::Regex::new(r"(?:\(|took )(\d+)ms").ok()?;
        let captures = re.captures(msg)?;
        captures.get(1)?.as_str().parse().ok()
    }

    /// Extract check interval from log message like "adjusted to 30s"
    fn extract_interval_seconds(msg: &str) -> Option<u64> {
        let re = regex::Regex::new(r"adjusted to (\d+)s").ok()?;
        let captures = re.captures(msg)?;
        captures.get(1)?.as_str().parse().ok()
    }

    /// Trim health check history to last 20 entries
    fn trim_health_history(&mut self) {
        if self.backend_check_history.len() > 20 {
            self.backend_check_history
                .drain(0..self.backend_check_history.len() - 20);
        }
    }

    /// Parse mirror health check logs
    fn parse_mirror_health_log(&mut self, entry: &LogEntry) {
        let msg = &entry.message;

        // Only parse logs from controller
        if !entry.source.contains("controller") {
            return;
        }

        // Parse "Mirror http://...onion is now REACHABLE (234ms, status: 302)" or "Mirror http://...onion check: REACHABLE (156ms, status: 302)" or "status: AVAILABLE"
        if msg.contains("Mirror ")
            && (msg.contains(" is now REACHABLE")
                || msg.contains(" check: REACHABLE")
                || msg.contains(" status: AVAILABLE")
                || msg.contains(" is now AVAILABLE"))
        {
            if let Some((mirror_addr, duration_ms)) = Self::extract_mirror_and_duration(msg) {
                let checks = self.mirror_health_checks.entry(mirror_addr).or_default();
                checks.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: true,
                    duration: Duration::from_millis(duration_ms),
                    error: None,
                });
                if checks.len() > 10 {
                    checks.drain(0..checks.len() - 10);
                }
            }
        }
        // Parse "Mirror http://...onion became UNREACHABLE: connection timeout"
        else if msg.contains("Mirror ") && msg.contains(" became UNREACHABLE") {
            if let Some((mirror_addr, _)) = Self::extract_mirror_and_duration(msg) {
                let checks = self.mirror_health_checks.entry(mirror_addr).or_default();
                checks.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: false,
                    duration: Duration::from_millis(0),
                    error: Some("Connection failed".to_string()),
                });
                if checks.len() > 10 {
                    checks.drain(0..checks.len() - 10);
                }
            }
        }
    }

    /// Parse log entries to update system status dashboard
    fn parse_system_status_log(&mut self, entry: &LogEntry) {
        use crate::logging::ComponentStatus;

        let msg = &entry.message;
        let source = &entry.source;

        // --- Tor daemon status ---
        // Tor runs separately - we infer its status from controller logs
        if source.contains("tor")
            || msg.contains("Tor")
            || msg.contains("tor")
            || msg.contains("SOCKS")
        {
            if msg.contains("Bootstrapped 100%")
                || msg.contains("Tor daemon is ready")
                || msg.contains("Tor started")
            {
                self.system_status.tor_daemon = ComponentStatus::Running;
            } else if msg.contains("Starting Tor")
                || msg.contains("Bootstrapped")
                || msg.contains("tor_service")
            {
                self.system_status.tor_daemon = ComponentStatus::Starting;
            } else if msg.contains("Tor daemon failed") || msg.contains("Tor error") {
                self.system_status.tor_daemon = ComponentStatus::Error;
            }
        }
        // If controller is using SOCKS proxy successfully, Tor must be running
        if source.contains("controller") && msg.contains("Using SOCKS proxy") {
            self.system_status.tor_daemon = ComponentStatus::Running;
        }
        // If mirrors become reachable via Tor, Tor is definitely working
        if msg.contains("is now REACHABLE") && msg.contains(".onion") {
            self.system_status.tor_daemon = ComponentStatus::Running;
        }
        // If backend is reachable through Tor, Tor is working
        if msg.contains("Backend is now REACHABLE") || msg.contains("Backend check: REACHABLE") {
            self.system_status.tor_daemon = ComponentStatus::Running;
        }

        // --- Gate status ---
        if source.contains("gate") || source.contains("fortify_gate") {
            if msg.contains("listening on")
                || msg.contains("Gate started")
                || msg.contains("server started")
            {
                self.system_status.gate = ComponentStatus::Running;
            } else if msg.contains("Starting gate") || msg.contains("Starting") {
                self.system_status.gate = ComponentStatus::Starting;
            } else if msg.contains("error") || msg.contains("failed") {
                self.system_status.gate = ComponentStatus::Error;
            }
        }

        // --- Controller status ---
        if source.contains("controller") || source.contains("fortify_controller") {
            if msg.contains("Controller ready") || msg.contains("Backend health checker started") {
                self.system_status.controller = ComponentStatus::Running;
            } else if msg.contains("Starting controller") || msg.contains("Starting") {
                self.system_status.controller = ComponentStatus::Starting;
            } else if msg.contains("error") {
                self.system_status.controller = ComponentStatus::Error;
            }
        }

        // --- Orchestrator status ---
        if source.contains("orchestrator") || source.contains("fortify_orchestrator") {
            // "Orchestrator starting" or "Orchestrator ready"
            if msg.contains("Orchestrator ready") || msg.contains("HTTP server listening") {
                self.system_status.orchestrator_status = ComponentStatus::Running;
                // Increment orchestrator count when we see "ready"
                // (will be corrected by API polling)
                if self.system_status.orchestrators.0 == 0 {
                    self.system_status.orchestrators.0 = 1;
                }
            } else if msg.contains("Orchestrator starting") || msg.contains("Starting") {
                self.system_status.orchestrator_status = ComponentStatus::Starting;
            }
        }

        // --- Mirror status (track individual spawns) ---
        if source.contains("orchestrator") || msg.contains("mirror") || msg.contains("Mirror") {
            // Count individual mirror spawns: "Mirror mirror-xxx spawned successfully"
            if msg.contains("spawned successfully") && !msg.contains("Standby") {
                let (live, standby, total) = self.system_status.mirrors;
                self.system_status.mirrors = (live + 1, standby, total + 1);
                self.system_status.mirror_status = ComponentStatus::Running;
            }
            // Count standby mirrors: "Standby mirror mirror-xxx spawned"
            if msg.contains("Standby mirror") && msg.contains("spawned") {
                let (live, standby, total) = self.system_status.mirrors;
                self.system_status.mirrors = (live, standby + 1, total + 1);
            }
            // Track when creating: "Creating hidden service" or "Spawning new mirror"
            if (msg.contains("Creating hidden service")
                || msg.contains("Spawning new mirror")
                || msg.contains("Spawning standby mirror"))
                && self.system_status.mirror_status != ComponentStatus::Running
            {
                self.system_status.mirror_status = ComponentStatus::Starting;
            }
        }

        // --- CAPTCHA pool status ---
        if msg.contains("CAPTCHA") || msg.contains("captcha") || msg.contains("Flex Core") {
            // Parse "CAPTCHA pool: size=450/500"
            if let Some((current, target)) = Self::extract_captcha_pool(msg) {
                self.system_status.captcha_pool = (current, target);
                self.system_status.captcha_status = if current >= target * 80 / 100 {
                    ComponentStatus::Running
                } else if current > 0 {
                    ComponentStatus::Starting
                } else {
                    ComponentStatus::Pending
                };
            }
            // "Starting Flex Core CAPTCHA pre-generation task (target: 500"
            if msg.contains("Starting Flex Core") || msg.contains("pre-generation task") {
                self.system_status.captcha_status = ComponentStatus::Starting;
                // Set target from log if visible: "target: 500"
                if let Some(target) = Self::extract_captcha_target(msg) {
                    self.system_status.captcha_pool.1 = target;
                }
            }
            // "CAPTCHA pool reached target" means we're at 100%
            if msg.contains("reached target") || msg.contains("persisting to disk") {
                let target = self.system_status.captcha_pool.1;
                if target > 0 {
                    self.system_status.captcha_pool.0 = target; // Set current = target
                } else {
                    // Fall back to configured value
                    let configured = self.config.captcha.pool_size;
                    self.system_status.captcha_pool = (configured, configured);
                }
                self.system_status.captcha_status = ComponentStatus::Running;
            }
        }

        // --- Deployment steps ---
        // Parse "Step 2/6: Initializing orchestrators"
        if msg.contains("Step ") {
            if let Some((current, total, desc)) = Self::extract_deploy_step(msg) {
                self.system_status.deploy_step = Some((current, total, desc));
            }
        }
        // Clear deploy step when deployment is complete
        if msg.contains("Deployment complete")
            || msg.contains("Deployment ready")
            || msg.contains("Deployment started successfully")
        {
            self.system_status.deploy_step = None;
        }

        self.system_status.touch();
    }

    /// Extract CAPTCHA target from log message like "target: 500"
    fn extract_captcha_target(msg: &str) -> Option<usize> {
        let re = regex::Regex::new(r"target:\s*(\d+)").ok()?;
        let captures = re.captures(msg)?;
        captures.get(1)?.as_str().parse().ok()
    }

    /// Extract CAPTCHA pool size from log message like "size=450/500"
    fn extract_captcha_pool(msg: &str) -> Option<(usize, usize)> {
        let re = regex::Regex::new(r"size=(\d+)/(\d+)").ok()?;
        let captures = re.captures(msg)?;
        let current: usize = captures.get(1)?.as_str().parse().ok()?;
        let target: usize = captures.get(2)?.as_str().parse().ok()?;
        Some((current, target))
    }

    /// Extract deployment step from log message like "Step 2/6: Initializing"
    fn extract_deploy_step(msg: &str) -> Option<(usize, usize, String)> {
        let re = regex::Regex::new(r"Step\s*(\d+)/(\d+):\s*(.+)").ok()?;
        let captures = re.captures(msg)?;
        let current: usize = captures.get(1)?.as_str().parse().ok()?;
        let total: usize = captures.get(2)?.as_str().parse().ok()?;
        let desc = captures.get(3)?.as_str().trim().to_string();
        Some((current, total, desc))
    }

    /// Parse log entries for network traffic events
    ///
    /// Log format from HTTP proxy:
    /// - `[{session_id}] +{idle} {path}` - session activity
    /// - `[{session_id}] +{idle} {path} → {event}` - session activity with event
    /// - `HEALTHY PATH: Routing {tier} user to backend: {path}`
    /// - `THREAT PATH: Proxying {tier} user to Gate for verification: {path}`
    /// - `GATE PATH: Routing to Gate service: {path}`
    fn parse_network_traffic_log(&mut self, entry: &LogEntry) {
        use crate::logging::{SessionEntry, SessionTrust};

        let msg = &entry.message;
        let source = &entry.source;

        // Only parse HTTP-related logs
        if !source.contains("http") && !source.contains("proxy") && !source.contains("fortify_http")
        {
            // Check for CAPTCHA-related logs from gate
            if source.contains("gate") || source.contains("fortify_gate") {
                // Track CAPTCHA verification
                if msg.contains("CAPTCHA")
                    && (msg.contains("verified")
                        || msg.contains("success")
                        || msg.contains("solved"))
                {
                    self.security_status.record_session_resolved();
                } else if msg.contains("CAPTCHA")
                    && (msg.contains("failed") || msg.contains("invalid"))
                {
                    self.security_status.record_failed_captcha();
                }
            }
            return;
        }

        // Pattern 1: Session activity log "[abc123] +5s /some/path"
        // This is the main traffic indicator
        if let Some(mut event) = Self::parse_session_activity_log(msg, entry.timestamp) {
            // Check if session trust level is known, or update from event content
            let session_key = event.session_id.clone();

            // Check if this is a new session (first time we've seen this ID)
            let is_new_session = !self.session_trust.contains_key(&session_key);
            if is_new_session {
                self.security_status.record_new_session();
            }

            // Check for trust indicators in the message
            if msg.contains("verified") || msg.contains("trusted") || msg.contains("authenticated")
            {
                // Session was verified - record this as resolved
                if self.session_trust.get(&session_key).map(|e| e.trust)
                    != Some(SessionTrust::Verified)
                {
                    self.security_status.record_session_resolved();
                }
                if let Some(entry) = self.session_trust.get_mut(&session_key) {
                    entry.update_trust(SessionTrust::Verified);
                } else {
                    self.session_trust.insert(
                        session_key.clone(),
                        SessionEntry::new(SessionTrust::Verified),
                    );
                }
            } else if msg.contains("banned")
                || msg.contains("killed")
                || msg.contains("threat")
                || msg.contains("suspicious")
            {
                if let Some(entry) = self.session_trust.get_mut(&session_key) {
                    entry.update_trust(SessionTrust::Threat);
                } else {
                    self.session_trust
                        .insert(session_key.clone(), SessionEntry::new(SessionTrust::Threat));
                }
                self.security_status
                    .add_suspicious_flag(&format!("threat:{}", session_key));
            } else if is_new_session {
                // New session with no trust indicators yet
                self.session_trust.insert(
                    session_key.clone(),
                    SessionEntry::new(SessionTrust::Unknown),
                );
            } else {
                // Existing session - just update last_seen
                if let Some(entry) = self.session_trust.get_mut(&session_key) {
                    entry.touch();
                }
            }

            // Get current trust level for this session
            let trust = self
                .session_trust
                .get(&session_key)
                .map(|e| e.trust)
                .unwrap_or(SessionTrust::Unknown);
            event.trust = trust;

            // Track unverified requests for security metrics
            if trust == SessionTrust::Unknown {
                self.security_status.record_unverified_request();
            }

            // Route to appropriate buffer - PENDING goes to verified panel (neutral), only THREAT goes to threat
            match trust {
                SessionTrust::Threat => self.threat_events.push(event),
                _ => self.network_events.push(event), // Verified and Unknown go to verified panel
            }

            // Update security level
            self.security_status.compute_level();
            return;
        }

        // Pattern 2: "HEALTHY PATH: Routing Verified user to backend: /path"
        if msg.contains("HEALTHY PATH:")
            || msg.contains("THREAT PATH:")
            || msg.contains("GATE PATH:")
        {
            if let Some(event) = Self::parse_routing_log(msg, entry.timestamp) {
                // Route based on trust level from the log
                match event.trust {
                    SessionTrust::Threat => {
                        self.security_status.record_unverified_request();
                        self.threat_events.push(event);
                    }
                    _ => self.network_events.push(event),
                }
            }
            // Update security level after routing logs
            self.security_status.compute_level();
            return;
        }

        // Pattern 3: Rate limiting - strong attack indicator
        // "Rate limited circuit: temp_unknown_Mozilla/5.0 tier=Unknown"
        if msg.contains("Rate limited") {
            self.security_status.record_unverified_request();
            self.security_status.add_suspicious_flag("rate_limited");
            self.security_status.compute_level();
            return;
        }

        // Pattern 4: Connection/request flood indicators
        if msg.contains("too many") || msg.contains("flood") || msg.contains("blocked") {
            self.security_status.record_unverified_request();
            self.security_status.add_suspicious_flag("flood_detected");
            self.security_status.compute_level();
        }
    }

    /// Parse session activity log: "[abc123] +5s /some/path" or "[abc123] +5s /path → event"
    fn parse_session_activity_log(
        msg: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Option<crate::logging::NetworkEvent> {
        use crate::logging::{HttpMethod, NetworkEvent, ResponseStatus};

        // Pattern: [SESSION] +IDLE PATH [→ EVENT]
        // Example: "[abc123] +5s /api/data" or "[abc123] +new /login → verified"
        let re = regex::Regex::new(r"\[([a-zA-Z0-9]+)\]\s+\+(\w+)\s+(\S+)(?:\s+→\s+(.+))?").ok()?;
        let captures = re.captures(msg)?;

        let session_id = captures.get(1)?.as_str().to_string();
        let path = captures.get(3)?.as_str().to_string();
        let event = captures.get(4).map(|m| m.as_str());

        // Determine status based on event
        let (status_code, status) = match event {
            Some(e) if e.contains("verified") || e.contains("success") => {
                (Some(200), ResponseStatus::Success)
            }
            Some(e) if e.contains("banned") || e.contains("killed") => {
                (Some(403), ResponseStatus::ClientError)
            }
            Some(e) if e.contains("error") => (Some(500), ResponseStatus::ServerError),
            Some(e) if e.contains("redirect") || e.contains("Gate") => {
                (Some(302), ResponseStatus::Redirect)
            }
            None => (Some(200), ResponseStatus::Success), // Normal request, assume success
            _ => (None, ResponseStatus::Pending),
        };

        // Infer HTTP method from path
        let method = if path.contains("/api/") || path.contains("/submit") || path.contains("/post")
        {
            HttpMethod::Post
        } else {
            HttpMethod::Get
        };

        Some(NetworkEvent {
            timestamp,
            session_id,
            method,
            path,
            status_code,
            status,
            duration_ms: None, // Not available from this log format
            size_bytes: None,
            mirror: None,
            is_asset_bundle: false,
            asset_count: 1,
            trust: crate::logging::SessionTrust::Unknown, // Will be set by caller
        })
    }

    /// Parse routing logs: "HEALTHY PATH: Routing Verified user to backend: /path"
    fn parse_routing_log(
        msg: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Option<crate::logging::NetworkEvent> {
        use crate::logging::{HttpMethod, NetworkEvent, ResponseStatus};

        // Extract the path from various routing log formats
        let path = if let Some(idx) = msg.rfind(": ") {
            msg[idx + 2..].trim().to_string()
        } else {
            return None;
        };

        // Skip internal paths
        if path.starts_with("/gate/") || path.starts_with("/ctrl_") {
            return None;
        }

        // Determine route type and status
        let (status, status_code, trust) = if msg.contains("HEALTHY PATH") {
            (
                ResponseStatus::Success,
                Some(200),
                crate::logging::SessionTrust::Verified,
            )
        } else if msg.contains("THREAT PATH") {
            (
                ResponseStatus::Redirect,
                Some(302),
                crate::logging::SessionTrust::Threat,
            )
        } else if msg.contains("GATE PATH") {
            (
                ResponseStatus::Redirect,
                Some(302),
                crate::logging::SessionTrust::Unknown,
            )
        } else {
            (
                ResponseStatus::Pending,
                None,
                crate::logging::SessionTrust::Unknown,
            )
        };

        // Generate a short session ID for display (since routing logs don't have session)
        // Use timestamp-based ID since we don't have rand crate
        let ts_part = timestamp.timestamp_millis() as u16;
        let session_id = format!("route-{:04x}", ts_part);

        Some(NetworkEvent {
            timestamp,
            session_id,
            method: HttpMethod::Get,
            path,
            status_code,
            status,
            duration_ms: None,
            size_bytes: None,
            mirror: None,
            is_asset_bundle: false,
            asset_count: 1,
            trust,
        })
    }

    /// Extract mirror address and duration from log message
    fn extract_mirror_and_duration(msg: &str) -> Option<(String, u64)> {
        // Extract mirror address (with or without http:// prefix)
        let re = regex::Regex::new(r"Mirror (?:https?://)?([a-z0-9]+\.onion)").ok()?;
        let captures = re.captures(msg)?;
        let mirror_addr = captures.get(1)?.as_str().to_string();

        // Extract duration
        let duration_ms = Self::extract_duration_ms(msg).unwrap_or(0);

        Some((mirror_addr, duration_ms))
    }

    /// Update mirror status from orchestrator API
    /// Also updates system_status dashboard with live data
    async fn update_mirror_status(&mut self) {
        use crate::logging::ComponentStatus;

        // Try to fetch from orchestrator at 127.0.0.1:8080
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let response = match client
            .get("http://127.0.0.1:8080/mirrors/extended")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return, // Orchestrator not ready yet
        };

        // If we got a response, orchestrator is running
        self.system_status.orchestrator_status = ComponentStatus::Running;

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(_) => return,
        };

        if let Some(mirrors) = json.get("mirrors").and_then(|m| m.as_array()) {
            let mut live_count = 0usize;
            let mut standby_count = 0usize;

            self.mirror_statuses = mirrors
                .iter()
                .enumerate()
                .filter_map(|(idx, m)| {
                    let onion = m.get("onion_address")?.as_str()?.to_string();
                    let status = m.get("status")?.as_str()?;
                    let is_standby = m
                        .get("is_standby")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let state = match status {
                        "active" => MirrorStatusState::Live,
                        "paused" => MirrorStatusState::Pending,
                        "burned" | "destroyed" => MirrorStatusState::Failed,
                        "dormant" => MirrorStatusState::Pending,
                        _ => MirrorStatusState::Verifying,
                    };

                    // Count for dashboard
                    if is_standby {
                        standby_count += 1;
                    } else if state == MirrorStatusState::Live {
                        live_count += 1;
                    }

                    // Extract just the address part (without .onion)
                    let address = onion.trim_end_matches(".onion").to_string();

                    Some(MirrorStatus {
                        index: idx,
                        address,
                        state,
                        is_standby,
                        vanity_prefix: None,
                        last_verified: Some(std::time::Instant::now()),
                    })
                })
                .collect();

            // Update system status dashboard with actual counts from API
            let total = live_count + standby_count;
            self.system_status.mirrors = (live_count, standby_count, total);
            self.system_status.mirror_status = if live_count > 0 {
                ComponentStatus::Running
            } else if total > 0 {
                ComponentStatus::Starting
            } else {
                ComponentStatus::Pending
            };
        }

        // Also try to get CAPTCHA pool stats from orchestrator
        if let Ok(stats_response) = client.get("http://127.0.0.1:8080/stats").send().await {
            if let Ok(stats_json) = stats_response.json::<serde_json::Value>().await {
                // Parse CAPTCHA pool stats if available
                if let Some(captcha) = stats_json.get("captcha_pool") {
                    let current = captcha
                        .get("current_size")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    let target = captcha
                        .get("target_size")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(500) as usize;
                    self.system_status.captcha_pool = (current, target);
                    self.system_status.captcha_status = if current >= target * 80 / 100 {
                        ComponentStatus::Running
                    } else if current > 0 {
                        ComponentStatus::Starting
                    } else {
                        ComponentStatus::Pending
                    };
                }

                // Parse orchestrator count if available
                if let Some(orch_count) = stats_json
                    .get("orchestrator_count")
                    .and_then(|v| v.as_u64())
                {
                    self.system_status.orchestrators.0 = orch_count as usize;
                }
            }
        }

        self.system_status.touch();
    }

    /// Handle key press
    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => {
                    if self.deployment.is_running() {
                        self.dialog = Dialog::Confirm {
                            title: "Stop Deployment".into(),
                            message: "Stop the running Fortify deployment?".into(),
                            on_confirm: DialogAction::Stop,
                        };
                        self.focus = Focus::Dialog;
                    } else {
                        self.should_quit = true;
                    }
                    return Ok(());
                }
                KeyCode::Char('q') => {
                    if self.deployment.is_running() {
                        self.dialog = Dialog::Confirm {
                            title: "Quit".into(),
                            message: "Stop deployment and quit?".into(),
                            on_confirm: DialogAction::Quit,
                        };
                        self.focus = Focus::Dialog;
                    } else {
                        self.should_quit = true;
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        // Handle dialog first
        if !matches!(self.dialog, Dialog::None) {
            return self.handle_dialog_key(key).await;
        }

        // Handle based on view and focus
        match (&self.view, self.focus) {
            (View::Home, Focus::Menu) => self.handle_menu_key(key).await,
            (View::Settings { .. }, Focus::Settings) => self.handle_settings_key(key).await,
            (View::DeployWizard { .. }, _) => self.handle_wizard_key(key).await,
            (View::ResumeSelect, _) => self.handle_resume_key(key).await,
            (View::Running, _) => self.handle_running_key(key).await,
            _ => self.handle_common_key(key).await,
        }
    }

    /// Handle menu navigation
    async fn handle_menu_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.menu_index > 0 {
                    self.menu_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.menu_index < MenuItem::all().len() - 1 {
                    self.menu_index += 1;
                }
            }
            KeyCode::Enter => {
                self.select_menu_item().await?;
            }
            KeyCode::Char(c) => {
                // Check hotkeys
                let upper = c.to_ascii_uppercase();
                for (i, item) in MenuItem::all().iter().enumerate() {
                    if item.hotkey() == upper {
                        self.menu_index = i;
                        self.select_menu_item().await?;
                        break;
                    }
                }
            }
            KeyCode::Tab => {
                self.focus = Focus::Logs;
            }
            _ => {}
        }
        Ok(())
    }

    /// Select current menu item
    async fn select_menu_item(&mut self) -> Result<()> {
        match MenuItem::all()[self.menu_index] {
            MenuItem::Deploy => {
                // Check if we have an existing config (previously deployed)
                let config_path = FortifyConfig::default_path();
                if config_path.exists() {
                    // Quick Deploy validation: Check if Tor data directory exists
                    // Backend address is USER-CONFIGURED and never changes - do NOT validate against Tor keys
                    let config = FortifyConfig::load(&config_path)?;
                    let tor_data_dir =
                        std::path::PathBuf::from(&config.network.data_dir).join("tor");

                    // Check if we have existing Tor data to reuse
                    let can_quick_deploy = tor_data_dir.exists()
                        && tor_data_dir.join("data").exists()
                        && tor_data_dir.join("torrc").exists();

                    if can_quick_deploy {
                        // Existing deployment found - offer Quick Deploy
                        self.dialog = Dialog::Confirm {
                            title: "Existing Configuration Found".into(),
                            message: "Previous deployment detected.\n\nQuick Deploy (Y) - Resume with existing configuration\nFull Setup (N) - Reconfigure all settings".into(),
                            on_confirm: DialogAction::QuickDeploy,
                        };
                        self.focus = Focus::Dialog;
                    } else {
                        // Config exists but Tor data missing - force full deploy
                        self.log_tx
                            .send(LogEntry::warn(
                                "Config exists but Tor data is missing. Full deployment required.",
                            ))
                            .await
                            .ok();
                        self.view = View::DeployWizard { step: 0 };
                        self.focus = Focus::Settings;
                    }
                } else {
                    // No existing config - go to wizard
                    self.view = View::DeployWizard { step: 0 };
                    self.focus = Focus::Settings;
                }
            }
            MenuItem::JoinNetwork => {
                self.view = View::JoinNetwork;
            }
            MenuItem::Settings => {
                self.view = View::Settings {
                    tab: SettingsTab::TrafficTier,
                    field_index: 0,
                };
                self.focus = Focus::Settings;
            }
            MenuItem::Status => {
                // If deployment is running, show the Running view instead of Status
                if self.deployment.is_running() {
                    self.view = View::Running;
                } else {
                    self.view = View::Status;
                }
            }
            MenuItem::Destroy => {
                // First confirmation
                self.dialog = Dialog::Confirm {
                    title: "⚠ DESTROY INSTANCE ⚠".into(),
                    message: "This will PERMANENTLY delete:\n• All configuration\n• All deployment data\n• All mirror keys\n• All logs\n\nThis action CANNOT be undone!\n\nAre you sure you want to continue?".into(),
                    on_confirm: DialogAction::DestroyConfirm1,
                };
                self.focus = Focus::Dialog;
            }
            MenuItem::Quit => {
                if self.deployment.is_running() {
                    self.dialog = Dialog::Confirm {
                        title: "Quit".into(),
                        message: "Stop deployment and quit?".into(),
                        on_confirm: DialogAction::Quit,
                    };
                    self.focus = Focus::Dialog;
                } else if self.config.is_dirty() {
                    // Warn about unsaved changes
                    self.dialog = Dialog::Confirm {
                        title: "Unsaved Changes".into(),
                        message: "You have unsaved configuration changes.\n\nQuit without saving?"
                            .into(),
                        on_confirm: DialogAction::Quit,
                    };
                    self.focus = Focus::Dialog;
                } else {
                    self.should_quit = true;
                }
            }
        }
        Ok(())
    }

    /// Handle dialog input
    async fn handle_dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        match &self.dialog {
            Dialog::Confirm { on_confirm, .. } => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        let action = on_confirm.clone();
                        self.dialog = Dialog::None;
                        self.focus = Focus::Menu;
                        self.execute_dialog_action(action).await?;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        // Special handling for QuickDeploy dialog - N goes to full wizard
                        if matches!(on_confirm, DialogAction::QuickDeploy)
                            && matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N'))
                        {
                            self.dialog = Dialog::None;
                            self.view = View::DeployWizard { step: 0 };
                            self.focus = Focus::Settings;
                        } else {
                            self.dialog = Dialog::None;
                            self.focus = Focus::Menu;
                        }
                    }
                    _ => {}
                }
            }
            Dialog::ApplyChanges { .. } => {
                match key.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        // Apply now
                        self.apply_changes_now().await?;
                        self.dialog = Dialog::None;
                        self.focus = Focus::Settings;
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        // Store for later
                        self.changes.store_for_restart();
                        self.status_message = Some((
                            "Changes stored for next restart".into(),
                            std::time::Instant::now(),
                        ));
                        self.dialog = Dialog::None;
                        self.focus = Focus::Settings;
                    }
                    KeyCode::Esc => {
                        self.dialog = Dialog::None;
                        self.focus = Focus::Settings;
                    }
                    _ => {}
                }
            }
            Dialog::Input { value, field, .. } => {
                match key.code {
                    KeyCode::Enter => {
                        let field = field.clone();
                        let value = value.clone();

                        // Special validation for Backend Address
                        if field == "Backend Address" {
                            if let Some(warning) = self.validate_backend_address(&value) {
                                self.log_tx.send(LogEntry::warn(&warning)).await.ok();
                                self.status_message =
                                    Some((warning.clone(), std::time::Instant::now()));
                            }
                        }

                        self.apply_input_value(&field, &value);

                        // Auto-save after each field edit for persistence
                        if let Err(e) = self.config.save() {
                            self.log_tx
                                .send(LogEntry::error(&format!("Failed to save config: {}", e)))
                                .await
                                .ok();
                        }

                        self.dialog = Dialog::None;
                        self.focus = Focus::Settings;
                    }
                    KeyCode::Esc => {
                        self.dialog = Dialog::None;
                        self.focus = Focus::Settings;
                    }
                    KeyCode::Backspace => {
                        if let Dialog::Input { value, .. } = &mut self.dialog {
                            value.pop();
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Dialog::Input { value, .. } = &mut self.dialog {
                            value.push(c);
                        }
                    }
                    _ => {}
                }
            }
            Dialog::Error { .. } | Dialog::Info { .. } => {
                // Accept any key to dismiss
                self.dialog = Dialog::None;
                self.focus = Focus::Menu;
            }
            Dialog::DependencyCheck { phase, .. } => {
                match key.code {
                    KeyCode::Esc => {
                        // Cancel deployment
                        self.dialog = Dialog::None;
                        self.focus = Focus::Menu;
                        self.log_tx
                            .send(LogEntry::info("Deployment cancelled"))
                            .await
                            .ok();
                    }
                    KeyCode::Char('r') | KeyCode::Char('R')
                        if *phase == DependencyCheckPhase::Failed =>
                    {
                        // Retry dependency check
                        self.dialog = Dialog::None;
                        self.start_deployment().await?;
                    }
                    _ => {
                        // Ignore other keys during check/install
                    }
                }
            }
            Dialog::None => {}
        }
        Ok(())
    }

    /// Execute dialog action
    async fn execute_dialog_action(&mut self, action: DialogAction) -> Result<()> {
        match action {
            DialogAction::Deploy => {
                self.start_deployment().await?;
            }
            DialogAction::QuickDeploy => {
                // Quick deploy - use existing config directly
                self.log_tx
                    .send(LogEntry::info("Quick Deploy: Using existing configuration"))
                    .await
                    .ok();
                self.start_deployment().await?;
            }
            DialogAction::Stop => {
                self.deployment.stop().await?;
                self.view = View::Home;
                self.log_tx
                    .send(LogEntry::info("Deployment stopped"))
                    .await
                    .ok();
            }
            DialogAction::Quit => {
                self.deployment.stop().await?;
                self.should_quit = true;
            }
            DialogAction::ApplyNow => {
                self.apply_changes_now().await?;
            }
            DialogAction::StoreLater => {
                self.changes.store_for_restart();
            }
            DialogAction::DestroyConfirm1 => {
                // Second confirmation for destroy
                self.dialog = Dialog::Confirm {
                    title: "⚠ FINAL WARNING ⚠".into(),
                    message: "You are about to DESTROY all Fortify data.\n\nType 'Y' to confirm complete destruction.\n\nThis is your LAST chance to cancel!".into(),
                    on_confirm: DialogAction::DestroyConfirm2,
                };
                self.focus = Focus::Dialog;
            }
            DialogAction::DestroyConfirm2 => {
                // Execute destruction
                self.destroy_instance().await?;
            }
            DialogAction::InstallDeps => {
                // Install missing dependencies
                self.install_missing_deps().await?;
            }
            DialogAction::None => {}
        }
        Ok(())
    }

    /// Destroy all Fortify data and configuration
    async fn destroy_instance(&mut self) -> Result<()> {
        // Stop any running deployment first
        if self.deployment.is_running() {
            self.deployment.stop().await?;
        }

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Warn,
                "destroy",
                "Beginning instance destruction...",
            ))
            .await
            .ok();

        // Remove all Fortify data directories (both old /tmp and new persistent locations)
        let home = std::env::var("HOME").unwrap_or_default();
        let paths_to_remove = [
            "/tmp/fortify".to_string(),
            "/var/lib/fortify".to_string(),
            format!("{}/.config/fortify", home),
            format!("{}/.local/share/fortify", home),
        ];

        for path in &paths_to_remove {
            let p = std::path::Path::new(path);
            if p.exists() {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Info,
                        "destroy",
                        &format!("Removing: {}", path),
                    ))
                    .await
                    .ok();

                if let Err(e) = std::fs::remove_dir_all(p) {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Warn,
                            "destroy",
                            &format!("Failed to remove {}: {}", path, e),
                        ))
                        .await
                        .ok();
                }
            }
        }

        // Kill any remaining Tor processes started by Fortify
        let _ = tokio::process::Command::new("pkill")
            .arg("-f")
            .arg("tor.*fortify")
            .status()
            .await;

        // Reset config to defaults
        self.config = FortifyConfig::default();
        self.mirror_statuses.clear();
        self.existing_deployments.clear();

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "destroy",
                "Instance destroyed. All data has been removed.",
            ))
            .await
            .ok();

        self.dialog = Dialog::Info {
            title: "Instance Destroyed".into(),
            message: "All Fortify data has been removed.\n\nThe system is now in a clean state."
                .into(),
        };
        self.focus = Focus::Dialog;

        Ok(())
    }

    /// Install missing dependencies
    async fn install_missing_deps(&mut self) -> Result<()> {
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Starting dependency installation...",
            ))
            .await
            .ok();

        let (installed, failed) = self.deployment.install_missing_dependencies().await?;

        if failed == 0 && installed > 0 {
            self.dialog = Dialog::Info {
                title: "Dependencies Installed".into(),
                message: format!("Successfully installed {} dependencies.\n\nYou can now proceed with deployment.", installed),
            };
        } else if failed > 0 {
            self.dialog = Dialog::Info {
                title: "Installation Incomplete".into(),
                message: format!(
                    "Installed: {}\nFailed: {}\n\nSome dependencies may need manual installation.",
                    installed, failed
                ),
            };
        } else {
            self.dialog = Dialog::Info {
                title: "Nothing to Install".into(),
                message: "All dependencies are already installed.".into(),
            };
        }
        self.focus = Focus::Dialog;

        Ok(())
    }

    /// Handle settings panel keys
    async fn handle_settings_key(&mut self, key: KeyEvent) -> Result<()> {
        if let View::Settings { tab, field_index } = &mut self.view {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    // Previous tab
                    let tabs = SettingsTab::all();
                    let current = tabs.iter().position(|t| t == tab).unwrap_or(0);
                    if current > 0 {
                        *tab = tabs[current - 1];
                        *field_index = 0;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    // Next tab
                    let tabs = SettingsTab::all();
                    let current = tabs.iter().position(|t| t == tab).unwrap_or(0);
                    if current < tabs.len() - 1 {
                        *tab = tabs[current + 1];
                        *field_index = 0;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if *field_index > 0 {
                        *field_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    *field_index += 1; // Will be clamped in render
                }
                KeyCode::Enter => {
                    self.edit_current_field().await?;
                }
                KeyCode::Tab => {
                    self.focus = Focus::Logs;
                }
                KeyCode::Esc => {
                    // Check for unsaved changes
                    if self.config.is_dirty() && self.deployment.is_running() {
                        self.dialog = Dialog::ApplyChanges {
                            changes: self
                                .changes
                                .pending_changes
                                .iter()
                                .map(|c| format!("{}: {} → {}", c.field, c.old_value, c.new_value))
                                .collect(),
                        };
                        self.focus = Focus::Dialog;
                    } else {
                        self.view = View::Home;
                        self.focus = Focus::Menu;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Handle wizard keys
    async fn handle_wizard_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle log panel scrolling when focused on logs
        if self.focus == Focus::Logs {
            match key.code {
                KeyCode::PageUp => {
                    self.log_scroll = self.log_scroll.saturating_add(10);
                    if self.log_select_mode {
                        self.log_selected_line = 0; // Reset selection on scroll
                    }
                    return Ok(());
                }
                KeyCode::PageDown => {
                    self.log_scroll = self.log_scroll.saturating_sub(10);
                    if self.log_select_mode {
                        self.log_selected_line = 0;
                    }
                    return Ok(());
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.log_select_mode {
                        // Move selection up
                        self.log_selected_line = self.log_selected_line.saturating_add(1);
                    } else {
                        self.log_scroll = self.log_scroll.saturating_add(1);
                    }
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.log_select_mode {
                        // Move selection down
                        self.log_selected_line = self.log_selected_line.saturating_sub(1);
                    } else {
                        self.log_scroll = self.log_scroll.saturating_sub(1);
                    }
                    return Ok(());
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Toggle selection mode
                    self.log_select_mode = !self.log_select_mode;
                    if self.log_select_mode {
                        self.log_selected_line = 0;
                        self.logs_paused = true; // Auto-pause when selecting
                    }
                    return Ok(());
                }
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    // Copy selected line to clipboard
                    if self.log_select_mode {
                        self.copy_selected_log();
                    }
                    return Ok(());
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.logs_paused = !self.logs_paused;
                    return Ok(());
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if !self.log_select_mode {
                        self.logs.clear();
                        self.log_scroll = 0;
                    }
                    return Ok(());
                }
                KeyCode::Tab => {
                    self.focus = Focus::Menu;
                    self.log_select_mode = false;
                    return Ok(());
                }
                KeyCode::Esc => {
                    if self.log_select_mode {
                        self.log_select_mode = false;
                    } else {
                        self.focus = Focus::Menu;
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        if let View::DeployWizard { step } = &mut self.view {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') if *step > 0 => {
                    *step -= 1;
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if *step < 6 {
                        *step += 1;
                    }
                }
                KeyCode::Enter => {
                    if *step >= 6 {
                        // Final step - deploy
                        self.dialog = Dialog::Confirm {
                            title: "Deploy".into(),
                            message: "Start deployment with current settings?".into(),
                            on_confirm: DialogAction::Deploy,
                        };
                        self.focus = Focus::Dialog;
                    } else {
                        *step += 1;
                    }
                }
                KeyCode::Char('i') | KeyCode::Char('I') if *step == 0 => {
                    // Install missing dependencies
                    self.dialog = Dialog::Confirm {
                        title: "Install Dependencies".into(),
                        message: "Install missing dependencies? This may require sudo.".into(),
                        on_confirm: DialogAction::InstallDeps,
                    };
                    self.focus = Focus::Dialog;
                }
                KeyCode::Esc => {
                    self.view = View::Home;
                    self.focus = Focus::Menu;
                }
                KeyCode::Tab => {
                    self.focus = Focus::Logs;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Handle resume selection keys
    async fn handle_resume_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.resume_index > 0 {
                    self.resume_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.resume_index < self.existing_deployments.len().saturating_sub(1) {
                    self.resume_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some((_, path)) = self.existing_deployments.get(self.resume_index) {
                    if let Ok(config) = FortifyConfig::load(path) {
                        self.config = config;
                        self.start_deployment().await?;
                    }
                }
            }
            KeyCode::Esc => {
                self.view = View::Home;
                self.focus = Focus::Menu;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle running view keys
    async fn handle_running_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle log scrolling when focused on logs
        if self.focus == Focus::Logs {
            match key.code {
                KeyCode::PageUp => {
                    self.log_scroll = self.log_scroll.saturating_add(10);
                    return Ok(());
                }
                KeyCode::PageDown => {
                    self.log_scroll = self.log_scroll.saturating_sub(10);
                    return Ok(());
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.log_scroll = self.log_scroll.saturating_add(1);
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.log_scroll = self.log_scroll.saturating_sub(1);
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.view = View::Settings {
                    tab: SettingsTab::TrafficTier,
                    field_index: 0,
                };
                self.focus = Focus::Settings;
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.logs_paused = !self.logs_paused;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.logs.clear();
                self.log_scroll = 0;
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Cycle log filter
                self.log_filter = match self.log_filter {
                    LogLevel::Trace => LogLevel::Debug,
                    LogLevel::Debug => LogLevel::Info,
                    LogLevel::Info => LogLevel::Warn,
                    LogLevel::Warn => LogLevel::Error,
                    LogLevel::Error => LogLevel::Trace,
                };
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // Export mirror addresses to file
                self.export_mirror_addresses();
            }
            KeyCode::Tab => {
                self.focus = if self.focus == Focus::Logs {
                    Focus::Menu
                } else {
                    Focus::Logs
                };
            }
            KeyCode::Esc => {
                self.dialog = Dialog::Confirm {
                    title: "Stop".into(),
                    message: "Stop the running deployment?".into(),
                    on_confirm: DialogAction::Stop,
                };
                self.focus = Focus::Dialog;
            }
            _ => {}
        }
        Ok(())
    }

    /// Export mirror addresses to a file
    fn export_mirror_addresses(&mut self) {
        if self.mirror_statuses.is_empty() {
            self.dialog = Dialog::Error {
                message: "No mirrors available to export yet.".into(),
            };
            self.focus = Focus::Dialog;
            return;
        }

        let export_path =
            std::path::PathBuf::from(&self.config.network.data_dir).join("mirror-addresses.txt");

        // Separate live and standby mirrors
        let live_mirrors: Vec<_> = self
            .mirror_statuses
            .iter()
            .filter(|m| !m.is_standby)
            .collect();
        let standby_mirrors: Vec<_> = self
            .mirror_statuses
            .iter()
            .filter(|m| m.is_standby)
            .collect();

        // Build the content
        let mut content = String::new();
        content.push_str("# Fortify Mirror Addresses\n");
        content.push_str(&format!(
            "# Exported: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        content.push_str(&format!(
            "# Service: {}\n",
            self.config.branding.service_name
        ));
        content.push_str(&format!(
            "# Backend: {}\n\n",
            self.config.network.backend_address
        ));

        // Control panel link (use first live mirror)
        let admin_path = "/ctrl_8f7k3m9x2n4p1q6w5v0b8c";
        if let Some(first_live) = live_mirrors.first() {
            content.push_str(&format!(
                "## CONTROL PANEL:\nhttp://{}.onion{}\n\n",
                first_live.address, admin_path
            ));
        }

        // Live mirrors section
        content.push_str(&format!("## LIVE MIRRORS ({}):\n", live_mirrors.len()));
        for mirror in &live_mirrors {
            let status = mirror.state.label();
            content.push_str(&format!("http://{}.onion  # {}\n", mirror.address, status));
        }

        // Standby mirrors section
        if !standby_mirrors.is_empty() {
            content.push_str(&format!(
                "\n## STANDBY MIRRORS ({}):\n",
                standby_mirrors.len()
            ));
            for mirror in &standby_mirrors {
                let status = mirror.state.label();
                content.push_str(&format!(
                    "http://{}.onion  # {} [STANDBY]\n",
                    mirror.address, status
                ));
            }
        }

        // Plain addresses section - live first, then standby
        content.push_str("\n# Plain addresses (for easy copying):\n\n");
        content.push_str("# Live:\n");
        for mirror in &live_mirrors {
            content.push_str(&format!("http://{}.onion\n", mirror.address));
        }
        if !standby_mirrors.is_empty() {
            content.push_str("\n# Standby:\n");
            for mirror in &standby_mirrors {
                content.push_str(&format!("http://{}.onion\n", mirror.address));
            }
        }

        // Write the file
        match std::fs::write(&export_path, &content) {
            Ok(_) => {
                // Open the file with the default text editor
                let open_result = std::process::Command::new("xdg-open")
                    .arg(&export_path)
                    .spawn();

                match open_result {
                    Ok(_) => {
                        self.dialog = Dialog::Info {
                            title: "Addresses Exported".into(),
                            message: format!(
                                "Opened {} addresses in text editor.\n\nFile: {}",
                                self.mirror_statuses.len(),
                                export_path.display()
                            ),
                        };
                    }
                    Err(_) => {
                        // Fallback: just show the path if xdg-open fails
                        self.dialog = Dialog::Info {
                            title: "Addresses Exported".into(),
                            message: format!(
                                "Mirror addresses saved to:\n\n{}\n\n{} addresses exported.",
                                export_path.display(),
                                self.mirror_statuses.len()
                            ),
                        };
                    }
                }
                self.focus = Focus::Dialog;
            }
            Err(e) => {
                self.dialog = Dialog::Error {
                    message: format!("Failed to export: {}", e),
                };
                self.focus = Focus::Dialog;
            }
        }
    }

    /// Handle common keys (log panel focus etc)
    async fn handle_common_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Menu => Focus::Logs,
                    Focus::Logs => Focus::Menu,
                    Focus::Settings => Focus::Logs,
                    Focus::Dialog => Focus::Dialog,
                };
            }
            KeyCode::Esc => {
                self.view = View::Home;
                self.focus = Focus::Menu;
            }
            KeyCode::Char('p') | KeyCode::Char('P') if self.focus == Focus::Logs => {
                self.logs_paused = !self.logs_paused;
            }
            KeyCode::Char('c') | KeyCode::Char('C') if self.focus == Focus::Logs => {
                self.logs.clear();
            }
            KeyCode::PageUp if self.focus == Focus::Logs => {
                self.log_scroll = self.log_scroll.saturating_add(10);
            }
            KeyCode::PageDown if self.focus == Focus::Logs => {
                self.log_scroll = self.log_scroll.saturating_sub(10);
            }
            _ => {}
        }
        Ok(())
    }

    /// Copy the selected log line to clipboard
    fn copy_selected_log(&mut self) {
        // Get visible logs
        let visible_height = 20; // Approximate, will be updated in render
        let logs = self
            .logs
            .scroll(self.log_scroll, visible_height, self.log_filter);

        if let Some(entry) = logs.get(self.log_selected_line) {
            let text = format!(
                "{} {} [{}] {}",
                entry.timestamp.format("%H:%M:%S"),
                entry.level.symbol(),
                entry.source,
                entry.message
            );

            // Try to copy to clipboard using various methods
            if Self::copy_to_clipboard(&text).is_err() {
                // Fallback: save to a temp file that user can access
                let path_buf =
                    std::path::PathBuf::from(&self.config.network.data_dir).join("copied_log.txt");
                let path_str = path_buf.to_str().unwrap_or("/tmp/fortify/copied_log.txt");
                let _ = std::fs::write(&path_buf, &text);
                self.status_message =
                    Some((format!("Saved to {}", path_str), std::time::Instant::now()));
            } else {
                self.status_message = Some((
                    "Copied to clipboard!".to_string(),
                    std::time::Instant::now(),
                ));
            }

            // Exit selection mode after copy
            self.log_select_mode = false;
        }
    }

    /// Copy text to system clipboard
    fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Try xclip first (Linux X11)
        if let Ok(mut child) = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            return Ok(());
        }

        // Try xsel (Linux X11 alternative)
        if let Ok(mut child) = Command::new("xsel")
            .arg("--clipboard")
            .arg("--input")
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            return Ok(());
        }

        // Try wl-copy (Wayland)
        if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            return Ok(());
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No clipboard tool found",
        ))
    }

    /// Edit the currently selected settings field
    async fn edit_current_field(&mut self) -> Result<()> {
        if let View::Settings { tab, field_index } = &self.view {
            let (field_name, current_value) = self.get_field_info(*tab, *field_index);

            self.dialog = Dialog::Input {
                title: format!("Edit {}", field_name),
                value: current_value,
                field: field_name,
            };
            self.focus = Focus::Dialog;
        }
        Ok(())
    }

    /// Get field name and current value for settings
    fn get_field_info(&self, tab: SettingsTab, index: usize) -> (String, String) {
        let unknown: (String, String) = ("Unknown".to_string(), String::new());
        match tab {
            SettingsTab::TrafficTier => {
                let tier = self.config.traffic_tier;
                let tier_name = match tier {
                    crate::config::TrafficTier::Micro => "Micro (~100/day)",
                    crate::config::TrafficTier::Small => "Small (~1,000/day)",
                    crate::config::TrafficTier::Medium => "Medium (~10,000/day)",
                    crate::config::TrafficTier::Large => "Large (~100,000/day)",
                    crate::config::TrafficTier::Enterprise => "Enterprise (~1M+/day)",
                };
                let fields: [(String, String); 6] = [
                    ("Traffic Tier".to_string(), tier_name.to_string()),
                    (
                        "Rate Limit".to_string(),
                        format!(
                            "{} RPM ({}x)",
                            tier.rate_limit_rpm(),
                            tier.rate_limit_multiplier()
                        ),
                    ),
                    ("CAPTCHA Pool".to_string(), tier.pool_size().to_string()),
                    (
                        "CPU (min/rec)".to_string(),
                        format!(
                            "{}/{} cores",
                            tier.min_cpu_cores(),
                            tier.recommended_cpu_cores()
                        ),
                    ),
                    (
                        "RAM (min/rec)".to_string(),
                        format!(
                            "{}GB/{}GB",
                            tier.min_ram_mb() / 1024,
                            tier.recommended_ram_mb() / 1024
                        ),
                    ),
                    (
                        "Disk (min)".to_string(),
                        format!("{}GB", tier.min_disk_mb() / 1024),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Branding => {
                let fields: [(String, String); 5] = [
                    (
                        "Service Name".to_string(),
                        self.config.branding.service_name.clone(),
                    ),
                    (
                        "Description".to_string(),
                        self.config.branding.description.clone(),
                    ),
                    (
                        "Welcome Message".to_string(),
                        self.config.branding.welcome_message.clone(),
                    ),
                    (
                        "Primary Color".to_string(),
                        self.config.branding.primary_color.clone(),
                    ),
                    (
                        "Secondary Color".to_string(),
                        self.config.branding.secondary_color.clone(),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Captcha => {
                // Format cycling types as comma-separated list
                let cycling_types_str: String = self
                    .config
                    .captcha
                    .cycling_types
                    .iter()
                    .map(|t| t.display_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                
                let fields: [(String, String); 13] = [
                    (
                        "Enabled".to_string(),
                        self.config.captcha.enabled.to_string(),
                    ),
                    (
                        "Gate CAPTCHA Type".to_string(),
                        self.config.captcha.gate_captcha_type.display_name().to_string(),
                    ),
                    (
                        "Threat Type Enabled".to_string(),
                        self.config.captcha.threat_captcha_enabled.to_string(),
                    ),
                    (
                        "Threat CAPTCHA Type".to_string(),
                        self.config.captcha.threat_captcha_type.display_name().to_string(),
                    ),
                    (
                        "Random Cycling".to_string(),
                        self.config.captcha.random_cycling.to_string(),
                    ),
                    (
                        "Cycling Types".to_string(),
                        cycling_types_str,
                    ),
                    (
                        "Pool Size".to_string(),
                        self.config.captcha.pool_size.to_string(),
                    ),
                    (
                        "Min Pool".to_string(),
                        self.config.captcha.min_pool_size.to_string(),
                    ),
                    (
                        "Max Pool".to_string(),
                        self.config.captcha.max_pool_size.to_string(),
                    ),
                    (
                        "Difficulty (1-10)".to_string(),
                        self.config.captcha.difficulty.to_string(),
                    ),
                    (
                        "Timeout (seconds)".to_string(),
                        self.config.captcha.timeout_seconds.to_string(),
                    ),
                    (
                        "Max Attempts".to_string(),
                        self.config.captcha.max_attempts.to_string(),
                    ),
                    (
                        "Rotation Days".to_string(),
                        self.config.captcha.rotation_interval_days.to_string(),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Thresholds => {
                let fields: [(String, String); 10] = [
                    (
                        "Rate Limit (req/min)".to_string(),
                        self.config.thresholds.rate_limit_rpm.to_string(),
                    ),
                    (
                        "CAPTCHA Fail Limit".to_string(),
                        self.config.thresholds.captcha_fail_limit.to_string(),
                    ),
                    (
                        "Temp Ban Duration (min)".to_string(),
                        self.config.thresholds.temp_ban_minutes.to_string(),
                    ),
                    (
                        "Perm Ban Threshold".to_string(),
                        self.config.thresholds.perm_ban_threshold.to_string(),
                    ),
                    (
                        "Suspicion Threshold".to_string(),
                        format!("{:.1}", self.config.thresholds.suspicion_threshold),
                    ),
                    (
                        "Threat Threshold".to_string(),
                        format!("{:.1}", self.config.thresholds.threat_threshold),
                    ),
                    (
                        "Burn Threshold".to_string(),
                        self.config.thresholds.burn_threshold.to_string(),
                    ),
                    (
                        "Auto Ban Enabled".to_string(),
                        self.config.thresholds.auto_ban_enabled.to_string(),
                    ),
                    (
                        "DDoS RPS Threshold".to_string(),
                        self.config.thresholds.ddos_rps_threshold.to_string(),
                    ),
                    (
                        "Probe Sensitivity (1-10)".to_string(),
                        self.config.thresholds.probe_sensitivity.to_string(),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Network => {
                let fields: [(String, String); 5] = [
                    (
                        "Backend Address".to_string(),
                        self.config.network.backend_address.clone(),
                    ),
                    (
                        "HTTP Bind".to_string(),
                        self.config.network.http_bind.clone(),
                    ),
                    (
                        "Gate Bind".to_string(),
                        self.config.network.gate_bind.clone(),
                    ),
                    (
                        "SOCKS Port".to_string(),
                        self.config.network.socks_port.to_string(),
                    ),
                    (
                        "Control Port".to_string(),
                        self.config.network.control_port.to_string(),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Mirrors => {
                let fields: [(String, String); 5] = [
                    (
                        "Min Mirrors".to_string(),
                        self.config.mirrors.min_mirrors.to_string(),
                    ),
                    (
                        "Max Mirrors".to_string(),
                        self.config.mirrors.max_mirrors.to_string(),
                    ),
                    (
                        "Standby Mirrors".to_string(),
                        self.config.mirrors.standby_mirrors.to_string(),
                    ),
                    (
                        "Rotation (sec)".to_string(),
                        self.config.mirrors.rotation_interval_seconds.to_string(),
                    ),
                    (
                        "Burn Min Days".to_string(),
                        self.config.mirrors.burn_interval_days_min.to_string(),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Vanity => {
                // MUST match the display order in ui/settings.rs draw_vanity()
                let fields: [(String, String); 7] = [
                    (
                        "Vanity Enabled".to_string(),
                        self.config.vanity.enabled.to_string(),
                    ),
                    ("Prefix".to_string(), self.config.vanity.prefix.clone()),
                    (
                        "Prefix Length".to_string(),
                        format!("{}/10", self.config.vanity.prefix.len()),
                    ), // Display only
                    (
                        "Safety Net Enabled".to_string(),
                        self.config.vanity.safety_net_enabled.to_string(),
                    ),
                    (
                        "Vanity Timeout (sec)".to_string(),
                        self.config.vanity.safety_net_timeout_seconds.to_string(),
                    ),
                    (
                        "Min Prefix Length".to_string(),
                        self.config.vanity.min_prefix_length.to_string(),
                    ),
                    (
                        "Warn Threshold".to_string(),
                        self.config.vanity.warn_threshold.to_string(),
                    ),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
        }
    }

    /// Apply input value to config
    fn apply_input_value(&mut self, field: &str, value: &str) {
        let old_value = self.get_field_value(field);

        match field {
            // Traffic Tier - cycles through tiers on Enter
            "Traffic Tier" => {
                use crate::config::TrafficTier;
                self.config.traffic_tier = match self.config.traffic_tier {
                    TrafficTier::Micro => TrafficTier::Small,
                    TrafficTier::Small => TrafficTier::Medium,
                    TrafficTier::Medium => TrafficTier::Large,
                    TrafficTier::Large => TrafficTier::Enterprise,
                    TrafficTier::Enterprise => TrafficTier::Micro,
                };
                // Auto-update related CAPTCHA pool settings
                let tier = self.config.traffic_tier;
                self.config.captcha.pool_size = tier.pool_size();
                self.config.captcha.min_pool_size = tier.min_pool_size();
                self.config.captcha.max_pool_size = tier.max_pool_size();
            }
            "Service Name" => self.config.branding.service_name = value.to_string(),
            "Description" => self.config.branding.description = value.to_string(),
            "Welcome Message" => self.config.branding.welcome_message = value.to_string(),
            "Primary Color" => self.config.branding.primary_color = value.to_string(),
            "Secondary Color" => self.config.branding.secondary_color = value.to_string(),
            "Enabled" => self.config.captcha.enabled = parse_yes_no(value, true),
            // CAPTCHA Type settings - cycle on Enter
            "Gate CAPTCHA Type" => {
                self.config.captcha.gate_captcha_type = self.config.captcha.gate_captcha_type.next();
            }
            "Threat Type Enabled" => {
                self.config.captcha.threat_captcha_enabled = !self.config.captcha.threat_captcha_enabled;
            }
            "Threat CAPTCHA Type" => {
                self.config.captcha.threat_captcha_type = self.config.captcha.threat_captcha_type.next();
            }
            "Random Cycling" => {
                self.config.captcha.random_cycling = !self.config.captcha.random_cycling;
            }
            "Cycling Types" => {
                // TODO: Implement multi-select UI for cycling types
                // For now, reset to defaults
                self.config.captcha.cycling_types = vec![
                    CaptchaType::BmpText,
                    CaptchaType::Emoji,
                    CaptchaType::Direction,
                ];
            }
            "Pool Size" => self.config.captcha.pool_size = value.parse().unwrap_or(500),
            "Min Pool" => self.config.captcha.min_pool_size = value.parse().unwrap_or(100),
            "Max Pool" => self.config.captcha.max_pool_size = value.parse().unwrap_or(1000),
            "Difficulty" => self.config.captcha.difficulty = value.parse().unwrap_or(5),
            "Difficulty (1-10)" => self.config.captcha.difficulty = value.parse().unwrap_or(5),
            "Timeout (sec)" => self.config.captcha.timeout_seconds = value.parse().unwrap_or(120),
            "Timeout (seconds)" => {
                self.config.captcha.timeout_seconds = value.parse().unwrap_or(120)
            }
            "Max Attempts" => self.config.captcha.max_attempts = value.parse().unwrap_or(3),
            "Rotation Days" => {
                self.config.captcha.rotation_interval_days = value.parse().unwrap_or(10)
            }
            "Rate Limit (RPM)" | "Rate Limit (req/min)" => {
                self.config.thresholds.rate_limit_rpm = value.parse().unwrap_or(60)
            }
            "CAPTCHA Fail Limit" => {
                self.config.thresholds.captcha_fail_limit = value.parse().unwrap_or(5)
            }
            "Temp Ban (min)" | "Temp Ban Duration (min)" => {
                self.config.thresholds.temp_ban_minutes = value.parse().unwrap_or(30)
            }
            "Perm Ban Threshold" => {
                self.config.thresholds.perm_ban_threshold = value.parse().unwrap_or(3)
            }
            "Suspicion Threshold" => {
                self.config.thresholds.suspicion_threshold = value.parse().unwrap_or(5.0)
            }
            "Threat Threshold" => {
                self.config.thresholds.threat_threshold = value.parse().unwrap_or(10.0)
            }
            "Burn Threshold" => {
                self.config.thresholds.burn_threshold = value.parse().unwrap_or(0.7)
            }
            "Auto Ban Enabled" => {
                self.config.thresholds.auto_ban_enabled = parse_yes_no(value, true)
            }
            "DDoS RPS Threshold" => {
                self.config.thresholds.ddos_rps_threshold = value.parse().unwrap_or(100)
            }
            "Probe Sensitivity (1-10)" => {
                self.config.thresholds.probe_sensitivity = value.parse().unwrap_or(5)
            }
            "Backend Address" => self.config.network.backend_address = value.to_string(),
            "HTTP Bind" => self.config.network.http_bind = value.to_string(),
            "Gate Bind" => self.config.network.gate_bind = value.to_string(),
            "SOCKS Port" => self.config.network.socks_port = value.parse().unwrap_or(9150),
            "Control Port" => self.config.network.control_port = value.parse().unwrap_or(9151),
            "Min Mirrors" => self.config.mirrors.min_mirrors = value.parse().unwrap_or(2),
            "Max Mirrors" => self.config.mirrors.max_mirrors = value.parse().unwrap_or(5),
            "Standby Mirrors" => self.config.mirrors.standby_mirrors = value.parse().unwrap_or(2),
            "Rotation (sec)" => {
                self.config.mirrors.rotation_interval_seconds = value.parse().unwrap_or(3600)
            }
            "Burn Min Days" => {
                self.config.mirrors.burn_interval_days_min = value.parse().unwrap_or(60)
            }
            // Vanity settings - order MUST match get_current_field() and draw_vanity()
            "Vanity Enabled" => self.config.vanity.enabled = parse_yes_no(value, false),
            "Prefix" => {
                // Limit prefix to 10 characters and lowercase alphanumeric only
                let cleaned: String = value
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(10)
                    .collect::<String>()
                    .to_lowercase();
                self.config.vanity.prefix = cleaned;
            }
            "Prefix Length" => {} // Display only - computed from prefix, not editable
            "Safety Net Enabled" => {
                self.config.vanity.safety_net_enabled = parse_yes_no(value, true)
            }
            "Vanity Timeout (sec)" => {
                self.config.vanity.safety_net_timeout_seconds = value.parse().unwrap_or(30)
            }
            "Min Prefix Length" => {
                self.config.vanity.min_prefix_length = value.parse().unwrap_or(1)
            }
            "Warn Threshold" => self.config.vanity.warn_threshold = value.parse().unwrap_or(7),
            _ => return,
        }

        // Record the change
        self.config.mark_dirty();
        self.changes.record_change(field, &old_value, value);
    }

    /// Get current value for a field
    fn get_field_value(&self, field: &str) -> String {
        match field {
            "Service Name" => self.config.branding.service_name.clone(),
            "Description" => self.config.branding.description.clone(),
            "Welcome Message" => self.config.branding.welcome_message.clone(),
            "Primary Color" => self.config.branding.primary_color.clone(),
            "Enabled" => {
                if self.config.captcha.enabled {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            "Pool Size" => self.config.captcha.pool_size.to_string(),
            "Min Pool" => self.config.captcha.min_pool_size.to_string(),
            "Max Pool" => self.config.captcha.max_pool_size.to_string(),
            "Difficulty" | "Difficulty (1-10)" => self.config.captcha.difficulty.to_string(),
            "Timeout (seconds)" => self.config.captcha.timeout_seconds.to_string(),
            "Max Attempts" => self.config.captcha.max_attempts.to_string(),
            "Rotation %" => self.config.captcha.rotation_percent.to_string(),
            "Rotation Days" => self.config.captcha.rotation_interval_days.to_string(),
            _ => String::new(),
        }
    }

    /// Apply changes immediately
    async fn apply_changes_now(&mut self) -> Result<()> {
        // Save config
        if let Err(e) = self.config.save() {
            self.log_tx
                .send(LogEntry::error(&format!("Failed to save config: {}", e)))
                .await
                .ok();
            return Err(e);
        }

        // Notify deployment to reload
        if self.deployment.is_running() {
            self.deployment.reload_config(&self.config).await?;
            self.log_tx
                .send(LogEntry::info("Configuration reloaded"))
                .await
                .ok();
        }

        self.changes.apply_all();
        self.config.dirty = false;
        self.status_message = Some(("Changes applied".into(), std::time::Instant::now()));

        Ok(())
    }

    /// Start deployment - shows dependency check dialog first
    async fn start_deployment(&mut self) -> Result<()> {
        // Save config first
        if let Err(e) = self.config.save() {
            self.dialog = Dialog::Error {
                message: format!("Failed to save configuration: {}", e),
            };
            return Ok(());
        }

        self.log_tx
            .send(LogEntry::info("Checking system dependencies..."))
            .await
            .ok();

        // Initialize dependency check dialog with all deps in Pending state
        // Dynamically mark dependencies as required based on config settings
        let vanity_enabled = self.config.vanity.enabled;
        let vanguards_enabled = self.config.network.vanguards_enabled;
        let deps = crate::deployment::get_dependencies();
        let statuses: Vec<DependencyStatus> = deps
            .iter()
            .map(|d| {
                // Determine if this dependency should be required based on config
                let is_required = match d.name {
                    // Vanity address dependencies (git, libsodium, autoconf needed to build mkp224o)
                    "mkp224o" | "autoconf" | "libsodium" | "git" if vanity_enabled => true,
                    // Vanguards dependency
                    "vanguards" if vanguards_enabled => true,
                    // Use the default required flag
                    _ => d.required,
                };
                DependencyStatus {
                    name: d.name.to_string(),
                    description: d.description.to_string(),
                    required: is_required,
                    state: DependencyState::Pending,
                }
            })
            .collect();

        self.dialog = Dialog::DependencyCheck {
            statuses,
            phase: DependencyCheckPhase::Checking,
            completed_at: None,
        };

        // Spawn the dependency check process
        self.run_dependency_check().await?;

        Ok(())
    }

    /// Run the dependency check and installation process
    async fn run_dependency_check(&mut self) -> Result<()> {
        let deps = crate::deployment::get_dependencies();
        let mut needs_install: Vec<usize> = Vec::new();

        // Get the required flags from statuses (which may have vanity overrides)
        let required_flags: Vec<bool> =
            if let Dialog::DependencyCheck { statuses, .. } = &self.dialog {
                statuses.iter().map(|s| s.required).collect()
            } else {
                deps.iter().map(|d| d.required).collect()
            };

        // Phase 1: Check all dependencies
        for (i, dep) in deps.iter().enumerate() {
            let is_required = required_flags.get(i).copied().unwrap_or(dep.required);

            // Update status to Checking
            if let Dialog::DependencyCheck { statuses, .. } = &mut self.dialog {
                if i < statuses.len() {
                    statuses[i].state = DependencyState::Checking;
                }
            }

            // Small delay so user can see the checking animation
            tokio::time::sleep(Duration::from_millis(100)).await;

            let available = dep.is_available();

            if let Dialog::DependencyCheck { statuses, .. } = &mut self.dialog {
                if i < statuses.len() {
                    if available {
                        statuses[i].state = DependencyState::Ok;
                        self.log_tx
                            .send(LogEntry::from_source(
                                crate::logging::LogLevel::Info,
                                "deps",
                                &format!("✓ {} is available", dep.name),
                            ))
                            .await
                            .ok();
                    } else if is_required {
                        statuses[i].state = DependencyState::Pending;
                        needs_install.push(i);
                        self.log_tx
                            .send(LogEntry::from_source(
                                crate::logging::LogLevel::Warn,
                                "deps",
                                &format!("✗ {} is missing (required)", dep.name),
                            ))
                            .await
                            .ok();
                    } else {
                        statuses[i].state = DependencyState::Skipped;
                        self.log_tx
                            .send(LogEntry::from_source(
                                crate::logging::LogLevel::Info,
                                "deps",
                                &format!("○ {} is missing (optional, skipping)", dep.name),
                            ))
                            .await
                            .ok();
                    }
                }
            }
        }

        // Phase 2: Install missing required dependencies
        if !needs_install.is_empty() {
            if let Dialog::DependencyCheck { phase, .. } = &mut self.dialog {
                *phase = DependencyCheckPhase::Installing;
            }

            self.log_tx
                .send(LogEntry::info("Installing missing dependencies..."))
                .await
                .ok();

            for &i in &needs_install {
                let dep = &deps[i];

                // Update to Installing state
                if let Dialog::DependencyCheck { statuses, .. } = &mut self.dialog {
                    if i < statuses.len() {
                        statuses[i].state = DependencyState::Installing;
                    }
                }

                // Attempt installation
                let success = self
                    .deployment
                    .install_dependency(dep)
                    .await
                    .unwrap_or(false);

                if let Dialog::DependencyCheck { statuses, .. } = &mut self.dialog {
                    if i < statuses.len() {
                        if success && dep.is_available() {
                            statuses[i].state = DependencyState::Ok;
                            self.log_tx
                                .send(LogEntry::from_source(
                                    crate::logging::LogLevel::Info,
                                    "deps",
                                    &format!("✓ {} installed successfully", dep.name),
                                ))
                                .await
                                .ok();
                        } else {
                            statuses[i].state =
                                DependencyState::Failed("Installation failed".to_string());
                            self.log_tx
                                .send(LogEntry::from_source(
                                    crate::logging::LogLevel::Error,
                                    "deps",
                                    &format!("✗ Failed to install {}", dep.name),
                                ))
                                .await
                                .ok();
                        }
                    }
                }
            }
        }

        // Phase 3: Complete or Failed
        if let Dialog::DependencyCheck {
            phase,
            completed_at,
            statuses,
        } = &mut self.dialog
        {
            // Check if any required deps failed
            let any_required_failed = statuses
                .iter()
                .any(|s| s.required && matches!(s.state, DependencyState::Failed(_)));

            if any_required_failed {
                *phase = DependencyCheckPhase::Failed;
                self.log_tx
                    .send(LogEntry::error("Dependency check failed - cannot proceed"))
                    .await
                    .ok();
            } else {
                *phase = DependencyCheckPhase::Complete;
                *completed_at = Some(std::time::Instant::now());
                self.log_tx
                    .send(LogEntry::info("All dependencies ready!"))
                    .await
                    .ok();
            }
        }

        Ok(())
    }

    /// Actually start the deployment (called after dependency check passes)
    async fn do_actual_deployment(&mut self) -> Result<()> {
        self.log_tx
            .send(LogEntry::info("Starting Fortify services..."))
            .await
            .ok();

        match self.deployment.start(&self.config).await {
            Ok(()) => {
                self.view = View::Running;
                self.focus = Focus::Menu;
                self.log_tx
                    .send(LogEntry::info("Deployment started successfully"))
                    .await
                    .ok();
            }
            Err(e) => {
                self.dialog = Dialog::Error {
                    message: format!("Failed to start deployment: {}", e),
                };
                self.log_tx
                    .send(LogEntry::error(&format!("Deployment failed: {}", e)))
                    .await
                    .ok();
            }
        }

        Ok(())
    }

    /// Set status message
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    /// Validate backend address and return warning if trailing path detected
    fn validate_backend_address(&self, addr: &str) -> Option<String> {
        // Check for trailing path after .onion domain
        if addr.contains(".onion") {
            if let Some(onion_pos) = addr.find(".onion") {
                let after_onion = &addr[onion_pos + 6..];
                // Check if there's a path after .onion
                if !after_onion.is_empty() && after_onion != "/" {
                    let path = after_onion.trim_start_matches('/');
                    if !path.is_empty() {
                        return Some(format!(
                            "⚠ Trailing path detected: '{}' - Is this intentional?",
                            after_onion
                        ));
                    }
                }
            }
        }

        // Check for trailing path on other URLs
        if let Some(scheme_end) = addr.find("://") {
            let after_scheme = &addr[scheme_end + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                let path = &after_scheme[slash_pos..];
                if path != "/" && !path.is_empty() {
                    return Some(format!(
                        "⚠ Backend URL includes path: '{}' - Verify this is correct.",
                        path
                    ));
                }
            }
        }

        None
    }
}
