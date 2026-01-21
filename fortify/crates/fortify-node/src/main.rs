use fortify_core::{logging::{init_logging, start_resource_logger}, SessionManager};
use fortify_node::{server::NodeServer, Node, NodeConfig, NodeMode};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing to file and console, plus resource telemetry
    init_logging("fortify-node");
    start_resource_logger("fortify-node", Duration::from_secs(3));

    let mode = match env::var("NODE_MODE")
        .unwrap_or_else(|_| "healthy".to_string())
        .as_str()
    {
        "threat" => NodeMode::Threat,
        _ => NodeMode::Healthy,
    };

    info!("Fortify Node starting in {:?} mode", mode);

    // Load configuration
    let mut config = NodeConfig::default();
    config.mode = mode;

    if let Ok(addr) = env::var("BIND_ADDR") {
        config.bind_addr = addr.parse()?;
    }

    if let Ok(backend) = env::var("BACKEND_ADDR") {
        config.backend_address = backend;
    }

    if let Ok(gate) = env::var("GATE_ADDR") {
        config.gate_address = gate;
    }

    if let Ok(socks) = env::var("FORTIFY_SOCKS_PORT") {
        config.socks_proxy = Some(format!("127.0.0.1:{}", socks));
    }

    // Create session manager (shared secret would come from config)
    let secret = env::var("SECRET_KEY")
        .unwrap_or_else(|_| "fortify-secret-key".to_string())
        .into_bytes();
    let session_manager = Arc::new(SessionManager::new(secret.clone()));

    // Create node
    let node = Arc::new(Node::new(config.clone(), session_manager, secret));

    // Start node
    node.start().await?;

    // Start HTTP server
    let server = NodeServer::new(Arc::clone(&node));

    info!("Node ready on {}", config.bind_addr);

    // Start server (this will block)
    if let Err(e) = server.start(config.bind_addr).await {
        tracing::error!("Server error: {}", e);
    }

    info!("Node shutting down");
    Ok(())
}
