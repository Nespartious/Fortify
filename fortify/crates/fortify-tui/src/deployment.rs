//! Deployment management - starting, stopping, monitoring

use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::config::FortifyConfig;
use crate::logging::{parse_log_line, LogEntry, LogLevel};
use crate::verification::{OnionVerifier, VerificationConfig};

/// Deployment state tracking (persisted to disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStateFile {
    /// Is deployment currently active
    pub active: bool,
    /// Last deployment start time
    pub last_started: Option<String>,
    /// Last deployment stop time
    pub last_stopped: Option<String>,
    /// Deployment ID
    pub deployment_id: String,
    /// Mirror addresses
    pub mirror_addresses: Vec<String>,
    /// Node addresses (healthy + threat)
    pub node_addresses: Vec<String>,
}

impl DeploymentStateFile {
    /// Load from disk
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save to disk
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default path (persistent location)
    pub fn default_path() -> std::path::PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".config");
            path.push("fortify");
            path.push("deployment-state.json");
            path
        } else {
            std::path::PathBuf::from("/tmp/fortify/config/deployment-state.json")
        }
    }
}

/// Required dependency information
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: &'static str,
    pub check_cmd: &'static str,
    pub check_args: &'static [&'static str],
    pub install_cmd: &'static str,
    pub install_args: &'static [&'static str],
    pub description: &'static str,
    pub required: bool,
    pub needs_sudo: bool,
}

impl Dependency {
    /// Check if the dependency is available
    pub fn is_available(&self) -> bool {
        std::process::Command::new(self.check_cmd)
            .args(self.check_args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// All dependencies required by Fortify
pub fn get_dependencies() -> Vec<Dependency> {
    vec![
        Dependency {
            name: "build-essential",
            check_cmd: "which",
            check_args: &["cc"],
            install_cmd: "apt-get",
            install_args: &["install", "-y", "build-essential"],
            description: "C compiler and build tools",
            required: true,
            needs_sudo: true,
        },
        Dependency {
            name: "git",
            check_cmd: "which",
            check_args: &["git"],
            install_cmd: "apt-get",
            install_args: &["install", "-y", "git"],
            description: "Git version control (for building mkp224o)",
            required: false, // Required when vanity is enabled
            needs_sudo: true,
        },
        Dependency {
            name: "tor",
            check_cmd: "which",
            check_args: &["tor"],
            install_cmd: "apt-get",
            install_args: &["install", "-y", "tor"],
            description: "Tor daemon for onion services",
            required: true,
            needs_sudo: true,
        },
        Dependency {
            name: "python3",
            check_cmd: "which",
            check_args: &["python3"],
            install_cmd: "apt-get",
            install_args: &["install", "-y", "python3", "python3-pip", "python3-venv"],
            description: "Python 3 runtime (for vanguards)",
            required: true,
            needs_sudo: true,
        },
        Dependency {
            name: "vanguards",
            check_cmd: "python3",
            check_args: &["-c", "import vanguards"],
            install_cmd: "pip3",
            install_args: &["install", "--break-system-packages", "vanguards"],
            description: "Tor vanguards for enhanced guard security",
            required: false, // Optional but recommended
            needs_sudo: false,
        },
        Dependency {
            name: "libsodium",
            check_cmd: "pkg-config",
            check_args: &["--exists", "libsodium"],
            install_cmd: "apt-get",
            install_args: &["install", "-y", "libsodium-dev"],
            description: "Cryptography library (for vanity addresses)",
            required: false,
            needs_sudo: true,
        },
        Dependency {
            name: "autoconf",
            check_cmd: "which",
            check_args: &["autoconf"],
            install_cmd: "apt-get",
            install_args: &["install", "-y", "autoconf", "automake"],
            description: "Build tools for mkp224o",
            required: false,
            needs_sudo: true,
        },
        Dependency {
            name: "mkp224o",
            check_cmd: "which",
            check_args: &["mkp224o"],
            install_cmd: "echo",
            install_args: &["mkp224o must be built from source"],
            description: "Vanity .onion address generator",
            required: false, // Optional - only needed for vanity addresses
            needs_sudo: false,
        },
    ]
}

/// Result of dependency check
#[derive(Debug, Clone)]
pub struct DependencyCheckResult {
    pub name: String,
    pub available: bool,
    pub required: bool,
    pub description: String,
    pub install_hint: String,
}

/// Check all dependencies and return results
pub fn check_dependencies() -> Vec<DependencyCheckResult> {
    get_dependencies()
        .iter()
        .map(|dep| {
            let install_hint = if dep.name == "mkp224o" {
                "Build from source (github.com/cathugger/mkp224o)".to_string()
            } else if dep.needs_sudo {
                format!("sudo {} {}", dep.install_cmd, dep.install_args.join(" "))
            } else {
                format!("{} {}", dep.install_cmd, dep.install_args.join(" "))
            };

            DependencyCheckResult {
                name: dep.name.to_string(),
                available: dep.is_available(),
                required: dep.required,
                description: dep.description.to_string(),
                install_hint,
            }
        })
        .collect()
}

/// Check if all required dependencies are available
#[allow(dead_code)]
pub fn all_required_available() -> bool {
    get_dependencies()
        .iter()
        .filter(|d| d.required)
        .all(|d| d.is_available())
}

/// Get list of missing dependencies
pub fn get_missing_dependencies() -> Vec<Dependency> {
    get_dependencies()
        .into_iter()
        .filter(|d| !d.is_available())
        .collect()
}

/// Deployment state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentState {
    /// Not deployed
    Stopped,
    /// Starting up
    Starting,
    /// Verifying onion addresses are reachable
    Verifying,
    /// Running normally
    Running,
    /// Stopping
    Stopping,
    /// Error state
    Error(String),
}

/// Manages the Fortify deployment lifecycle
pub struct DeploymentManager {
    /// Current state
    state: Arc<Mutex<DeploymentState>>,
    /// Child process handles
    children: Arc<Mutex<Vec<Child>>>,
    /// Log sender
    log_tx: mpsc::Sender<LogEntry>,
    /// Shutdown signal
    shutdown_tx: Option<broadcast::Sender<()>>,
    /// Current config
    config: Arc<Mutex<Option<FortifyConfig>>>,
}

impl DeploymentManager {
    pub fn new(log_tx: mpsc::Sender<LogEntry>) -> Self {
        Self {
            state: Arc::new(Mutex::new(DeploymentState::Stopped)),
            children: Arc::new(Mutex::new(Vec::new())),
            log_tx,
            shutdown_tx: None,
            config: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if deployment is running
    pub fn is_running(&self) -> bool {
        // Use try_lock since this is called synchronously
        if let Ok(state) = self.state.try_lock() {
            matches!(*state, DeploymentState::Running | DeploymentState::Starting)
        } else {
            false
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> DeploymentState {
        self.state.lock().await.clone()
    }

    /// Check dependencies and log results
    pub async fn check_and_log_dependencies(&self) -> Vec<DependencyCheckResult> {
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "deps",
                "Checking system dependencies...",
            ))
            .await
            .ok();

        let results = check_dependencies();

        for result in &results {
            let status = if result.available { "✓" } else { "✗" };
            let level = if result.available {
                LogLevel::Info
            } else if result.required {
                LogLevel::Error
            } else {
                LogLevel::Warn
            };

            self.log_tx
                .send(LogEntry::from_source(
                    level,
                    "deps",
                    &format!("[{}] {} - {}", status, result.name, result.description),
                ))
                .await
                .ok();
        }

        results
    }

    /// Install a specific dependency
    pub async fn install_dependency(&self, dep: &Dependency) -> Result<bool> {
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                &format!("Installing {}...", dep.name),
            ))
            .await
            .ok();

        // Special handling for mkp224o (needs to be built from source)
        if dep.name == "mkp224o" {
            return self.install_mkp224o().await;
        }

        // Special handling for vanguards (pip can be tricky on modern Ubuntu)
        if dep.name == "vanguards" {
            return self.install_vanguards().await;
        }

        // Build command with or without sudo
        let (cmd, args) = if dep.needs_sudo {
            ("sudo", {
                let mut v = vec![dep.install_cmd];
                v.extend(dep.install_args.iter().copied());
                v
            })
        } else {
            (dep.install_cmd, dep.install_args.to_vec())
        };

        let output = Command::new(cmd)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Info,
                    "install",
                    &format!("Successfully installed {}", dep.name),
                ))
                .await
                .ok();
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Error,
                    "install",
                    &format!("Failed to install {}: {}", dep.name, stderr.trim()),
                ))
                .await
                .ok();
            Ok(false)
        }
    }

    /// Install mkp224o from source
    async fn install_mkp224o(&self) -> Result<bool> {
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Building mkp224o from source...",
            ))
            .await
            .ok();

        let temp_dir = std::env::temp_dir().join("mkp224o-build");
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Clone repository
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Cloning mkp224o repository...",
            ))
            .await
            .ok();

        let clone = Command::new("git")
            .args(["clone", "https://github.com/cathugger/mkp224o.git"])
            .arg(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !clone.status.success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Error,
                    "install",
                    "Failed to clone mkp224o repository",
                ))
                .await
                .ok();
            return Ok(false);
        }

        // Run autogen.sh
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Running autogen.sh...",
            ))
            .await
            .ok();

        let autogen = Command::new("./autogen.sh")
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        if autogen.is_err() || !autogen.as_ref().unwrap().status.success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Error,
                    "install",
                    "autogen.sh failed",
                ))
                .await
                .ok();
            return Ok(false);
        }

        // Run configure
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Running configure...",
            ))
            .await
            .ok();

        let configure = Command::new("./configure")
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !configure.status.success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Error,
                    "install",
                    "configure failed - you may need libsodium-dev",
                ))
                .await
                .ok();
            return Ok(false);
        }

        // Run make
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Compiling mkp224o...",
            ))
            .await
            .ok();

        let make = Command::new("make")
            .args(["-j4"])
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !make.status.success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Error,
                    "install",
                    "make failed",
                ))
                .await
                .ok();
            return Ok(false);
        }

        // Copy to /usr/local/bin
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Installing mkp224o to /usr/local/bin (requires sudo)...",
            ))
            .await
            .ok();

        let install = Command::new("sudo")
            .args(["cp", "mkp224o", "/usr/local/bin/"])
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);

        if install.status.success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Info,
                    "install",
                    "mkp224o installed successfully",
                ))
                .await
                .ok();
            Ok(true)
        } else {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Error,
                    "install",
                    "Failed to copy mkp224o to /usr/local/bin",
                ))
                .await
                .ok();
            Ok(false)
        }
    }

    /// Install vanguards via pip (handles multiple pip configurations)
    async fn install_vanguards(&self) -> Result<bool> {
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "install",
                "Installing vanguards via pip...",
            ))
            .await
            .ok();

        // First, check if pip3 is available - if not, install it
        let pip_check = Command::new("which")
            .arg("pip3")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if pip_check.is_err() || !pip_check.unwrap().success() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Info,
                    "install",
                    "pip3 not found, installing python3-pip first...",
                ))
                .await
                .ok();

            let pip_install = Command::new("sudo")
                .args(["apt-get", "install", "-y", "python3-pip"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            match pip_install {
                Ok(result) if result.status.success() => {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Info,
                            "install",
                            "python3-pip installed successfully",
                        ))
                        .await
                        .ok();
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Error,
                            "install",
                            &format!(
                                "Failed to install python3-pip: {}",
                                stderr.lines().next().unwrap_or("unknown error")
                            ),
                        ))
                        .await
                        .ok();
                    return Ok(false);
                }
                Err(e) => {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Error,
                            "install",
                            &format!("Failed to run apt-get: {}", e),
                        ))
                        .await
                        .ok();
                    return Ok(false);
                }
            }
        }

        // Try multiple methods in order of preference
        let methods = [
            // Method 1: pip3 with --break-system-packages (Ubuntu 23.04+)
            (
                "pip3",
                vec!["install", "--break-system-packages", "vanguards"],
            ),
            // Method 2: pip3 with --user flag
            ("pip3", vec!["install", "--user", "vanguards"]),
            // Method 3: pipx (if available)
            ("pipx", vec!["install", "vanguards"]),
            // Method 4: Plain pip3 (older systems)
            ("pip3", vec!["install", "vanguards"]),
        ];

        for (cmd, args) in &methods {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Info,
                    "install",
                    &format!("Trying: {} {}", cmd, args.join(" ")),
                ))
                .await
                .ok();

            let output = Command::new(cmd)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            match output {
                Ok(result) if result.status.success() => {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Info,
                            "install",
                            "vanguards installed successfully",
                        ))
                        .await
                        .ok();
                    return Ok(true);
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Warn,
                            "install",
                            &format!(
                                "Method failed: {}",
                                stderr.lines().next().unwrap_or("unknown error")
                            ),
                        ))
                        .await
                        .ok();
                }
                Err(_) => {
                    // Command not found, try next method
                }
            }
        }

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Error,
                "install",
                "Failed to install vanguards - all methods failed",
            ))
            .await
            .ok();
        Ok(false)
    }

    /// Install all missing dependencies
    pub async fn install_missing_dependencies(&self) -> Result<(usize, usize)> {
        let missing = get_missing_dependencies();
        let mut installed = 0;
        let mut failed = 0;

        for dep in &missing {
            match self.install_dependency(dep).await {
                Ok(true) => installed += 1,
                Ok(false) => failed += 1,
                Err(e) => {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Error,
                            "install",
                            &format!("Error installing {}: {}", dep.name, e),
                        ))
                        .await
                        .ok();
                    failed += 1;
                }
            }
        }

        Ok((installed, failed))
    }

    /// Start deployment with given config
    pub async fn start(&mut self, config: &FortifyConfig) -> Result<()> {
        // Check if already running
        {
            let state = self.state.lock().await;
            if matches!(*state, DeploymentState::Running | DeploymentState::Starting) {
                return Err(anyhow::anyhow!("Deployment already running"));
            }
        }

        // Update state
        *self.state.lock().await = DeploymentState::Starting;
        *self.config.lock().await = Some(config.clone());

        // Create shutdown channel
        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Save config to expected location
        let config_path = config
            .config_path
            .clone()
            .unwrap_or_else(FortifyConfig::default_path);
        config.save_to(&config_path)?;

        // Check if backend address is still the default - this may be intentional or a mistake
        let default_backend = "http://127.0.0.1:9000";
        if config.network.backend_address == default_backend {
            self.log_tx.send(LogEntry::from_source(
                LogLevel::Warn,
                "config",
                &format!("Backend address is still the default ({}). If you intended to proxy to a .onion service, please update the Network settings.", default_backend)
            )).await.ok();
        } else if !config.network.backend_address.contains(".onion")
            && !config.network.backend_address.starts_with("http://127.")
        {
            self.log_tx.send(LogEntry::from_source(
                LogLevel::Warn,
                "config",
                &format!("Backend address ({}) is not a .onion address. Tor circuit pre-warming will be disabled.", config.network.backend_address)
            )).await.ok();
        }

        // Log vanity config status
        if config.vanity.enabled {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Info,
                    "deploy",
                    &format!(
                        "Vanity generation enabled: prefix='{}', timeout={}s",
                        config.vanity.prefix, config.vanity.safety_net_timeout_seconds
                    ),
                ))
                .await
                .ok();

            let prefix_len = config.vanity.prefix.len();
            if prefix_len > 5 {
                // Each additional character increases time by ~32x
                // 5 chars = ~1 second, 6 chars = ~32 seconds, 7 chars = ~17 minutes
                let estimated = match prefix_len {
                    6 => "~30 seconds",
                    7 => "~15-20 minutes",
                    _ => "hours to days (likely timeout)",
                };
                self.log_tx.send(LogEntry::from_source(
                    LogLevel::Warn,
                    "vanity",
                    &format!("Prefix '{}' ({} chars) estimated generation time: {} per mirror. Consider using 4-5 chars.",
                        config.vanity.prefix,
                        prefix_len,
                        estimated)
                )).await.ok();
            }
        }

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "deploy",
                "Initializing Tor daemon...",
            ))
            .await
            .ok();

        // CRITICAL FIX: Clean up stale processes and files before starting
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "cleanup",
                "Cleaning up any stale processes and data from previous deployments...",
            ))
            .await
            .ok();

        // Delete stale mirror-addresses.txt to prevent confusion
        // This file will be regenerated when user exports mirrors after deployment
        let mirror_addresses_file =
            std::path::PathBuf::from(&config.network.data_dir).join("mirror-addresses.txt");
        if mirror_addresses_file.exists() {
            if let Err(e) = tokio::fs::remove_file(&mirror_addresses_file).await {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Warn,
                        "cleanup",
                        &format!("Could not delete stale mirror-addresses.txt: {}", e),
                    ))
                    .await
                    .ok();
            } else {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Debug,
                        "cleanup",
                        "Deleted stale mirror-addresses.txt",
                    ))
                    .await
                    .ok();
            }
        }

        // Kill any existing Fortify processes
        let _ = tokio::process::Command::new("pkill")
            .args(["-9", "-f", "fortify-controller"])
            .status()
            .await;
        let _ = tokio::process::Command::new("pkill")
            .args(["-9", "-f", "fortify-orchestrator"])
            .status()
            .await;
        let _ = tokio::process::Command::new("pkill")
            .args(["-9", "-f", "fortify-node"])
            .status()
            .await;
        let _ = tokio::process::Command::new("pkill")
            .args(["-9", "-f", "fortify-gate"])
            .status()
            .await;
        let _ = tokio::process::Command::new("pkill")
            .args(["-9", "-f", "fortify-http"])
            .status()
            .await;

        // Give processes time to terminate
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Start Tor daemon
        self.start_tor(config).await?;

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "deploy",
                "Tor ready, starting Fortify controller...",
            ))
            .await
            .ok();

        // Start controller (which manages all other services)
        self.start_controller(config).await?;

        // Wait for controller to stabilize and services to start
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Verify orchestrator and mirrors are responding
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "health",
                "Verifying deployment health...",
            ))
            .await
            .ok();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        match client.get("http://127.0.0.1:8080/mirrors").send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(mirrors) = json.get("mirrors").and_then(|m| m.as_array()) {
                        self.log_tx
                            .send(LogEntry::from_source(
                                LogLevel::Info,
                                "health",
                                &format!(
                                    "✓ Orchestrator healthy, {} mirrors configured",
                                    mirrors.len()
                                ),
                            ))
                            .await
                            .ok();

                        // Collect mirror addresses for export
                        let mut mirror_addresses = Vec::new();
                        for mirror in mirrors {
                            if let Some(addr) = mirror.as_str() {
                                self.log_tx
                                    .send(LogEntry::from_source(
                                        LogLevel::Info,
                                        "mirror",
                                        &format!("Mirror: {}", addr),
                                    ))
                                    .await
                                    .ok();
                                mirror_addresses.push(addr.to_string());
                            }
                        }

                        // Auto-export mirror addresses to file
                        if !mirror_addresses.is_empty() {
                            let export_path = std::path::PathBuf::from(&config.network.data_dir)
                                .join("mirror-addresses.txt");

                            let mut content = String::new();
                            content.push_str("# Fortify Mirror Addresses\n");
                            content.push_str(&format!(
                                "# Auto-exported: {}\n",
                                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                            ));
                            content.push_str(&format!(
                                "# Backend: {}\n\n",
                                config.network.backend_address
                            ));
                            content.push_str("## LIVE MIRRORS:\n");
                            for addr in &mirror_addresses {
                                content.push_str(&format!("http://{}\n", addr));
                            }

                            if let Err(e) = tokio::fs::write(&export_path, &content).await {
                                self.log_tx
                                    .send(LogEntry::from_source(
                                        LogLevel::Warn,
                                        "export",
                                        &format!("Failed to export mirror addresses: {}", e),
                                    ))
                                    .await
                                    .ok();
                            } else {
                                self.log_tx
                                    .send(LogEntry::from_source(
                                        LogLevel::Info,
                                        "export",
                                        &format!(
                                            "Mirror addresses exported to {}",
                                            export_path.display()
                                        ),
                                    ))
                                    .await
                                    .ok();
                            }
                        }

                        self.log_tx.send(LogEntry::from_source(
                            LogLevel::Info,
                            "health",
                            "Note: Mirrors need 30-60 seconds to become reachable via Tor network"
                        )).await.ok();
                    }
                }
            }
            Ok(resp) => {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Warn,
                        "health",
                        &format!("⚠ Orchestrator responded with status {}", resp.status()),
                    ))
                    .await
                    .ok();
            }
            Err(e) => {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Error,
                        "health",
                        &format!("✗ Cannot reach orchestrator: {}. Check logs for errors.", e),
                    ))
                    .await
                    .ok();
            }
        }

        *self.state.lock().await = DeploymentState::Running;
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "deploy",
                "✓ Deployment ready - monitor logs for mirror status",
            ))
            .await
            .ok();

        // Save deployment state to disk
        let state_file = DeploymentStateFile {
            active: true,
            last_started: Some(Local::now().to_rfc3339()),
            last_stopped: None,
            deployment_id: config.deployment_id.clone(),
            mirror_addresses: Vec::new(), // Will be populated by orchestrator API
            node_addresses: Vec::new(),
        };

        let state_path = DeploymentStateFile::default_path();
        if let Some(parent) = state_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(e) = state_file.save(&state_path) {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Warn,
                    "state",
                    &format!("Failed to save deployment state: {}", e),
                ))
                .await
                .ok();
        } else {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Debug,
                    "state",
                    "Deployment state saved",
                ))
                .await
                .ok();
        }

        Ok(())
    }

    /// Start Tor daemon
    async fn start_tor(&mut self, config: &FortifyConfig) -> Result<()> {
        let data_dir = &config.network.data_dir;
        let tor_dir = data_dir.join("tor");
        std::fs::create_dir_all(&tor_dir)?;

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Debug,
                "tor",
                &format!("Tor data directory: {}", tor_dir.display()),
            ))
            .await
            .ok();

        // Kill any existing Tor process that might be using our ports
        let _ = tokio::process::Command::new("pkill")
            .arg("-f")
            .arg(format!("tor.*{}", tor_dir.display()))
            .status()
            .await;

        // Remove stale lock file if it exists (prevents startup failures)
        let lock_file = tor_dir.join("data").join("lock");
        if lock_file.exists() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Debug,
                    "tor",
                    "Removing stale lock file",
                ))
                .await
                .ok();
            let _ = std::fs::remove_file(&lock_file);
        }

        // Create torrc if needed
        let torrc_path = tor_dir.join("torrc");
        let torrc_inc_path = tor_dir.join("torrc.inc");

        if !torrc_path.exists() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Debug,
                    "tor",
                    &format!("Creating torrc at {}", torrc_path.display()),
                ))
                .await
                .ok();

            // Create a main torrc with basic settings
            // Tor will regenerate this file on startup, but we use a separate .inc file
            // for persistent mirror configurations that we manually append after Tor's rewrite
            let torrc_content = format!(
                "DataDirectory {}\nControlPort 127.0.0.1:{}\nSocksPort 127.0.0.1:{}\nCookieAuthentication 1\nLog notice file {}\n",
                tor_dir.join("data").display(),
                config.network.control_port,
                config.network.socks_port,
                tor_dir.join("tor.log").display()
            );
            std::fs::create_dir_all(tor_dir.join("data"))?;
            std::fs::write(&torrc_path, torrc_content)?;

            // Create empty torrc.inc file for mirror configs
            std::fs::write(&torrc_inc_path, "")?;
        } else {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Debug,
                    "tor",
                    "Using existing torrc",
                ))
                .await
                .ok();
        }

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "tor",
                &format!(
                    "Starting Tor (SOCKS:{}, Control:{})",
                    config.network.socks_port, config.network.control_port
                ),
            ))
            .await
            .ok();

        // Start tor
        let mut child = Command::new("tor")
            .arg("-f")
            .arg(&torrc_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Debug,
                "tor",
                "Tor process spawned, waiting for bootstrap...",
            ))
            .await
            .ok();

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let log_tx = self.log_tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(entry) = parse_log_line(&line) {
                        log_tx
                            .send(LogEntry::from_source(entry.level, "tor", &entry.message))
                            .await
                            .ok();
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let log_tx = self.log_tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log_tx
                        .send(LogEntry::from_source(LogLevel::Warn, "tor", &line))
                        .await
                        .ok();
                }
            });
        }

        self.children.lock().await.push(child);

        // Wait for Tor to be ready (check control port)
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Debug,
                "tor",
                "Waiting for Tor control port to be available...",
            ))
            .await
            .ok();

        let control_addr = format!("127.0.0.1:{}", config.network.control_port);
        let mut connected = false;
        for attempt in 1..=15 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if std::net::TcpStream::connect(&control_addr).is_ok() {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Info,
                        "tor",
                        &format!("Tor control port ready after {}s", attempt),
                    ))
                    .await
                    .ok();
                connected = true;
                break;
            }
            if attempt % 5 == 0 {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Debug,
                        "tor",
                        &format!("Still waiting for Tor... ({}s)", attempt),
                    ))
                    .await
                    .ok();
            }
        }

        if !connected {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Warn,
                    "tor",
                    "Tor control port not responding after 15s, continuing anyway...",
                ))
                .await
                .ok();
        }

        Ok(())
    }

    /// Start Fortify controller
    async fn start_controller(&mut self, config: &FortifyConfig) -> Result<()> {
        let controller_bin = Self::find_binary("fortify-controller")?;

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Debug,
                "deploy",
                &format!("Found controller: {}", controller_bin.display()),
            ))
            .await
            .ok();

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "deploy",
                "Spawning Fortify controller...",
            ))
            .await
            .ok();

        let mut cmd = Command::new(&controller_bin);
        cmd.env("FORTIFY_DATA_DIR", &config.network.data_dir)
            .env("NODE_BACKEND_ADDR", &config.network.backend_address)
            .env("FORTIFY_SOCKS_PORT", config.network.socks_port.to_string())
            .env(
                "FORTIFY_CONTROL_PORT",
                config.network.control_port.to_string(),
            )
            .env(
                "TOR_CONTROL_ADDR",
                format!("127.0.0.1:{}", config.network.control_port),
            )
            .env(
                "TOR_COOKIE_PATH",
                config
                    .network
                    .data_dir
                    .join("tor/data/control_auth_cookie")
                    .to_string_lossy()
                    .to_string(),
            )
            // CAPTCHA pool configuration
            .env("CAPTCHA_ENABLED", config.captcha.enabled.to_string())
            .env("CAPTCHA_POOL_SIZE", config.captcha.pool_size.to_string())
            .env("CAPTCHA_MIN_POOL", config.captcha.min_pool_size.to_string())
            .env("CAPTCHA_MAX_POOL", config.captcha.max_pool_size.to_string())
            .env(
                "CAPTCHA_ROTATION_PERCENT",
                config.captcha.rotation_percent.to_string(),
            )
            .env(
                "CAPTCHA_ROTATION_DAYS",
                config.captcha.rotation_interval_days.to_string(),
            )
            .env("RUST_LOG", "info,fortify_controller=debug");

        // Pass vanity configuration - controller will forward to orchestrators for mirror generation
        // Note: Vanity applies to MIRRORS only, not nodes (healthy/threat nodes use random addresses)
        if config.vanity.enabled && !config.vanity.prefix.is_empty() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Info,
                    "vanity",
                    &format!(
                        "Vanity enabled for mirrors: prefix='{}', timeout={}s",
                        config.vanity.prefix, config.vanity.safety_net_timeout_seconds
                    ),
                ))
                .await
                .ok();

            cmd.env("VANITY_ENABLED", "true")
                .env("VANITY_PREFIX", &config.vanity.prefix)
                .env(
                    "VANITY_TIMEOUT",
                    config.vanity.safety_net_timeout_seconds.to_string(),
                );
        }

        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Debug,
                "deploy",
                "Controller process spawned",
            ))
            .await
            .ok();

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let log_tx = self.log_tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(entry) = parse_log_line(&line) {
                        log_tx.send(entry).await.ok();
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let log_tx = self.log_tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(entry) = parse_log_line(&line) {
                        log_tx.send(entry).await.ok();
                    } else {
                        log_tx
                            .send(LogEntry::from_source(LogLevel::Error, "controller", &line))
                            .await
                            .ok();
                    }
                }
            });
        }

        self.children.lock().await.push(child);

        // Give controller time to start
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "deploy",
                "Controller started, deployment ready",
            ))
            .await
            .ok();

        Ok(())
    }

    /// Verify that onion addresses are reachable via Tor
    pub async fn verify_onion_addresses(&self, addresses: &[String], socks_port: u16) -> Vec<crate::verification::VerificationResult> {
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "verify",
                &format!("Starting verification of {} onion address(es)...", addresses.len()),
            ))
            .await
            .ok();

        *self.state.lock().await = DeploymentState::Verifying;

        let config = VerificationConfig::with_socks_port(socks_port);
        let verifier = OnionVerifier::new(config);
        
        let mut results = Vec::new();
        for (i, address) in addresses.iter().enumerate() {
            self.log_tx
                .send(LogEntry::from_source(
                    LogLevel::Debug,
                    "verify",
                    &format!("Verifying address {}/{}: {}", i + 1, addresses.len(), address),
                ))
                .await
                .ok();

            let result = verifier.verify(address).await;
            
            if result.reachable {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Info,
                        "verify",
                        &format!("✓ {} is reachable ({}ms)", 
                            address,
                            result.response_time_ms.unwrap_or(0)
                        ),
                    ))
                    .await
                    .ok();
            } else {
                self.log_tx
                    .send(LogEntry::from_source(
                        LogLevel::Warn,
                        "verify",
                        &format!("✗ {} is NOT reachable: {}", 
                            address,
                            result.error.as_deref().unwrap_or("Unknown error")
                        ),
                    ))
                    .await
                    .ok();
            }
            
            results.push(result);
        }

        // Summarize results
        let reachable_count = results.iter().filter(|r| r.reachable).count();
        self.log_tx
            .send(LogEntry::from_source(
                LogLevel::Info,
                "verify",
                &format!("Verification complete: {}/{} addresses reachable", 
                    reachable_count, 
                    addresses.len()
                ),
            ))
            .await
            .ok();

        results
    }

    /// Find binary in target directory or PATH
    fn find_binary(name: &str) -> Result<std::path::PathBuf> {
        // Check target/release first
        let release = std::path::PathBuf::from(format!("target/release/{}", name));
        if release.exists() {
            return Ok(release);
        }

        // Check target/debug
        let debug = std::path::PathBuf::from(format!("target/debug/{}", name));
        if debug.exists() {
            return Ok(debug);
        }

        // Check relative to current exe
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let sibling = dir.join(name);
                if sibling.exists() {
                    return Ok(sibling);
                }
            }
        }

        // Fall back to PATH
        Ok(std::path::PathBuf::from(name))
    }

    /// Stop deployment
    pub async fn stop(&mut self) -> Result<()> {
        *self.state.lock().await = DeploymentState::Stopping;

        self.log_tx
            .send(LogEntry::info("Stopping deployment..."))
            .await
            .ok();

        // Signal shutdown
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }

        // Kill child processes
        let mut children = self.children.lock().await;
        for child in children.iter_mut() {
            let _ = child.kill().await;
        }
        children.clear();

        // Also kill any remaining fortify/tor processes
        let _ = tokio::process::Command::new("pkill")
            .arg("-f")
            .arg("fortify-")
            .status()
            .await;

        let _ = tokio::process::Command::new("pkill")
            .arg("tor")
            .status()
            .await;

        *self.state.lock().await = DeploymentState::Stopped;

        // Update deployment state file
        let state_path = DeploymentStateFile::default_path();
        if state_path.exists() {
            if let Ok(mut state) = DeploymentStateFile::load(&state_path) {
                state.active = false;
                state.last_stopped = Some(Local::now().to_rfc3339());
                if let Err(e) = state.save(&state_path) {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Warn,
                            "state",
                            &format!("Failed to update deployment state: {}", e),
                        ))
                        .await
                        .ok();
                } else {
                    self.log_tx
                        .send(LogEntry::from_source(
                            LogLevel::Debug,
                            "state",
                            "Deployment state updated (stopped)",
                        ))
                        .await
                        .ok();
                }
            }
        }

        *self.config.lock().await = None;

        self.log_tx
            .send(LogEntry::info("Deployment stopped"))
            .await
            .ok();

        Ok(())
    }

    /// Reload configuration (hot reload)
    pub async fn reload_config(&mut self, config: &FortifyConfig) -> Result<()> {
        self.log_tx
            .send(LogEntry::info("Reloading configuration..."))
            .await
            .ok();

        // Save updated config
        let mut cfg = config.clone();
        cfg.save()?;

        // Send SIGHUP to controller to trigger reload
        // For now, just log - full hot reload would need IPC
        self.log_tx
            .send(LogEntry::info(
                "Configuration saved (restart required for some changes)",
            ))
            .await
            .ok();

        *self.config.lock().await = Some(config.clone());

        Ok(())
    }

    /// Get deployment info
    pub async fn get_info(&self) -> DeploymentInfo {
        let state = self.state.lock().await.clone();
        let config = self.config.lock().await.clone();
        let child_count = self.children.lock().await.len();

        DeploymentInfo {
            state,
            config,
            process_count: child_count,
        }
    }
}

/// Deployment information for display
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub state: DeploymentState,
    pub config: Option<FortifyConfig>,
    pub process_count: usize,
}
