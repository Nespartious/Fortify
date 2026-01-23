//! Onion address verification module
//!
//! Verifies that generated .onion addresses are reachable via Tor.
//! Implements retry logic with exponential backoff.

use std::time::Duration;

/// Result of a verification attempt
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// The .onion address that was verified
    pub address: String,
    /// Whether the address is reachable
    pub reachable: bool,
    /// HTTP status code if reachable
    pub status_code: Option<u16>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error message if verification failed
    pub error: Option<String>,
    /// Number of attempts made
    pub attempts: u32,
}

/// Configuration for verification attempts
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// SOCKS proxy address (e.g., "socks5h://127.0.0.1:9050")
    pub socks_proxy: String,
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            socks_proxy: "socks5h://127.0.0.1:9050".to_string(),
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
            timeout_seconds: 30,
        }
    }
}

impl VerificationConfig {
    /// Create config with custom SOCKS port
    pub fn with_socks_port(port: u16) -> Self {
        Self {
            socks_proxy: format!("socks5h://127.0.0.1:{}", port),
            ..Default::default()
        }
    }
}

/// Verifier for .onion addresses
pub struct OnionVerifier {
    config: VerificationConfig,
}

impl OnionVerifier {
    /// Create a new verifier with the given configuration
    pub fn new(config: VerificationConfig) -> Self {
        Self { config }
    }

    /// Create a new verifier with default configuration
    pub fn with_defaults() -> Self {
        Self::new(VerificationConfig::default())
    }

    /// Verify a single .onion address with retry logic
    pub async fn verify(&self, address: &str) -> VerificationResult {
        let url = if address.starts_with("http://") || address.starts_with("https://") {
            address.to_string()
        } else {
            format!("http://{}", address)
        };

        let mut last_error = None;
        let mut delay_ms = self.config.initial_delay_ms;

        for attempt in 1..=self.config.max_attempts {
            let _start = std::time::Instant::now();

            match self.try_verify(&url).await {
                Ok((status_code, response_time_ms)) => {
                    return VerificationResult {
                        address: address.to_string(),
                        reachable: true,
                        status_code: Some(status_code),
                        response_time_ms: Some(response_time_ms),
                        error: None,
                        attempts: attempt,
                    };
                }
                Err(e) => {
                    last_error = Some(e);

                    // Don't sleep after the last attempt
                    if attempt < self.config.max_attempts {
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        // Exponential backoff
                        delay_ms = (delay_ms * 2).min(self.config.max_delay_ms);
                    }
                }
            }
        }

        VerificationResult {
            address: address.to_string(),
            reachable: false,
            status_code: None,
            response_time_ms: None,
            error: last_error,
            attempts: self.config.max_attempts,
        }
    }

    /// Verify multiple addresses concurrently
    pub async fn verify_all(&self, addresses: &[String]) -> Vec<VerificationResult> {
        let futures: Vec<_> = addresses
            .iter()
            .map(|addr| self.verify(addr))
            .collect();

        futures::future::join_all(futures).await
    }

    /// Single verification attempt (no retry)
    async fn try_verify(&self, url: &str) -> Result<(u16, u64), String> {
        let proxy = reqwest::Proxy::all(&self.config.socks_proxy)
            .map_err(|e| format!("Invalid proxy configuration: {}", e))?;

        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .danger_accept_invalid_certs(true) // Accept self-signed certs
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let start = std::time::Instant::now();

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();

        // Consider 2xx, 3xx as success (redirects are expected for Fortify gate)
        if response.status().is_success() || response.status().is_redirection() {
            Ok((status, elapsed_ms))
        } else {
            Err(format!("HTTP error: {}", status))
        }
    }
}

/// Quick verification helper function
#[allow(dead_code)]
pub async fn verify_onion(address: &str, socks_port: u16) -> VerificationResult {
    let config = VerificationConfig::with_socks_port(socks_port);
    let verifier = OnionVerifier::new(config);
    verifier.verify(address).await
}

/// Verify multiple addresses with a progress callback
#[allow(dead_code)]
pub async fn verify_with_progress<F>(
    addresses: &[String],
    socks_port: u16,
    mut on_progress: F,
) -> Vec<VerificationResult>
where
    F: FnMut(usize, usize, &VerificationResult),
{
    let config = VerificationConfig::with_socks_port(socks_port);
    let verifier = OnionVerifier::new(config);
    let total = addresses.len();
    let mut results = Vec::with_capacity(total);

    for (idx, address) in addresses.iter().enumerate() {
        let result = verifier.verify(address).await;
        on_progress(idx + 1, total, &result);
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_config_defaults() {
        let config = VerificationConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_verification_config_with_port() {
        let config = VerificationConfig::with_socks_port(9150);
        assert_eq!(config.socks_proxy, "socks5h://127.0.0.1:9150");
    }
}
