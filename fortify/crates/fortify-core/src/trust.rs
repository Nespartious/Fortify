use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Error, Debug)]
pub enum TrustError {
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Token is burned")]
    TokenBurned,
    #[error("Invalid trust transition: {0}")]
    InvalidTransition(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid token encoding: {0}")]
    InvalidEncoding(String),
    #[error("User-Agent mismatch")]
    UserAgentMismatch,
}

pub type Result<T> = std::result::Result<T, TrustError>;

/// Trust tiers for session classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustTier {
    Burned = -2,
    Suspicious = -1,
    Unknown = 0,
    Verified = 1,
    Trusted = 2,
}

impl TrustTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustTier::Burned => "burned",
            TrustTier::Suspicious => "suspicious",
            TrustTier::Unknown => "unknown",
            TrustTier::Verified => "verified",
            TrustTier::Trusted => "trusted",
        }
    }

    /// Check if this tier allows service access
    pub fn allows_access(&self) -> bool {
        matches!(self, TrustTier::Verified | TrustTier::Trusted)
    }

    /// Check if this tier requires gate challenges
    pub fn requires_gate(&self) -> bool {
        matches!(
            self,
            TrustTier::Unknown | TrustTier::Burned | TrustTier::Suspicious
        )
    }

    /// Check if promotion is possible from this tier
    pub fn can_promote(&self) -> bool {
        *self < TrustTier::Trusted
    }

    /// Check if demotion is possible from this tier
    pub fn can_demote(&self) -> bool {
        *self > TrustTier::Burned
    }
}

/// Session token issued after verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub session_id: String,
    pub trust_tier: TrustTier,
    pub issued_at: u64,
    pub expires_at: u64,
    pub user_agent_hash: String,
    pub signature: Vec<u8>,
}

impl SessionToken {
    /// Create a new session token
    pub fn new(
        session_id: String,
        trust_tier: TrustTier,
        lifetime_seconds: u64,
        user_agent: &str,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            session_id,
            trust_tier,
            issued_at: now,
            expires_at: now + lifetime_seconds,
            user_agent_hash: Self::hash_user_agent(user_agent),
            signature: Vec::new(),
        }
    }

    /// Hash the User-Agent for binding
    fn hash_user_agent(user_agent: &str) -> String {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(user_agent.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate that the current User-Agent matches the token binding
    pub fn validate_user_agent(&self, current_user_agent: &str) -> bool {
        self.user_agent_hash == Self::hash_user_agent(current_user_agent)
    }

    /// Sign the token with a secret key
    pub fn sign(&mut self, secret: &[u8]) -> Result<()> {
        let payload = self.serialize_payload()?;
        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(&payload);
        self.signature = mac.finalize().into_bytes().to_vec();
        Ok(())
    }

    /// Verify the token signature
    pub fn verify(&self, secret: &[u8]) -> Result<()> {
        let payload = self.serialize_payload()?;
        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(&payload);

        mac.verify_slice(&self.signature)
            .map_err(|_| TrustError::InvalidSignature)?;

        Ok(())
    }

    /// Serialize token payload for signing
    fn serialize_payload(&self) -> Result<Vec<u8>> {
        // Use a deterministic struct for signing instead of json! macro (which might vary key order)
        #[derive(Serialize)]
        struct SigningPayload<'a> {
            session_id: &'a str,
            trust_tier: &'a str,
            issued_at: u64,
            expires_at: u64,
            user_agent_hash: &'a str,
        }

        let payload = SigningPayload {
            session_id: &self.session_id,
            trust_tier: self.trust_tier.as_str(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            user_agent_hash: &self.user_agent_hash,
        };

        Ok(serde_json::to_vec(&payload)?)
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.expires_at
    }

    /// Check if token is valid (not expired, not burned, valid signature)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && self.trust_tier != TrustTier::Burned
    }

    /// Encode token to string (base64 JSON)
    pub fn encode(&self) -> Result<String> {
        let mut token_copy = self.clone();
        token_copy.signature.clone_from(&self.signature);
        let json = serde_json::to_vec(&token_copy)?;
        Ok(base64_encode(&json))
    }

    /// Decode token from string
    pub fn decode(encoded: &str) -> Result<Self> {
        let json =
            base64_decode(encoded).map_err(|err| TrustError::InvalidEncoding(err.to_string()))?;
        let token: SessionToken = serde_json::from_slice(&json)?;
        Ok(token)
    }

    /// Get time until expiration in seconds
    #[allow(clippy::cast_possible_wrap)]
    pub fn time_until_expiry(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (self.expires_at as i64) - (now as i64)
    }
}

/// Session state machine
#[derive(Debug, Clone)]
pub struct Session {
    pub token: SessionToken,
    pub request_count: u64,
    pub violation_count: u32,
    pub last_activity: u64,
}

impl Session {
    /// Create a new session
    pub fn new(token: SessionToken) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            token,
            request_count: 0,
            violation_count: 0,
            last_activity: now,
        }
    }

    /// Record a request
    pub fn record_request(&mut self) {
        self.request_count += 1;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Record a violation
    pub fn record_violation(&mut self) {
        self.violation_count += 1;
    }

    /// Promote session to higher trust tier
    pub fn promote(&mut self) -> Result<()> {
        let new_tier = match self.token.trust_tier {
            TrustTier::Unknown | TrustTier::Suspicious => TrustTier::Verified,
            TrustTier::Verified => TrustTier::Trusted,
            _ => {
                return Err(TrustError::InvalidTransition(format!(
                    "Cannot promote from {:?}",
                    self.token.trust_tier
                )))
            }
        };

        self.token.trust_tier = new_tier;
        Ok(())
    }

    /// Demote session to lower trust tier
    pub fn demote(&mut self) -> Result<()> {
        let new_tier = match self.token.trust_tier {
            TrustTier::Trusted | TrustTier::Verified => TrustTier::Suspicious,
            TrustTier::Suspicious => TrustTier::Burned,
            _ => {
                return Err(TrustError::InvalidTransition(format!(
                    "Cannot demote from {:?}",
                    self.token.trust_tier
                )))
            }
        };

        self.token.trust_tier = new_tier;
        Ok(())
    }

    /// Burn the session (permanent ban)
    pub fn burn(&mut self) {
        self.token.trust_tier = TrustTier::Burned;
    }

    /// Check if session should be demoted based on violations
    pub fn should_demote(&self) -> bool {
        match self.token.trust_tier {
            TrustTier::Verified | TrustTier::Trusted => self.violation_count >= 3,
            TrustTier::Suspicious => self.violation_count >= 2,
            _ => false,
        }
    }

    /// Check if session should be burned
    pub fn should_burn(&self) -> bool {
        self.violation_count >= 10
            || (self.token.trust_tier == TrustTier::Suspicious && self.violation_count >= 5)
    }

    /// Check if session is idle
    pub fn is_idle(&self, timeout_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.last_activity) > timeout_seconds
    }
}

// Base64 encoding helpers
fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder =
            base64::write::EncoderWriter::new(&mut buf, &base64::engine::general_purpose::STANDARD);
        encoder.write_all(data).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

fn base64_decode(data: &str) -> std::result::Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_tier_ordering() {
        assert!(TrustTier::Burned < TrustTier::Unknown);
        assert!(TrustTier::Unknown < TrustTier::Verified);
        assert!(TrustTier::Verified < TrustTier::Trusted);
    }

    #[test]
    fn test_trust_tier_access() {
        assert!(!TrustTier::Unknown.allows_access());
        assert!(TrustTier::Verified.allows_access());
        assert!(TrustTier::Trusted.allows_access());
        assert!(!TrustTier::Burned.allows_access());
    }

    #[test]
    fn test_token_creation() {
        let token = SessionToken::new("test".into(), TrustTier::Verified, 3600, "test-agent");
        assert!(!token.is_expired());
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_signing() {
        let mut token = SessionToken::new("test".into(), TrustTier::Verified, 3600, "test-agent");
        let secret = b"test-secret-key";

        token.sign(secret).unwrap();
        assert!(!token.signature.is_empty());
        assert!(token.verify(secret).is_ok());
    }

    #[test]
    fn test_token_invalid_signature() {
        let mut token = SessionToken::new("test".into(), TrustTier::Verified, 3600, "test-agent");
        let secret = b"test-secret-key";
        let wrong_secret = b"wrong-secret-key";

        token.sign(secret).unwrap();
        assert!(token.verify(wrong_secret).is_err());
    }

    #[test]
    fn test_burned_token_invalid() {
        let mut token = SessionToken::new("test".into(), TrustTier::Burned, 3600, "test-agent");
        let secret = b"test-secret-key";
        token.sign(secret).unwrap();

        assert!(!token.is_valid());
    }

    #[test]
    fn test_session_promotion() {
        let token = SessionToken::new("test".into(), TrustTier::Unknown, 3600, "test-agent");
        let mut session = Session::new(token);

        session.promote().unwrap();
        assert_eq!(session.token.trust_tier, TrustTier::Verified);

        session.promote().unwrap();
        assert_eq!(session.token.trust_tier, TrustTier::Trusted);

        // Cannot promote beyond Trusted
        assert!(session.promote().is_err());
    }

    #[test]
    fn test_session_demotion() {
        let token = SessionToken::new("test".into(), TrustTier::Trusted, 3600, "test-agent");
        let mut session = Session::new(token);

        session.demote().unwrap();
        assert_eq!(session.token.trust_tier, TrustTier::Suspicious);

        session.demote().unwrap();
        assert_eq!(session.token.trust_tier, TrustTier::Burned);

        // Cannot demote beyond Burned
        assert!(session.demote().is_err());
    }

    #[test]
    fn test_session_violations() {
        let token = SessionToken::new("test".into(), TrustTier::Verified, 3600, "test-agent");
        let mut session = Session::new(token);

        assert!(!session.should_demote());

        session.record_violation();
        session.record_violation();
        assert!(!session.should_demote());

        session.record_violation();
        assert!(session.should_demote());
    }

    #[test]
    fn test_session_burn_threshold() {
        let token = SessionToken::new("test".into(), TrustTier::Verified, 3600, "test-agent");
        let mut session = Session::new(token);

        for _ in 0..10 {
            session.record_violation();
        }

        assert!(session.should_burn());
    }
}
