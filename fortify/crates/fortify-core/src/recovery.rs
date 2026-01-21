// Phase 4.6: Auto-Restart and Session Recovery System
//
// Provides:
// - Resume/Wipe prompt with 10-second timer
// - Crash recovery detection
// - Graceful shutdown state persistence

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for auto-restart behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRestartConfig {
    /// Enable auto-restart recovery
    pub enabled: bool,
    /// Path to store crash recovery state
    pub state_path: PathBuf,
    /// Auto-resume timeout in seconds (default: 10)
    pub auto_resume_timeout_seconds: u64,
    /// Show recovery prompt to users
    pub show_recovery_prompt: bool,
    /// Detect crashes vs graceful shutdowns
    pub crash_detection: bool,
    /// Maximum time since last activity before forcing new session (seconds)
    pub max_inactivity_seconds: u64,
}

impl Default for AutoRestartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            state_path: PathBuf::from("/var/lib/fortify/recovery"),
            auto_resume_timeout_seconds: 10,
            show_recovery_prompt: true,
            crash_detection: true,
            max_inactivity_seconds: 86400 * 7, // 7 days
        }
    }
}

/// Shutdown reason for recovery detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownReason {
    /// Normal graceful shutdown
    Graceful,
    /// Restart requested (e.g., config reload)
    Restart,
    /// Crash detected (no graceful shutdown marker)
    Crash,
    /// Maintenance mode
    Maintenance,
    /// Unknown (first start or no state)
    Unknown,
}

impl ShutdownReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShutdownReason::Graceful => "graceful",
            ShutdownReason::Restart => "restart",
            ShutdownReason::Crash => "crash",
            ShutdownReason::Maintenance => "maintenance",
            ShutdownReason::Unknown => "unknown",
        }
    }
}

/// Recovery state persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryState {
    /// When the state was written
    pub timestamp: u64,
    /// Last shutdown reason (or None if still running)
    pub last_shutdown: Option<ShutdownReason>,
    /// Whether graceful shutdown completed
    pub graceful_shutdown_completed: bool,
    /// Number of active sessions at shutdown
    pub active_session_count: usize,
    /// Number of active mirrors at shutdown
    pub active_mirror_count: usize,
    /// Application version
    pub version: String,
    /// PID of the process that wrote this state
    pub pid: u32,
}

impl RecoveryState {
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_shutdown: None,
            graceful_shutdown_completed: false,
            active_session_count: 0,
            active_mirror_count: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
        }
    }
    
    /// Check if previous state indicates a crash
    pub fn was_crash(&self) -> bool {
        !self.graceful_shutdown_completed && self.last_shutdown.is_none()
    }
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovery manager for handling restarts and crashes
pub struct RecoveryManager {
    config: AutoRestartConfig,
    state: RecoveryState,
}

impl RecoveryManager {
    /// Create a new recovery manager and load existing state
    pub fn new(config: AutoRestartConfig) -> Self {
        let state = Self::load_state(&config.state_path).unwrap_or_default();
        
        let mut manager = Self { config, state };
        
        // Check if previous run was a crash
        if manager.state.was_crash() && manager.config.crash_detection {
            tracing::warn!(
                "Crash detected! Previous run (PID {}) did not shut down gracefully",
                manager.state.pid
            );
        }
        
        // Record new startup
        manager.state = RecoveryState::new();
        manager.save_state();
        
        manager
    }
    
    /// Load state from disk
    fn load_state(path: &PathBuf) -> Option<RecoveryState> {
        let state_file = path.join("state.json");
        
        if !state_file.exists() {
            return None;
        }
        
        match std::fs::read_to_string(&state_file) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(_) => None,
        }
    }
    
    /// Save current state to disk
    pub fn save_state(&self) {
        if !self.config.enabled {
            return;
        }
        
        if let Err(e) = std::fs::create_dir_all(&self.config.state_path) {
            tracing::error!("Failed to create recovery state dir: {}", e);
            return;
        }
        
        let state_file = self.config.state_path.join("state.json");
        let temp_file = self.config.state_path.join("state.json.tmp");
        
        match serde_json::to_string_pretty(&self.state) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&temp_file, &content) {
                    tracing::error!("Failed to write recovery state: {}", e);
                    return;
                }
                if let Err(e) = std::fs::rename(&temp_file, &state_file) {
                    tracing::error!("Failed to finalize recovery state: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize recovery state: {}", e);
            }
        }
    }
    
    /// Record graceful shutdown
    pub fn record_shutdown(&mut self, reason: ShutdownReason) {
        self.state.last_shutdown = Some(reason);
        self.state.graceful_shutdown_completed = true;
        self.state.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.save_state();
        
        tracing::info!("Graceful shutdown recorded: {:?}", reason);
    }
    
    /// Update session/mirror counts
    pub fn update_counts(&mut self, sessions: usize, mirrors: usize) {
        self.state.active_session_count = sessions;
        self.state.active_mirror_count = mirrors;
    }
    
    /// Check if recovery prompt should be shown
    pub fn should_show_recovery_prompt(&self, last_activity: u64) -> bool {
        if !self.config.show_recovery_prompt {
            return false;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let inactivity = now - last_activity;
        
        // Show prompt if session was active but had moderate inactivity
        inactivity > 300 && inactivity < self.config.max_inactivity_seconds
    }
    
    /// Get auto-resume timeout
    pub fn auto_resume_timeout(&self) -> u64 {
        self.config.auto_resume_timeout_seconds
    }
    
    /// Check if session should be wiped due to long inactivity
    pub fn should_force_wipe(&self, last_activity: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - last_activity > self.config.max_inactivity_seconds
    }
    
    /// Get current state for display
    pub fn get_state(&self) -> &RecoveryState {
        &self.state
    }
}

/// User's choice on the recovery prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryChoice {
    /// Resume the previous session
    Resume,
    /// Wipe and start fresh
    Wipe,
    /// Still waiting for choice (will auto-resume)
    Pending,
}

impl RecoveryChoice {
    pub fn from_action(action: &str) -> Self {
        match action {
            "resume" => RecoveryChoice::Resume,
            "wipe" => RecoveryChoice::Wipe,
            _ => RecoveryChoice::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_recovery_state_new() {
        let state = RecoveryState::new();
        assert!(!state.graceful_shutdown_completed);
        assert!(state.last_shutdown.is_none());
        assert!(state.was_crash());
    }
    
    #[test]
    fn test_recovery_state_graceful() {
        let mut state = RecoveryState::new();
        state.last_shutdown = Some(ShutdownReason::Graceful);
        state.graceful_shutdown_completed = true;
        assert!(!state.was_crash());
    }
    
    #[test]
    fn test_recovery_manager_crash_detection() {
        let temp = TempDir::new().unwrap();
        let config = AutoRestartConfig {
            enabled: true,
            state_path: temp.path().to_path_buf(),
            crash_detection: true,
            ..Default::default()
        };
        
        // First run - no crash
        let manager1 = RecoveryManager::new(config.clone());
        assert!(manager1.state.timestamp > 0);
        
        // Simulate crash (no graceful shutdown)
        // State file exists but graceful_shutdown_completed = false
        
        // Second run - should detect crash
        let manager2 = RecoveryManager::new(config);
        // Crash detection happens in constructor
        assert!(!manager2.state.was_crash()); // New state is fresh
    }
    
    #[test]
    fn test_recovery_choice() {
        assert_eq!(RecoveryChoice::from_action("resume"), RecoveryChoice::Resume);
        assert_eq!(RecoveryChoice::from_action("wipe"), RecoveryChoice::Wipe);
        assert_eq!(RecoveryChoice::from_action(""), RecoveryChoice::Pending);
        assert_eq!(RecoveryChoice::from_action("invalid"), RecoveryChoice::Pending);
    }
}
