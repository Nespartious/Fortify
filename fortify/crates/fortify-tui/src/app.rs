//! Main application state and loop

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::{ChangeManager, FortifyConfig};
use crate::deployment::DeploymentManager;
use crate::logging::{LogEntry, LogLevel, LogBuffer};
use crate::ui;

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
            MirrorStatusState::Pending | MirrorStatusState::Verifying | MirrorStatusState::Generating => "◐",
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

/// Backend health state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendHealthState {
    /// Not yet checked
    Unknown,
    /// Currently checking
    Checking,
    /// Backend is reachable
    Reachable,
    /// Backend is unreachable
    Unreachable,
}

impl BackendHealthState {
    pub fn color(&self) -> Color {
        match self {
            BackendHealthState::Unknown => Color::DarkGray,
            BackendHealthState::Checking => Color::Yellow,
            BackendHealthState::Reachable => Color::Green,
            BackendHealthState::Unreachable => Color::Red,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            BackendHealthState::Unknown => "⚫",
            BackendHealthState::Checking => "🟡",
            BackendHealthState::Reachable => "🟢",
            BackendHealthState::Unreachable => "🔴",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BackendHealthState::Unknown => "UNKNOWN",
            BackendHealthState::Checking => "CHECKING",
            BackendHealthState::Reachable => "REACHABLE",
            BackendHealthState::Unreachable => "UNREACHABLE",
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
    Settings { tab: SettingsTab, field_index: usize },
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
}

impl App {
    /// Create new application
    pub async fn new() -> Result<Self> {
        let (log_tx, log_rx) = mpsc::channel(1000);
        
        // Ensure config directory exists
        std::fs::create_dir_all("/tmp/fortify/config")?;
        
        // Create log directory and open log file for persistent logging
        std::fs::create_dir_all("/tmp/fortify/logs")?;
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/fortify/logs/deployment.log")
            .ok();
        
        // Try to load existing config or use default
        let config_path = FortifyConfig::default_path();
        let config = if config_path.exists() {
            match FortifyConfig::load(&config_path) {
                Ok(cfg) => cfg,
                Err(_) => {
                    // Load failed, use default but keep the path
                    let mut cfg = FortifyConfig::default();
                    cfg.config_path = Some(config_path);
                    cfg
                }
            }
        } else {
            let mut cfg = FortifyConfig::default();
            cfg.config_path = Some(config_path);
            cfg
        };

        // Load existing deployments
        let existing_deployments = FortifyConfig::list_deployments().unwrap_or_default();

        // Check if there's an existing session
        let deployment = DeploymentManager::new(log_tx.clone());

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
            backend_health: BackendHealthState::Unknown,
            backend_last_check: None,
            backend_check_history: Vec::new(),
            backend_check_interval: 15, // Start with 15 second checks
            mirror_health_checks: std::collections::HashMap::new(),
        })
    }

    /// Main run loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Add initial log
        self.log_tx.send(LogEntry::info("Fortify TUI started")).await.ok();

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
                
                if !self.logs_paused {
                    self.logs.push(entry);
                }
            }
            
            // Poll for mirror status when running (every 2 seconds)
            if self.deployment.is_running() {
                let should_poll = self.last_mirror_poll
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

            // Check if should quit
            if self.should_quit {
                break;
            }
        }

        Ok(())
    }
    
    /// Parse backend health check logs and update state
    fn parse_backend_health_log(&mut self, entry: &LogEntry) {
        let msg = &entry.message;
        
        // Only parse logs from controller
        if !entry.source.contains("controller") {
            return;
        }
        
        // Parse "Backend is now REACHABLE (took 234ms) - scaling down check frequency"
        if msg.contains("Backend is now REACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_health = BackendHealthState::Reachable;
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: true,
                    duration: Duration::from_millis(duration_ms),
                    error: None,
                });
                self.trim_health_history();
            }
        }
        
        // Parse "Backend check: REACHABLE (123ms)"
        else if msg.contains("Backend check: REACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_health = BackendHealthState::Reachable;
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: true,
                    duration: Duration::from_millis(duration_ms),
                    error: None,
                });
                self.trim_health_history();
            }
        }
        
        // Parse "Backend became UNREACHABLE (456ms) - increasing check frequency"
        else if msg.contains("Backend became UNREACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_health = BackendHealthState::Unreachable;
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: false,
                    duration: Duration::from_millis(duration_ms),
                    error: Some("Connection timeout or circuit not ready".to_string()),
                });
                self.trim_health_history();
            }
        }
        
        // Parse "Backend check: UNREACHABLE (789ms) - circuits may still be building..."
        else if msg.contains("Backend check: UNREACHABLE") {
            if let Some(duration_ms) = Self::extract_duration_ms(msg) {
                self.backend_health = BackendHealthState::Unreachable;
                self.backend_last_check = Some(std::time::Instant::now());
                self.backend_check_history.push(BackendHealthCheck {
                    timestamp: std::time::Instant::now(),
                    success: false,
                    duration: Duration::from_millis(duration_ms),
                    error: Some("Circuits still building".to_string()),
                });
                self.trim_health_history();
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
            self.backend_check_history.drain(0..self.backend_check_history.len() - 20);
        }
    }
    
    /// Parse mirror health check logs
    fn parse_mirror_health_log(&mut self, entry: &LogEntry) {
        let msg = &entry.message;
        
        // Only parse logs from controller
        if !entry.source.contains("controller") {
            return;
        }
        
        // Parse "Mirror http://...onion is now REACHABLE (234ms, status: 302)"
        if msg.contains("Mirror ") && msg.contains(" is now REACHABLE") {
            if let Some((mirror_addr, duration_ms)) = Self::extract_mirror_and_duration(msg) {
                let checks = self.mirror_health_checks.entry(mirror_addr).or_insert_with(Vec::new);
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
        
        // Parse "Mirror http://...onion check: REACHABLE (156ms, status: 302)" or "check: REACHABLE (configured)"
        else if msg.contains("Mirror ") && msg.contains(" check: REACHABLE") {
            if let Some((mirror_addr, duration_ms)) = Self::extract_mirror_and_duration(msg) {
                let checks = self.mirror_health_checks.entry(mirror_addr).or_insert_with(Vec::new);
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
        
        // Parse "Mirror http://...onion status: AVAILABLE" (alternate format)
        else if msg.contains("Mirror ") && (msg.contains(" status: AVAILABLE") || msg.contains(" is now AVAILABLE")) {
            if let Some((mirror_addr, duration_ms)) = Self::extract_mirror_and_duration(msg) {
                let checks = self.mirror_health_checks.entry(mirror_addr).or_insert_with(Vec::new);
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
                let checks = self.mirror_health_checks.entry(mirror_addr).or_insert_with(Vec::new);
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
    
    /// Update mirror status from orchestrator
    async fn update_mirror_status(&mut self) {
        // Try to fetch from orchestrator at 127.0.0.1:8080
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build() 
        {
            Ok(c) => c,
            Err(_) => return,
        };
        
        let response = match client.get("http://127.0.0.1:8080/mirrors/extended").send().await {
            Ok(r) => r,
            Err(_) => return, // Orchestrator not ready yet
        };
        
        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(_) => return,
        };
        
        if let Some(mirrors) = json.get("mirrors").and_then(|m| m.as_array()) {
            self.mirror_statuses = mirrors.iter().enumerate().filter_map(|(idx, m)| {
                let onion = m.get("onion_address")?.as_str()?.to_string();
                let status = m.get("status")?.as_str()?;
                let is_standby = m.get("is_standby").and_then(|v| v.as_bool()).unwrap_or(false);
                
                let state = match status {
                    "active" => MirrorStatusState::Live,
                    "paused" => MirrorStatusState::Pending,
                    "burned" | "destroyed" => MirrorStatusState::Failed,
                    "dormant" => MirrorStatusState::Pending,
                    _ => MirrorStatusState::Verifying,
                };
                
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
            }).collect();
        }
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
                    let tor_data_dir = std::path::PathBuf::from(&config.network.data_dir).join("tor");
                    
                    // Check if we have existing Tor data to reuse
                    let can_quick_deploy = tor_data_dir.exists() && 
                                          tor_data_dir.join("data").exists() &&
                                          tor_data_dir.join("torrc").exists();
                    
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
                        self.log_tx.send(LogEntry::warn("Config exists but Tor data is missing. Full deployment required.")).await.ok();
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
                    tab: SettingsTab::Branding,
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
                        if matches!(on_confirm, DialogAction::QuickDeploy) && 
                           matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N')) {
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
                        self.apply_input_value(&field, &value);
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
                self.log_tx.send(LogEntry::info("Quick Deploy: Using existing configuration")).await.ok();
                self.start_deployment().await?;
            }
            DialogAction::Stop => {
                self.deployment.stop().await?;
                self.view = View::Home;
                self.log_tx.send(LogEntry::info("Deployment stopped")).await.ok();
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
        
        self.log_tx.send(LogEntry::from_source(
            LogLevel::Warn,
            "destroy",
            "Beginning instance destruction..."
        )).await.ok();
        
        // Remove all Fortify data directories
        let paths_to_remove = [
            "/tmp/fortify",
            "/var/lib/fortify",
        ];
        
        for path in &paths_to_remove {
            let p = std::path::Path::new(path);
            if p.exists() {
                self.log_tx.send(LogEntry::from_source(
                    LogLevel::Info,
                    "destroy",
                    &format!("Removing: {}", path)
                )).await.ok();
                
                if let Err(e) = std::fs::remove_dir_all(p) {
                    self.log_tx.send(LogEntry::from_source(
                        LogLevel::Warn,
                        "destroy",
                        &format!("Failed to remove {}: {}", path, e)
                    )).await.ok();
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
        
        self.log_tx.send(LogEntry::from_source(
            LogLevel::Info,
            "destroy",
            "Instance destroyed. All data has been removed."
        )).await.ok();
        
        self.dialog = Dialog::Info {
            title: "Instance Destroyed".into(),
            message: "All Fortify data has been removed.\n\nThe system is now in a clean state.".into(),
        };
        self.focus = Focus::Dialog;
        
        Ok(())
    }
    
    /// Install missing dependencies
    async fn install_missing_deps(&mut self) -> Result<()> {
        self.log_tx.send(LogEntry::from_source(
            LogLevel::Info,
            "install",
            "Starting dependency installation..."
        )).await.ok();
        
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
                            changes: self.changes.pending_changes.iter()
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
                    tab: SettingsTab::Branding,
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

        let export_path = std::path::PathBuf::from("/tmp/fortify/mirror-addresses.txt");
        
        // Separate live and standby mirrors
        let live_mirrors: Vec<_> = self.mirror_statuses.iter()
            .filter(|m| !m.is_standby)
            .collect();
        let standby_mirrors: Vec<_> = self.mirror_statuses.iter()
            .filter(|m| m.is_standby)
            .collect();
        
        // Build the content
        let mut content = String::new();
        content.push_str("# Fortify Mirror Addresses\n");
        content.push_str(&format!("# Exported: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
        content.push_str(&format!("# Service: {}\n", self.config.branding.service_name));
        content.push_str(&format!("# Backend: {}\n\n", self.config.network.backend_address));
        
        // Control panel link (use first live mirror)
        let admin_path = "/ctrl_8f7k3m9x2n4p1q6w5v0b8c";
        if let Some(first_live) = live_mirrors.first() {
            content.push_str(&format!("## CONTROL PANEL:\nhttp://{}.onion{}\n\n", first_live.address, admin_path));
        }
        
        // Live mirrors section
        content.push_str(&format!("## LIVE MIRRORS ({}):\n", live_mirrors.len()));
        for mirror in &live_mirrors {
            let status = mirror.state.label();
            content.push_str(&format!(
                "http://{}.onion  # {}\n",
                mirror.address,
                status
            ));
        }
        
        // Standby mirrors section
        if !standby_mirrors.is_empty() {
            content.push_str(&format!("\n## STANDBY MIRRORS ({}):\n", standby_mirrors.len()));
            for mirror in &standby_mirrors {
                let status = mirror.state.label();
                content.push_str(&format!(
                    "http://{}.onion  # {} [STANDBY]\n",
                    mirror.address,
                    status
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
        let logs = self.logs.scroll(self.log_scroll, visible_height, self.log_filter);
        
        if let Some(entry) = logs.get(self.log_selected_line) {
            let text = format!(
                "{} {} [{}] {}",
                entry.timestamp.format("%H:%M:%S"),
                entry.level.symbol(),
                entry.source,
                entry.message
            );
            
            // Try to copy to clipboard using various methods
            if let Err(_) = Self::copy_to_clipboard(&text) {
                // Fallback: save to a temp file that user can access
                let path = "/tmp/fortify/copied_log.txt";
                let _ = std::fs::write(path, &text);
                self.status_message = Some((
                    format!("Saved to {}", path),
                    std::time::Instant::now()
                ));
            } else {
                self.status_message = Some((
                    "Copied to clipboard!".to_string(),
                    std::time::Instant::now()
                ));
            }
            
            // Exit selection mode after copy
            self.log_select_mode = false;
        }
    }
    
    /// Copy text to system clipboard
    fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
        use std::process::{Command, Stdio};
        use std::io::Write;
        
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
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            return Ok(());
        }
        
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No clipboard tool found"))
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
            SettingsTab::Branding => {
                let fields: [(String, String); 5] = [
                    ("Service Name".to_string(), self.config.branding.service_name.clone()),
                    ("Description".to_string(), self.config.branding.description.clone()),
                    ("Welcome Message".to_string(), self.config.branding.welcome_message.clone()),
                    ("Primary Color".to_string(), self.config.branding.primary_color.clone()),
                    ("Logo Path".to_string(), self.config.branding.logo_path.as_ref()
                        .map(|p| p.display().to_string()).unwrap_or_default()),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Captcha => {
                let fields: [(String, String); 5] = [
                    ("Enabled".to_string(), self.config.captcha.enabled.to_string()),
                    ("Pool Size".to_string(), self.config.captcha.pool_size.to_string()),
                    ("Difficulty".to_string(), self.config.captcha.difficulty.to_string()),
                    ("Timeout (sec)".to_string(), self.config.captcha.timeout_seconds.to_string()),
                    ("Max Attempts".to_string(), self.config.captcha.max_attempts.to_string()),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Thresholds => {
                let fields: [(String, String); 5] = [
                    ("Rate Limit (RPM)".to_string(), self.config.thresholds.rate_limit_rpm.to_string()),
                    ("CAPTCHA Fail Limit".to_string(), self.config.thresholds.captcha_fail_limit.to_string()),
                    ("Temp Ban (min)".to_string(), self.config.thresholds.temp_ban_minutes.to_string()),
                    ("Burn Threshold".to_string(), self.config.thresholds.burn_threshold.to_string()),
                    ("DDoS RPS Threshold".to_string(), self.config.thresholds.ddos_rps_threshold.to_string()),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Network => {
                let fields: [(String, String); 5] = [
                    ("Backend Address".to_string(), self.config.network.backend_address.clone()),
                    ("HTTP Bind".to_string(), self.config.network.http_bind.clone()),
                    ("Gate Bind".to_string(), self.config.network.gate_bind.clone()),
                    ("SOCKS Port".to_string(), self.config.network.socks_port.to_string()),
                    ("Control Port".to_string(), self.config.network.control_port.to_string()),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Mirrors => {
                let fields: [(String, String); 5] = [
                    ("Min Mirrors".to_string(), self.config.mirrors.min_mirrors.to_string()),
                    ("Max Mirrors".to_string(), self.config.mirrors.max_mirrors.to_string()),
                    ("Standby Mirrors".to_string(), self.config.mirrors.standby_mirrors.to_string()),
                    ("Rotation (sec)".to_string(), self.config.mirrors.rotation_interval_seconds.to_string()),
                    ("Burn Min Days".to_string(), self.config.mirrors.burn_interval_days_min.to_string()),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
            SettingsTab::Vanity => {
                // MUST match the display order in ui/settings.rs draw_vanity()
                let fields: [(String, String); 7] = [
                    ("Vanity Enabled".to_string(), self.config.vanity.enabled.to_string()),
                    ("Prefix".to_string(), self.config.vanity.prefix.clone()),
                    ("Prefix Length".to_string(), format!("{}/10", self.config.vanity.prefix.len())), // Display only
                    ("Safety Net Enabled".to_string(), self.config.vanity.safety_net_enabled.to_string()),
                    ("Vanity Timeout (sec)".to_string(), self.config.vanity.safety_net_timeout_seconds.to_string()),
                    ("Min Prefix Length".to_string(), self.config.vanity.min_prefix_length.to_string()),
                    ("Warn Threshold".to_string(), self.config.vanity.warn_threshold.to_string()),
                ];
                fields.get(index).cloned().unwrap_or(unknown)
            }
        }
    }

    /// Apply input value to config
    fn apply_input_value(&mut self, field: &str, value: &str) {
        let old_value = self.get_field_value(field);
        
        match field {
            "Service Name" => self.config.branding.service_name = value.to_string(),
            "Description" => self.config.branding.description = value.to_string(),
            "Welcome Message" => self.config.branding.welcome_message = value.to_string(),
            "Primary Color" => self.config.branding.primary_color = value.to_string(),
            "Logo Path" => self.config.branding.logo_path = if value.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(value))
            },
            "Enabled" => self.config.captcha.enabled = value.parse().unwrap_or(true),
            "Pool Size" => self.config.captcha.pool_size = value.parse().unwrap_or(500),
            "Difficulty" => self.config.captcha.difficulty = value.parse().unwrap_or(5),
            "Timeout (sec)" => self.config.captcha.timeout_seconds = value.parse().unwrap_or(120),
            "Max Attempts" => self.config.captcha.max_attempts = value.parse().unwrap_or(3),
            "Rate Limit (RPM)" => self.config.thresholds.rate_limit_rpm = value.parse().unwrap_or(60),
            "CAPTCHA Fail Limit" => self.config.thresholds.captcha_fail_limit = value.parse().unwrap_or(5),
            "Temp Ban (min)" => self.config.thresholds.temp_ban_minutes = value.parse().unwrap_or(30),
            "Burn Threshold" => self.config.thresholds.burn_threshold = value.parse().unwrap_or(0.7),
            "DDoS RPS Threshold" => self.config.thresholds.ddos_rps_threshold = value.parse().unwrap_or(100),
            "Backend Address" => self.config.network.backend_address = value.to_string(),
            "HTTP Bind" => self.config.network.http_bind = value.to_string(),
            "Gate Bind" => self.config.network.gate_bind = value.to_string(),
            "SOCKS Port" => self.config.network.socks_port = value.parse().unwrap_or(9150),
            "Control Port" => self.config.network.control_port = value.parse().unwrap_or(9151),
            "Min Mirrors" => self.config.mirrors.min_mirrors = value.parse().unwrap_or(2),
            "Max Mirrors" => self.config.mirrors.max_mirrors = value.parse().unwrap_or(5),
            "Standby Mirrors" => self.config.mirrors.standby_mirrors = value.parse().unwrap_or(2),
            "Rotation (sec)" => self.config.mirrors.rotation_interval_seconds = value.parse().unwrap_or(3600),
            "Burn Min Days" => self.config.mirrors.burn_interval_days_min = value.parse().unwrap_or(60),
            // Vanity settings - order MUST match get_current_field() and draw_vanity()
            "Vanity Enabled" => self.config.vanity.enabled = value.parse().unwrap_or(false),
            "Prefix" => {
                // Limit prefix to 10 characters and lowercase alphanumeric only
                let cleaned: String = value.chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(10)
                    .collect::<String>()
                    .to_lowercase();
                self.config.vanity.prefix = cleaned;
            },
            "Prefix Length" => {}, // Display only - computed from prefix, not editable
            "Safety Net Enabled" => self.config.vanity.safety_net_enabled = value.parse().unwrap_or(true),
            "Vanity Timeout (sec)" => self.config.vanity.safety_net_timeout_seconds = value.parse().unwrap_or(30),
            "Min Prefix Length" => self.config.vanity.min_prefix_length = value.parse().unwrap_or(1),
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
            "Enabled" => self.config.captcha.enabled.to_string(),
            "Pool Size" => self.config.captcha.pool_size.to_string(),
            _ => String::new(),
        }
    }

    /// Apply changes immediately
    async fn apply_changes_now(&mut self) -> Result<()> {
        // Save config
        if let Err(e) = self.config.save() {
            self.log_tx.send(LogEntry::error(&format!("Failed to save config: {}", e))).await.ok();
            return Err(e);
        }

        // Notify deployment to reload
        if self.deployment.is_running() {
            self.deployment.reload_config(&self.config).await?;
            self.log_tx.send(LogEntry::info("Configuration reloaded")).await.ok();
        }

        self.changes.apply_all();
        self.config.dirty = false;
        self.status_message = Some(("Changes applied".into(), std::time::Instant::now()));
        
        Ok(())
    }

    /// Start deployment
    async fn start_deployment(&mut self) -> Result<()> {
        // Save config first
        if let Err(e) = self.config.save() {
            self.dialog = Dialog::Error {
                message: format!("Failed to save configuration: {}", e),
            };
            return Ok(());
        }

        self.log_tx.send(LogEntry::info("Starting deployment...")).await.ok();
        
        match self.deployment.start(&self.config).await {
            Ok(()) => {
                self.view = View::Running;
                self.focus = Focus::Menu;
                self.log_tx.send(LogEntry::info("Deployment started successfully")).await.ok();
            }
            Err(e) => {
                self.dialog = Dialog::Error {
                    message: format!("Failed to start deployment: {}", e),
                };
                self.log_tx.send(LogEntry::error(&format!("Deployment failed: {}", e))).await.ok();
            }
        }
        
        Ok(())
    }

    /// Set status message
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }
}
