pub mod config;
mod health;
mod http;
mod mirror_health;
pub mod resource;
pub mod scaling;
pub mod service;
pub mod tor;
pub mod vanguards;

use config::ControllerConfig;
use health::HealthChecker;
use http::spawn_http_server;
use mirror_health::MirrorHealthChecker;
use resource::ResourceMonitor;
use scaling::ScalingPolicy;
use serde::{Deserialize, Serialize};
use service::{ServiceManager, ServiceType};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{atomic::AtomicUsize, atomic::Ordering, Arc, Mutex as SyncMutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;
use tor::TorManager;
use vanguards::{VanguardsConfig, VanguardsManager, VanguardsStatus};

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Service error: {0}")]
    ServiceError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Controller metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControllerMetrics {
    pub services_running: usize,
    pub services_failed: usize,
    pub services_restarted: usize,
    pub scaling_events: usize,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u64,
    pub total_memory_mb: u64,
    pub vanguards_status: String,
    pub vanguards_uptime_secs: Option<u64>,
}

/// Main controller for managing all Fortify services
pub struct Controller {
    config: ControllerConfig,
    service_manager: Arc<Mutex<ServiceManager>>,
    resource_monitor: Arc<Mutex<ResourceMonitor>>,
    scaling_policy: Arc<ScalingPolicy>,
    metrics: Arc<Mutex<ControllerMetrics>>,
    orchestrator_env: Arc<OrchestratorEnvFactory>,
    healthy_node_env: Arc<NodeEnvFactory>,
    threat_node_env: Arc<NodeEnvFactory>,
    vanguards_manager: Arc<Mutex<VanguardsManager>>,
    /// Session blacklist: session_id -> (expiry_time, demotion_count)
    /// Prevents demoted sessions from reusing tokens
    session_blacklist: Arc<SyncMutex<HashMap<String, (Instant, u8)>>>,
}

impl Controller {
    pub fn new(config: ControllerConfig) -> Self {
        // Create vanguards config from controller config
        let (tor_host, tor_port) = if let Some(ref addr) = config.tor_control_addr {
            let parts: Vec<&str> = addr.split(':').collect();
            let host = parts
                .get(0)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "127.0.0.1".to_string());
            let port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(9151);
            (host, port)
        } else {
            ("127.0.0.1".to_string(), 9151)
        };

        let vanguards_config = VanguardsConfig {
            enabled: config.vanguards_enabled,
            tor_control_addr: tor_host,
            tor_control_port: tor_port,
            layer2_guards: config.vanguards_layer2_guards,
            layer3_guards: config.vanguards_layer3_guards,
            circ_max_age_hours: config.vanguards_circ_max_age_hours,
            circ_max_megabytes: config.vanguards_circ_max_megabytes,
            ..VanguardsConfig::default()
        };

        Self {
            config: config.clone(),
            service_manager: Arc::new(Mutex::new(ServiceManager::new())),
            resource_monitor: Arc::new(Mutex::new(ResourceMonitor::new())),
            scaling_policy: Arc::new(ScalingPolicy::new(
                config.min_orchestrators,
                config.max_orchestrators,
                config.min_healthy_nodes,
                config.max_healthy_nodes,
            )),
            metrics: Arc::new(Mutex::new(ControllerMetrics::default())),
            orchestrator_env: Arc::new(OrchestratorEnvFactory::new(&config)),
            healthy_node_env: Arc::new(NodeEnvFactory::new_healthy(&config)),
            threat_node_env: Arc::new(NodeEnvFactory::new_threat(&config)),
            vanguards_manager: Arc::new(Mutex::new(VanguardsManager::new(vanguards_config))),
            session_blacklist: Arc::new(SyncMutex::new(HashMap::new())),
        }
    }

    /// Start the controller
    pub async fn start(&self) -> Result<(), ControllerError> {
        tracing::info!("Starting Fortify Controller");

        // Start vanguards if enabled
        self.start_vanguards().await?;

        // Start initial services
        self.start_initial_services().await?;

        // Start controller HTTP API
        self.start_http_api()?;

        // Start backend health checker for circuit pre-warming
        self.start_backend_health_checker().await;

        // Start mirror health checker
        self.start_mirror_health_checker().await;

        // Start monitoring tasks
        self.start_monitoring().await;

        // Start scaling task
        self.start_scaling().await;

        Ok(())
    }

    fn start_http_api(&self) -> Result<(), ControllerError> {
        let addr: SocketAddr = self.config.controller_bind_addr.parse().map_err(|e| {
            ControllerError::ConfigError(format!("Invalid controller bind addr: {}", e))
        })?;

        let manager = Arc::clone(&self.service_manager);
        spawn_http_server(addr, manager).map_err(|e| ControllerError::ServiceError(e.to_string()))
    }

    /// Start initial services
    async fn start_initial_services(&self) -> Result<(), ControllerError> {
        let mut manager = self.service_manager.lock().await;

        // Start Gate (single instance)
        manager.spawn(
            ServiceType::Gate,
            self.config.gate_bind_addr.clone(),
            self.gate_env(),
        )?;

        // Start Healthy Node pool (10 nodes by default)
        for i in 0..self.config.min_healthy_nodes {
            let env = self
                .healthy_node_env
                .next_env()
                .map_err(|e| ControllerError::ConfigError(e))?;
            manager.spawn(ServiceType::Node, format!("healthy-{}", i), env)?;
        }

        // Start Threat Node pool (3 nodes by default)
        for i in 0..self.config.min_threat_nodes {
            let env = self
                .threat_node_env
                .next_env()
                .map_err(|e| ControllerError::ConfigError(e))?;
            manager.spawn(ServiceType::Node, format!("threat-{}", i), env)?;
        }

        // Start HTTP Proxy (single instance)
        manager.spawn(
            ServiceType::HttpProxy,
            self.config.proxy_bind_addr.clone(),
            self.proxy_env(),
        )?;

        // Start minimum orchestrators
        for i in 0..self.config.min_orchestrators {
            let env = self
                .orchestrator_env
                .next_env()
                .map_err(|e| ControllerError::ConfigError(e))?;
            manager.spawn(
                ServiceType::Orchestrator,
                format!("orchestrator-{}", i),
                env,
            )?;
        }

        tracing::info!(
            "Started {} orchestrators, {} healthy nodes, {} threat nodes, gate, and proxy",
            self.config.min_orchestrators,
            self.config.min_healthy_nodes,
            self.config.min_threat_nodes
        );

        Ok(())
    }

    /// Start vanguards addon if enabled and available
    async fn start_vanguards(&self) -> Result<(), ControllerError> {
        if !self.config.vanguards_enabled {
            tracing::info!("Vanguards is disabled, skipping");
            return Ok(());
        }

        // Check if vanguards is available
        if !VanguardsManager::is_available() {
            tracing::warn!("Vanguards addon not found. Install with: pip3 install vanguards");
            tracing::warn!(
                "Continuing without vanguards protection (guard discovery attacks possible)"
            );
            return Ok(());
        }

        let mut vanguards = self.vanguards_manager.lock().await;
        match vanguards.start() {
            Ok(()) => {
                tracing::info!("Vanguards addon started successfully");
                tracing::info!(
                    "  Layer 2 guards: {}, Layer 3 guards: {}",
                    self.config.vanguards_layer2_guards,
                    self.config.vanguards_layer3_guards
                );
            }
            Err(e) => {
                tracing::error!("Failed to start vanguards: {}", e);
                tracing::warn!("Continuing without vanguards protection");
            }
        }

        Ok(())
    }

    /// Start backend health checker for circuit pre-warming
    async fn start_backend_health_checker(&self) {
        // Get backend URL from environment
        let backend_url = std::env::var("NODE_BACKEND_ADDR")
            .unwrap_or_else(|_| "http://backend.onion".to_string());

        // Only start if backend is a .onion address
        if !backend_url.contains(".onion") {
            tracing::warn!(
                "Backend health checker disabled: backend is not a .onion address ({})",
                backend_url
            );
            return;
        }

        tracing::info!("Starting backend health checker for circuit pre-warming");

        match HealthChecker::new(backend_url.clone()) {
            Ok(checker) => {
                tokio::spawn(async move {
                    checker.run().await;
                });
                tracing::info!("Backend health checker started successfully");
            }
            Err(e) => {
                tracing::error!("Failed to start backend health checker: {}", e);
            }
        }
    }

    /// Start mirror health checker
    async fn start_mirror_health_checker(&self) {
        tracing::info!("Starting mirror health checker");

        match MirrorHealthChecker::new() {
            Ok(checker) => {
                // Use first orchestrator for fetching mirror list
                let orchestrator_url = "http://127.0.0.1:8080/mirrors/extended".to_string();

                tokio::spawn(async move {
                    checker.run(orchestrator_url).await;
                });
                tracing::info!("Mirror health checker started successfully");
            }
            Err(e) => {
                tracing::error!("Failed to start mirror health checker: {}", e);
            }
        }
    }

    /// Start monitoring tasks
    async fn start_monitoring(&self) {
        let resource_monitor = Arc::clone(&self.resource_monitor);
        let service_manager = Arc::clone(&self.service_manager);
        let metrics = Arc::clone(&self.metrics);
        let vanguards_manager = Arc::clone(&self.vanguards_manager);
        let check_interval = self.config.health_check_interval;

        // Resource monitoring task
        let resource_metrics = Arc::clone(&metrics);
        let vanguards_metrics = Arc::clone(&vanguards_manager);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;

                let mut monitor = resource_monitor.lock().await;
                monitor.update();

                let mut m = resource_metrics.lock().await;
                m.cpu_usage_percent = monitor.cpu_usage_percent();
                m.memory_usage_mb = monitor.memory_used_mb();
                m.total_memory_mb = monitor.memory_total_mb();

                // Update vanguards metrics
                let vg = vanguards_metrics.lock().await;
                m.vanguards_status = format!("{:?}", vg.status());
                m.vanguards_uptime_secs = vg.uptime_secs();

                // Check for attacks detected by vanguards
                let alerts = vg.check_for_attacks();
                for alert in alerts {
                    tracing::warn!("Vanguards alert: {}", alert);
                }
            }
        });

        // Session blacklist cleanup task
        let blacklist_cleanup = Arc::clone(&self.session_blacklist);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                let mut blacklist = blacklist_cleanup.lock().unwrap();
                let now = Instant::now();
                let before = blacklist.len();

                // Remove expired entries
                blacklist.retain(|_, (expiry, _)| *expiry > now);

                // Enforce 72-hour retention limit
                let max_age = Duration::from_secs(72 * 3600);
                blacklist.retain(|_, (expiry, _)| expiry.saturating_duration_since(now) < max_age);

                // Cap at 10K entries (remove oldest 20% if exceeded)
                if blacklist.len() > 10_000 {
                    let mut entries: Vec<_> =
                        blacklist.iter().map(|(k, v)| (k.clone(), *v)).collect();
                    entries.sort_by_key(|(_, (expiry, _))| *expiry);
                    let to_remove = entries.len() / 5;
                    let keys_to_remove: Vec<String> = entries
                        .iter()
                        .take(to_remove)
                        .map(|(k, _)| k.clone())
                        .collect();
                    for session_id in keys_to_remove {
                        blacklist.remove(&session_id);
                    }
                    tracing::warn!(
                        "Blacklist exceeded 10K entries, removed oldest {}",
                        to_remove
                    );
                }

                let after = blacklist.len();
                if before > after {
                    tracing::debug!("Cleaned blacklist: {} -> {} entries", before, after);
                }
            }
        });

        // Vanguards health checking task
        let vanguards_health = Arc::clone(&vanguards_manager);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                let mut vg = vanguards_health.lock().await;
                if !vg.is_alive() && vg.status() == VanguardsStatus::Running {
                    tracing::warn!("Vanguards process died, attempting restart");
                    if let Err(e) = vg.restart() {
                        tracing::error!("Failed to restart vanguards: {}", e);
                    }
                }
            }
        });

        // Health checking task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;

                let mut manager = service_manager.lock().await;
                let failed = manager.check_health().await;

                // Restart failed services
                for service_id in failed {
                    tracing::warn!("Service {} failed, restarting", service_id);
                    if let Err(e) = manager.restart(&service_id).await {
                        tracing::error!("Failed to restart service {}: {}", service_id, e);
                    } else {
                        let mut m = metrics.lock().await;
                        m.services_restarted += 1;
                    }
                }

                // Update metrics
                let mut m = metrics.lock().await;
                m.services_running = manager.running_count();
                m.services_failed = manager.failed_count();
            }
        });
    }

    /// Start scaling task
    async fn start_scaling(&self) {
        let service_manager = Arc::clone(&self.service_manager);
        let resource_monitor = Arc::clone(&self.resource_monitor);
        let scaling_policy = Arc::clone(&self.scaling_policy);
        let metrics = Arc::clone(&self.metrics);
        let scaling_interval = self.config.scaling_check_interval;
        let orchestrator_env = Arc::clone(&self.orchestrator_env);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(scaling_interval);
            loop {
                interval.tick().await;

                let monitor = resource_monitor.lock().await;
                let mut manager = service_manager.lock().await;

                // Get current counts
                let orchestrator_count = manager.count_by_type(ServiceType::Orchestrator);

                // Check if should scale orchestrators
                if let Some(decision) = scaling_policy.should_scale_orchestrators(
                    orchestrator_count,
                    monitor.cpu_usage_percent(),
                    monitor.memory_usage_percent(),
                ) {
                    match decision {
                        scaling::ScalingDecision::ScaleUp => {
                            tracing::info!("Scaling up orchestrators");
                            let env = match orchestrator_env.next_env() {
                                Ok(env) => env,
                                Err(e) => {
                                    tracing::error!("Failed to allocate orchestrator port: {}", e);
                                    continue;
                                }
                            };
                            if let Err(e) = manager.spawn(
                                ServiceType::Orchestrator,
                                format!("orchestrator-{}", orchestrator_count),
                                env,
                            ) {
                                tracing::error!("Failed to scale up orchestrator: {}", e);
                            } else {
                                let mut m = metrics.lock().await;
                                m.scaling_events += 1;
                            }
                        }
                        scaling::ScalingDecision::ScaleDown => {
                            tracing::info!("Scaling down orchestrators");
                            if let Some(id) = manager.select_for_removal(ServiceType::Orchestrator)
                            {
                                if let Err(e) = manager.stop(&id).await {
                                    tracing::error!("Failed to scale down orchestrator: {}", e);
                                } else {
                                    let mut m = metrics.lock().await;
                                    m.scaling_events += 1;
                                }
                            }
                        }
                    }
                }

                // Node auto-scaling is currently disabled until the proxy gains
                // dynamic backend reconfiguration support.
            }
        });
    }

    /// Add session to blacklist with progressive penalties
    /// 1st demotion: 60 seconds, 2nd: 300 seconds (5 min), 3rd: 1800 seconds (30 min)
    pub fn add_to_blacklist(&self, session_id: String, demotion_count: u8) {
        let duration_secs = match demotion_count {
            1 => 60,
            2 => 300,
            3 => 1800,
            _ => 1800, // Cap at 30 minutes
        };

        let expiry = Instant::now() + Duration::from_secs(duration_secs);
        let mut blacklist = self.session_blacklist.lock().unwrap();
        blacklist.insert(session_id.clone(), (expiry, demotion_count));

        tracing::info!(
            "Session {} blacklisted for {} seconds (demotion #{})",
            session_id,
            duration_secs,
            demotion_count
        );
    }

    /// Check if session is blacklisted
    pub fn is_blacklisted(&self, session_id: &str) -> bool {
        let blacklist = self.session_blacklist.lock().unwrap();

        if let Some((expiry, _)) = blacklist.get(session_id) {
            if Instant::now() < *expiry {
                return true; // Still blacklisted
            }
            // Expired, will be cleaned up later
        }

        false
    }

    /// Get reference to session blacklist for cleanup tasks
    pub fn get_blacklist(&self) -> Arc<SyncMutex<HashMap<String, (Instant, u8)>>> {
        Arc::clone(&self.session_blacklist)
    }

    /// Cleanup expired blacklist entries (called periodically)
    /// Also enforces 72-hour hard limit and 10K entry cap
    pub fn cleanup_blacklist(&self) {
        let mut blacklist = self.session_blacklist.lock().unwrap();
        let now = Instant::now();
        let seventy_two_hours = Duration::from_secs(72 * 60 * 60);
        let oldest_allowed = now.checked_sub(seventy_two_hours).unwrap_or(now);

        // Remove expired entries and enforce 72-hour limit
        blacklist.retain(|_, (expiry, _)| *expiry > now && *expiry > oldest_allowed);

        // If still over 10K entries, remove oldest 20%
        if blacklist.len() > 10_000 {
            let to_remove = blacklist.len() / 5; // Remove 20%
            let mut entries: Vec<_> = blacklist.iter().map(|(k, v)| (k.clone(), *v)).collect();
            entries.sort_by_key(|(_, (expiry, _))| *expiry);

            let keys_to_remove: Vec<String> = entries
                .iter()
                .take(to_remove)
                .map(|(k, _)| k.clone())
                .collect();
            for session_id in keys_to_remove {
                blacklist.remove(&session_id);
            }

            tracing::warn!(
                "Blacklist over capacity, removed {} oldest entries (size now: {})",
                to_remove,
                blacklist.len()
            );
        }
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<(), ControllerError> {
        tracing::info!("Starting graceful shutdown");

        // Stop vanguards first
        let mut vanguards = self.vanguards_manager.lock().await;
        if let Err(e) = vanguards.stop() {
            tracing::warn!("Failed to stop vanguards: {}", e);
        }
        drop(vanguards);

        let mut manager = self.service_manager.lock().await;
        manager.shutdown_all().await?;

        tracing::info!("Shutdown complete");
        Ok(())
    }

    /// Get controller metrics
    pub async fn get_metrics(&self) -> ControllerMetrics {
        self.metrics.lock().await.clone()
    }

    fn gate_env(&self) -> Vec<String> {
        vec![
            format!("GATE_BIND_ADDR={}", self.config.gate_bind_addr),
            format!("SECRET_KEY={}", self.config.secret_key),
        ]
    }

    fn proxy_env(&self) -> Vec<String> {
        let mut env = vec![
            format!("PROXY_BIND_ADDR={}", self.config.proxy_bind_addr),
            format!("SECRET_KEY={}", self.config.secret_key),
            format!("GATE_ADDRESS=http://{}", self.config.gate_bind_addr),
            format!("NODE_BACKEND_ADDR={}", self.config.node_backend_addr),
        ];

        // Healthy nodes for verified traffic
        let healthy_allocations = self.healthy_node_env.node_allocations();
        if !healthy_allocations.is_empty() {
            let addrs: Vec<String> = healthy_allocations
                .iter()
                .map(|a| a.local_addr.clone())
                .collect();
            env.push(format!("HEALTHY_NODES={}", addrs.join(",")));

            // Also pass onion addresses for admin panel display
            let onions: Vec<String> = healthy_allocations
                .iter()
                .map(|a| a.onion_addr.clone().unwrap_or_default())
                .collect();
            env.push(format!("HEALTHY_ONIONS={}", onions.join(",")));
        }

        // Threat nodes for suspicious/unknown traffic
        let threat_allocations = self.threat_node_env.node_allocations();
        if !threat_allocations.is_empty() {
            let addrs: Vec<String> = threat_allocations
                .iter()
                .map(|a| a.local_addr.clone())
                .collect();
            env.push(format!("THREAT_NODES={}", addrs.join(",")));

            let onions: Vec<String> = threat_allocations
                .iter()
                .map(|a| a.onion_addr.clone().unwrap_or_default())
                .collect();
            env.push(format!("THREAT_ONIONS={}", onions.join(",")));
        } else {
            // Fallback to Gate if no threat nodes
            env.push(format!(
                "THREAT_NODES=http://{}",
                self.config.gate_bind_addr
            ));
        }

        env
    }
}

struct OrchestratorEnvFactory {
    base_addr: String,
    gate_url: String,
    proxy_port: u16,
    /// Base data directory for Fortify (passed to orchestrators)
    data_dir: std::path::PathBuf,
    tor_control_addr: Option<String>,
    tor_cookie_path: Option<String>,
    /// Vanity configuration for mirror addresses
    vanity_enabled: bool,
    vanity_prefix: String,
    vanity_timeout: u64,
    /// CAPTCHA configuration
    captcha_enabled: bool,
    captcha_pool_size: usize,
    captcha_min_pool: usize,
    captcha_max_pool: usize,
    captcha_rotation_percent: u8,
    captcha_rotation_days: u32,
    next_offset: AtomicUsize,
}

impl OrchestratorEnvFactory {
    const PORT_STRIDE: usize = 100;

    fn new(config: &ControllerConfig) -> Self {
        let proxy_port = config
            .proxy_bind_addr
            .parse::<SocketAddr>()
            .map(|s| s.port())
            .unwrap_or(8082);

        Self {
            base_addr: config.orchestrator_bind_addr.clone(),
            gate_url: format!("http://{}", config.gate_bind_addr),
            proxy_port,
            data_dir: config.data_dir.clone(),
            tor_control_addr: config.tor_control_addr.clone(),
            tor_cookie_path: config.tor_cookie_path.clone(),
            vanity_enabled: config.vanity_enabled,
            vanity_prefix: config.vanity_prefix.clone(),
            vanity_timeout: config.vanity_timeout_seconds,
            captcha_enabled: config.captcha_enabled,
            captcha_pool_size: config.captcha_pool_size,
            captcha_min_pool: config.captcha_min_pool,
            captcha_max_pool: config.captcha_max_pool,
            captcha_rotation_percent: config.captcha_rotation_percent,
            captcha_rotation_days: config.captcha_rotation_days,
            next_offset: AtomicUsize::new(0),
        }
    }

    fn next_env(&self) -> Result<Vec<String>, String> {
        let offset = self.next_offset.fetch_add(1, Ordering::SeqCst);
        self.build_env(offset)
    }

    fn build_env(&self, offset: usize) -> Result<Vec<String>, String> {
        let addr = Self::format_addr(&self.base_addr, offset)?;
        let mut env = vec![
            format!("ORCH_BIND_ADDR={}", addr),
            format!("GATE_ADDRESS={}", self.gate_url),
            format!("PROXY_PORT={}", self.proxy_port),
            format!("ORCH_ID={}", offset),
            format!("FORTIFY_DATA_DIR={}", self.data_dir.display()),
        ];

        if let Some(ctrl) = &self.tor_control_addr {
            env.push(format!("TOR_CONTROL_ADDR={}", ctrl));
        }

        if let Some(cookie) = &self.tor_cookie_path {
            env.push(format!("TOR_COOKIE_PATH={}", cookie));
        }

        // Vanity configuration for mirror addresses
        if self.vanity_enabled && !self.vanity_prefix.is_empty() {
            env.push("VANITY_ENABLED=true".to_string());
            env.push(format!("VANITY_PREFIX={}", self.vanity_prefix));
            env.push(format!("VANITY_TIMEOUT={}", self.vanity_timeout));
        }

        // CAPTCHA configuration
        env.push(format!("CAPTCHA_ENABLED={}", self.captcha_enabled));
        env.push(format!("CAPTCHA_POOL_SIZE={}", self.captcha_pool_size));
        env.push(format!("CAPTCHA_MIN_POOL={}", self.captcha_min_pool));
        env.push(format!("CAPTCHA_MAX_POOL={}", self.captcha_max_pool));
        env.push(format!(
            "CAPTCHA_ROTATION_PERCENT={}",
            self.captcha_rotation_percent
        ));
        env.push(format!(
            "CAPTCHA_ROTATION_DAYS={}",
            self.captcha_rotation_days
        ));

        Ok(env)
    }

    fn format_addr(base: &str, offset: usize) -> Result<String, String> {
        let mut socket: SocketAddr = base
            .parse()
            .map_err(|e| format!("Invalid ORCH_BIND_ADDR {}: {}", base, e))?;
        let current = socket.port() as usize;
        let bump = offset
            .checked_mul(Self::PORT_STRIDE)
            .ok_or_else(|| "Orchestrator port overflow".to_string())?;
        let target = current
            .checked_add(bump)
            .ok_or_else(|| "Orchestrator port overflow".to_string())?;
        if target > u16::MAX as usize {
            return Err("Orchestrator port exceeds u16 range".into());
        }
        socket.set_port(target as u16);
        Ok(socket.to_string())
    }
}

/// Node allocation info with local address and optional onion address
#[derive(Clone, Debug)]
struct NodeAllocation {
    local_addr: String,
    onion_addr: Option<String>,
}

struct NodeEnvFactory {
    base_addr: SocketAddr,
    backend_addr: String,
    secret_key: String,
    mode: String,
    allocations: SyncMutex<Vec<NodeAllocation>>,
    tor_manager: Option<TorManager>,
}

impl NodeEnvFactory {
    fn new_healthy(config: &ControllerConfig) -> Self {
        let base_addr = config
            .healthy_node_bind_base
            .parse()
            .expect("healthy_node_bind_base validated");

        // Nodes do not use vanity addresses - only mirrors do
        let tor_manager = if let (Some(ctrl), Some(cookie)) =
            (&config.tor_control_addr, &config.tor_cookie_path)
        {
            Some(TorManager::new(ctrl.clone(), cookie.clone()))
        } else {
            None
        };

        Self {
            base_addr,
            backend_addr: config.node_backend_addr.clone(),
            secret_key: config.secret_key.clone(),
            mode: "healthy".to_string(),
            allocations: SyncMutex::new(Vec::new()),
            tor_manager,
        }
    }

    fn new_threat(config: &ControllerConfig) -> Self {
        let base_addr = config
            .threat_node_bind_base
            .parse()
            .expect("threat_node_bind_base validated");

        // Nodes do not use vanity addresses - only mirrors do
        let tor_manager = if let (Some(ctrl), Some(cookie)) =
            (&config.tor_control_addr, &config.tor_cookie_path)
        {
            Some(TorManager::new(ctrl.clone(), cookie.clone()))
        } else {
            None
        };

        Self {
            base_addr,
            backend_addr: config.node_backend_addr.clone(),
            secret_key: config.secret_key.clone(),
            mode: "threat".to_string(),
            allocations: SyncMutex::new(Vec::new()),
            tor_manager,
        }
    }

    fn next_env(&self) -> Result<Vec<String>, String> {
        let mut allocations = self
            .allocations
            .lock()
            .expect("node allocation mutex poisoned");
        let offset = allocations.len();
        let addr = self.next_bind_addr(offset)?;
        let addr_str = addr.to_string();

        // Try to create a Tor hidden service for this node
        let onion_addr = if let Some(ref tor) = self.tor_manager {
            match tor.create_hidden_service(addr.port()) {
                Ok(onion) => {
                    tracing::info!(
                        "Created onion {} for {} node on {}",
                        onion,
                        self.mode,
                        addr_str
                    );
                    Some(onion)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create onion for {} node on {}: {}",
                        self.mode,
                        addr_str,
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        allocations.push(NodeAllocation {
            local_addr: format!("http://{}", addr_str),
            onion_addr: onion_addr.clone(),
        });

        let mut env = vec![
            format!("BIND_ADDR={}", addr_str),
            format!("BACKEND_ADDR={}", self.backend_addr),
            format!("SECRET_KEY={}", self.secret_key),
            format!("NODE_MODE={}", self.mode),
        ];

        // Pass onion address to node so it knows its public identity
        if let Some(ref onion) = onion_addr {
            env.push(format!("ONION_ADDRESS={}", onion));
        }

        Ok(env)
    }

    /// Get list of local addresses for backwards compat
    #[allow(dead_code)]
    fn nodes_list(&self) -> Vec<String> {
        self.allocations
            .lock()
            .expect("node allocation mutex poisoned")
            .iter()
            .map(|a| a.local_addr.clone())
            .collect()
    }

    /// Get detailed node allocations with onion addresses
    fn node_allocations(&self) -> Vec<NodeAllocation> {
        self.allocations
            .lock()
            .expect("node allocation mutex poisoned")
            .clone()
    }

    fn next_bind_addr(&self, offset: usize) -> Result<SocketAddr, String> {
        let base_port = self.base_addr.port() as usize;
        let target_port = base_port
            .checked_add(offset)
            .ok_or_else(|| "Node port overflow".to_string())?;

        if target_port > u16::MAX as usize {
            return Err("Node port exceeds u16 range".into());
        }

        let mut socket = self.base_addr;
        socket.set_port(target_port as u16);
        Ok(socket)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_controller_creation() {
        let config = ControllerConfig::default();
        let controller = Controller::new(config);

        let metrics = controller.get_metrics().await;
        assert_eq!(metrics.services_running, 0);
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let config = ControllerConfig::default();
        let controller = Controller::new(config);

        {
            let mut metrics = controller.metrics.lock().await;
            metrics.services_running = 5;
            metrics.scaling_events = 2;
        }

        let metrics = controller.get_metrics().await;
        assert_eq!(metrics.services_running, 5);
        assert_eq!(metrics.scaling_events, 2);
    }
}
