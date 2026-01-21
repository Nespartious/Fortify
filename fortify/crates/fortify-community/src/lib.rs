pub mod crypto;
pub mod discovery;
pub mod registry;
pub mod server;

use crypto::{KeyPair, Seed};
use discovery::PeerDiscovery;
use registry::SeedRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum CommunityError {
    #[error("Signature verification failed")]
    InvalidSignature,
    #[error("Seed expired")]
    SeedExpired,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid seed format: {0}")]
    InvalidSeed(String),
    #[error("Discovery error: {0}")]
    DiscoveryError(String),
}

/// Community network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityConfig {
    pub enabled: bool,
    pub bind_addr: String,
    pub max_seeds: usize,
    pub seed_ttl: Duration,
    pub discovery_enabled: bool,
    pub max_discovery_hops: usize,
    pub share_rate_limit: usize, // requests per minute
}

impl Default for CommunityConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in
            bind_addr: "127.0.0.1:9005".to_string(),
            max_seeds: 100,
            seed_ttl: Duration::from_secs(86400 * 7), // 7 days
            discovery_enabled: true,
            max_discovery_hops: 3,
            share_rate_limit: 10, // 10 req/min
        }
    }
}

/// Community network metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommunityMetrics {
    pub seeds_total: usize,
    pub seeds_active: usize,
    pub seeds_expired: usize,
    pub discoveries_performed: usize,
    pub peers_discovered: usize,
    pub signatures_verified: usize,
    pub signatures_failed: usize,
}

/// Community network manager
pub struct CommunityNetwork {
    config: CommunityConfig,
    keypair: KeyPair,
    registry: Arc<Mutex<SeedRegistry>>,
    discovery: Arc<Mutex<PeerDiscovery>>,
    metrics: Arc<Mutex<CommunityMetrics>>,
}

impl CommunityNetwork {
    pub fn new(config: CommunityConfig, keypair: KeyPair) -> Self {
        Self {
            config: config.clone(),
            keypair,
            registry: Arc::new(Mutex::new(SeedRegistry::new(
                config.max_seeds,
                config.seed_ttl,
            ))),
            discovery: Arc::new(Mutex::new(PeerDiscovery::new(
                config.max_discovery_hops,
                config.share_rate_limit,
            ))),
            metrics: Arc::new(Mutex::new(CommunityMetrics::default())),
        }
    }

    /// Start the community network
    pub async fn start(&self) -> Result<(), CommunityError> {
        if !self.config.enabled {
            tracing::info!("Community network disabled");
            return Ok(());
        }

        tracing::info!("Starting community network on {}", self.config.bind_addr);

        // Start cleanup task
        self.start_cleanup_task().await;

        Ok(())
    }

    /// Add a seed to the registry
    pub async fn add_seed(&self, seed: Seed) -> Result<(), CommunityError> {
        // Verify signature
        if !self.verify_seed(&seed) {
            let mut metrics = self.metrics.lock().await;
            metrics.signatures_failed += 1;
            return Err(CommunityError::InvalidSignature);
        }

        let mut metrics = self.metrics.lock().await;
        metrics.signatures_verified += 1;

        let mut registry = self.registry.lock().await;
        registry.add(seed)?;

        metrics.seeds_total = registry.total_count();
        metrics.seeds_active = registry.active_count();

        Ok(())
    }

    /// Get active seeds
    pub async fn get_seeds(&self) -> Vec<Seed> {
        let registry = self.registry.lock().await;
        registry.get_active()
    }

    /// Discover peers from known seeds
    pub async fn discover_peers(&self, max_results: usize) -> Result<Vec<Seed>, CommunityError> {
        if !self.config.discovery_enabled {
            return Ok(Vec::new());
        }

        let registry = self.registry.lock().await;
        let seeds = registry.get_active();
        drop(registry);

        let mut discovery = self.discovery.lock().await;
        let discovered = discovery.discover_from_seeds(seeds, max_results).await?;

        let mut metrics = self.metrics.lock().await;
        metrics.discoveries_performed += 1;
        metrics.peers_discovered += discovered.len();

        Ok(discovered)
    }

    /// Verify seed signature
    fn verify_seed(&self, seed: &Seed) -> bool {
        crypto::verify_seed_signature(seed)
    }

    /// Sign a seed with our keypair
    pub fn sign_seed(&self, seed: &mut Seed) {
        crypto::sign_seed(&self.keypair, seed);
    }

    /// Start cleanup task
    async fn start_cleanup_task(&self) {
        let registry = Arc::clone(&self.registry);
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;

                let mut reg = registry.lock().await;
                let removed = reg.cleanup_expired();

                let mut m = metrics.lock().await;
                m.seeds_expired += removed;
                m.seeds_total = reg.total_count();
                m.seeds_active = reg.active_count();

                tracing::info!("Cleaned up {} expired seeds", removed);
            }
        });
    }

    /// Get metrics
    pub async fn get_metrics(&self) -> CommunityMetrics {
        self.metrics.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_config_default() {
        let config = CommunityConfig::default();

        assert!(!config.enabled); // Opt-in
        assert_eq!(config.max_seeds, 100);
        assert_eq!(config.max_discovery_hops, 3);
    }

    #[tokio::test]
    async fn test_community_network_creation() {
        let config = CommunityConfig::default();
        let keypair = KeyPair::generate();
        let network = CommunityNetwork::new(config, keypair);

        let metrics = network.get_metrics().await;
        assert_eq!(metrics.seeds_total, 0);
    }

    #[tokio::test]
    async fn test_disabled_network() {
        let config = CommunityConfig::default(); // disabled by default
        let keypair = KeyPair::generate();
        let network = CommunityNetwork::new(config, keypair);

        // Should succeed without error
        assert!(network.start().await.is_ok());
    }
}
