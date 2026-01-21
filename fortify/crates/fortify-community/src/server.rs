use crate::{crypto::Seed, CommunityNetwork};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

/// Community network HTTP server
pub struct CommunityServer {
    network: Arc<CommunityNetwork>,
}

impl CommunityServer {
    pub fn new(network: Arc<CommunityNetwork>) -> Self {
        Self { network }
    }

    /// Start the HTTP server
    pub async fn start(&self, addr: SocketAddr) -> anyhow::Result<()> {
        let network = Arc::clone(&self.network);

        let make_svc = make_service_fn(move |_conn| {
            let network = Arc::clone(&network);

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    handle_request(req, Arc::clone(&network))
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        tracing::info!("Community server listening on {}", addr);

        server.await?;
        Ok(())
    }
}

/// Handle incoming request
async fn handle_request(
    req: Request<Body>,
    network: Arc<CommunityNetwork>,
) -> std::result::Result<Response<Body>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/community/health") => health_check(),
        (&Method::GET, "/community/seeds") => get_seeds(Arc::clone(&network)).await,
        (&Method::POST, "/community/seeds") => add_seed(req, Arc::clone(&network)).await,
        (&Method::GET, "/community/discover") => discover_peers(Arc::clone(&network)).await,
        (&Method::GET, "/community/metrics") => get_metrics(Arc::clone(&network)).await,
        _ => not_found(),
    };

    tracing::debug!("{} {} - {}", method, path, response.status());

    Ok(response)
}

/// Health check endpoint
fn health_check() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status":"healthy"}"#))
        .unwrap()
}

/// Get seeds endpoint
async fn get_seeds(network: Arc<CommunityNetwork>) -> Response<Body> {
    let seeds = network.get_seeds().await;

    match serde_json::to_string(&seeds) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(json))
            .unwrap(),
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "Serialization failed"),
    }
}

/// Add seed endpoint
async fn add_seed(req: Request<Body>, network: Arc<CommunityNetwork>) -> Response<Body> {
    let body_bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(bytes) => bytes,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Failed to read body"),
    };

    let seed: Seed = match serde_json::from_slice(&body_bytes) {
        Ok(s) => s,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid JSON"),
    };

    match network.add_seed(seed).await {
        Ok(_) => Response::builder()
            .status(StatusCode::CREATED)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"status":"added"}"#))
            .unwrap(),
        Err(e) => error_response(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

/// Discover peers endpoint
async fn discover_peers(network: Arc<CommunityNetwork>) -> Response<Body> {
    match network.discover_peers(20).await {
        Ok(peers) => match serde_json::to_string(&peers) {
            Ok(json) => Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(json))
                .unwrap(),
            Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "Serialization failed"),
        },
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Get metrics endpoint
async fn get_metrics(network: Arc<CommunityNetwork>) -> Response<Body> {
    let metrics = network.get_metrics().await;

    match serde_json::to_string(&metrics) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(json))
            .unwrap(),
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "Serialization failed"),
    }
}

/// 404 response
fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found"))
        .unwrap()
}

/// Error response
fn error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Body::from(message.to_string()))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{crypto::KeyPair, CommunityConfig};

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("healthy"));
    }

    #[tokio::test]
    async fn test_get_seeds() {
        let config = CommunityConfig::default();
        let keypair = KeyPair::generate();
        let network = Arc::new(CommunityNetwork::new(config, keypair));

        let response = get_seeds(network).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let seeds: Vec<Seed> = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(seeds.len(), 0); // Empty initially
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let config = CommunityConfig::default();
        let keypair = KeyPair::generate();
        let network = Arc::new(CommunityNetwork::new(config, keypair));

        let response = get_metrics(network).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("seeds_total"));
    }

    #[test]
    fn test_not_found() {
        let response = not_found();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
