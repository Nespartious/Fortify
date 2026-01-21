use crate::{crypto::Seed, CommunityError};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Seed registry
pub struct SeedRegistry {
    seeds: HashMap<String, Seed>,
    max_seeds: usize,
    seed_ttl: Duration,
}

impl SeedRegistry {
    pub fn new(max_seeds: usize, seed_ttl: Duration) -> Self {
        Self {
            seeds: HashMap::new(),
            max_seeds,
            seed_ttl,
        }
    }

    /// Add a seed to the registry
    pub fn add(&mut self, seed: Seed) -> Result<(), CommunityError> {
        // Check if seed is expired
        if self.is_expired(&seed) {
            return Err(CommunityError::SeedExpired);
        }

        // Check capacity
        if self.seeds.len() >= self.max_seeds && !self.seeds.contains_key(&seed.onion_address) {
            // Remove oldest seed
            self.remove_oldest();
        }

        self.seeds.insert(seed.onion_address.clone(), seed);
        Ok(())
    }

    /// Get active (non-expired) seeds
    pub fn get_active(&self) -> Vec<Seed> {
        self.seeds
            .values()
            .filter(|s| !self.is_expired(s))
            .cloned()
            .collect()
    }

    /// Get seed by onion address
    pub fn get(&self, onion_address: &str) -> Option<&Seed> {
        self.seeds.get(onion_address)
    }

    /// Remove a seed
    pub fn remove(&mut self, onion_address: &str) -> Option<Seed> {
        self.seeds.remove(onion_address)
    }

    /// Clean up expired seeds
    pub fn cleanup_expired(&mut self) -> usize {
        let before_count = self.seeds.len();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ttl = self.seed_ttl.as_secs();

        self.seeds.retain(|_, seed| {
            let age = now.saturating_sub(seed.timestamp);
            age <= ttl
        });

        before_count - self.seeds.len()
    }

    /// Check if seed is expired
    fn is_expired(&self, seed: &Seed) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age = now.saturating_sub(seed.timestamp);
        age > self.seed_ttl.as_secs()
    }

    /// Remove oldest seed
    fn remove_oldest(&mut self) {
        if let Some(oldest_address) = self
            .seeds
            .iter()
            .min_by_key(|(_, seed)| seed.timestamp)
            .map(|(addr, _)| addr.clone())
        {
            self.seeds.remove(&oldest_address);
        }
    }

    /// Get total seed count
    pub fn total_count(&self) -> usize {
        self.seeds.len()
    }

    /// Get active seed count
    pub fn active_count(&self) -> usize {
        self.get_active().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    fn create_test_seed(onion_address: &str, age_seconds: u64) -> Seed {
        let keypair = KeyPair::generate();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(age_seconds);

        Seed {
            onion_address: onion_address.to_string(),
            public_key: keypair.public_key_bytes(),
            timestamp,
            gate_address: "http://127.0.0.1:9002".to_string(),
            signature: vec![0u8; 64],
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = SeedRegistry::new(100, Duration::from_secs(86400));

        assert_eq!(registry.total_count(), 0);
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_add_seed() {
        let mut registry = SeedRegistry::new(100, Duration::from_secs(86400));
        let seed = create_test_seed("test1.onion", 0);

        assert!(registry.add(seed).is_ok());
        assert_eq!(registry.total_count(), 1);
    }

    #[test]
    fn test_add_expired_seed() {
        let mut registry = SeedRegistry::new(100, Duration::from_secs(86400));
        let seed = create_test_seed("test1.onion", 86400 + 1); // Expired

        assert!(registry.add(seed).is_err());
        assert_eq!(registry.total_count(), 0);
    }

    #[test]
    fn test_get_active() {
        let mut registry = SeedRegistry::new(100, Duration::from_secs(86400));

        // Add fresh seed
        let seed1 = create_test_seed("test1.onion", 0);
        registry.add(seed1).unwrap();

        // Add expired seed (should be rejected)
        let seed2 = create_test_seed("test2.onion", 86400 + 1);
        let _ = registry.add(seed2);

        let active = registry.get_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].onion_address, "test1.onion");
    }

    #[test]
    fn test_cleanup_expired() {
        let mut registry = SeedRegistry::new(100, Duration::from_secs(100));

        // Add fresh seed
        let seed1 = create_test_seed("test1.onion", 0);
        registry.add(seed1).unwrap();

        // Manually add an old seed (bypass expiry check)
        let old_seed = create_test_seed("test2.onion", 200);
        registry
            .seeds
            .insert(old_seed.onion_address.clone(), old_seed);

        assert_eq!(registry.total_count(), 2);

        // Cleanup
        let removed = registry.cleanup_expired();
        assert_eq!(removed, 1);
        assert_eq!(registry.total_count(), 1);
    }

    #[test]
    fn test_max_capacity() {
        let mut registry = SeedRegistry::new(3, Duration::from_secs(86400));

        // Add 4 seeds (should evict oldest)
        for i in 0..4 {
            let seed = create_test_seed(&format!("test{}.onion", i), i * 10);
            registry.add(seed).unwrap();
        }

        // Should have exactly 3 seeds
        assert_eq!(registry.total_count(), 3);
    }

    #[test]
    fn test_get_seed() {
        let mut registry = SeedRegistry::new(100, Duration::from_secs(86400));
        let seed = create_test_seed("test1.onion", 0);

        registry.add(seed).unwrap();

        let retrieved = registry.get("test1.onion");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().onion_address, "test1.onion");
    }

    #[test]
    fn test_remove_seed() {
        let mut registry = SeedRegistry::new(100, Duration::from_secs(86400));
        let seed = create_test_seed("test1.onion", 0);

        registry.add(seed).unwrap();
        assert_eq!(registry.total_count(), 1);

        registry.remove("test1.onion");
        assert_eq!(registry.total_count(), 0);
    }
}
