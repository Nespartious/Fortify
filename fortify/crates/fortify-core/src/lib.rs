pub mod behavioral;
pub mod branding;
pub mod config;
pub mod logging;
pub mod session;
pub mod trust;

pub use behavioral::*;
pub use branding::{html_escape, is_valid_hex_color, render_html_template, TemplateBranding};
pub use config::*;
pub use logging::*;
pub use session::*;
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
