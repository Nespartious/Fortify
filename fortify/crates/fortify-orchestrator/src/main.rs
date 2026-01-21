use fortify_core::logging::{init_logging, start_resource_logger};
use fortify_orchestrator::{server::OrchestratorServer, Orchestrator, OrchestratorConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with rotation and resource telemetry
    init_logging("fortify-orchestrator");
    start_resource_logger("fortify-orchestrator", Duration::from_secs(3));

    info!("Fortify Orchestrator starting");

    // Load configuration (would load from file in production)
    let mut config = OrchestratorConfig::default();

    if let Ok(addr) = std::env::var("ORCH_BIND_ADDR") {
        config.public_bind_addr = addr;
    }
    if let Ok(gate) = std::env::var("GATE_ADDRESS") {
        config.gate_address = gate;
    }
    if let Ok(proxy_port) = std::env::var("PROXY_PORT") {
        if let Ok(port) = proxy_port.parse() {
            config.proxy_port = port;
        }
    }
    if let Ok(ctrl) = std::env::var("TOR_CONTROL_ADDR") {
        config.tor_control_addr = Some(ctrl);
    }
    if let Ok(cookie) = std::env::var("TOR_COOKIE_PATH") {
        config.tor_cookie_path = Some(PathBuf::from(cookie));
    }
    
    // Get base data directory from controller (or use default)
    let base_data_dir = std::env::var("FORTIFY_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Default to ~/.local/share/fortify if HOME is set
            if let Some(home) = std::env::var_os("HOME") {
                let mut path = PathBuf::from(home);
                path.push(".local");
                path.push("share");
                path.push("fortify");
                path
            } else {
                PathBuf::from("/tmp/fortify")
            }
        });
    
    // Use orchestrator-specific data directory to prevent multiple orchestrators
    // from managing the same mirrors
    if let Ok(orch_id) = std::env::var("ORCH_ID") {
        config.tor_data_dir = base_data_dir.join("tor").join("mirrors").join(format!("orch-{}", orch_id));
        info!("Orchestrator {} using data dir: {:?}", orch_id, config.tor_data_dir);
    } else {
        config.tor_data_dir = base_data_dir.join("tor").join("mirrors");
    }
    
    // Store base data dir in config for torrc path resolution
    config.base_data_dir = Some(base_data_dir);
    
    // Vanity address configuration for mirrors
    if let Ok(val) = std::env::var("VANITY_ENABLED") {
        config.vanity_enabled = val.parse().unwrap_or(false);
    }
    if let Ok(val) = std::env::var("VANITY_PREFIX") {
        config.vanity_prefix = val;
    }
    if let Ok(val) = std::env::var("VANITY_TIMEOUT") {
        config.vanity_timeout = val.parse().unwrap_or(30);
    }
    
    if config.vanity_enabled && !config.vanity_prefix.is_empty() {
        info!("Vanity addresses enabled: prefix='{}', timeout={}s", 
            config.vanity_prefix, config.vanity_timeout);
    }

    // CAPTCHA pool configuration from TUI settings
    if let Ok(val) = std::env::var("CAPTCHA_POOL_SIZE") {
        if let Ok(size) = val.parse() {
            config.multi_daemon.flex_core.captcha_pregen.target_pool_size = size;
            info!("CAPTCHA target pool size: {}", size);
        }
    }
    if let Ok(val) = std::env::var("CAPTCHA_MIN_POOL") {
        if let Ok(size) = val.parse() {
            config.multi_daemon.flex_core.captcha_pregen.min_pool_size = size;
        }
    }
    if let Ok(val) = std::env::var("CAPTCHA_MAX_POOL") {
        if let Ok(size) = val.parse() {
            config.multi_daemon.flex_core.captcha_pregen.max_pool_size = size;
        }
    }
    if let Ok(val) = std::env::var("CAPTCHA_ENABLED") {
        config.multi_daemon.flex_core.captcha_pregen.enabled = val.parse().unwrap_or(true);
    }
    if let Ok(val) = std::env::var("CAPTCHA_ROTATION_PERCENT") {
        if let Ok(pct) = val.parse() {
            config.multi_daemon.flex_core.captcha_pregen.rotation_percent = pct;
        }
    }
    if let Ok(val) = std::env::var("CAPTCHA_ROTATION_DAYS") {
        if let Ok(days) = val.parse() {
            config.multi_daemon.flex_core.captcha_pregen.rotation_interval_days = days;
        }
    }
    if let Ok(val) = std::env::var("CAPTCHA_BATCH_SIZE") {
        if let Ok(size) = val.parse() {
            config.multi_daemon.flex_core.captcha_pregen.batch_size = size;
        }
    }

    // Create orchestrator
    let orchestrator = Arc::new(Orchestrator::new(config.clone()));

    // Start orchestrator
    orchestrator.start().await?;

    // Start HTTP server
    let bind_addr = config
        .public_bind_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid ORCH_BIND_ADDR: {}", e))?;
    let server = OrchestratorServer::new(
        bind_addr,
        config.gate_address.clone(),
        Arc::clone(&orchestrator),
    );

    info!("Orchestrator ready");

    // Clone orchestrator for shutdown handler
    let shutdown_orchestrator = Arc::clone(&orchestrator);
    
    // Start server with shutdown signal handling
    tokio::select! {
        result = server.start() => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }
    
    // Signal background tasks to shutdown and save state
    shutdown_orchestrator.shutdown();

    info!("Orchestrator shutting down");
    Ok(())
}
