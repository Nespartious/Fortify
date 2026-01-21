use crate::{CompromiseSignal, Mirror, SignalType};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compromise detection engine
pub struct CompromiseDetector {
    traffic_window: VecDeque<TrafficSample>,
    window_size: usize,
    anomaly_threshold: f32,
}

/// Traffic sample for anomaly detection
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TrafficSample {
    timestamp: u64,
    request_count: u64,
    bytes: u64,
    failures: u64,
    avg_response_time_ms: f64,
}

impl CompromiseDetector {
    pub fn new(window_size: usize, anomaly_threshold: f32) -> Self {
        Self {
            traffic_window: VecDeque::with_capacity(window_size),
            window_size,
            anomaly_threshold,
        }
    }

    /// Analyze mirror for compromise signals
    pub fn analyze(&mut self, mirror: &Mirror) -> Vec<CompromiseSignal> {
        let mut signals = Vec::new();

        // Check failure rate
        if let Some(signal) = self.check_failure_rate(mirror) {
            signals.push(signal);
        }

        // Check traffic anomalies
        if let Some(signal) = self.check_traffic_anomaly(mirror) {
            signals.push(signal);
        }

        // Check response time anomalies
        if let Some(signal) = self.check_response_time_anomaly(mirror) {
            signals.push(signal);
        }

        // Check for suspicious patterns
        if let Some(signal) = self.check_suspicious_patterns(mirror) {
            signals.push(signal);
        }

        signals
    }

    fn check_failure_rate(&self, mirror: &Mirror) -> Option<CompromiseSignal> {
        let failure_rate = mirror.metrics.failure_rate();

        if failure_rate > 0.3 {
            Some(CompromiseSignal::new(
                SignalType::RepeatedFailures,
                (failure_rate as f32).min(1.0),
                format!("High failure rate: {:.2}%", failure_rate * 100.0),
            ))
        } else {
            None
        }
    }

    fn check_traffic_anomaly(&mut self, mirror: &Mirror) -> Option<CompromiseSignal> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Add current sample
        let sample = TrafficSample {
            timestamp: now,
            request_count: mirror.metrics.requests_total,
            bytes: mirror.metrics.bytes_transferred,
            failures: mirror.metrics.requests_failed,
            avg_response_time_ms: mirror.metrics.average_response_time_ms,
        };

        self.traffic_window.push_back(sample);

        // Keep window size limited
        while self.traffic_window.len() > self.window_size {
            self.traffic_window.pop_front();
        }

        // Need enough samples for analysis
        if self.traffic_window.len() < 3 {
            return None;
        }

        // Calculate baseline from older samples
        let baseline_samples: Vec<_> = self
            .traffic_window
            .iter()
            .take(self.traffic_window.len() - 1)
            .collect();

        if baseline_samples.is_empty() {
            return None;
        }

        let avg_requests: f64 = baseline_samples
            .iter()
            .map(|s| s.request_count as f64)
            .sum::<f64>()
            / baseline_samples.len() as f64;

        // Check current against baseline
        let current = self.traffic_window.back().unwrap();
        let deviation = (current.request_count as f64 - avg_requests).abs() / avg_requests.max(1.0);

        if deviation > self.anomaly_threshold as f64 {
            Some(CompromiseSignal::new(
                SignalType::UnusualTraffic,
                (deviation as f32 * 0.5).min(1.0),
                format!("Traffic spike: {:.1}x normal", deviation + 1.0),
            ))
        } else {
            None
        }
    }

    fn check_response_time_anomaly(&self, mirror: &Mirror) -> Option<CompromiseSignal> {
        if self.traffic_window.len() < 3 {
            return None;
        }

        let baseline_samples: Vec<_> = self
            .traffic_window
            .iter()
            .take(self.traffic_window.len() - 1)
            .collect();

        let avg_response_time: f64 = baseline_samples
            .iter()
            .map(|s| s.avg_response_time_ms)
            .sum::<f64>()
            / baseline_samples.len() as f64;

        let current_time = mirror.metrics.average_response_time_ms;
        let slowdown = current_time / avg_response_time.max(1.0);

        if slowdown > 3.0 {
            Some(CompromiseSignal::new(
                SignalType::TimingAnomaly,
                ((slowdown - 1.0) as f32 * 0.2).min(1.0),
                format!("Response time {:.1}x slower", slowdown),
            ))
        } else {
            None
        }
    }

    fn check_suspicious_patterns(&self, mirror: &Mirror) -> Option<CompromiseSignal> {
        // Check for rapid signal accumulation
        let recent_signals = mirror
            .signals
            .iter()
            .filter(|s| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now - s.timestamp < 60
            })
            .count();

        if recent_signals >= 5 {
            Some(CompromiseSignal::new(
                SignalType::NetworkAnomaly,
                0.8,
                format!(
                    "Rapid signal accumulation: {} signals in 60s",
                    recent_signals
                ),
            ))
        } else {
            None
        }
    }

    /// Reset detection state
    pub fn reset(&mut self) {
        self.traffic_window.clear();
    }
}

/// Burn decision maker
pub struct BurnDecider {
    burn_threshold: f32,
    max_age_seconds: u64,
}

impl BurnDecider {
    pub fn new(burn_threshold: f32, max_age_seconds: u64) -> Self {
        Self {
            burn_threshold,
            max_age_seconds,
        }
    }

    /// Decide if mirror should be burned
    pub fn should_burn(&self, mirror: &Mirror) -> (bool, String) {
        // Check compromise score
        if mirror.metrics.compromise_score >= self.burn_threshold {
            return (
                true,
                format!(
                    "Compromise score {:.2} exceeds threshold {:.2}",
                    mirror.metrics.compromise_score, self.burn_threshold
                ),
            );
        }

        // Check age
        if mirror.age_seconds() > self.max_age_seconds {
            return (
                true,
                format!(
                    "Mirror age {}s exceeds maximum {}s",
                    mirror.age_seconds(),
                    self.max_age_seconds
                ),
            );
        }

        // Check failure rate
        if mirror.metrics.failure_rate() > 0.5 {
            return (
                true,
                format!(
                    "Failure rate {:.2}% exceeds 50%",
                    mirror.metrics.failure_rate() * 100.0
                ),
            );
        }

        (false, String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mirror;
    use std::path::PathBuf;

    #[test]
    fn test_failure_rate_detection() {
        let mut detector = CompromiseDetector::new(10, 0.5);
        let mut mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        // Simulate high failure rate
        for _ in 0..10 {
            mirror.metrics.record_request(false, 100.0, 0);
        }

        let signals = detector.analyze(&mirror);
        assert!(!signals.is_empty());

        let has_failure_signal = signals
            .iter()
            .any(|s| s.signal_type == SignalType::RepeatedFailures);
        assert!(has_failure_signal);
    }

    #[test]
    fn test_burn_decision_compromise_score() {
        let decider = BurnDecider::new(0.7, 3600);
        let mut mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        mirror.metrics.compromise_score = 0.8;
        let (should_burn, reason) = decider.should_burn(&mirror);

        assert!(should_burn);
        assert!(reason.contains("Compromise score"));
    }

    #[test]
    fn test_burn_decision_age() {
        let decider = BurnDecider::new(0.7, 10);
        let mut mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        // Manually set old creation time
        mirror.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 20;

        let (should_burn, reason) = decider.should_burn(&mirror);

        assert!(should_burn);
        assert!(reason.contains("age"));
    }

    #[test]
    fn test_burn_decision_healthy_mirror() {
        let decider = BurnDecider::new(0.7, 3600);
        let mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        let (should_burn, _) = decider.should_burn(&mirror);

        assert!(!should_burn);
    }

    #[test]
    fn test_detector_reset() {
        let mut detector = CompromiseDetector::new(10, 0.5);
        let mirror = Mirror::new("test".into(), PathBuf::from("/tmp"));

        detector.analyze(&mirror);
        assert!(!detector.traffic_window.is_empty());

        detector.reset();
        assert!(detector.traffic_window.is_empty());
    }
}
