use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// Main Fortify configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FortifyConfig {
    pub service: ServiceConfig,
    pub controller: ControllerConfig,
    pub orchestrator: OrchestratorConfig,
    pub gate: GateConfig,
    pub http_proxy: HttpProxyConfig,
    pub community: CommunityConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub real_onion_address: String,
    pub real_service_port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ControllerConfig {
    pub bind_address: String,
    pub max_orchestrators: usize,
    pub max_healthy_nodes: usize,
    pub max_threat_nodes: usize,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrchestratorConfig {
    pub bind_address: String,
    pub max_connections_per_minute: usize,
    pub max_failed_challenges: usize,
    pub rotation_interval_hours: u64,
    pub tor_control_port: String,
    pub tor_socks_port: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GateConfig {
    pub bind_address: String,
    pub max_concurrent_verifications: usize,
    pub verification_timeout_seconds: u64,
    pub captcha_difficulty: String,
    pub pow_difficulty: u32,
    pub token_lifetime_seconds: u64,
    pub token_signing_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpProxyConfig {
    pub bind_address: String,
    pub max_concurrent_connections: usize,
    pub connection_timeout_seconds: u64,
    pub max_request_size_bytes: usize,
    pub queue_size: usize,
    pub reject_when_full: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommunityConfig {
    pub enabled: bool,
    pub mode: String,
    pub registry_url: String,
    pub update_interval_seconds: u64,
    pub signing_key_path: String,
}

/// Behavioral analysis configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BehavioralConfig {
    /// Master enable/disable switch
    pub enabled: bool,
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
    /// Maximum payload size in bytes
    pub max_payload_size: usize,
    /// Sequential path detection threshold
    pub sequential_path_threshold: u32,
}

impl Default for BehavioralConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ua_analysis_enabled: true,
            referer_analysis_enabled: true,
            path_analysis_enabled: true,
            enumeration_detection_enabled: true,
            form_tracking_enabled: true,
            payload_analysis_enabled: true,
            max_unique_paths_per_minute: 60,
            max_form_submissions_per_minute: 10,
            max_payload_size: 10 * 1024 * 1024,
            sequential_path_threshold: 5,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub output: String,
    pub log_file: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub drop_privileges: bool,
    pub chroot_path: String,
    pub secure_memory: bool,
}

impl FortifyConfig {
    /// Load configuration from TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: FortifyConfig =
            toml::from_str(&contents).map_err(|e| CoreError::Parse(e.to_string()))?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.service.real_onion_address.is_empty() {
            return Err(CoreError::Config(
                "real_onion_address cannot be empty".into(),
            ));
        }
        if self.service.real_service_port == 0 {
            return Err(CoreError::Config("real_service_port cannot be 0".into()));
        }
        Ok(())
    }
}
