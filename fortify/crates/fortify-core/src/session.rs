use crate::trust::{Session, SessionToken, TrustTier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// In-memory session manager
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    /// Reserved for future HMAC-based session signing
    #[allow(dead_code)]
    secret_key: Vec<u8>,
}

impl SessionManager {
    /// Create a new session manager with the given secret key
    pub fn new(secret_key: Vec<u8>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            secret_key,
        }
    }

    /// Create a new session with Unknown trust tier
    pub fn create_session(&self, session_id: String) -> Session {
        let token = SessionToken::new(session_id.clone(), TrustTier::Unknown, 3600, "unknown");
        let session = Session::new(token);

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id, session.clone());

        session
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Update a session
    pub fn update_session(&self, session: Session) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session.token.session_id.clone(), session);
    }

    /// Remove a session
    pub fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id);
    }

    /// Clean up expired and idle sessions
    pub fn cleanup(&self, idle_timeout: u64) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, session| !session.token.is_expired() && !session.is_idle(idle_timeout));
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.lock().unwrap();
        sessions.len()
    }

    /// Get session count by trust tier
    pub fn count_by_tier(&self) -> HashMap<TrustTier, usize> {
        let sessions = self.sessions.lock().unwrap();
        let mut counts = HashMap::new();

        for session in sessions.values() {
            *counts.entry(session.token.trust_tier).or_insert(0) += 1;
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manager_create() {
        let manager = SessionManager::new(b"test-secret".to_vec());
        let session = manager.create_session("test-123".into());

        assert_eq!(session.token.trust_tier, TrustTier::Unknown);
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_manager_update() {
        let manager = SessionManager::new(b"test-secret".to_vec());
        let mut session = manager.create_session("test-123".into());

        session.promote().unwrap();
        manager.update_session(session);

        let updated = manager.get_session("test-123").unwrap();
        assert_eq!(updated.token.trust_tier, TrustTier::Verified);
    }

    #[test]
    fn test_session_manager_remove() {
        let manager = SessionManager::new(b"test-secret".to_vec());
        manager.create_session("test-123".into());

        assert_eq!(manager.session_count(), 1);
        manager.remove_session("test-123");
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_count_by_tier() {
        let manager = SessionManager::new(b"test-secret".to_vec());

        manager.create_session("s1".into());

        let mut session2 = manager.create_session("s2".into());
        session2.promote().unwrap();
        manager.update_session(session2);

        let counts = manager.count_by_tier();
        assert_eq!(*counts.get(&TrustTier::Unknown).unwrap_or(&0), 1);
        assert_eq!(*counts.get(&TrustTier::Verified).unwrap_or(&0), 1);
    }
}

// ============================================================================
// Phase 4.5: Session Continuity
// ============================================================================

/// Session snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub trust_tier: String,
    pub request_count: u64,
    pub violation_count: u32,
    pub created_at: u64,
    pub last_activity: u64,
    /// Mirror this session was last seen on
    pub last_mirror: Option<String>,
}

impl From<&Session> for SessionSnapshot {
    fn from(session: &Session) -> Self {
        Self {
            session_id: session.token.session_id.clone(),
            trust_tier: format!("{:?}", session.token.trust_tier),
            request_count: session.request_count,
            violation_count: session.violation_count,
            created_at: session.token.issued_at,
            last_activity: session.last_activity,
            last_mirror: None,
        }
    }
}

/// Configuration for session persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPersistenceConfig {
    /// Enable session persistence
    pub enabled: bool,
    /// Path to session storage directory
    pub storage_path: PathBuf,
    /// How often to save sessions (seconds)
    pub save_interval_seconds: u64,
    /// How long to keep session history (days)
    pub history_retention_days: u64,
    /// Enable VM pause recovery
    pub vm_pause_recovery: bool,
    /// Maximum gap before session is considered abandoned (seconds)
    /// Default: 3600 (1 hour) - if > 1 hour between save and restore, warn
    pub max_pause_gap_seconds: u64,
}

impl Default for SessionPersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_path: PathBuf::from("/var/lib/fortify/sessions"),
            save_interval_seconds: 60, // Save every minute
            history_retention_days: 7,
            vm_pause_recovery: true,
            max_pause_gap_seconds: 3600,
        }
    }
}

/// Persistent session manager with file-based backup
/// Provides VM pause recovery and 7-day session history
pub struct PersistentSessionManager {
    /// Inner in-memory manager
    inner: SessionManager,
    /// Persistence configuration
    config: SessionPersistenceConfig,
    /// Last save timestamp
    last_save: Arc<Mutex<u64>>,
    /// Session history (session_id -> snapshots over time)
    history: Arc<Mutex<HashMap<String, Vec<SessionSnapshot>>>>,
}

impl PersistentSessionManager {
    /// Create a new persistent session manager
    pub fn new(secret_key: Vec<u8>, config: SessionPersistenceConfig) -> Self {
        let manager = Self {
            inner: SessionManager::new(secret_key),
            config,
            last_save: Arc::new(Mutex::new(0)),
            history: Arc::new(Mutex::new(HashMap::new())),
        };

        // Try to load existing sessions
        if manager.config.enabled {
            manager.load_sessions();
        }

        manager
    }

    /// Load sessions from disk (VM pause recovery)
    fn load_sessions(&self) {
        let snapshot_path = self.config.storage_path.join("sessions.json");
        
        if !snapshot_path.exists() {
            return;
        }

        match std::fs::read_to_string(&snapshot_path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<SessionSnapshot>>(&content) {
                    Ok(snapshots) => {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        
                        let mut restored = 0;
                        let mut expired = 0;
                        
                        for snapshot in snapshots {
                            // Check if session is too old (> 7 days)
                            let age_days = (now - snapshot.created_at) / 86400;
                            if age_days > self.config.history_retention_days {
                                expired += 1;
                                continue;
                            }
                            
                            // Check pause gap for VM recovery warning
                            let pause_gap = now - snapshot.last_activity;
                            if pause_gap > self.config.max_pause_gap_seconds {
                                tracing::warn!(
                                    "Session {} was paused for {} seconds (threshold: {})",
                                    snapshot.session_id, pause_gap, self.config.max_pause_gap_seconds
                                );
                            }
                            
                            // Restore session
                            let trust_tier = match snapshot.trust_tier.as_str() {
                                "Unknown" => TrustTier::Unknown,
                                "Verified" => TrustTier::Verified,
                                "Trusted" => TrustTier::Trusted,
                                "Suspicious" => TrustTier::Suspicious,
                                "Burned" => TrustTier::Burned,
                                _ => TrustTier::Unknown,
                            };
                            
                            let token = SessionToken {
                                session_id: snapshot.session_id.clone(),
                                trust_tier,
                                issued_at: snapshot.created_at,
                                expires_at: snapshot.last_activity + 86400, // Extend 1 day
                                user_agent_hash: String::from("unknown"), // Restored sessions don't have UA binding
                                signature: Vec::new(), // Will be re-signed if needed
                            };
                            
                            let session = Session {
                                token,
                                request_count: snapshot.request_count,
                                violation_count: snapshot.violation_count,
                                last_activity: snapshot.last_activity,
                            };
                            
                            self.inner.update_session(session);
                            restored += 1;
                        }
                        
                        tracing::info!(
                            "Session recovery: {} restored, {} expired",
                            restored, expired
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse session snapshot: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read session snapshot: {}", e);
            }
        }
    }

    /// Save sessions to disk
    pub fn save_sessions(&self) -> Result<(), std::io::Error> {
        if !self.config.enabled {
            return Ok(());
        }

        // Create directory if needed
        std::fs::create_dir_all(&self.config.storage_path)?;

        let sessions = self.inner.sessions.lock().unwrap();
        let snapshots: Vec<SessionSnapshot> = sessions
            .values()
            .map(SessionSnapshot::from)
            .collect();

        let content = serde_json::to_string_pretty(&snapshots)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let snapshot_path = self.config.storage_path.join("sessions.json");
        let temp_path = self.config.storage_path.join("sessions.json.tmp");

        // Atomic write: write to temp file, then rename
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &snapshot_path)?;

        // Update last save time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        *self.last_save.lock().unwrap() = now;

        Ok(())
    }

    /// Check if it's time to save
    pub fn should_save(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last = *self.last_save.lock().unwrap();
        now - last >= self.config.save_interval_seconds
    }

    /// Periodic save (call from background task)
    pub fn periodic_save(&self) {
        if self.should_save() {
            if let Err(e) = self.save_sessions() {
                tracing::error!("Failed to save sessions: {}", e);
            }
        }
    }

    /// Record session history snapshot
    pub fn record_history(&self, session_id: &str, mirror_id: Option<String>) {
        let session = match self.inner.get_session(session_id) {
            Some(s) => s,
            None => return,
        };

        let mut snapshot = SessionSnapshot::from(&session);
        snapshot.last_mirror = mirror_id;

        let mut history = self.history.lock().unwrap();
        history
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(snapshot);

        // Keep only last 100 snapshots per session
        if let Some(snapshots) = history.get_mut(session_id) {
            while snapshots.len() > 100 {
                snapshots.remove(0);
            }
        }
    }

    /// Get session history
    pub fn get_history(&self, session_id: &str) -> Vec<SessionSnapshot> {
        let history = self.history.lock().unwrap();
        history.get(session_id).cloned().unwrap_or_default()
    }

    /// Clean old history
    pub fn cleanup_history(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let retention_secs = self.config.history_retention_days * 86400;

        let mut history = self.history.lock().unwrap();
        for snapshots in history.values_mut() {
            snapshots.retain(|s| now - s.last_activity < retention_secs);
        }
        // Remove empty entries
        history.retain(|_, v| !v.is_empty());
    }

    // Delegate to inner SessionManager
    pub fn create_session(&self, session_id: String) -> Session {
        self.inner.create_session(session_id)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.inner.get_session(session_id)
    }

    pub fn update_session(&self, session: Session) {
        self.inner.update_session(session)
    }

    pub fn remove_session(&self, session_id: &str) {
        self.inner.remove_session(session_id)
    }

    pub fn cleanup(&self, idle_timeout: u64) {
        self.inner.cleanup(idle_timeout)
    }

    pub fn session_count(&self) -> usize {
        self.inner.session_count()
    }

    pub fn count_by_tier(&self) -> HashMap<TrustTier, usize> {
        self.inner.count_by_tier()
    }
}
