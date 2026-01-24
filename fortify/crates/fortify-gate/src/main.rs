use fortify_core::{
    logging::{init_logging, start_resource_logger},
    templates::BrandingVars,
    SessionManager,
};
use fortify_gate::{
    captcha_types::{CaptchaConfig, CaptchaType},
    server::GateServer,
    Gate,
};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// Parse CaptchaType from string (matches Rust Debug format)
fn parse_captcha_type(s: &str) -> CaptchaType {
    match s.to_lowercase().as_str() {
        "bmptext" => CaptchaType::BmpText,
        "emoji" => CaptchaType::Emoji,
        "direction" => CaptchaType::Direction,
        "sequence" => CaptchaType::Sequence,
        "wordunscramble" => CaptchaType::WordUnscramble,
        "imagerotation" => CaptchaType::ImageRotation,
        "silhouette" => CaptchaType::Silhouette,
        _ => CaptchaType::BmpText, // Default fallback
    }
}

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

    // Load CAPTCHA type settings from environment (passed by controller)
    let gate_captcha_type = env::var("CAPTCHA_GATE_TYPE")
        .map(|s| parse_captcha_type(&s))
        .unwrap_or(CaptchaType::BmpText);
    let threat_captcha_type = env::var("CAPTCHA_THREAT_TYPE")
        .map(|s| parse_captcha_type(&s))
        .unwrap_or(CaptchaType::BmpText);
    let threat_captcha_enabled = env::var("CAPTCHA_THREAT_ENABLED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(true);
    let captcha_difficulty = env::var("CAPTCHA_DIFFICULTY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5u8);
    let captcha_timeout = env::var("CAPTCHA_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120u64);

    info!(
        "CAPTCHA config: gate_type={:?}, threat_type={:?}, threat_enabled={}, difficulty={}, timeout={}s",
        gate_captcha_type, threat_captcha_type, threat_captcha_enabled, captcha_difficulty, captcha_timeout
    );

    // Load branding from environment variables (passed by controller)
    let branding = BrandingVars::from_env();
    info!(
        "Gate branding: service_name='{}', primary_color='{}'",
        branding.service_name, branding.primary_color
    );

    let session_manager = Arc::new(SessionManager::new(secret_key.clone()));
    let gate = Arc::new(Gate::with_branding(
        bind_addr,
        max_concurrent,
        pow_difficulty,
        verification_timeout,
        Arc::clone(&session_manager),
        secret_key,
        branding,
    ));

    // Apply CAPTCHA configuration from environment
    let captcha_config = CaptchaConfig {
        gate_captcha_type,
        threat_captcha_type,
        threat_captcha_enabled,
        ..CaptchaConfig::default()
    };
    gate.update_captcha_config(captcha_config);

    // Store difficulty and timeout for later use (Gate uses pow_difficulty and verification_timeout)
    // Note: The pow_difficulty and verification_timeout are already passed to Gate::with_branding
    // CAPTCHA_DIFFICULTY maps to visual captcha difficulty (1-10 scale)
    // CAPTCHA_TIMEOUT is the time allowed to solve (seconds)
    let _ = (captcha_difficulty, captcha_timeout); // Currently stored in Gate constructor params

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
