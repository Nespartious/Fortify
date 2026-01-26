//! Pre-rendered CAPTCHA Page Pool
//!
//! This module provides a pool of pre-generated HTML CAPTCHA pages that can be
//! served instantly without runtime generation. This is critical for surviving
//! DDoS attacks where on-demand generation would fail.
//!
//! ## Architecture
//!
//! - **Pool**: Stores complete HTML pages with embedded CAPTCHA challenges
//! - **State Tracking**: Available → InUse → Solved lifecycle
//! - **Lazy Registration**: Session created on verification, not on page serve
//! - **Traffic-Aware**: Generation rate adjusts based on load

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bitmap;
use crate::CaptchaDifficulty;

/// Configuration for the pre-rendered page pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Target number of pages to maintain in pool
    pub target_size: usize,
    /// Timeout for InUse pages before returning to Available (seconds)
    pub timeout_seconds: u64,
    /// Maximum age of a page before regeneration (seconds)
    pub max_age_seconds: u64,
    /// Whether to fill pool on startup
    pub initial_fill: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            target_size: 500,
            timeout_seconds: 120,
            max_age_seconds: 600,
            initial_fill: true,
        }
    }
}

/// State of a pooled page
#[derive(Debug, Clone)]
pub enum PageState {
    /// Ready to be served to a user
    Available,
    /// Currently being used by a user
    InUse { session_id: String, served_at: u64 },
    /// Solved correctly, needs regeneration
    Solved,
}

/// A pre-rendered CAPTCHA page ready to serve
#[derive(Debug, Clone)]
pub struct PooledPage {
    /// Unique identifier for this CAPTCHA
    pub captcha_id: String,
    /// Expected answer for verification
    pub answer: String,
    /// Complete HTML page (with {{SESSION_ID}} placeholder)
    pub html: String,
    /// Current state of the page
    pub state: PageState,
    /// When the page was generated
    pub generated_at: u64,
    /// CAPTCHA difficulty level
    pub difficulty: CaptchaDifficulty,
}

impl PooledPage {
    /// Check if this page has expired based on max age
    pub fn is_expired(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.generated_at) > max_age_seconds
    }

    /// Check if InUse timeout has expired
    pub fn is_timeout_expired(&self, timeout_seconds: u64) -> bool {
        if let PageState::InUse { served_at, .. } = self.state {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            (now - served_at) > timeout_seconds
        } else {
            false
        }
    }
}

/// Metrics for monitoring pool health
#[derive(Debug, Default)]
pub struct PoolMetrics {
    /// Total pages served from pool
    pub pages_served: AtomicU64,
    /// Total pages generated
    pub pages_generated: AtomicU64,
    /// Total pages reclaimed from timeout
    pub pages_reclaimed: AtomicU64,
    /// Total pages solved and regenerated
    pub pages_solved: AtomicU64,
    /// Current available count
    pub available_count: AtomicUsize,
    /// Current in-use count
    pub in_use_count: AtomicUsize,
    /// Requests in last minute (for rate calculation)
    pub requests_last_minute: AtomicU64,
    /// Last metrics update timestamp
    pub last_update: AtomicU64,
}

/// Pre-rendered CAPTCHA page pool
///
/// Maintains a pool of ready-to-serve HTML pages with embedded CAPTCHA challenges.
/// Pages are served instantly and sessions are registered lazily on verification.
pub struct PrerenderedPagePool {
    /// All pages in the pool
    pages: Arc<Mutex<Vec<PooledPage>>>,
    /// Mapping of captcha_id → answer for lazy verification
    answers: Arc<Mutex<HashMap<String, String>>>,
    /// Mapping of session_id → captcha_id for verification lookup
    session_to_captcha: Arc<Mutex<HashMap<String, String>>>,
    /// Pool configuration
    pub config: PoolConfig,
    /// Pool metrics
    pub metrics: PoolMetrics,
    /// Template engine for rendering pages
    template_engine: fortify_core::TemplateEngine,
    /// Branding configuration
    branding: Arc<Mutex<fortify_core::BrandingVars>>,
}

impl PrerenderedPagePool {
    /// Create a new pre-rendered page pool
    pub fn new(config: PoolConfig, branding: fortify_core::BrandingVars) -> Self {
        Self {
            pages: Arc::new(Mutex::new(Vec::with_capacity(config.target_size))),
            answers: Arc::new(Mutex::new(HashMap::new())),
            session_to_captcha: Arc::new(Mutex::new(HashMap::new())),
            config,
            metrics: PoolMetrics::default(),
            template_engine: fortify_core::TemplateEngine::new(),
            branding: Arc::new(Mutex::new(branding)),
        }
    }

    /// Get the number of available pages
    pub fn available_count(&self) -> usize {
        let pages = self.pages.lock().unwrap();
        pages
            .iter()
            .filter(|p| matches!(p.state, PageState::Available))
            .count()
    }

    /// Get the number of in-use pages
    pub fn in_use_count(&self) -> usize {
        let pages = self.pages.lock().unwrap();
        pages
            .iter()
            .filter(|p| matches!(p.state, PageState::InUse { .. }))
            .count()
    }

    /// Get total pool size
    pub fn total_size(&self) -> usize {
        self.pages.lock().unwrap().len()
    }

    /// Take an available page from the pool
    ///
    /// Returns the page with {{SESSION_ID}} placeholder still in HTML.
    /// Caller must inject the session_id and call `mark_in_use()`.
    pub fn take_available(&self) -> Option<PooledPage> {
        let mut pages = self.pages.lock().unwrap();

        // Find first available, non-expired page
        let idx = pages.iter().position(|p| {
            matches!(p.state, PageState::Available) && !p.is_expired(self.config.max_age_seconds)
        })?;

        // Remove from pool (will be re-added after use or regenerated)
        let page = pages.remove(idx);

        self.metrics.pages_served.fetch_add(1, Ordering::Relaxed);

        Some(page)
    }

    /// Mark a page as in-use by a specific session
    ///
    /// This registers the session → captcha mapping for lazy verification.
    pub fn mark_in_use(&self, captcha_id: &str, session_id: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Register session → captcha mapping
        {
            let mut session_map = self.session_to_captcha.lock().unwrap();
            session_map.insert(session_id.to_string(), captcha_id.to_string());
        }

        // Update page state if it's still in pool (for tracking)
        let mut pages = self.pages.lock().unwrap();
        if let Some(page) = pages.iter_mut().find(|p| p.captcha_id == captcha_id) {
            page.state = PageState::InUse {
                session_id: session_id.to_string(),
                served_at: now,
            };
        }

        self.metrics.in_use_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Mark a page as solved (needs regeneration)
    pub fn mark_solved(&self, session_id: &str) {
        // Look up captcha_id from session
        let captcha_id = {
            let session_map = self.session_to_captcha.lock().unwrap();
            session_map.get(session_id).cloned()
        };

        if let Some(cid) = captcha_id {
            // Remove from answers (no longer valid)
            {
                let mut answers = self.answers.lock().unwrap();
                answers.remove(&cid);
            }

            // Clean up session mapping
            {
                let mut session_map = self.session_to_captcha.lock().unwrap();
                session_map.remove(session_id);
            }

            self.metrics.pages_solved.fetch_add(1, Ordering::Relaxed);
            self.metrics.in_use_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get the expected answer for a session (for lazy verification)
    pub fn get_answer(&self, session_id: &str) -> Option<String> {
        // Look up captcha_id from session
        let captcha_id = {
            let session_map = self.session_to_captcha.lock().unwrap();
            session_map.get(session_id).cloned()
        }?;

        // Look up answer from captcha_id
        let answers = self.answers.lock().unwrap();
        answers.get(&captcha_id).cloned()
    }

    /// Reclaim expired InUse pages back to Available state
    ///
    /// Called periodically by the maintenance task.
    pub fn reclaim_expired_pages(&self) -> usize {
        let mut pages = self.pages.lock().unwrap();
        let mut reclaimed = 0;

        for page in pages.iter_mut() {
            if page.is_timeout_expired(self.config.timeout_seconds) {
                // Return to available state
                page.state = PageState::Available;
                reclaimed += 1;

                // Clean up session mapping
                if let PageState::InUse { ref session_id, .. } = page.state {
                    let mut session_map = self.session_to_captcha.lock().unwrap();
                    session_map.remove(session_id);
                }
            }
        }

        if reclaimed > 0 {
            self.metrics
                .pages_reclaimed
                .fetch_add(reclaimed as u64, Ordering::Relaxed);
            tracing::debug!("Reclaimed {} expired InUse pages", reclaimed);
        }

        reclaimed
    }

    /// Remove solved and expired pages from the pool
    ///
    /// Returns the number of pages removed (need regeneration).
    pub fn remove_stale_pages(&self) -> usize {
        let mut pages = self.pages.lock().unwrap();
        let before = pages.len();

        pages.retain(|p| {
            !matches!(p.state, PageState::Solved) && !p.is_expired(self.config.max_age_seconds)
        });

        let removed = before - pages.len();
        if removed > 0 {
            tracing::debug!("Removed {} stale pages from pool", removed);
        }
        removed
    }

    /// Generate a new pooled page
    pub fn generate_page(&self, difficulty: CaptchaDifficulty) -> PooledPage {
        let captcha_id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Generate CAPTCHA text and image
        let text = Self::generate_captcha_text(6);
        let image_data = bitmap::generate_bmp_with_difficulty(&text, difficulty);

        // Convert to data URI
        use base64::{engine::general_purpose, Engine as _};
        let b64 = general_purpose::STANDARD.encode(&image_data);
        let data_uri = format!("data:image/bmp;base64,{}", b64);

        // Render HTML template with {{SESSION_ID}} placeholder
        let branding = self.branding.lock().unwrap().clone();
        let mut extra_vars = std::collections::HashMap::new();

        // CAPTCHA content with image
        let captcha_content = format!(
            r#"<img src="{}" alt="Security Challenge" style="max-width: 100%; height: auto; display: block; margin: 0 auto;">"#,
            data_uri
        );

        extra_vars.insert("CAPTCHA_CONTENT".to_string(), captcha_content);
        extra_vars.insert(
            "CAPTCHA_INSTRUCTION".to_string(),
            "Type the characters shown in the image".to_string(),
        );
        extra_vars.insert(
            "CAPTCHA_INPUT".to_string(),
            r#"<div class="input-group">
                <label for="captcha">Enter Code</label>
                <input type="text" id="captcha" name="captcha" placeholder="• • • • • •" required autofocus autocomplete="off">
            </div>"#
                .to_string(),
        );
        extra_vars.insert(
            "CAPTCHA_SUBMIT".to_string(),
            r#"<button type="submit">Verify &amp; Enter</button>"#.to_string(),
        );
        // Use placeholder - will be replaced when serving
        extra_vars.insert("SESSION_ID".to_string(), "{{SESSION_ID}}".to_string());
        extra_vars.insert("CAPTCHA_ID".to_string(), captcha_id.clone());

        let html = self.template_engine.render_with_branding(
            fortify_core::TemplateType::GateChallenge,
            &branding,
            Some(&extra_vars),
        );

        // Store answer for verification
        {
            let mut answers = self.answers.lock().unwrap();
            answers.insert(captcha_id.clone(), text.clone());
        }

        self.metrics.pages_generated.fetch_add(1, Ordering::Relaxed);

        PooledPage {
            captcha_id,
            answer: text,
            html,
            state: PageState::Available,
            generated_at: now,
            difficulty,
        }
    }

    /// Add a page to the pool
    pub fn add_page(&self, page: PooledPage) {
        let mut pages = self.pages.lock().unwrap();
        if pages.len() < self.config.target_size * 2 {
            // Allow some overflow
            pages.push(page);
            // Count available directly since we hold the lock
            let available = pages
                .iter()
                .filter(|p| matches!(p.state, PageState::Available))
                .count();
            self.metrics
                .available_count
                .store(available, Ordering::Relaxed);
        }
    }

    /// Fill pool to target size
    ///
    /// Generates pages up to the target size. Returns number of pages generated.
    pub fn fill_to_target(&self, difficulty: CaptchaDifficulty) -> usize {
        let current = self.available_count();
        let needed = self.config.target_size.saturating_sub(current);

        if needed == 0 {
            return 0;
        }

        tracing::info!(
            "Filling pre-rendered pool: {} available, {} needed, target {}",
            current,
            needed,
            self.config.target_size
        );

        for _ in 0..needed {
            let page = self.generate_page(difficulty);
            self.add_page(page);
        }

        needed
    }

    /// Calculate dynamic generation rate based on traffic
    pub fn calculate_generation_rate(&self) -> u64 {
        let rpm = self.metrics.requests_last_minute.load(Ordering::Relaxed);
        let utilization = self.get_utilization_percent();

        match (rpm, utilization) {
            (0..=10, _) => 2,         // Idle: 2 pages/sec
            (11..=50, 0..=50) => 5,   // Light load, low util: 5/sec
            (11..=50, 51..=80) => 10, // Light load, high util: 10/sec
            (51..=100, _) => 25,      // Medium load: 25/sec
            _ => 50,                  // Heavy load/attack: 50/sec MAX
        }
    }

    /// Get pool utilization percentage
    pub fn get_utilization_percent(&self) -> u64 {
        let total = self.total_size();
        if total == 0 {
            return 100; // Empty pool = fully utilized
        }
        let available = self.available_count();
        ((total - available) * 100 / total) as u64
    }

    /// Update metrics (call periodically)
    pub fn update_metrics(&self) {
        self.metrics
            .available_count
            .store(self.available_count(), Ordering::Relaxed);
        self.metrics
            .in_use_count
            .store(self.in_use_count(), Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.metrics.last_update.store(now, Ordering::Relaxed);
    }

    /// Update branding configuration
    pub fn update_branding(&self, branding: fortify_core::BrandingVars) {
        let mut current = self.branding.lock().unwrap();
        *current = branding;
    }

    /// Generate random CAPTCHA text
    fn generate_captcha_text(length: usize) -> String {
        use rand::Rng;
        let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
        let mut rng = rand::rng();
        (0..length)
            .map(|_| chars[rng.random_range(0..chars.len())])
            .collect()
    }

    /// Get pool statistics for logging/monitoring
    pub fn get_stats(&self) -> PoolStats {
        PoolStats {
            total: self.total_size(),
            available: self.available_count(),
            in_use: self.in_use_count(),
            target: self.config.target_size,
            pages_served: self.metrics.pages_served.load(Ordering::Relaxed),
            pages_generated: self.metrics.pages_generated.load(Ordering::Relaxed),
            pages_reclaimed: self.metrics.pages_reclaimed.load(Ordering::Relaxed),
            pages_solved: self.metrics.pages_solved.load(Ordering::Relaxed),
            generation_rate: self.calculate_generation_rate(),
            utilization_percent: self.get_utilization_percent(),
        }
    }
}

/// Pool statistics for monitoring
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total: usize,
    pub available: usize,
    pub in_use: usize,
    pub target: usize,
    pub pages_served: u64,
    pub pages_generated: u64,
    pub pages_reclaimed: u64,
    pub pages_solved: u64,
    pub generation_rate: u64,
    pub utilization_percent: u64,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool[{}/{} available, {} in-use, {}% util, {} served, {} gen/sec]",
            self.available,
            self.target,
            self.in_use,
            self.utilization_percent,
            self.pages_served,
            self.generation_rate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let config = PoolConfig {
            target_size: 10,
            ..Default::default()
        };
        let branding = fortify_core::BrandingVars::default();
        let pool = PrerenderedPagePool::new(config, branding);

        assert_eq!(pool.available_count(), 0);
        assert_eq!(pool.total_size(), 0);
    }

    #[test]
    fn test_page_generation() {
        let config = PoolConfig {
            target_size: 10,
            ..Default::default()
        };
        let branding = fortify_core::BrandingVars::default();
        let pool = PrerenderedPagePool::new(config, branding);

        let page = pool.generate_page(CaptchaDifficulty::Medium);

        assert!(!page.captcha_id.is_empty());
        assert!(!page.answer.is_empty());
        assert!(page.html.contains("{{SESSION_ID}}"));
        assert!(matches!(page.state, PageState::Available));
    }

    #[test]
    fn test_take_and_mark_in_use() {
        let config = PoolConfig {
            target_size: 10,
            ..Default::default()
        };
        let branding = fortify_core::BrandingVars::default();
        let pool = PrerenderedPagePool::new(config, branding);

        // Add a page
        let page = pool.generate_page(CaptchaDifficulty::Medium);
        let captcha_id = page.captcha_id.clone();
        pool.add_page(page);

        assert_eq!(pool.available_count(), 1);

        // Take the page
        let taken = pool.take_available().unwrap();
        assert_eq!(taken.captcha_id, captcha_id);

        // Mark as in-use
        let session_id = "test-session-123";
        pool.mark_in_use(&captcha_id, session_id);

        // Verify answer lookup works
        let answer = pool.get_answer(session_id);
        assert!(answer.is_some());
    }

    #[test]
    fn test_fill_to_target() {
        let config = PoolConfig {
            target_size: 5,
            ..Default::default()
        };
        let branding = fortify_core::BrandingVars::default();
        let pool = PrerenderedPagePool::new(config, branding);

        let generated = pool.fill_to_target(CaptchaDifficulty::Medium);

        assert_eq!(generated, 5);
        assert_eq!(pool.available_count(), 5);
    }

    #[test]
    fn test_mark_solved() {
        let config = PoolConfig {
            target_size: 5,
            ..Default::default()
        };
        let branding = fortify_core::BrandingVars::default();
        let pool = PrerenderedPagePool::new(config, branding);

        // Generate and add page
        let page = pool.generate_page(CaptchaDifficulty::Medium);
        let captcha_id = page.captcha_id.clone();
        pool.add_page(page);

        // Simulate serving
        let session_id = "test-session";
        pool.mark_in_use(&captcha_id, session_id);

        // Verify answer exists
        assert!(pool.get_answer(session_id).is_some());

        // Mark as solved
        pool.mark_solved(session_id);

        // Answer should be removed
        assert!(pool.get_answer(session_id).is_none());
    }
}
