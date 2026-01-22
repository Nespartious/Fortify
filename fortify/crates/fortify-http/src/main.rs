use fortify_core::{
    logging::{init_logging, start_resource_logger},
    SessionManager,
};
use fortify_http::{BackendNode, HttpProxy};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

fn parse_backend_list(
    source: Option<String>,
    fallback: &[&str],
    healthy: bool,
    max_connections: usize,
) -> Vec<BackendNode> {
    let entries: Vec<String> = source
        .map(|value| {
            value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_else(|| fallback.iter().map(|s| s.to_string()).collect());

    entries
        .into_iter()
        .map(|addr| BackendNode::new(addr, healthy, max_connections))
        .collect()
}

/// Parse onion addresses from environment (comma-separated, can be empty strings)
fn parse_onion_list(source: Option<String>) -> Vec<Option<String>> {
    source
        .map(|value| {
            value
                .split(',')
                .map(|s| {
                    let s = s.trim();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("fortify-http");
    start_resource_logger("fortify-http", Duration::from_secs(3));

    let bind_addr: SocketAddr = env::var("PROXY_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8082".to_string())
        .parse()?;
    let max_concurrent = env::var("PROXY_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let max_connections_per_node = env::var("NODE_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let secret_key = env::var("SECRET_KEY")
        .unwrap_or_else(|_| "fortify-secret-key".to_string())
        .into_bytes();

    let healthy_nodes = parse_backend_list(
        env::var("HEALTHY_NODES").ok(),
        &["http://127.0.0.1:8083"],
        true,
        max_connections_per_node,
    );
    let healthy_onions = parse_onion_list(env::var("HEALTHY_ONIONS").ok());

    let threat_nodes = parse_backend_list(
        env::var("THREAT_NODES").ok(),
        &["http://127.0.0.1:8084"],
        false,
        max_connections_per_node,
    );
    let threat_onions = parse_onion_list(env::var("THREAT_ONIONS").ok());

    // Gate address for redirecting unknown users
    let gate_address =
        env::var("GATE_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());

    if healthy_nodes.is_empty() && threat_nodes.is_empty() {
        anyhow::bail!("No backend nodes configured for Fortify HTTP proxy");
    }

    let session_manager = Arc::new(SessionManager::new(secret_key.clone()));
    let proxy = HttpProxy::new_with_onions(
        bind_addr,
        max_concurrent,
        secret_key,
        session_manager,
        healthy_nodes,
        healthy_onions,
        threat_nodes,
        threat_onions,
        gate_address,
    );

    info!("Fortify HTTP proxy listening on {}", bind_addr);

    if let Err(e) = proxy.start().await {
        error!("HTTP proxy error: {}", e);
    }

    Ok(())
}
