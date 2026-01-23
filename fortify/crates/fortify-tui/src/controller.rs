//! Controller integration module
//!
//! Provides communication with the fortify-controller HTTP API
//! for runtime management and status monitoring.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Controller API client configuration
#[derive(Debug, Clone)]
pub struct ControllerConfig {
    /// Controller HTTP API base URL
    pub base_url: String,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:9090".to_string(),
            timeout: Duration::from_secs(10),
        }
    }
}

impl ControllerConfig {
    /// Create config with custom port
    pub fn with_port(port: u16) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", port),
            ..Default::default()
        }
    }
}

/// Service type as returned by controller
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    Orchestrator,
    Node,
    Gate,
    Tor,
    Vanguards,
    Unknown,
}

/// Service status as returned by controller
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Restarting,
}

/// Service snapshot from controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSnapshot {
    pub id: String,
    pub service_type: ServiceType,
    pub status: ServiceStatus,
    pub uptime_seconds: Option<u64>,
    pub restart_count: u32,
    pub last_health_check: Option<String>,
}

/// Controller health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerHealth {
    pub services: usize,
    pub running: usize,
    pub failed: usize,
}

/// Controller API client
pub struct ControllerClient {
    client: reqwest::Client,
    config: ControllerConfig,
}

impl ControllerClient {
    /// Create a new controller client
    pub fn new(config: ControllerConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { client, config })
    }

    /// Create with default config
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(ControllerConfig::default())
    }

    /// Get controller health status
    pub async fn get_health(&self) -> Result<ControllerHealth, String> {
        let url = format!("{}/health", self.config.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response
            .json::<ControllerHealth>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get all services
    pub async fn get_services(&self) -> Result<Vec<ServiceSnapshot>, String> {
        let url = format!("{}/services", self.config.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response
            .json::<Vec<ServiceSnapshot>>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get node services only
    pub async fn get_nodes(&self) -> Result<Vec<ServiceSnapshot>, String> {
        let url = format!("{}/nodes", self.config.base_url);
        
        #[derive(Deserialize)]
        struct NodesResponse {
            nodes: Vec<ServiceSnapshot>,
            #[allow(dead_code)]
            count: usize,
        }

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let nodes_response = response
            .json::<NodesResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(nodes_response.nodes)
    }

    /// Check if controller is reachable
    pub async fn is_reachable(&self) -> bool {
        self.get_health().await.is_ok()
    }

    /// Get running service count
    pub async fn running_service_count(&self) -> Result<usize, String> {
        let health = self.get_health().await?;
        Ok(health.running)
    }

    /// Get failed service count
    pub async fn failed_service_count(&self) -> Result<usize, String> {
        let health = self.get_health().await?;
        Ok(health.failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_config_defaults() {
        let config = ControllerConfig::default();
        assert_eq!(config.base_url, "http://127.0.0.1:9090");
    }

    #[test]
    fn test_controller_config_with_port() {
        let config = ControllerConfig::with_port(8080);
        assert_eq!(config.base_url, "http://127.0.0.1:8080");
    }
}
