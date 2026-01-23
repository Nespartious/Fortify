use chrono::{DateTime, Duration, Utc};
use fortify_core::{safe_lock, SessionManager, SessionToken, TrustTier};
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub mod bitmap;
pub mod captcha_html;
pub mod captcha_types;
pub mod server;

pub use bitmap::CaptchaDifficulty;
pub use captcha_html::{
    render_captcha_page, render_captcha_page_with_reason, render_captcha_page_with_timer,
    render_captcha_page_with_timer_and_reason,
};
pub use captcha_types::{
    CaptchaConfig, CaptchaData, CaptchaType, CaptchaTypeConfig, DirectionChallenge, EmojiChallenge,
    ImageRotationChallenge, SequenceChallenge, SilhouetteChallenge, WordUnscrambleChallenge,
};

/// Single-use verification token issued after CAPTCHA solve
/// Prevents session cloning attacks by requiring token upgrade before session creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationToken {
    pub user_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub uses_remaining: u8,
    pub user_agent_hash: String,
    pub signature: String,
}

impl VerificationToken {
    /// Create a new verification token (60 second TTL, single-use)
    pub fn new(user_agent: &str) -> Self {
        let now = Utc::now();
        let user_id = Uuid::new_v4().to_string();
        let user_agent_hash = Self::hash_user_agent(user_agent);

        Self {
            user_id,
            issued_at: now,
            expires_at: now + Duration::seconds(60),
            uses_remaining: 1,
            user_agent_hash,
            signature: String::new(),
        }
    }

    /// Check if token is valid (not expired, has uses remaining)
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        now < self.expires_at && self.uses_remaining > 0
    }

    /// Mark token as used
    pub fn mark_used(&mut self) {
        self.uses_remaining = 0;
    }

    /// Hash User-Agent for binding (Tor-stable within session)
    fn hash_user_agent(ua: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(ua.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Encode token to string (JWT-like format)
    pub fn encode(&self) -> String {
        // Serialize token without signature
        let payload = serde_json::to_string(&self).unwrap();
        use base64::{engine::general_purpose, Engine as _};
        let encoded = general_purpose::STANDARD.encode(payload);
        let signature = Self::sign(&encoded);
        format!("{}.{}", encoded, signature)
    }

    /// Decode token from string
    pub fn decode(token_str: &str) -> Result<Self> {
        let parts: Vec<&str> = token_str.split('.').collect();
        if parts.len() != 2 {
            return Err(GateError::InvalidCaptcha);
        }

        // Verify signature
        let expected_sig = Self::sign(parts[0]);
        if parts[1] != expected_sig {
            return Err(GateError::InvalidCaptcha);
        }

        // Decode payload
        use base64::{engine::general_purpose, Engine as _};
        let payload = general_purpose::STANDARD
            .decode(parts[0])
            .map_err(|_| GateError::InvalidCaptcha)?;

        serde_json::from_slice(&payload).map_err(|_| GateError::InvalidCaptcha)
    }

    /// Sign data with HMAC-SHA256
    fn sign(data: &str) -> String {
        // Use secret from FORTIFY_GATE_SECRET env var (loaded at startup)
        let mut mac = HmacSha256::new_from_slice(&HMAC_SECRET).unwrap();
        mac.update(data.as_bytes());
        format!("{:x}", mac.finalize().into_bytes())
    }

    /// Validate User-Agent matches token binding
    pub fn validate_user_agent(&self, current_ua: &str) -> bool {
        let current_hash = Self::hash_user_agent(current_ua);
        self.user_agent_hash == current_hash
    }
}

// Global cache of verification tokens (user_id -> token)
// Used to prevent replay attacks
lazy_static::lazy_static! {
    pub static ref VERIFICATION_TOKEN_CACHE: Arc<Mutex<HashMap<String, VerificationToken>>> =
        Arc::new(Mutex::new(HashMap::new()));

    /// HMAC secret for signing verification tokens
    /// Load from FORTIFY_GATE_SECRET environment variable
    /// Falls back to a default for development ONLY - production MUST set this!
    static ref HMAC_SECRET: Vec<u8> = {
        match std::env::var("FORTIFY_GATE_SECRET") {
            Ok(secret) if !secret.is_empty() => {
                tracing::info!("Loaded HMAC secret from FORTIFY_GATE_SECRET environment variable");
                secret.into_bytes()
            }
            _ => {
                tracing::warn!(
                    "FORTIFY_GATE_SECRET not set! Using default secret. \
                    THIS IS INSECURE - set FORTIFY_GATE_SECRET in production!"
                );
                b"fortify-verification-secret-change-in-production".to_vec()
            }
        }
    };
}

#[derive(Error, Debug)]
pub enum GateError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid captcha solution")]
    InvalidCaptcha,
    #[error("Invalid proof-of-work solution")]
    InvalidProofOfWork,
    #[error("Challenge expired")]
    ChallengeExpired,
    #[error("Challenge not found")]
    ChallengeNotFound,
    #[error("Queue full")]
    QueueFull,
    #[error("Cookie compliance failed")]
    CookieComplianceFailed,
    #[error("Additional captcha required")]
    AdditionalCaptchaRequired,
    #[error("Verification token expired")]
    VerificationTokenExpired,
    #[error("Verification token already used")]
    VerificationTokenUsed,
    #[error("User-Agent mismatch")]
    UserAgentMismatch,
    #[error("Verification token not found")]
    VerificationTokenNotFound,
}

pub type Result<T> = std::result::Result<T, GateError>;

/// Captcha challenge
#[derive(Debug, Clone)]
pub struct CaptchaChallenge {
    pub challenge_id: String,
    pub text: String,
    pub image_data: Vec<u8>,
    pub created_at: u64,
    pub difficulty: CaptchaDifficulty,
    pub failed_attempts: u32,
}

impl CaptchaChallenge {
    /// Generate a new captcha challenge with default (Medium) difficulty
    pub fn generate() -> Self {
        Self::generate_with_difficulty(CaptchaDifficulty::Medium)
    }

    /// Generate a new captcha challenge with specified difficulty
    pub fn generate_with_difficulty(difficulty: CaptchaDifficulty) -> Self {
        let text = Self::generate_text(6);
        let image_data = bitmap::generate_bmp_with_difficulty(&text, difficulty);
        let challenge_id = Self::generate_id();

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            challenge_id,
            text,
            image_data,
            created_at,
            difficulty,
            failed_attempts: 0,
        }
    }

    fn generate_text(length: usize) -> String {
        let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
        let mut rng = rand::rng();
        (0..length)
            .map(|_| chars[rng.random_range(0..chars.len())])
            .collect()
    }

    fn generate_id() -> String {
        let mut rng = rand::rng();
        let random_bytes: Vec<u8> = (0..16).map(|_| rng.random()).collect();
        hex::encode(random_bytes)
    }

    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.created_at) > timeout_seconds
    }

    pub fn verify(&self, solution: &str) -> bool {
        // Case-insensitive comparison
        self.text.eq_ignore_ascii_case(solution)
    }
}

/// Proof-of-work challenge
#[derive(Debug, Clone)]
pub struct ProofOfWorkChallenge {
    pub challenge_id: String,
    pub challenge: Vec<u8>,
    pub difficulty: u32,
    pub created_at: u64,
}

impl ProofOfWorkChallenge {
    /// Generate a new PoW challenge
    pub fn new(difficulty: u32) -> Self {
        let mut rng = rand::rng();
        let challenge: Vec<u8> = (0..32).map(|_| rng.random()).collect();
        let challenge_id = hex::encode(&challenge[..16]);

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            challenge_id,
            challenge,
            difficulty,
            created_at,
        }
    }

    /// Verify a PoW solution
    pub fn verify(&self, nonce: u64) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&self.challenge);
        hasher.update(nonce.to_le_bytes());
        let hash = hasher.finalize();

        // Check if hash has required number of leading zero bits
        self.count_leading_zeros(&hash) >= self.difficulty
    }

    fn count_leading_zeros(&self, hash: &[u8]) -> u32 {
        let mut count = 0;
        for byte in hash {
            if *byte == 0 {
                count += 8;
            } else {
                count += byte.leading_zeros();
                break;
            }
        }
        count
    }

    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.created_at) > timeout_seconds
    }
}

/// Verification state for a session going through the gate
#[derive(Debug, Clone)]
pub struct VerificationState {
    pub session_id: String,
    /// Legacy BMP text captcha (for backwards compatibility)
    pub captcha_challenge: Option<CaptchaChallenge>,
    /// Multi-type captcha data
    pub captcha_data: Option<CaptchaData>,
    /// What type of captcha this is
    pub captcha_type: CaptchaType,
    pub pow_challenge: Option<ProofOfWorkChallenge>,
    pub captcha_solved: bool,
    pub pow_solved: bool,
    pub created_at: u64,
    /// Whether this is a threat/demotion re-verification
    pub is_threat: bool,
    /// Number of captchas remaining (threat sessions need 2, regular need 1)
    pub captchas_remaining: u8,
    /// Number of captchas already solved in this verification
    pub captchas_solved: u8,
}

impl VerificationState {
    pub fn new(session_id: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            session_id,
            captcha_challenge: None,
            captcha_data: None,
            captcha_type: CaptchaType::BmpText,
            pow_challenge: None,
            captcha_solved: false,
            pow_solved: false,
            created_at,
            is_threat: false,
            captchas_remaining: 1, // Default: 1 captcha needed
            captchas_solved: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.captcha_solved && self.pow_solved && self.captchas_remaining == 0
    }
}

/// Rate limiter for connections
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<u64>>>>,
    max_requests: usize,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_seconds,
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut requests = safe_lock(&self.requests);
        let timestamps = requests.entry(key.to_string()).or_default();

        // Remove old timestamps outside window
        timestamps.retain(|&t| (now - t) <= self.window_seconds);

        if timestamps.len() >= self.max_requests {
            return Err(GateError::RateLimitExceeded);
        }

        timestamps.push(now);
        Ok(())
    }

    pub fn cleanup(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut requests = safe_lock(&self.requests);
        requests.retain(|_, timestamps| {
            timestamps.retain(|&t| (now - t) <= self.window_seconds);
            !timestamps.is_empty()
        });
    }
}

/// Gate verification system
pub struct Gate {
    bind_addr: SocketAddr,
    max_concurrent: usize,
    pow_difficulty: u32,
    verification_timeout: u64,
    /// Reserved for future session validation integration
    #[allow(dead_code)]
    session_manager: Arc<SessionManager>,
    rate_limiter: Arc<RateLimiter>,
    verification_states: Arc<Mutex<HashMap<String, VerificationState>>>,
    secret_key: Vec<u8>,
    /// Shared captcha configuration (updated from admin panel)
    captcha_config: Arc<Mutex<CaptchaConfig>>,
    /// Pre-generated CAPTCHA pool for instant serving
    captcha_pool: Arc<Mutex<Vec<CaptchaChallenge>>>,
    /// Target pool size for pre-generation
    captcha_pool_target: usize,
    /// Branding configuration for HTML rendering
    branding: Arc<fortify_core::templates::BrandingVars>,
}

impl Gate {
    pub fn new(
        bind_addr: SocketAddr,
        max_concurrent: usize,
        pow_difficulty: u32,
        verification_timeout: u64,
        session_manager: Arc<SessionManager>,
        secret_key: Vec<u8>,
    ) -> Self {
        Self::with_branding(
            bind_addr,
            max_concurrent,
            pow_difficulty,
            verification_timeout,
            session_manager,
            secret_key,
            fortify_core::templates::BrandingVars::default(),
        )
    }

    /// Create a Gate with custom branding configuration
    pub fn with_branding(
        bind_addr: SocketAddr,
        max_concurrent: usize,
        pow_difficulty: u32,
        verification_timeout: u64,
        session_manager: Arc<SessionManager>,
        secret_key: Vec<u8>,
        branding: fortify_core::templates::BrandingVars,
    ) -> Self {
        let pool_target = 200; // Pre-generate 200 CAPTCHAs
        let captcha_pool = Arc::new(Mutex::new(Vec::with_capacity(pool_target)));

        // Pre-generate initial pool
        {
            let mut pool = captcha_pool.lock().unwrap();
            tracing::info!("Pre-generating {} CAPTCHAs for pool...", pool_target);
            for _ in 0..pool_target {
                pool.push(CaptchaChallenge::generate_with_difficulty(
                    CaptchaDifficulty::Medium,
                ));
            }
            tracing::info!("CAPTCHA pool initialized with {} challenges", pool.len());
        }

        Self {
            bind_addr,
            max_concurrent,
            pow_difficulty,
            verification_timeout,
            session_manager,
            rate_limiter: Arc::new(RateLimiter::new(10, 60)), // 10 requests per minute
            verification_states: Arc::new(Mutex::new(HashMap::new())),
            secret_key,
            captcha_config: Arc::new(Mutex::new(CaptchaConfig::default())),
            captcha_pool,
            captcha_pool_target: pool_target,
            branding: Arc::new(branding),
        }
    }

    /// Get the branding configuration
    pub fn branding(&self) -> &fortify_core::templates::BrandingVars {
        &self.branding
    }

    /// Take a pre-generated CAPTCHA from the pool, or generate on-demand if empty
    /// CRITICAL: Reset created_at to NOW when serving, since pool captchas may be stale
    fn take_captcha(&self, difficulty: CaptchaDifficulty) -> CaptchaChallenge {
        let mut pool = safe_lock(&self.captcha_pool);
        if let Some(mut captcha) = pool.pop() {
            // Reset created_at to now - pool captchas were generated at startup
            // and would otherwise appear expired after a few minutes
            captcha.created_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            tracing::debug!(
                "Served CAPTCHA from pool (remaining: {}), reset timestamp",
                pool.len()
            );
            captcha
        } else {
            tracing::warn!("CAPTCHA pool empty, generating on-demand");
            CaptchaChallenge::generate_with_difficulty(difficulty)
        }
    }

    /// Refill the CAPTCHA pool (call during idle periods)
    pub fn refill_captcha_pool(&self) {
        let mut pool = safe_lock(&self.captcha_pool);
        let current = pool.len();
        let needed = self.captcha_pool_target.saturating_sub(current);

        if needed > 0 {
            tracing::debug!(
                "Refilling CAPTCHA pool: {} -> {}",
                current,
                current + needed
            );
            for _ in 0..needed {
                pool.push(CaptchaChallenge::generate_with_difficulty(
                    CaptchaDifficulty::Medium,
                ));
            }
        }
    }

    /// Get current CAPTCHA pool size
    pub fn captcha_pool_size(&self) -> usize {
        safe_lock(&self.captcha_pool).len()
    }

    /// Get the current captcha configuration
    pub fn get_captcha_config(&self) -> CaptchaConfig {
        safe_lock(&self.captcha_config).clone()
    }

    /// Update the captcha configuration
    pub fn update_captcha_config(&self, config: CaptchaConfig) {
        *safe_lock(&self.captcha_config) = config;
    }

    /// Get verification timeout in seconds
    pub fn get_verification_timeout(&self) -> u64 {
        self.verification_timeout
    }

    /// Start the gate server
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Gate starting on {}", self.bind_addr);

        // Placeholder: Would implement actual HTTP server using hyper
        // - Serve captcha challenges
        // - Validate proof-of-work
        // - Issue session tokens
        // - Rate limit requests

        Ok(())
    }

    /// Create a new verification session
    pub fn create_verification(&self, session_id: String) -> Result<VerificationState> {
        self.create_verification_with_difficulty(session_id, CaptchaDifficulty::Medium)
    }

    /// Create a new verification session with specified captcha difficulty
    pub fn create_verification_with_difficulty(
        &self,
        session_id: String,
        difficulty: CaptchaDifficulty,
    ) -> Result<VerificationState> {
        self.create_verification_with_type(session_id, CaptchaType::BmpText, difficulty, false)
    }

    /// Create a new verification session with specified captcha type
    pub fn create_verification_with_type(
        &self,
        session_id: String,
        captcha_type: CaptchaType,
        difficulty: CaptchaDifficulty,
        is_threat: bool,
    ) -> Result<VerificationState> {
        // Rate limit verification creation (10 per minute per IP)
        // This prevents attackers from flooding the Gate with verification requests
        if self.rate_limiter.check_rate_limit(&session_id).is_err() {
            tracing::warn!(
                "Rate limit exceeded for verification creation: {}",
                session_id
            );
            return Err(GateError::RateLimitExceeded);
        }

        let mut states = safe_lock(&self.verification_states);

        // Check if we're at capacity
        if states.len() >= self.max_concurrent {
            return Err(GateError::QueueFull);
        }

        let mut state = VerificationState::new(session_id.clone());
        state.captcha_type = captcha_type;
        state.is_threat = is_threat;
        // Threat sessions (demoted users) need 2 captchas, regular users need 1
        state.captchas_remaining = if is_threat { 2 } else { 1 };
        state.captchas_solved = 0;

        tracing::info!(
            "Created verification session {}: is_threat={}, captchas_remaining={}",
            session_id,
            is_threat,
            state.captchas_remaining
        );

        // Generate the appropriate captcha based on type
        let config = CaptchaTypeConfig::default_for(captcha_type);
        match captcha_type {
            CaptchaType::BmpText => {
                // Use pre-generated CAPTCHA from pool when available
                let challenge = self.take_captcha(difficulty);
                tracing::debug!(
                    "Using CAPTCHA from pool, pool size now: {}",
                    self.captcha_pool_size()
                );
                state.captcha_data = Some(CaptchaData::BmpText {
                    text: challenge.text.clone(),
                    image_data: challenge.image_data.clone(),
                });
                state.captcha_challenge = Some(challenge);
            }
            CaptchaType::Emoji => {
                state.captcha_data = Some(CaptchaData::Emoji(EmojiChallenge::generate(
                    config.option_count,
                )));
            }
            CaptchaType::Direction => {
                let include_diagonals = config.difficulty >= 2;
                state.captcha_data = Some(CaptchaData::Direction(DirectionChallenge::generate(
                    include_diagonals,
                )));
            }
            CaptchaType::Sequence => {
                state.captcha_data = Some(CaptchaData::Sequence(SequenceChallenge::generate(
                    config.option_count,
                )));
            }
            CaptchaType::WordUnscramble => {
                state.captcha_data = Some(CaptchaData::WordUnscramble(
                    WordUnscrambleChallenge::generate(config.difficulty),
                ));
            }
            CaptchaType::ImageRotation => {
                state.captcha_data = Some(CaptchaData::ImageRotation(
                    ImageRotationChallenge::generate(),
                ));
            }
            CaptchaType::Silhouette => {
                state.captcha_data = Some(CaptchaData::Silhouette(SilhouetteChallenge::generate(
                    config.option_count,
                )));
            }
        }

        state.pow_challenge = Some(ProofOfWorkChallenge::new(self.pow_difficulty));

        states.insert(session_id, state.clone());
        Ok(state)
    }

    pub fn get_verification_state(&self, session_id: &str) -> Option<VerificationState> {
        let states = safe_lock(&self.verification_states);
        states.get(session_id).cloned()
    }

    pub fn get_captcha_challenge(&self, session_id: &str) -> Option<CaptchaChallenge> {
        let states = safe_lock(&self.verification_states);
        states
            .get(session_id)
            .and_then(|s| s.captcha_challenge.clone())
    }

    /// Verify full submission and issue token
    pub fn verify_submission(
        &self,
        session_id: &str,
        captcha: &str,
        pow_nonce: u64,
    ) -> Result<String> {
        // verify_captcha and verify_pow update state in place.
        // We need to call them sequentially.

        self.verify_captcha(session_id, captcha)?;
        self.verify_pow(session_id, pow_nonce)?;

        let token = self.issue_token(session_id, &self.secret_key)?;

        // Return full encoded token
        token.encode().map_err(|_| GateError::InvalidCaptcha)
    }

    /// Verify captcha solution - handles all captcha types
    pub fn verify_captcha(&self, session_id: &str, solution: &str) -> Result<()> {
        let mut states = safe_lock(&self.verification_states);
        let state = match states.get_mut(session_id) {
            Some(s) => s,
            None => {
                tracing::error!(
                    "CAPTCHA verify failed: session {} not found in states (have {} sessions)",
                    session_id,
                    states.len()
                );
                return Err(GateError::ChallengeNotFound);
            }
        };

        // Log session state for debugging
        tracing::debug!(
            "verify_captcha: session={}, is_threat={}, captchas_remaining={}, captcha_solved={}, has_captcha_data={}",
            session_id, state.is_threat, state.captchas_remaining, state.captcha_solved, state.captcha_data.is_some()
        );

        let timeout = std::time::Duration::from_secs(self.verification_timeout);

        // Check for timeout based on captcha type
        let is_expired = match &state.captcha_data {
            Some(CaptchaData::BmpText { .. }) => state
                .captcha_challenge
                .as_ref()
                .map(|c| c.is_expired(self.verification_timeout))
                .unwrap_or(true),
            Some(CaptchaData::Emoji(c)) => c.is_expired(timeout),
            Some(CaptchaData::Direction(c)) => c.is_expired(timeout),
            Some(CaptchaData::Sequence(c)) => c.is_expired(timeout),
            Some(CaptchaData::WordUnscramble(c)) => c.is_expired(timeout),
            Some(CaptchaData::ImageRotation(c)) => c.is_expired(timeout),
            Some(CaptchaData::Silhouette(c)) => c.is_expired(timeout),
            None => {
                // Legacy fallback - check old captcha_challenge field
                state
                    .captcha_challenge
                    .as_ref()
                    .map(|c| c.is_expired(self.verification_timeout))
                    .unwrap_or(true)
            }
        };

        if is_expired {
            // Get age for debugging
            let age_secs = if let Some(ref c) = state.captcha_challenge {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now.saturating_sub(c.created_at)
            } else {
                0
            };
            tracing::warn!(
                "CAPTCHA verify failed: session {} challenge expired (age={}s, timeout={}s, captchas_remaining={})", 
                session_id, age_secs, self.verification_timeout, state.captchas_remaining
            );
            return Err(GateError::ChallengeExpired);
        }

        // Verify based on captcha type
        let is_valid = match &state.captcha_data {
            Some(CaptchaData::BmpText { text, .. }) => {
                let result = solution.eq_ignore_ascii_case(text);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=BmpText, expected='{}', submitted='{}', match={}",
                    session_id, text, solution, result
                );
                result
            }
            Some(CaptchaData::Emoji(c)) => {
                let result = c.verify(solution);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=Emoji, target='{}', correct_idx={}, submitted='{}', match={}",
                    session_id, c.target_description, c.correct_index, solution, result
                );
                result
            }
            Some(CaptchaData::Direction(c)) => {
                let result = c.verify(solution);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=Direction, submitted='{}', match={}",
                    session_id,
                    solution,
                    result
                );
                result
            }
            Some(CaptchaData::Sequence(c)) => {
                let result = c.verify(solution);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=Sequence, submitted='{}', match={}",
                    session_id,
                    solution,
                    result
                );
                result
            }
            Some(CaptchaData::WordUnscramble(c)) => {
                let result = c.verify(solution);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=WordUnscramble, submitted='{}', match={}",
                    session_id, solution, result
                );
                result
            }
            Some(CaptchaData::ImageRotation(c)) => {
                let result = c.verify(solution);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=ImageRotation, submitted='{}', match={}",
                    session_id, solution, result
                );
                result
            }
            Some(CaptchaData::Silhouette(c)) => {
                let result = c.verify(solution);
                tracing::info!(
                    "CAPTCHA verification: session={}, type=Silhouette, submitted='{}', match={}",
                    session_id,
                    solution,
                    result
                );
                result
            }
            None => {
                // Legacy fallback - check old captcha_challenge field
                tracing::warn!(
                    "CAPTCHA verification: session={}, no captcha_data found, using legacy fallback",
                    session_id
                );
                state
                    .captcha_challenge
                    .as_ref()
                    .map(|c| c.verify(solution))
                    .unwrap_or(false)
            }
        };

        if !is_valid {
            // Track failed attempt for progressive delay
            if let Some(captcha) = state.captcha_challenge.as_mut() {
                captcha.failed_attempts += 1;
            }
            tracing::warn!(
                "CAPTCHA verify failed: session {} submitted invalid answer (attempts={})",
                session_id,
                state
                    .captcha_challenge
                    .as_ref()
                    .map(|c| c.failed_attempts)
                    .unwrap_or(0)
            );
            return Err(GateError::InvalidCaptcha);
        }

        // Captcha was valid - decrement remaining count and increment solved
        state.captchas_solved += 1;
        if state.captchas_remaining > 0 {
            state.captchas_remaining -= 1;
        }

        tracing::info!(
            "Session {} captcha verified: is_threat={}, captchas_remaining={}, captchas_solved={}",
            session_id,
            state.is_threat,
            state.captchas_remaining,
            state.captchas_solved
        );

        // Only mark fully solved if no more captchas remain
        if state.captchas_remaining == 0 {
            state.captcha_solved = true;
            Ok(())
        } else {
            // More captchas needed - return special error to signal UI
            tracing::info!("Session {} needs additional captcha", session_id);
            Err(GateError::AdditionalCaptchaRequired)
        }
    }

    /// Get the number of failed attempts for a session
    pub fn get_failed_attempts(&self, session_id: &str) -> u32 {
        let states = safe_lock(&self.verification_states);
        states
            .get(session_id)
            .and_then(|s| s.captcha_challenge.as_ref())
            .map(|c| c.failed_attempts)
            .unwrap_or(0)
    }

    /// Get the number of captchas remaining for a session
    pub fn get_captchas_remaining(&self, session_id: &str) -> u8 {
        let states = safe_lock(&self.verification_states);
        states
            .get(session_id)
            .map(|s| s.captchas_remaining)
            .unwrap_or(0)
    }

    /// Get the number of captchas already solved for a session
    pub fn get_captchas_solved(&self, session_id: &str) -> u8 {
        let states = safe_lock(&self.verification_states);
        states
            .get(session_id)
            .map(|s| s.captchas_solved)
            .unwrap_or(0)
    }

    /// Check if session is a threat/demoted session
    pub fn is_threat_session(&self, session_id: &str) -> bool {
        let states = safe_lock(&self.verification_states);
        states.get(session_id).map(|s| s.is_threat).unwrap_or(false)
    }

    /// Generate a new captcha for an existing session (used for second captcha in threat verification)
    pub fn regenerate_captcha(&self, session_id: &str, captcha_type: CaptchaType) -> Result<()> {
        let mut states = safe_lock(&self.verification_states);
        let state = states
            .get_mut(session_id)
            .ok_or(GateError::ChallengeNotFound)?;

        // Update captcha type for the new challenge
        state.captcha_type = captcha_type;

        // Generate the appropriate captcha based on type
        let config = CaptchaTypeConfig::default_for(captcha_type);
        match captcha_type {
            CaptchaType::BmpText => {
                let challenge =
                    CaptchaChallenge::generate_with_difficulty(CaptchaDifficulty::Medium);
                state.captcha_data = Some(CaptchaData::BmpText {
                    text: challenge.text.clone(),
                    image_data: challenge.image_data.clone(),
                });
                state.captcha_challenge = Some(challenge);
            }
            CaptchaType::Emoji => {
                state.captcha_data = Some(CaptchaData::Emoji(EmojiChallenge::generate(
                    config.option_count,
                )));
            }
            CaptchaType::Direction => {
                let include_diagonals = config.difficulty >= 2;
                state.captcha_data = Some(CaptchaData::Direction(DirectionChallenge::generate(
                    include_diagonals,
                )));
            }
            CaptchaType::Sequence => {
                state.captcha_data = Some(CaptchaData::Sequence(SequenceChallenge::generate(
                    config.option_count,
                )));
            }
            CaptchaType::WordUnscramble => {
                state.captcha_data = Some(CaptchaData::WordUnscramble(
                    WordUnscrambleChallenge::generate(config.difficulty),
                ));
            }
            CaptchaType::ImageRotation => {
                state.captcha_data = Some(CaptchaData::ImageRotation(
                    ImageRotationChallenge::generate(),
                ));
            }
            CaptchaType::Silhouette => {
                state.captcha_data = Some(CaptchaData::Silhouette(SilhouetteChallenge::generate(
                    config.option_count,
                )));
            }
        }

        // Reset the created_at for the new captcha timeout
        state.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(())
    }

    /// Calculate progressive delay based on failed attempts (in seconds)
    pub fn calculate_delay(&self, failed_attempts: u32) -> u64 {
        match failed_attempts {
            0 => 0,
            1 => 2,  // 2 seconds after 1st fail
            2 => 5,  // 5 seconds after 2nd fail
            3 => 10, // 10 seconds after 3rd fail
            4 => 20, // 20 seconds after 4th fail
            _ => 30, // 30 seconds cap for 5+ fails
        }
    }

    /// Verify proof-of-work solution
    pub fn verify_pow(&self, session_id: &str, _nonce: u64) -> Result<()> {
        let mut states = safe_lock(&self.verification_states);
        let state = states
            .get_mut(session_id)
            .ok_or(GateError::ChallengeNotFound)?;

        let challenge = state
            .pow_challenge
            .as_ref()
            .ok_or(GateError::ChallengeNotFound)?;

        if challenge.is_expired(self.verification_timeout) {
            return Err(GateError::ChallengeExpired);
        }

        // JS is banned, so client cannot perform PoW.
        // We bypass the verification for now.
        // if !challenge.verify(nonce) {
        //     return Err(GateError::InvalidProofOfWork);
        // }

        state.pow_solved = true;
        Ok(())
    }

    /// Issue token after successful verification
    pub fn issue_token(&self, session_id: &str, secret_key: &[u8]) -> Result<SessionToken> {
        let states = safe_lock(&self.verification_states);
        let state = states.get(session_id).ok_or(GateError::ChallengeNotFound)?;

        if !state.is_complete() {
            return Err(GateError::InvalidCaptcha); // Generic error
        }

        // Create session with Verified tier
        let mut token = SessionToken::new(
            session_id.to_string(),
            TrustTier::Verified,
            3600,      // 1 hour
            "unknown", // Backward compatibility: old flow doesn't have UA
        );

        token
            .sign(secret_key)
            .map_err(|_| GateError::InvalidCaptcha)?;

        Ok(token)
    }

    /// Create a session token directly (for token upgrade flow)
    pub fn create_session_token(
        &self,
        session_id: &str,
        tier: TrustTier,
        user_agent: &str,
    ) -> String {
        let mut token = SessionToken::new(
            session_id.to_string(),
            tier,
            86400, // 24 hours (until demotion)
            user_agent,
        );

        // Sign token
        if let Err(e) = token.sign(&self.secret_key) {
            tracing::error!("Failed to sign session token: {}", e);
        }

        // Encode token to base64-encoded JSON string for cookie
        token.encode().unwrap_or_else(|_| String::new())
    }

    /// Clean up expired verification states
    pub fn cleanup(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut states = safe_lock(&self.verification_states);
        states.retain(|_, state| (now - state.created_at) <= self.verification_timeout);

        self.rate_limiter.cleanup();
    }
}

/// Background task to clean up expired verification tokens
pub async fn start_token_cleanup_task() {
    use tokio::time::{interval, Duration};

    tokio::spawn(async {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let mut cache = safe_lock(&VERIFICATION_TOKEN_CACHE);
            let now = Utc::now();
            let before_count = cache.len();
            cache.retain(|_, token| now < token.expires_at);
            let after_count = cache.len();

            if before_count != after_count {
                tracing::info!(
                    "Token cleanup: removed {} expired tokens, {} active verification tokens remaining",
                    before_count - after_count,
                    after_count
                );
            }
        }
    });
}

pub mod verification {
    pub use super::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captcha_generation() {
        let challenge = CaptchaChallenge::generate();
        assert_eq!(challenge.text.len(), 6);
        assert!(!challenge.challenge_id.is_empty());
    }

    #[test]
    fn test_captcha_verification() {
        let challenge = CaptchaChallenge::generate();
        assert!(challenge.verify(&challenge.text));
        assert!(challenge.verify(&challenge.text.to_lowercase()));
        assert!(!challenge.verify("WRONG"));
    }

    #[test]
    fn test_pow_verification() {
        let challenge = ProofOfWorkChallenge::new(4); // Easy difficulty for test

        // Brute force find a valid nonce
        let mut valid_nonce = None;
        for nonce in 0..100000 {
            if challenge.verify(nonce) {
                valid_nonce = Some(nonce);
                break;
            }
        }

        assert!(valid_nonce.is_some());
        assert!(challenge.verify(valid_nonce.unwrap()));
    }

    #[test]
    fn test_verification_state() {
        let mut state = VerificationState::new("test-123".into());
        assert!(!state.is_complete());

        state.captcha_solved = true;
        assert!(!state.is_complete());

        state.pow_solved = true;
        assert!(!state.is_complete()); // Still need to solve captchas

        state.captchas_remaining = 0;
        assert!(state.is_complete()); // Now complete
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, 60);

        assert!(limiter.check_rate_limit("ip1").is_ok());
        assert!(limiter.check_rate_limit("ip1").is_ok());
        assert!(limiter.check_rate_limit("ip1").is_ok());
        assert!(limiter.check_rate_limit("ip1").is_err()); // 4th request should fail

        // Different IP should work
        assert!(limiter.check_rate_limit("ip2").is_ok());
    }

    #[test]
    fn test_gate_verification_flow() {
        let secret = b"test-secret-key";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
        let gate = Gate::new(
            "127.0.0.1:8081".parse().unwrap(),
            10,
            4,
            300,
            session_manager,
            secret.to_vec(),
        );

        // Create verification
        let state = gate.create_verification("test-123".into()).unwrap();
        assert!(!state.is_complete());

        // Solve captcha
        let captcha_solution = state.captcha_challenge.as_ref().unwrap().text.clone();
        gate.verify_captcha("test-123", &captcha_solution).unwrap();

        // Solve PoW (brute force for test)
        let pow_challenge = state.pow_challenge.as_ref().unwrap();
        let mut valid_nonce = 0;
        for nonce in 0..100000 {
            if pow_challenge.verify(nonce) {
                valid_nonce = nonce;
                break;
            }
        }
        gate.verify_pow("test-123", valid_nonce).unwrap();

        // Issue token
        let token = gate.issue_token("test-123", secret).unwrap();
        assert_eq!(token.trust_tier, TrustTier::Verified);
        assert!(token.verify(secret).is_ok());
    }

    #[test]
    fn test_gate_queue_full() {
        let secret = b"test-secret-key";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
        let gate = Gate::new(
            "127.0.0.1:8081".parse().unwrap(),
            2, // Max 2 concurrent
            4,
            300,
            session_manager,
            secret.to_vec(),
        );

        gate.create_verification("s1".into()).unwrap();
        gate.create_verification("s2".into()).unwrap();

        // Third should fail
        assert!(matches!(
            gate.create_verification("s3".into()),
            Err(GateError::QueueFull)
        ));
    }
}
