//! Backend health checker module
//!
//! Pre-warms circuits by periodically testing backend connectivity via SOCKS proxy.
//! Starts with frequent checks (15s) and scales down to less frequent (60s) once reachable.

use anyhow::Result;
use reqwest::Client;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Backend health checker
pub struct HealthChecker {
    /// Backend .onion address
    backend_url: String,
    /// SOCKS proxy address
    socks_proxy: String,
    /// HTTP client with SOCKS proxy
    client: Client,
    /// Current check interval (in seconds)
    check_interval: u64,
    /// Whether backend is currently reachable
    is_reachable: bool,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(backend_url: String) -> Result<Self> {
        // Get SOCKS port from environment, default to 9050
        let socks_port = std::env::var("FORTIFY_SOCKS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9050);

        let socks_proxy = format!("socks5h://127.0.0.1:{}", socks_port);

        // Create HTTP client with SOCKS proxy
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(&socks_proxy)?)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            backend_url,
            socks_proxy,
            client,
            check_interval: 15, // Start with 15 second checks
            is_reachable: false,
        })
    }

    /// Start health checking loop
    pub async fn run(mut self) {
        info!("Backend health checker started for {}", self.backend_url);
        info!("Using SOCKS proxy: {}", self.socks_proxy);

        loop {
            let start = Instant::now();

            // Perform health check
            match self.check_backend().await {
                Ok(true) => {
                    let duration = start.elapsed();
                    if !self.is_reachable {
                        info!(
                            "Backend is now REACHABLE (took {}ms) - scaling down check frequency",
                            duration.as_millis()
                        );
                        self.is_reachable = true;
                        self.adjust_check_interval();
                    } else {
                        info!("Backend check: REACHABLE ({}ms)", duration.as_millis());
                    }
                }
                Ok(false) | Err(_) => {
                    let duration = start.elapsed();
                    if self.is_reachable {
                        warn!(
                            "Backend became UNREACHABLE ({}ms) - increasing check frequency",
                            duration.as_millis()
                        );
                        self.is_reachable = false;
                        self.check_interval = 15; // Reset to frequent checks
                    } else {
                        info!(
                            "Backend check: UNREACHABLE ({}ms) - circuits may still be building...",
                            duration.as_millis()
                        );
                    }
                }
            }

            // Wait before next check
            sleep(Duration::from_secs(self.check_interval)).await;
        }
    }

    /// Check if backend is reachable
    async fn check_backend(&self) -> Result<bool> {
        match self.client.get(&self.backend_url).send().await {
            Ok(response) => {
                let status = response.status();
                // ANY HTTP response means the backend is reachable
                // This includes 2xx, 3xx, 4xx, AND 5xx status codes
                // Only network-level failures indicate unreachability
                debug!("Backend responded with status: {}", status);
                Ok(true)
            }
            Err(e) => {
                // Connection failure means circuits not ready
                debug!("Backend check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Adjust check interval based on reachability
    fn adjust_check_interval(&mut self) {
        if self.is_reachable {
            // Scale down: 15s -> 30s -> 60s
            self.check_interval = match self.check_interval {
                15 => 30,
                30 => 60,
                _ => 60, // Stay at 60s once reached
            };
            info!("Check interval adjusted to {}s", self.check_interval);
        } else {
            // Scale up: reset to 15s when unreachable
            self.check_interval = 15;
        }
    }
}
