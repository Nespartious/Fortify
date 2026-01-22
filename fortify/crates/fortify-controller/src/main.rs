use fortify_controller::{config::ControllerConfig, Controller};
use fortify_core::logging::{init_logging, start_resource_logger};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("fortify-controller");
    start_resource_logger("fortify-controller", Duration::from_secs(3));

    info!("Fortify Controller starting");

    let config = ControllerConfig::from_env()?;
    info!(
        "Configuration: orchestrators={}-{}, healthy_nodes={}-{}, threat_nodes={}-{}",
        config.min_orchestrators,
        config.max_orchestrators,
        config.min_healthy_nodes,
        config.max_healthy_nodes,
        config.min_threat_nodes,
        config.max_threat_nodes
    );

    // Log vanity config for mirrors
    info!(
        "Vanity config (for mirrors): enabled={}, prefix='{}', timeout={}s",
        config.vanity_enabled, config.vanity_prefix, config.vanity_timeout_seconds
    );

    let controller = Controller::new(config);
    controller.start().await?;
    info!("Controller ready");

    tokio::signal::ctrl_c().await?;

    info!("Controller shutting down gracefully");
    controller.shutdown().await?;
    info!("Shutdown complete");

    Ok(())
}
