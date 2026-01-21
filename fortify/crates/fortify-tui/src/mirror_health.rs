///! Mirror health checking and Tor connectivity verification

use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn, info};

/// Mirror health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorHealth {
    /// Haven't checked yet
    Unknown,
    /// Checking now
    Checking,
    /// Reachable via Tor
    Healthy,
    /// Cannot connect
    Unreachable,
    /// Error during check
    Error,
}

/// Result of a mirror health check
#[derive(Debug, Clone)]
pub struct MirrorHealthResult {
    pub address: String,
    pub health: MirrorHealth,
    pub last_checked: Instant,
    pub check_duration_ms: u64,
    pub error_message: Option<String>,
}

/// Mirror health checker - verifies onion addresses are reachable via Tor
pub struct MirrorHealthChecker {
    /// SOCKS proxy for Tor (e.g., "127.0.0.1:9150")
    socks_proxy: String,
    /// Timeout for connectivity checks
    check_timeout: Duration,
}

impl MirrorHealthChecker {
    pub fn new(socks_proxy: String) -> Self {
        Self {
            socks_proxy,
            check_timeout: Duration::from_secs(30),
        }
    }

    /// Check if a mirror is reachable via Tor
    pub async fn check_mirror(&self, onion_address: &str) -> MirrorHealthResult {
        let start = Instant::now();
        
        // Normalize address
        let address = if onion_address.ends_with(".onion") {
            onion_address.to_string()
        } else {
            format!("{}.onion", onion_address)
        };
        
        info!("Checking mirror health: {}", address);
        
        // Try to connect via curl through Tor SOCKS proxy
        let url = format!("http://{}/health", address);
        let result = self.check_via_curl(&url).await;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(true) => {
                info!("Mirror {} is HEALTHY ({}ms)", address, duration_ms);
                MirrorHealthResult {
                    address,
                    health: MirrorHealth::Healthy,
                    last_checked: Instant::now(),
                    check_duration_ms: duration_ms,
                    error_message: None,
                }
            }
            Ok(false) => {
                warn!("Mirror {} is UNREACHABLE ({}ms)", address, duration_ms);
                MirrorHealthResult {
                    address,
                    health: MirrorHealth::Unreachable,
                    last_checked: Instant::now(),
                    check_duration_ms: duration_ms,
                    error_message: Some("Connection failed or returned error".to_string()),
                }
            }
            Err(e) => {
                warn!("Mirror {} check ERROR: {} ({}ms)", address, e, duration_ms);
                MirrorHealthResult {
                    address,
                    health: MirrorHealth::Error,
                    last_checked: Instant::now(),
                    check_duration_ms: duration_ms,
                    error_message: Some(e.to_string()),
                }
            }
        }
    }
    
    /// Check multiple mirrors in parallel
    pub async fn check_mirrors(&self, addresses: &[String]) -> Vec<MirrorHealthResult> {
        let mut handles = vec![];
        
        for address in addresses {
            let checker = self.clone();
            let addr = address.clone();
            handles.push(tokio::spawn(async move {
                checker.check_mirror(&addr).await
            }));
        }
        
        let mut results = vec![];
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        
        results
    }
    
    /// Use curl with Tor SOCKS proxy to check connectivity
    async fn check_via_curl(&self, url: &str) -> Result<bool> {
        let output = Command::new("curl")
            .args(&[
                "-s",  // Silent
                "-f",  // Fail on HTTP errors
                "--socks5-hostname", &self.socks_proxy,  // Use Tor SOCKS proxy with remote DNS
                "--max-time", &format!("{}", self.check_timeout.as_secs()),
                "--connect-timeout", "10",
                url,
            ])
            .output()
            .await?;
        
        // Consider it healthy if curl succeeded (exit code 0)
        Ok(output.status.success())
    }
}

impl Clone for MirrorHealthChecker {
    fn clone(&self) -> Self {
        Self {
            socks_proxy: self.socks_proxy.clone(),
            check_timeout: self.check_timeout,
        }
    }
}

/// Manager for tracking mirror health over time
pub struct MirrorHealthTracker {
    checker: MirrorHealthChecker,
    results: std::sync::Arc<tokio::sync::Mutex<Vec<MirrorHealthResult>>>,
}

impl MirrorHealthTracker {
    pub fn new(socks_proxy: String) -> Self {
        Self {
            checker: MirrorHealthChecker::new(socks_proxy),
            results: std::sync::Arc::new(tokio::sync::Mutex::new(vec![])),
        }
    }
    
    /// Start monitoring mirrors with adaptive check frequency
    /// - Every 5 seconds for first 2 minutes (initial deployment)
    /// - Every 30 seconds for next 3 minutes (stabilization)
    /// - Every 60 seconds thereafter (steady state)
    pub fn start_monitoring(
        &self,
        addresses: Vec<String>,
        deployment_start: Instant,
    ) -> tokio::task::JoinHandle<()> {
        let checker = self.checker.clone();
        let results = self.results.clone();
        
        tokio::spawn(async move {
            loop {
                // Determine check interval based on deployment age
                let age = deployment_start.elapsed();
                let interval = if age < Duration::from_secs(120) {
                    // First 2 minutes: check every 5 seconds
                    Duration::from_secs(5)
                } else if age < Duration::from_secs(300) {
                    // Next 3 minutes: check every 30 seconds
                    Duration::from_secs(30)
                } else {
                    // After 5 minutes: check every 60 seconds
                    Duration::from_secs(60)
                };
                
                debug!("Checking {} mirrors (age: {:?}, interval: {:?})", addresses.len(), age, interval);
                
                // Check all mirrors
                let check_results = checker.check_mirrors(&addresses).await;
                
                // Update stored results
                let mut results_lock = results.lock().await;
                *results_lock = check_results;
                drop(results_lock);
                
                // Wait until next check
                tokio::time::sleep(interval).await;
            }
        })
    }
    
    /// Get latest health results
    pub async fn get_results(&self) -> Vec<MirrorHealthResult> {
        self.results.lock().await.clone()
    }
    
    /// Get health summary
    pub async fn get_summary(&self) -> MirrorHealthSummary {
        let results = self.results.lock().await;
        
        let total = results.len();
        let healthy = results.iter().filter(|r| r.health == MirrorHealth::Healthy).count();
        let checking = results.iter().filter(|r| r.health == MirrorHealth::Checking).count();
        let unreachable = results.iter().filter(|r| r.health == MirrorHealth::Unreachable).count();
        let errors = results.iter().filter(|r| r.health == MirrorHealth::Error).count();
        
        MirrorHealthSummary {
            total,
            healthy,
            checking,
            unreachable,
            errors,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MirrorHealthSummary {
    pub total: usize,
    pub healthy: usize,
    pub checking: usize,
    pub unreachable: usize,
    pub errors: usize,
}

impl MirrorHealthSummary {
    pub fn all_healthy(&self) -> bool {
        self.total > 0 && self.healthy == self.total
    }
    
    pub fn any_healthy(&self) -> bool {
        self.healthy > 0
    }
    
    pub fn health_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.healthy as f64 / self.total as f64) * 100.0
        }
    }
}
