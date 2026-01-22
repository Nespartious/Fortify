//! Configuration types and management for Fortify TUI

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Root configuration for a Fortify deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortifyConfig {
    /// Unique deployment identifier
    pub deployment_id: String,
    /// Branding configuration
    pub branding: BrandingConfig,
    /// CAPTCHA settings
    pub captcha: CaptchaConfig,
    /// Threshold and limits configuration
    pub thresholds: ThresholdConfig,
    /// Network settings
    pub network: NetworkConfig,
    /// Mirror configuration
    pub mirrors: MirrorConfig,
    /// Vanity address configuration
    pub vanity: VanityConfig,
    /// Path to config file
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
    /// Whether config has unsaved changes
    #[serde(skip)]
    pub dirty: bool,
}

/// Branding configuration for the protected service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Display name/title for the service
    pub service_name: String,
    /// Short description
    pub description: String,
    /// Path to logo image (PNG/JPG, max 256x256)
    pub logo_path: Option<PathBuf>,
    /// Logo as base64 (loaded from path)
    #[serde(skip)]
    pub logo_base64: Option<String>,
    /// Maximum logo width in pixels
    pub logo_max_width: u32,
    /// Maximum logo height in pixels
    pub logo_max_height: u32,
    /// Primary color (hex)
    pub primary_color: String,
    /// Custom CSS for gate pages
    pub custom_css: Option<String>,
    /// Welcome message on CAPTCHA page
    pub welcome_message: String,
}

/// CAPTCHA pool and behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaConfig {
    /// Enable CAPTCHA challenges
    pub enabled: bool,
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
    /// Enable audio CAPTCHA option
    pub audio_enabled: bool,
    /// Rotate pool percentage
    pub rotation_percent: u8,
    /// Rotation interval in days
    pub rotation_interval_days: u32,
}

/// Threshold and limit settings for threat detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Requests per minute before rate limiting
    pub rate_limit_rpm: u32,
    /// Failed CAPTCHAs before temporary ban
    pub captcha_fail_limit: u32,
    /// Temporary ban duration in minutes
    pub temp_ban_minutes: u32,
    /// Permanent ban threshold (infractions)
    pub perm_ban_threshold: u32,
    /// Suspicious behavior score threshold
    pub suspicion_threshold: f32,
    /// Threat score threshold for immediate action
    pub threat_threshold: f32,
    /// Mirror burn threshold
    pub burn_threshold: f32,
    /// Enable automatic banning
    pub auto_ban_enabled: bool,
    /// DDoS detection: requests per second threshold
    pub ddos_rps_threshold: u32,
    /// Probe detection sensitivity (1-10)
    pub probe_sensitivity: u8,
}

/// Network and Tor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Backend service address to protect
    pub backend_address: String,
    /// SOCKS proxy port
    pub socks_port: u16,
    /// Control port
    pub control_port: u16,
    /// HTTP proxy bind address
    pub http_bind: String,
    /// Gate bind address  
    pub gate_bind: String,
    /// Enable vanguards addon
    pub vanguards_enabled: bool,
    /// Vanguards layer 2 guards
    pub vanguards_layer2: u8,
    /// Vanguards layer 3 guards
    pub vanguards_layer3: u8,
    /// Data directory
    pub data_dir: PathBuf,
}

/// Mirror management settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    /// Minimum active mirrors
    pub min_mirrors: usize,
    /// Maximum mirrors allowed
    pub max_mirrors: usize,
    /// Standby mirrors to maintain
    pub standby_mirrors: usize,
    /// Rotation interval in seconds
    pub rotation_interval_seconds: u64,
    /// Enable proactive burning
    pub proactive_burn_enabled: bool,
    /// Minimum days before proactive burn
    pub burn_interval_days_min: u32,
    /// Maximum days before proactive burn
    pub burn_interval_days_max: u32,
    /// Retirement page display hours
    pub retirement_page_hours: u32,
}

/// Vanity address generation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VanityConfig {
    /// Enable vanity address generation for all mirrors
    pub enabled: bool,
    /// Prefix to match (max 10 characters)
    pub prefix: String,
    /// Enable safety net timeout
    pub safety_net_enabled: bool,
    /// Maximum seconds to search before reducing prefix
    /// Default: 900 (15 min), Testing: 30
    pub safety_net_timeout_seconds: u64,
    /// Minimum prefix length before giving up
    pub min_prefix_length: usize,
    /// Warn threshold - warn if prefix exceeds this length
    pub warn_threshold: usize,
}

impl Default for FortifyConfig {
    fn default() -> Self {
        Self {
            deployment_id: uuid_short(),
            branding: BrandingConfig::default(),
            captcha: CaptchaConfig::default(),
            thresholds: ThresholdConfig::default(),
            network: NetworkConfig::default(),
            mirrors: MirrorConfig::default(),
            vanity: VanityConfig::default(),
            config_path: None,
            dirty: false,
        }
    }
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            service_name: "Protected Service".to_string(),
            description: "A Fortify-protected onion service".to_string(),
            logo_path: None,
            logo_base64: None,
            logo_max_width: 256,
            logo_max_height: 256,
            primary_color: "#6B46C1".to_string(), // Purple
            custom_css: None,
            welcome_message: "Please complete the verification to continue.".to_string(),
        }
    }
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pool_size: 500,
            min_pool_size: 100,
            max_pool_size: 1000,
            difficulty: 5,
            timeout_seconds: 120,
            max_attempts: 3,
            audio_enabled: false,
            rotation_percent: 25,
            rotation_interval_days: 10,
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            rate_limit_rpm: 60,
            captcha_fail_limit: 5,
            temp_ban_minutes: 30,
            perm_ban_threshold: 10,
            suspicion_threshold: 0.5,
            threat_threshold: 0.7,
            burn_threshold: 0.7,
            auto_ban_enabled: true,
            ddos_rps_threshold: 100,
            probe_sensitivity: 5,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        // Use ~/.local/share/fortify for runtime data (persists across reboots)
        let data_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path
        } else {
            PathBuf::from("/tmp/fortify")
        };

        Self {
            backend_address: "http://127.0.0.1:9000".to_string(),
            socks_port: 9150,
            control_port: 9151,
            http_bind: "127.0.0.1:8082".to_string(),
            gate_bind: "127.0.0.1:8081".to_string(),
            vanguards_enabled: true,
            vanguards_layer2: 4,
            vanguards_layer3: 8,
            data_dir,
        }
    }
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            min_mirrors: 2,
            max_mirrors: 5,
            standby_mirrors: 2,
            rotation_interval_seconds: 3600,
            proactive_burn_enabled: true,
            burn_interval_days_min: 60,
            burn_interval_days_max: 120,
            retirement_page_hours: 72,
        }
    }
}

impl Default for VanityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prefix: String::new(),
            safety_net_enabled: true,
            // 30 seconds for testing, change to 900 (15 min) for production
            safety_net_timeout_seconds: 30,
            min_prefix_length: 1,
            warn_threshold: 5,
        }
    }
}

impl FortifyConfig {
    /// Load configuration from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: FortifyConfig = toml::from_str(&content)?;
        config.config_path = Some(path.to_path_buf());
        config.dirty = false;

        // Load logo if path specified
        if let Some(ref logo_path) = config.branding.logo_path {
            if logo_path.exists() {
                if let Ok(data) = std::fs::read(logo_path) {
                    config.branding.logo_base64 = Some(base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &data,
                    ));
                }
            }
        }

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&mut self) -> Result<()> {
        // Use config_path if set, otherwise use default path
        let path = match self.config_path.as_ref() {
            Some(p) => p.clone(),
            None => {
                let default = Self::default_path();
                self.config_path = Some(default.clone());
                default
            }
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Save to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Mark config as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if config has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get default config path (persistent across reboots)
    pub fn default_path() -> PathBuf {
        // Use ~/.config/fortify for persistent storage (survives reboots)
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("fortify");
            path.push("deployment.toml");
            return path;
        }
        // Fallback to /tmp if HOME not available
        PathBuf::from("/tmp/fortify/config/deployment.toml")
    }

    /// List existing deployments
    pub fn list_deployments() -> Result<Vec<(String, PathBuf)>> {
        let config_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("fortify");
            path.push("deployments");
            path
        } else {
            PathBuf::from("/tmp/fortify/deployments")
        };
        let mut deployments = Vec::new();

        if config_dir.exists() {
            for entry in std::fs::read_dir(config_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let config_file = path.join("config.toml");
                    if config_file.exists() {
                        if let Ok(config) = Self::load(&config_file) {
                            deployments.push((config.deployment_id.clone(), config_file));
                        }
                    }
                }
            }
        }

        Ok(deployments)
    }
}

/// Configuration change that can be applied or stored
#[derive(Debug, Clone)]
pub struct PendingChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Manages pending configuration changes
#[derive(Debug, Default)]
pub struct ChangeManager {
    pub pending_changes: Vec<PendingChange>,
    pub stored_for_restart: Vec<PendingChange>,
}

impl ChangeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_change(&mut self, field: &str, old: &str, new: &str) {
        self.pending_changes.push(PendingChange {
            field: field.to_string(),
            old_value: old.to_string(),
            new_value: new.to_string(),
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    pub fn store_for_restart(&mut self) {
        self.stored_for_restart.append(&mut self.pending_changes);
    }

    pub fn clear_pending(&mut self) {
        self.pending_changes.clear();
    }

    pub fn apply_all(&mut self) {
        self.pending_changes.clear();
        self.stored_for_restart.clear();
    }
}

/// Generate a short UUID for deployment IDs
fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("deploy-{:x}", now & 0xFFFFFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FortifyConfig::default();
        assert!(config.captcha.enabled);
        assert_eq!(config.mirrors.min_mirrors, 2);
        assert_eq!(config.thresholds.burn_threshold, 0.7);
    }
}
