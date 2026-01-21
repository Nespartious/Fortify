use crate::{BackendNode, Metrics, ProxyError, Result};
use hyper::{Body, Client, Request, Response, Uri};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Proxy a request to a backend node
pub async fn proxy_request(
    mut req: Request<Body>,
    node: &BackendNode,
    metrics: Arc<Mutex<Metrics>>,
) -> Result<Response<Body>> {
    let start = Instant::now();

    // Acquire connection slot
    if !node.acquire() {
        return Err(ProxyError::BackpressureExceeded);
    }

    // Build backend URI
    let backend_uri = format!(
        "{}{}",
        node.address,
        req.uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/")
    );
    let uri: Uri = backend_uri
        .parse()
        .map_err(|_| ProxyError::BackendUnavailable)?;

    // Update request URI
    *req.uri_mut() = uri;

    // Remove hop-by-hop headers
    remove_hop_by_hop_headers(req.headers_mut());

    // Forward request to backend
    let client = Client::new();
    let response = client.request(req).await.map_err(|_| {
        node.release();
        metrics.lock().unwrap().record_backend_error();
        ProxyError::BackendUnavailable
    })?;

    // Release connection slot
    node.release();

    let duration = start.elapsed();
    tracing::debug!("Proxied request to {} in {:?}", node.address, duration);

    Ok(response)
}

/// Remove hop-by-hop headers that shouldn't be forwarded
fn remove_hop_by_hop_headers(headers: &mut hyper::HeaderMap) {
    let hop_headers = [
        "Connection",
        "Keep-Alive",
        "Proxy-Authenticate",
        "Proxy-Authorization",
        "Te",
        "Trailers",
        "Transfer-Encoding",
        "Upgrade",
    ];

    for header in &hop_headers {
        headers.remove(*header);
    }
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
        let mut active = self.active_requests.lock().unwrap();

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
        *self.active_requests.lock().unwrap()
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> usize {
        let active = *self.active_requests.lock().unwrap();
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
        let mut active = self.active_requests.lock().unwrap();
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
    fn test_remove_hop_by_hop_headers() {
        let mut headers = hyper::HeaderMap::new();
        headers.insert("Connection", "keep-alive".parse().unwrap());
        headers.insert("Keep-Alive", "timeout=5".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("Upgrade", "websocket".parse().unwrap());

        remove_hop_by_hop_headers(&mut headers);

        // Hop-by-hop headers should be removed
        assert!(!headers.contains_key("Connection"));
        assert!(!headers.contains_key("Keep-Alive"));
        assert!(!headers.contains_key("Upgrade"));

        // Normal headers should remain
        assert!(headers.contains_key("Content-Type"));
    }
}
