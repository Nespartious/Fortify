pub mod behavioral;
pub mod branding;
pub mod config;
pub mod logging;
pub mod session;
pub mod templates;
pub mod trust;

pub use behavioral::*;
pub use branding::{html_escape, is_valid_hex_color, render_html_template, TemplateBranding};
pub use config::*;
pub use logging::*;
pub use session::*;
pub use templates::{BrandingVars, PrerenderedCaptchaPage, TemplateEngine, TemplateType};
// Export trust types explicitly to avoid ambiguous Result re-export
pub use trust::{Session, SessionToken, TrustError, TrustTier};

// ============================================================================
// Safe Lock Access Utilities
// ============================================================================
// These helpers recover from poisoned locks instead of panicking,
// preventing cascading failures from a single thread panic.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Safely acquire a Mutex lock, recovering from poisoning
/// 
/// If the lock is poisoned (a thread panicked while holding it),
/// we recover the data rather than propagating the panic.
/// This is acceptable for Fortify because:
/// 1. Partial state is better than full service failure
/// 2. Most state is reconstructable from external sources
/// 3. A DoS via panic is worse than stale data
#[inline]
pub fn safe_lock<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| {
        tracing::warn!("Recovered from poisoned Mutex lock");
        poisoned.into_inner()
    })
}

/// Safely acquire a RwLock read lock, recovering from poisoning
#[inline]
pub fn safe_read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| {
        tracing::warn!("Recovered from poisoned RwLock (read)");
        poisoned.into_inner()
    })
}

/// Safely acquire a RwLock write lock, recovering from poisoning
#[inline]
pub fn safe_write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| {
        tracing::warn!("Recovered from poisoned RwLock (write)");
        poisoned.into_inner()
    })
}

// ============================================================================
// Timeout Jitter Utilities
// ============================================================================
// These helpers add ±15% random variation to timeouts to prevent
// timing-based fingerprinting attacks.

use std::time::Duration;

/// Apply ±15% jitter to a timeout in seconds
/// 
/// This prevents attackers from fingerprinting the service by observing
/// consistent timeout values. Returns a Duration with random variation.
/// 
/// # Arguments
/// * `base_secs` - The base timeout in seconds
/// 
/// # Returns
/// A Duration with ±15% random variation, minimum 1 second
pub fn jittered_timeout(base_secs: u64) -> Duration {
    use rand::Rng;
    let mut rng = rand::rng();
    let jitter_range = ((base_secs as f64) * 0.15).max(1.0) as u64;
    let jitter = rng.random_range(0..=(jitter_range * 2));
    let jittered = base_secs.saturating_sub(jitter_range).saturating_add(jitter);
    Duration::from_secs(jittered.max(1))
}

/// Apply ±15% jitter to an existing Duration
/// 
/// Convenience wrapper for when you already have a Duration.
pub fn jittered_duration(base: Duration) -> Duration {
    jittered_timeout(base.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jittered_timeout_stays_within_bounds() {
        // Run multiple times to verify randomness stays within ±15%
        for _ in 0..100 {
            let result = jittered_timeout(100);
            let secs = result.as_secs();
            assert!(secs >= 85, "Jitter went below -15%: {}", secs);
            assert!(secs <= 115, "Jitter went above +15%: {}", secs);
        }
    }

    #[test]
    fn test_jittered_timeout_minimum_one_second() {
        // Even with very small base, should return at least 1 second
        for _ in 0..100 {
            let result = jittered_timeout(1);
            assert!(result.as_secs() >= 1, "Jitter returned less than 1 second");
        }
    }

    #[test]
    fn test_jittered_timeout_produces_variation() {
        // Collect 50 samples and verify they're not all identical
        let samples: Vec<u64> = (0..50)
            .map(|_| jittered_timeout(100).as_secs())
            .collect();
        let first = samples[0];
        let has_variation = samples.iter().any(|&s| s != first);
        assert!(has_variation, "Jitter should produce different values");
    }
}
