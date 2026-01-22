use crate::Node;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Node HTTP server
pub struct NodeServer {
    node: Arc<Node>,
}

impl NodeServer {
    pub fn new(node: Arc<Node>) -> Self {
        Self { node }
    }

    /// Start the HTTP server
    pub async fn start(&self, addr: SocketAddr) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        tracing::info!("Node HTTP server listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let node = Arc::clone(&self.node);

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    handle_request(req, Arc::clone(&node))
                });

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    tracing::error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

/// Handle incoming request
async fn handle_request(
    req: Request<Incoming>,
    node: Arc<Node>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let response = match (req.method().as_str(), req.uri().path()) {
        ("GET", "/health") => health_check(Arc::clone(&node)),
        ("GET", "/metrics") => metrics_endpoint(Arc::clone(&node)),
        _ => {
            // Extract session ID from header
            let session_id = match extract_session_id(&req) {
                Some(id) => id,
                None => {
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        "Missing X-Session-ID header",
                    ));
                }
            };

            // Process request
            node.process_request(session_id, req).await.unwrap()
        }
    };

    tracing::debug!("{} {} - {}", method, path, response.status());

    Ok(response)
}

/// Health check endpoint
fn health_check(node: Arc<Node>) -> Response<Full<Bytes>> {
    let metrics = node.get_metrics();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::json!({
                "status": "healthy",
                "requests_total": metrics.requests_total,
                "requests_forwarded": metrics.requests_forwarded,
            })
            .to_string(),
        )))
        .unwrap()
}

/// Metrics endpoint
fn metrics_endpoint(node: Arc<Node>) -> Response<Full<Bytes>> {
    let metrics = node.get_metrics();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_string(&serde_json::json!({
                "requests_total": metrics.requests_total,
                "requests_forwarded": metrics.requests_forwarded,
                "requests_blocked": metrics.requests_blocked,
                "violations_detected": metrics.violations_detected,
                "sessions_demoted": metrics.sessions_demoted,
                "sessions_promoted": metrics.sessions_promoted,
                "backend_errors": metrics.backend_errors,
                "average_response_time_ms": metrics.average_response_time_ms,
            }))
            .unwrap(),
        )))
        .unwrap()
}

/// Extract session ID from request header or cookie
fn extract_session_id(req: &Request<Incoming>) -> Option<String> {
    // Priority 1: X-Session-ID header (API/Programmatic)
    if let Some(id) = req
        .headers()
        .get("X-Session-ID")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        return Some(id);
    }

    // Priority 2: Cookie (Browser)
    if let Some(cookie_header) = req.headers().get(hyper::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(val) = cookie.strip_prefix("fortify_session=") {
                    return Some(val.to_string());
                }
            }
        }
    }

    None
}

/// Generate error response
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeConfig;
    use fortify_core::SessionManager;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_health_check() {
        let secret = b"test-secret";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
        let config = NodeConfig::default();
        let node = Arc::new(Node::new(config, session_manager, secret.to_vec()));

        let response = health_check(node);
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("status"));
        assert!(body_str.contains("healthy"));
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let secret = b"test-secret";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
        let config = NodeConfig::default();
        let node = Arc::new(Node::new(config, session_manager, secret.to_vec()));

        let response = metrics_endpoint(node);
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("requests_total"));
        assert!(body_str.contains("violations_detected"));
    }
}
