use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::time::Duration;

/// Controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerConfig {
    // Base data directory for all Fortify components
    pub data_dir: std::path::PathBuf,

    // Service limits
    pub min_orchestrators: usize,
    pub max_orchestrators: usize,
    pub min_healthy_nodes: usize,
    pub max_healthy_nodes: usize,
    pub min_threat_nodes: usize,
    pub max_threat_nodes: usize,
    // Legacy aliases for backwards compat
    #[serde(skip)]
    pub min_nodes: usize,
    #[serde(skip)]
    pub max_nodes: usize,

    // Service addresses
    pub gate_bind_addr: String,
    pub proxy_bind_addr: String,
    pub orchestrator_bind_addr: String,
    pub controller_bind_addr: String,
    pub healthy_node_bind_base: String,
    pub threat_node_bind_base: String,
    pub node_backend_addr: String,
    // Legacy alias
    #[serde(skip)]
    pub node_bind_base: String,

    // Monitoring intervals
    pub health_check_interval: Duration,
    pub scaling_check_interval: Duration,

    // Restart policy
    pub max_restart_attempts: usize,
    pub restart_backoff: Duration,

    // Shared secrets and Tor wiring
    pub secret_key: String,
    pub tor_control_addr: Option<String>,
    pub tor_cookie_path: Option<String>,

    // Vanguards configuration
    pub vanguards_enabled: bool,
    pub vanguards_layer2_guards: u8,
    pub vanguards_layer3_guards: u8,
    pub vanguards_circ_max_age_hours: u32,
    pub vanguards_circ_max_megabytes: u32,

    // Vanity address configuration for mirrors (forwarded to orchestrators)
    // Note: Nodes do NOT use vanity addresses - only mirrors do
    pub vanity_enabled: bool,
    pub vanity_prefix: String,
    pub vanity_timeout_seconds: u64,

    // CAPTCHA configuration (forwarded to orchestrators)
    pub captcha_enabled: bool,
    pub captcha_pool_size: usize,
    pub captcha_min_pool: usize,
    pub captcha_max_pool: usize,
    pub captcha_rotation_percent: u8,
    pub captcha_rotation_days: u32,

    // Branding configuration (forwarded to Gate)
    pub branding_service_name: String,
    pub branding_description: String,
    pub branding_welcome_message: String,
    pub branding_primary_color: String,
    pub branding_secondary_color: String,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        // Use ~/.local/share/fortify if HOME is set, otherwise /tmp/fortify
        let data_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path
        } else {
            std::path::PathBuf::from("/tmp/fortify")
        };

        Self {
            data_dir,
            min_orchestrators: 2,
            max_orchestrators: 10,
            min_healthy_nodes: 10,
            max_healthy_nodes: 20,
            min_threat_nodes: 3,
            max_threat_nodes: 10,
            // Legacy aliases (updated in from_env)
            min_nodes: 10,
            max_nodes: 20,
            gate_bind_addr: "127.0.0.1:8081".to_string(),
            proxy_bind_addr: "127.0.0.1:8082".to_string(),
            orchestrator_bind_addr: "127.0.0.1:8080".to_string(),
            controller_bind_addr: "127.0.0.1:7000".to_string(),
            healthy_node_bind_base: "127.0.0.1:9100".to_string(),
            threat_node_bind_base: "127.0.0.1:9200".to_string(),
            node_bind_base: "127.0.0.1:9100".to_string(),
            node_backend_addr: "http://127.0.0.1:9000".to_string(),
            health_check_interval: Duration::from_secs(30),
            scaling_check_interval: Duration::from_secs(60),
            max_restart_attempts: 3,
            restart_backoff: Duration::from_secs(10),
            secret_key: "fortify-secret-key".to_string(),
            tor_control_addr: None,
            tor_cookie_path: None,
            // Vanguards defaults
            vanguards_enabled: true,
            vanguards_layer2_guards: 4,
            vanguards_layer3_guards: 8,
            vanguards_circ_max_age_hours: 24,
            vanguards_circ_max_megabytes: 0,
            // Vanity defaults - forwarded to orchestrators for mirror addresses
            vanity_enabled: false,
            vanity_prefix: String::new(),
            vanity_timeout_seconds: 30,
            // CAPTCHA defaults
            captcha_enabled: true,
            captcha_pool_size: 500,
            captcha_min_pool: 150,
            captcha_max_pool: 1000,
            captcha_rotation_percent: 25,
            captcha_rotation_days: 10,
            // Branding defaults - forwarded to Gate
            branding_service_name: "Protected Service".to_string(),
            branding_description: "A Fortify-protected onion service".to_string(),
            branding_welcome_message: "Please complete the verification to continue.".to_string(),
            branding_primary_color: "#c9a227".to_string(),
            branding_secondary_color: "#a68b5b".to_string(),
        }
    }
}

impl ControllerConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = ControllerConfig::default();

        // Base data directory - passed from TUI
        if let Ok(val) = env::var("FORTIFY_DATA_DIR") {
            config.data_dir = std::path::PathBuf::from(val);
        }

        if let Ok(val) = env::var("MIN_ORCHESTRATORS") {
            config.min_orchestrators = val.parse()?;
        }
        if let Ok(val) = env::var("MAX_ORCHESTRATORS") {
            config.max_orchestrators = val.parse()?;
        }
        // Support both old MIN_NODES and new MIN_HEALTHY_NODES
        if let Ok(val) = env::var("MIN_HEALTHY_NODES") {
            config.min_healthy_nodes = val.parse()?;
        } else if let Ok(val) = env::var("MIN_NODES") {
            config.min_healthy_nodes = val.parse()?;
        }
        if let Ok(val) = env::var("MAX_HEALTHY_NODES") {
            config.max_healthy_nodes = val.parse()?;
        } else if let Ok(val) = env::var("MAX_NODES") {
            config.max_healthy_nodes = val.parse()?;
        }
        if let Ok(val) = env::var("MIN_THREAT_NODES") {
            config.min_threat_nodes = val.parse()?;
        }
        if let Ok(val) = env::var("MAX_THREAT_NODES") {
            config.max_threat_nodes = val.parse()?;
        }
        // Update legacy aliases
        config.min_nodes = config.min_healthy_nodes;
        config.max_nodes = config.max_healthy_nodes;

        if let Ok(addr) = env::var("GATE_BIND_ADDR") {
            config.gate_bind_addr = addr;
        }
        if let Ok(addr) = env::var("PROXY_BIND_ADDR") {
            config.proxy_bind_addr = addr;
        }
        if let Ok(addr) = env::var("ORCH_BIND_ADDR") {
            config.orchestrator_bind_addr = addr;
        }
        if let Ok(addr) = env::var("CONTROLLER_BIND_ADDR") {
            config.controller_bind_addr = addr;
        }
        // Support both old NODE_BIND_BASE and new HEALTHY_NODE_BIND_BASE
        if let Ok(addr) = env::var("HEALTHY_NODE_BIND_BASE") {
            config.healthy_node_bind_base = addr.clone();
            config.node_bind_base = addr;
        } else if let Ok(addr) = env::var("NODE_BIND_BASE") {
            config.healthy_node_bind_base = addr.clone();
            config.node_bind_base = addr;
        }
        if let Ok(addr) = env::var("THREAT_NODE_BIND_BASE") {
            config.threat_node_bind_base = addr;
        }
        if let Ok(addr) = env::var("NODE_BACKEND_ADDR") {
            config.node_backend_addr = addr;
        }

        if let Ok(secret) = env::var("SECRET_KEY") {
            config.secret_key = secret;
        }
        if let Ok(addr) = env::var("TOR_CONTROL_ADDR") {
            config.tor_control_addr = Some(addr);
        }
        if let Ok(path) = env::var("TOR_COOKIE_PATH") {
            config.tor_cookie_path = Some(path);
        }

        // Vanguards configuration
        if let Ok(val) = env::var("VANGUARDS_ENABLED") {
            config.vanguards_enabled = val.parse().unwrap_or(true);
        }
        if let Ok(val) = env::var("VANGUARDS_LAYER2_GUARDS") {
            config.vanguards_layer2_guards = val.parse().unwrap_or(4);
        }
        if let Ok(val) = env::var("VANGUARDS_LAYER3_GUARDS") {
            config.vanguards_layer3_guards = val.parse().unwrap_or(8);
        }
        if let Ok(val) = env::var("VANGUARDS_CIRC_MAX_AGE_HOURS") {
            config.vanguards_circ_max_age_hours = val.parse().unwrap_or(24);
        }
        if let Ok(val) = env::var("VANGUARDS_CIRC_MAX_MEGABYTES") {
            config.vanguards_circ_max_megabytes = val.parse().unwrap_or(0);
        }

        // Vanity configuration
        if let Ok(val) = env::var("VANITY_ENABLED") {
            config.vanity_enabled = val.parse().unwrap_or(false);
        }
        if let Ok(val) = env::var("VANITY_PREFIX") {
            config.vanity_prefix = val;
        }
        if let Ok(val) = env::var("VANITY_TIMEOUT") {
            config.vanity_timeout_seconds = val.parse().unwrap_or(30);
        }

        // CAPTCHA configuration
        if let Ok(val) = env::var("CAPTCHA_ENABLED") {
            config.captcha_enabled = val.parse().unwrap_or(true);
        }
        if let Ok(val) = env::var("CAPTCHA_POOL_SIZE") {
            config.captcha_pool_size = val.parse().unwrap_or(500);
        }
        if let Ok(val) = env::var("CAPTCHA_MIN_POOL") {
            config.captcha_min_pool = val.parse().unwrap_or(150);
        }
        if let Ok(val) = env::var("CAPTCHA_MAX_POOL") {
            config.captcha_max_pool = val.parse().unwrap_or(1000);
        }
        if let Ok(val) = env::var("CAPTCHA_ROTATION_PERCENT") {
            config.captcha_rotation_percent = val.parse().unwrap_or(25);
        }
        if let Ok(val) = env::var("CAPTCHA_ROTATION_DAYS") {
            config.captcha_rotation_days = val.parse().unwrap_or(10);
        }

        // Branding config - forwarded to Gate
        if let Ok(val) = env::var("BRANDING_SERVICE_NAME") {
            config.branding_service_name = val;
        }
        if let Ok(val) = env::var("BRANDING_DESCRIPTION") {
            config.branding_description = val;
        }
        if let Ok(val) = env::var("BRANDING_WELCOME_MESSAGE") {
            config.branding_welcome_message = val;
        }
        if let Ok(val) = env::var("BRANDING_PRIMARY_COLOR") {
            config.branding_primary_color = val;
        }
        if let Ok(val) = env::var("BRANDING_SECONDARY_COLOR") {
            config.branding_secondary_color = val;
        }

        config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.min_orchestrators > self.max_orchestrators {
            return Err("min_orchestrators cannot exceed max_orchestrators".to_string());
        }

        if self.min_healthy_nodes > self.max_healthy_nodes {
            return Err("min_healthy_nodes cannot exceed max_healthy_nodes".to_string());
        }

        if self.min_threat_nodes > self.max_threat_nodes {
            return Err("min_threat_nodes cannot exceed max_threat_nodes".to_string());
        }

        if self.min_orchestrators == 0 {
            return Err("min_orchestrators must be at least 1".to_string());
        }

        if self.min_healthy_nodes == 0 {
            return Err("min_healthy_nodes must be at least 1".to_string());
        }

        for (label, addr) in [
            ("gate_bind_addr", &self.gate_bind_addr),
            ("proxy_bind_addr", &self.proxy_bind_addr),
            ("orchestrator_bind_addr", &self.orchestrator_bind_addr),
            ("controller_bind_addr", &self.controller_bind_addr),
            ("healthy_node_bind_base", &self.healthy_node_bind_base),
            ("threat_node_bind_base", &self.threat_node_bind_base),
        ] {
            addr.parse::<SocketAddr>()
                .map_err(|e| format!("{} is invalid ({}): {}", label, addr, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ControllerConfig::default();

        assert_eq!(config.min_orchestrators, 2);
        assert_eq!(config.max_orchestrators, 10);
        assert_eq!(config.min_healthy_nodes, 10);
        assert_eq!(config.max_healthy_nodes, 20);
        assert_eq!(config.min_threat_nodes, 3);
        assert_eq!(config.max_threat_nodes, 10);
        assert_eq!(config.orchestrator_bind_addr, "127.0.0.1:8080");
        assert_eq!(config.controller_bind_addr, "127.0.0.1:7000");
    }

    #[test]
    fn test_config_validation_valid() {
        let config = ControllerConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_orchestrator_limits() {
        let config = ControllerConfig {
            min_orchestrators: 15,
            max_orchestrators: 10,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_healthy_node_limits() {
        let config = ControllerConfig {
            min_healthy_nodes: 25,
            max_healthy_nodes: 20,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_threat_node_limits() {
        let config = ControllerConfig {
            min_threat_nodes: 15,
            max_threat_nodes: 10,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_orchestrators() {
        let config = ControllerConfig {
            min_orchestrators: 0,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_healthy_nodes() {
        let config = ControllerConfig {
            min_healthy_nodes: 0,
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }
}
