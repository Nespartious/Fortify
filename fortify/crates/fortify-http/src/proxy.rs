use fortify_core::safe_lock;
use crate::{BackendNode, Metrics, ProxyError, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Timeout for backend proxy requests (60 seconds)
/// Accommodates Tor latency while preventing slow-loris exhaustion
const BACKEND_REQUEST_TIMEOUT_SECS: u64 = 60;

/// Timeout for establishing connection to backend (10 seconds)
const BACKEND_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Type alias for the response body type
type BoxBody = Full<Bytes>;

/// Proxy a request to a backend node
pub async fn proxy_request(
    req: Request<Incoming>,
    node: &BackendNode,
    metrics: Arc<Mutex<Metrics>>,
) -> Result<Response<BoxBody>> {
    let start = Instant::now();

    // Acquire connection slot
    if !node.acquire() {
        return Err(ProxyError::BackpressureExceeded);
    }

    // Extract all needed info BEFORE consuming the body
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let backend_url = format!("{}{}", node.address, path_and_query);

    let method = match req.method().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    // Collect headers before consuming body
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            let name_str = name.as_str().to_lowercase();
            if !is_hop_by_hop_header(&name_str) {
                value
                    .to_str()
                    .ok()
                    .map(|v| (name.as_str().to_string(), v.to_string()))
            } else {
                None
            }
        })
        .collect();

    // Now collect request body (consumes req)
    let body_bytes = req
        .collect()
        .await
        .map_err(|_| {
            node.release();
            safe_lock(&metrics).record_backend_error();
            ProxyError::BackendUnavailable
        })?
        .to_bytes();

    // Use reqwest for proxying with explicit timeouts
    // Connect timeout prevents slow-loris on connection phase
    // Request timeout prevents hanging on slow/malicious backends
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(BACKEND_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(BACKEND_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build HTTP client: {}", e);
            node.release();
            ProxyError::BackendUnavailable
        })?;
    let mut request_builder = client.request(method, &backend_url);

    // Copy saved headers
    for (name, value) in headers {
        request_builder = request_builder.header(name, value);
    }

    request_builder = request_builder.body(body_bytes.to_vec());

    let response = request_builder.send().await.map_err(|e| {
        node.release();
        safe_lock(&metrics).record_backend_error();
        
        // Distinguish timeout errors for better observability
        if e.is_timeout() {
            tracing::warn!(
                "Backend request to {} timed out after {}s",
                node.address,
                BACKEND_REQUEST_TIMEOUT_SECS
            );
            ProxyError::BackendTimeout(BACKEND_REQUEST_TIMEOUT_SECS)
        } else if e.is_connect() {
            tracing::warn!(
                "Failed to connect to backend {}: {}",
                node.address,
                e
            );
            ProxyError::BackendUnavailable
        } else {
            tracing::warn!("Backend request error: {}", e);
            ProxyError::BackendUnavailable
        }
    })?;

    // Release connection slot
    node.release();

    let duration = start.elapsed();
    tracing::debug!("Proxied request to {} in {:?}", node.address, duration);

    // Convert reqwest response to hyper response
    let status = hyper::StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = Response::builder().status(status);

    for (name, value) in response.headers() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|_| ProxyError::BackendUnavailable)?;

    builder
        .body(Full::new(Bytes::from(body_bytes.to_vec())))
        .map_err(|_| ProxyError::BackendUnavailable)
}

/// Check if a header is a hop-by-hop header
fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name,
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "host"
    )
}

/// Backpressure controller
pub struct BackpressureController {
    max_concurrent: usize,
    active_requests: Arc<Mutex<usize>>,
    /// Reserved for future queuing/waiting implementation
    #[allow(dead_code)]
    queue_wait_timeout: Duration,
}

impl BackpressureController {
    pub fn new(max_concurrent: usize, queue_wait_timeout: Duration) -> Self {
        Self {
            max_concurrent,
            active_requests: Arc::new(Mutex::new(0)),
            queue_wait_timeout,
        }
    }

    /// Try to acquire a request slot
    pub fn try_acquire(&self) -> Result<RequestGuard> {
        let mut active = safe_lock(&self.active_requests);

        if *active >= self.max_concurrent {
            return Err(ProxyError::BackpressureExceeded);
        }

        *active += 1;
        Ok(RequestGuard {
            active_requests: Arc::clone(&self.active_requests),
        })
    }

    /// Get current active request count
    pub fn active_count(&self) -> usize {
        *safe_lock(&self.active_requests)
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> usize {
        let active = *safe_lock(&self.active_requests);
        self.max_concurrent.saturating_sub(active)
    }

    /// Check if at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.available_capacity() == 0
    }
}

/// RAII guard for request slots
pub struct RequestGuard {
    active_requests: Arc<Mutex<usize>>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        let mut active = safe_lock(&self.active_requests);
        if *active > 0 {
            *active -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_controller() {
        let controller = BackpressureController::new(3, Duration::from_secs(5));

        assert_eq!(controller.active_count(), 0);
        assert_eq!(controller.available_capacity(), 3);
        assert!(!controller.is_at_capacity());

        // Acquire slots
        let _guard1 = controller.try_acquire().unwrap();
        assert_eq!(controller.active_count(), 1);

        let _guard2 = controller.try_acquire().unwrap();
        assert_eq!(controller.active_count(), 2);

        let _guard3 = controller.try_acquire().unwrap();
        assert_eq!(controller.active_count(), 3);
        assert!(controller.is_at_capacity());

        // Should fail when at capacity
        assert!(controller.try_acquire().is_err());

        // Release one slot
        drop(_guard1);
        assert_eq!(controller.active_count(), 2);
        assert!(!controller.is_at_capacity());

        // Should succeed now
        let _guard4 = controller.try_acquire().unwrap();
        assert_eq!(controller.active_count(), 3);
    }

    #[test]
    fn test_request_guard_auto_release() {
        let controller = BackpressureController::new(2, Duration::from_secs(5));

        {
            let _guard = controller.try_acquire().unwrap();
            assert_eq!(controller.active_count(), 1);
            // Guard drops here
        }

        assert_eq!(controller.active_count(), 0);
    }

    #[test]
    fn test_is_hop_by_hop_header() {
        assert!(is_hop_by_hop_header("connection"));
        assert!(is_hop_by_hop_header("keep-alive"));
        assert!(is_hop_by_hop_header("upgrade"));
        assert!(!is_hop_by_hop_header("content-type"));
        assert!(!is_hop_by_hop_header("accept"));
    }
}
