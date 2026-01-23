//! Status polling module
//!
//! Polls the orchestrator API for real-time system status updates.
//! Provides non-blocking background polling with configurable intervals.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

/// System status from orchestrator
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemStatus {
    /// Whether the system is healthy
    pub healthy: bool,
    /// Number of active mirrors
    pub active_mirrors: usize,
    /// Number of standby mirrors
    pub standby_mirrors: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Number of threat nodes
    pub threat_nodes: usize,
    /// Total active sessions
    pub active_sessions: usize,
    /// Requests in last minute
    pub requests_per_minute: u64,
    /// Current security level
    pub security_level: String,
    /// Last updated timestamp
    pub last_updated: u64,
    /// Controller running
    pub controller_running: bool,
    /// Orchestrator running
    pub orchestrator_running: bool,
    /// Gate running
    pub gate_running: bool,
    /// Error message if any
    pub error: Option<String>,
}

/// Mirror status from orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorStatus {
    pub id: String,
    pub onion_address: String,
    pub state: String, // "Active", "Standby", "Burned", "Paused"
    pub age_hours: f64,
    pub request_count: u64,
    pub pow_enabled: bool,
}

/// Node status from orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub id: String,
    pub node_type: String, // "Healthy", "Threat"
    pub address: String,
    pub active_connections: u32,
    pub status: String, // "Online", "Offline", "Overloaded"
}

/// Full status response from orchestrator
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrchestratorStatusResponse {
    pub system: SystemStatus,
    #[serde(default)]
    pub mirrors: Vec<MirrorStatus>,
    #[serde(default)]
    pub nodes: Vec<NodeStatus>,
}

/// Status update message for UI
#[derive(Debug, Clone)]
pub enum StatusMessage {
    /// Full status update
    Update(OrchestratorStatusResponse),
    /// Connection to orchestrator failed
    ConnectionFailed(String),
    /// Polling stopped
    Stopped,
}

/// Configuration for status polling
#[derive(Debug, Clone)]
pub struct StatusPollerConfig {
    /// Orchestrator base URL
    pub orchestrator_url: String,
    /// Admin authentication token
    pub auth_token: String,
    /// Polling interval
    pub poll_interval: Duration,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for StatusPollerConfig {
    fn default() -> Self {
        Self {
            orchestrator_url: "http://127.0.0.1:8082".to_string(),
            auth_token: String::new(),
            poll_interval: Duration::from_secs(5),
            timeout: Duration::from_secs(10),
        }
    }
}

/// Status poller that runs in the background
pub struct StatusPoller {
    config: StatusPollerConfig,
    /// Channel to send status updates
    tx: mpsc::Sender<StatusMessage>,
    /// Shutdown signal
    shutdown: Arc<RwLock<bool>>,
}

impl StatusPoller {
    /// Create a new status poller
    pub fn new(config: StatusPollerConfig, tx: mpsc::Sender<StatusMessage>) -> Self {
        Self {
            config,
            tx,
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Start polling in the background
    pub fn start(self) -> StatusPollerHandle {
        let shutdown = self.shutdown.clone();
        let handle = tokio::spawn(async move {
            self.poll_loop().await;
        });

        StatusPollerHandle { shutdown, handle }
    }

    /// Main polling loop
    async fn poll_loop(self) {
        let client = match reqwest::Client::builder()
            .timeout(self.config.timeout)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = self
                    .tx
                    .send(StatusMessage::ConnectionFailed(format!(
                        "Failed to create HTTP client: {}",
                        e
                    )))
                    .await;
                return;
            }
        };

        loop {
            // Check for shutdown
            if *self.shutdown.read().await {
                let _ = self.tx.send(StatusMessage::Stopped).await;
                break;
            }

            // Poll status
            match self.fetch_status(&client).await {
                Ok(status) => {
                    if self.tx.send(StatusMessage::Update(status)).await.is_err() {
                        // Receiver dropped, stop polling
                        break;
                    }
                }
                Err(e) => {
                    if self
                        .tx
                        .send(StatusMessage::ConnectionFailed(e))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }

            // Wait for next poll
            tokio::time::sleep(self.config.poll_interval).await;
        }
    }

    /// Fetch status from orchestrator
    async fn fetch_status(
        &self,
        client: &reqwest::Client,
    ) -> Result<OrchestratorStatusResponse, String> {
        let url = format!("{}/status", self.config.orchestrator_url);

        let mut request = client.get(&url);
        if !self.config.auth_token.is_empty() {
            request = request.header("X-Fortify-Admin-Token", &self.config.auth_token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response
            .json::<OrchestratorStatusResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

/// Handle to control the background poller
pub struct StatusPollerHandle {
    shutdown: Arc<RwLock<bool>>,
    handle: tokio::task::JoinHandle<()>,
}

impl StatusPollerHandle {
    /// Stop the poller
    pub async fn stop(self) {
        *self.shutdown.write().await = true;
        let _ = self.handle.await;
    }

    /// Check if the poller is still running
    pub fn is_running(&self) -> bool {
        !self.handle.is_finished()
    }
}

/// Create a status polling channel and start polling
pub fn start_status_polling(
    orchestrator_url: &str,
    auth_token: &str,
    poll_interval_secs: u64,
) -> (mpsc::Receiver<StatusMessage>, StatusPollerHandle) {
    let (tx, rx) = mpsc::channel(32);

    let config = StatusPollerConfig {
        orchestrator_url: orchestrator_url.to_string(),
        auth_token: auth_token.to_string(),
        poll_interval: Duration::from_secs(poll_interval_secs),
        ..Default::default()
    };

    let poller = StatusPoller::new(config, tx);
    let handle = poller.start();

    (rx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_poller_config_defaults() {
        let config = StatusPollerConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_system_status_defaults() {
        let status = SystemStatus::default();
        assert!(!status.healthy);
        assert_eq!(status.active_mirrors, 0);
    }
}
