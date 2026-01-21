use crate::Mirror;
use serde::{Deserialize, Serialize};

/// Mirror rotation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationStrategy {
    /// Rotate based on age
    AgeBased,
    /// Rotate based on request count
    RequestBased,
    /// Rotate based on compromise score
    RiskBased,
}

/// Mirror lifecycle manager
pub struct MirrorLifecycle {
    rotation_strategy: RotationStrategy,
    max_age_seconds: u64,
    max_requests: u64,
    risk_threshold: f32,
}

impl MirrorLifecycle {
    pub fn new(
        rotation_strategy: RotationStrategy,
        max_age_seconds: u64,
        max_requests: u64,
        risk_threshold: f32,
    ) -> Self {
        Self {
            rotation_strategy,
            max_age_seconds,
            max_requests,
            risk_threshold,
        }
    }

    /// Check if mirror should be rotated
    pub fn should_rotate(&self, mirror: &Mirror) -> (bool, String) {
        match self.rotation_strategy {
            RotationStrategy::AgeBased => self.check_age_rotation(mirror),
            RotationStrategy::RequestBased => self.check_request_rotation(mirror),
            RotationStrategy::RiskBased => self.check_risk_rotation(mirror),
        }
    }

    fn check_age_rotation(&self, mirror: &Mirror) -> (bool, String) {
        if mirror.age_seconds() >= self.max_age_seconds {
            (
                true,
                format!(
                    "Mirror age {}s exceeds maximum {}s",
                    mirror.age_seconds(),
                    self.max_age_seconds
                ),
            )
        } else {
            (false, String::new())
        }
    }

    fn check_request_rotation(&self, mirror: &Mirror) -> (bool, String) {
        if mirror.metrics.requests_total >= self.max_requests {
            (
                true,
                format!(
                    "Request count {} exceeds maximum {}",
                    mirror.metrics.requests_total, self.max_requests
                ),
            )
        } else {
            (false, String::new())
        }
    }

    fn check_risk_rotation(&self, mirror: &Mirror) -> (bool, String) {
        if mirror.metrics.compromise_score >= self.risk_threshold {
            (
                true,
                format!(
                    "Risk score {:.2} exceeds threshold {:.2}",
                    mirror.metrics.compromise_score, self.risk_threshold
                ),
            )
        } else {
            (false, String::new())
        }
    }

    /// Get time until next rotation
    pub fn time_until_rotation(&self, mirror: &Mirror) -> Option<u64> {
        match self.rotation_strategy {
            RotationStrategy::AgeBased => {
                let age = mirror.age_seconds();
                if age < self.max_age_seconds {
                    Some(self.max_age_seconds - age)
                } else {
                    Some(0)
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_age_based_rotation() {
        let lifecycle = MirrorLifecycle::new(RotationStrategy::AgeBased, 100, 1000, 0.7);

        let mut mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        // Manually set old creation time
        mirror.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 150;

        let (should_rotate, reason) = lifecycle.should_rotate(&mirror);
        assert!(should_rotate);
        assert!(reason.contains("age"));
    }

    #[test]
    fn test_request_based_rotation() {
        let lifecycle = MirrorLifecycle::new(RotationStrategy::RequestBased, 3600, 100, 0.7);

        let mut mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));
        mirror.metrics.requests_total = 150;

        let (should_rotate, reason) = lifecycle.should_rotate(&mirror);
        assert!(should_rotate);
        assert!(reason.contains("Request count"));
    }

    #[test]
    fn test_risk_based_rotation() {
        let lifecycle = MirrorLifecycle::new(RotationStrategy::RiskBased, 3600, 1000, 0.7);

        let mut mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));
        mirror.metrics.compromise_score = 0.8;

        let (should_rotate, reason) = lifecycle.should_rotate(&mirror);
        assert!(should_rotate);
        assert!(reason.contains("Risk score"));
    }

    #[test]
    fn test_time_until_rotation() {
        let lifecycle = MirrorLifecycle::new(RotationStrategy::AgeBased, 100, 1000, 0.7);

        let mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        let time_left = lifecycle.time_until_rotation(&mirror);
        assert!(time_left.is_some());
        assert!(time_left.unwrap() <= 100);
    }
}
