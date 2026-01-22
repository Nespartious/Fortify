use fortify_core::{
    logging::{init_logging, start_resource_logger},
    SessionManager,
};
use fortify_gate::{server::GateServer, Gate};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("fortify-gate");
    start_resource_logger("fortify-gate", Duration::from_secs(3));

    let bind_addr: SocketAddr = env::var("GATE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8081".to_string())
        .parse()?;
    let max_concurrent = env::var("GATE_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let pow_difficulty = env::var("GATE_POW_DIFFICULTY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let verification_timeout = env::var("GATE_VERIFICATION_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(45); // 45 seconds default - tighter window to prevent automation
    let static_dir = env::var("GATE_STATIC_DIR").unwrap_or_else(|_| "assets/html".to_string());
    let secret_key = env::var("SECRET_KEY")
        .unwrap_or_else(|_| "fortify-secret-key".to_string())
        .into_bytes();

    let session_manager = Arc::new(SessionManager::new(secret_key.clone()));
    let gate = Arc::new(Gate::new(
        bind_addr,
        max_concurrent,
        pow_difficulty,
        verification_timeout,
        Arc::clone(&session_manager),
        secret_key,
    ));

    // Periodic cleanup to prune stale verification states
    let cleanup_gate = Arc::clone(&gate);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            cleanup_gate.cleanup();
        }
    });

    let server = GateServer::new(Arc::clone(&gate), static_dir);
    info!("Fortify Gate listening on {}", bind_addr);

    if let Err(e) = server.start(bind_addr).await {
        error!("Gate server error: {}", e);
    }

    Ok(())
}
