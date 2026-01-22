//! Vanguards addon management for Fortify
//!
//! Vanguards provides additional guard layers to protect against
//! guard discovery and deanonymization attacks.

use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use tracing::{error, info};

/// Vanguards process status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VanguardsStatus {
    NotConfigured,
    Starting,
    Running,
    Failed,
    Stopped,
}

/// Vanguards configuration
#[derive(Debug, Clone)]
pub struct VanguardsConfig {
    /// Path to vanguards config file
    pub config_path: String,
    /// Path to vanguards state file
    pub state_path: String,
    /// Path to vanguards log file
    pub log_path: String,
    /// Tor control port address
    pub tor_control_addr: String,
    /// Tor control port
    pub tor_control_port: u16,
    /// Whether vanguards is enabled
    pub enabled: bool,
    /// Layer 2 guard count
    pub layer2_guards: u8,
    /// Layer 3 guard count
    pub layer3_guards: u8,
    /// Maximum circuit age in hours
    pub circ_max_age_hours: u32,
    /// Maximum megabytes per circuit (0 = unlimited)
    pub circ_max_megabytes: u32,
}

impl Default for VanguardsConfig {
    fn default() -> Self {
        // Use persistent path if HOME is set
        let base_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path
        } else {
            std::path::PathBuf::from("/tmp/fortify")
        };

        Self {
            config_path: base_dir
                .join("config")
                .join("vanguards.conf")
                .to_string_lossy()
                .to_string(),
            state_path: base_dir
                .join("vanguards")
                .join("vanguards.state")
                .to_string_lossy()
                .to_string(),
            log_path: base_dir
                .join("log")
                .join("vanguards.log")
                .to_string_lossy()
                .to_string(),
            tor_control_addr: "127.0.0.1".to_string(),
            tor_control_port: 9151,
            enabled: true,
            layer2_guards: 4,
            layer3_guards: 8,
            circ_max_age_hours: 24,
            circ_max_megabytes: 0,
        }
    }
}

/// Vanguards addon manager
pub struct VanguardsManager {
    config: VanguardsConfig,
    process: Option<Child>,
    status: VanguardsStatus,
    started_at: Option<Instant>,
    restart_count: usize,
}

impl VanguardsManager {
    /// Create a new vanguards manager
    pub fn new(config: VanguardsConfig) -> Self {
        let initial_status = if config.enabled {
            VanguardsStatus::NotConfigured
        } else {
            VanguardsStatus::Stopped
        };
        Self {
            config,
            process: None,
            status: initial_status,
            started_at: None,
            restart_count: 0,
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(VanguardsConfig::default())
    }

    /// Check if vanguards is available (binary or Python module)
    pub fn is_available() -> bool {
        Self::find_vanguards_path().is_some()
    }

    /// Find vanguards binary or Python module path
    /// Returns (command, args) tuple for execution
    fn find_vanguards_path() -> Option<(String, Vec<String>)> {
        // First try common binary locations
        let paths = [
            "vanguards",
            "/usr/local/bin/vanguards",
            "/usr/bin/vanguards",
        ];

        for path in paths {
            if Command::new(path)
                .arg("--help")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
            {
                return Some((path.to_string(), vec![]));
            }
        }

        // Check home directory ~/.local/bin (pip --user install location)
        if let Ok(home) = std::env::var("HOME") {
            let local_path = format!("{}/.local/bin/vanguards", home);
            if Path::new(&local_path).exists()
                && Command::new(&local_path)
                    .arg("--help")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_ok()
            {
                return Some((local_path, vec![]));
            }
        }

        // Check Fortify venv (try both persistent and legacy locations)
        let base_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path
        } else {
            std::path::PathBuf::from("/tmp/fortify")
        };
        let venv_path = base_dir.join("venv").join("bin").join("vanguards");
        if venv_path.exists()
            && Command::new(&venv_path)
                .arg("--help")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
        {
            return Some((venv_path.to_string_lossy().to_string(), vec![]));
        }

        // Also check legacy /tmp location
        let legacy_venv = "/tmp/fortify/venv/bin/vanguards";
        if Path::new(legacy_venv).exists()
            && Command::new(legacy_venv)
                .arg("--help")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
        {
            return Some((legacy_venv.to_string(), vec![]));
        }

        // Try as Python module: python3 -m vanguards
        for python in ["python3", "python"] {
            if Command::new(python)
                .args(["-m", "vanguards", "--help"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
            {
                return Some((
                    python.to_string(),
                    vec!["-m".to_string(), "vanguards".to_string()],
                ));
            }
        }

        // Check /opt/vanguards (git clone location)
        let opt_path = "/opt/vanguards/src/vanguards.py";
        if Path::new(opt_path).exists() {
            return Some(("python3".to_string(), vec![opt_path.to_string()]));
        }

        None
    }

    /// Generate vanguards config file
    pub fn generate_config(&self) -> Result<(), String> {
        let config_content = format!(
            r#"# Vanguards Configuration for Fortify
# Auto-generated - do not edit manually

[Global]
control_ip = {control_addr}
control_port = {control_port}
state_file = {state_path}
loglevel = NOTICE
logfile = {log_path}

[Vanguards]
num_layer2_guards = {layer2}
min_layer2_lifetime_days = 1
max_layer2_lifetime_days = 30
num_layer3_guards = {layer3}
min_layer3_lifetime_hours = 1
max_layer3_lifetime_hours = 48

[Bandguards]
circ_max_age_hours = {max_age}
circ_max_megabytes = {max_mb}
circ_max_dropped_cells = 30

[Rendguard]
enabled = True
min_rend_relay_weight = 0.0
rend_use_max_scale = 1.0

[Cbtverify]
enabled = True
cbt_circuits_for_timeout = 1000
"#,
            control_addr = self.config.tor_control_addr,
            control_port = self.config.tor_control_port,
            state_path = self.config.state_path,
            log_path = self.config.log_path,
            layer2 = self.config.layer2_guards,
            layer3 = self.config.layer3_guards,
            max_age = self.config.circ_max_age_hours,
            max_mb = self.config.circ_max_megabytes,
        );

        // Create parent directories
        if let Some(parent) = Path::new(&self.config.config_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        if let Some(parent) = Path::new(&self.config.state_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create state directory: {}", e))?;
        }

        fs::write(&self.config.config_path, config_content)
            .map_err(|e| format!("Failed to write vanguards config: {}", e))?;

        info!("Generated vanguards config at {}", self.config.config_path);
        Ok(())
    }

    /// Start vanguards process
    pub fn start(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            info!("Vanguards is disabled, skipping start");
            return Ok(());
        }

        // Generate config file
        self.generate_config()?;

        // Find vanguards binary/module
        let (cmd_path, extra_args) = Self::find_vanguards_path().ok_or_else(|| {
            "Vanguards not found. Install with: pip3 install vanguards".to_string()
        })?;

        info!("Starting vanguards: {} {:?}", cmd_path, extra_args);
        self.status = VanguardsStatus::Starting;

        // Build command
        let mut cmd = Command::new(&cmd_path);

        // Add any module args (e.g., "-m vanguards" for Python module mode)
        for arg in &extra_args {
            cmd.arg(arg);
        }

        // Add config argument
        cmd.args(["--config", &self.config.config_path]);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        // Spawn process
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn vanguards: {}", e))?;

        self.process = Some(child);
        self.status = VanguardsStatus::Running;
        self.started_at = Some(Instant::now());

        info!("Vanguards started successfully");
        Ok(())
    }

    /// Stop vanguards process
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.process.take() {
            child
                .kill()
                .map_err(|e| format!("Failed to kill vanguards: {}", e))?;
            self.status = VanguardsStatus::Stopped;
            info!("Vanguards stopped");
        }
        Ok(())
    }

    /// Check if vanguards is still running
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.status = VanguardsStatus::Failed;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.status = VanguardsStatus::Failed;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Restart vanguards
    pub fn restart(&mut self) -> Result<(), String> {
        self.stop()?;
        self.restart_count += 1;
        self.start()
    }

    /// Get current status
    pub fn status(&self) -> VanguardsStatus {
        self.status.clone()
    }

    /// Get restart count
    pub fn restart_count(&self) -> usize {
        self.restart_count
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> Option<u64> {
        self.started_at.map(|t| t.elapsed().as_secs())
    }

    /// Check vanguards log for attack indicators
    pub fn check_for_attacks(&self) -> Vec<String> {
        let mut alerts = Vec::new();

        if !Path::new(&self.config.log_path).exists() {
            return alerts;
        }

        // Read last 100 lines of log
        if let Ok(content) = fs::read_to_string(&self.config.log_path) {
            let lines: Vec<&str> = content.lines().rev().take(100).collect();

            for line in lines {
                // Check for attack indicators
                if line.contains("WARN") || line.contains("ERROR") {
                    if line.contains("circuit") && line.contains("killed") {
                        alerts.push(format!("Circuit killed: {}", line));
                    }
                    if line.contains("RELAY_EARLY") {
                        alerts.push(format!("RELAY_EARLY attack detected: {}", line));
                    }
                    if line.contains("bandwidth") && line.contains("exceeded") {
                        alerts.push(format!("Bandwidth threshold exceeded: {}", line));
                    }
                    if line.contains("guard") && line.contains("suspicious") {
                        alerts.push(format!("Suspicious guard activity: {}", line));
                    }
                }
            }
        }

        alerts
    }
}

impl Drop for VanguardsManager {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            error!("Failed to stop vanguards on drop: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VanguardsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.layer2_guards, 4);
        assert_eq!(config.layer3_guards, 8);
    }

    #[test]
    fn test_manager_creation() {
        let manager = VanguardsManager::with_defaults();
        assert_eq!(manager.status(), VanguardsStatus::NotConfigured);
        assert_eq!(manager.restart_count(), 0);
    }

    #[test]
    fn test_config_generation() {
        let mut config = VanguardsConfig::default();
        config.config_path = "/tmp/test_vanguards.conf".to_string();
        config.state_path = "/tmp/test_vanguards.state".to_string();

        let manager = VanguardsManager::new(config);

        // This should succeed (creates the config file)
        let result = manager.generate_config();
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_file("/tmp/test_vanguards.conf");
    }
}
