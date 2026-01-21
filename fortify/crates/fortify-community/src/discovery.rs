use crate::{crypto::Seed, CommunityError};
use hyper::{Body, Client, Request, Uri};
use std::collections::HashMap;
use std::time::Instant;

/// Peer discovery manager
pub struct PeerDiscovery {
    /// Reserved for future multi-hop peer discovery
    #[allow(dead_code)]
    max_hops: usize,
    rate_limiter: RateLimiter,
    client: Client<hyper::client::HttpConnector>,
}

impl PeerDiscovery {
    pub fn new(max_hops: usize, requests_per_minute: usize) -> Self {
        Self {
            max_hops,
            rate_limiter: RateLimiter::new(requests_per_minute),
            client: Client::new(),
        }
    }

    /// Discover peers from known seeds
    pub async fn discover_from_seeds(
        &mut self,
        seeds: Vec<Seed>,
        max_results: usize,
    ) -> Result<Vec<Seed>, CommunityError> {
        let mut discovered = Vec::new();

        for seed in seeds.iter().take(5) {
            // Rate limit
            if !self.rate_limiter.allow() {
                break;
            }

            // Query seed for more peers
            match self.query_peer(&seed.gate_address).await {
                Ok(peers) => {
                    for peer in peers {
                        if discovered.len() >= max_results {
                            break;
                        }
                        discovered.push(peer);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to query {}: {}", seed.onion_address, e);
                }
            }

            if discovered.len() >= max_results {
                break;
            }
        }

        Ok(discovered)
    }

    /// Query a peer for its seed list
    async fn query_peer(&self, gate_address: &str) -> Result<Vec<Seed>, CommunityError> {
        let uri: Uri = format!("{}/community/seeds", gate_address)
            .parse()
            .map_err(|e| CommunityError::DiscoveryError(format!("Invalid URI: {}", e)))?;

        let req = Request::builder()
            .uri(uri)
            .body(Body::empty())
            .map_err(|e| CommunityError::DiscoveryError(format!("Request build failed: {}", e)))?;

        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| CommunityError::DiscoveryError(format!("Request failed: {}", e)))?;

        let body_bytes = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| CommunityError::DiscoveryError(format!("Body read failed: {}", e)))?;

        let seeds: Vec<Seed> = serde_json::from_slice(&body_bytes)
            .map_err(|e| CommunityError::DiscoveryError(format!("JSON parse failed: {}", e)))?;

        Ok(seeds)
    }
}

/// Simple rate limiter
struct RateLimiter {
    requests_per_minute: usize,
    requests: HashMap<u64, usize>,
}

impl RateLimiter {
    fn new(requests_per_minute: usize) -> Self {
        Self {
            requests_per_minute,
            requests: HashMap::new(),
        }
    }

    /// Check if request is allowed
    fn allow(&mut self) -> bool {
        let now = Instant::now().elapsed().as_secs() / 60;

        let count = self.requests.entry(now).or_insert(0);

        if *count >= self.requests_per_minute {
            return false;
        }

        *count += 1;

        // Clean old entries
        self.requests.retain(|&minute, _| minute >= now - 1);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_discovery_creation() {
        let discovery = PeerDiscovery::new(3, 10);

        assert_eq!(discovery.max_hops, 3);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5);

        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.allow());
        }

        // 6th request should be denied
        assert!(!limiter.allow());
    }

    #[tokio::test]
    async fn test_discover_from_empty_seeds() {
        let mut discovery = PeerDiscovery::new(3, 10);

        let result = discovery.discover_from_seeds(vec![], 10).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
