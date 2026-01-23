//! Tor hidden service management for nodes
//!
//! Note: Vanity addresses are NOT used for nodes (healthy/threat).
//! Vanity addresses are only for mirrors and are handled by the orchestrator.
//!
//! Deployment Strategy:
//! 1. PRIMARY: Static file deployment (HiddenServiceDir) - supports PoW defense
//! 2. FALLBACK: ADD_ONION ephemeral services - no PoW until future Tor release
//!
//! When future Tor versions support ADD_ONION with PoW, this code will automatically
//! work without changes by adding PoW flags to the ADD_ONION command.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};

/// PoW defense configuration for hidden services
#[derive(Debug, Clone)]
pub struct PowConfig {
    /// Enable PoW defense
    pub enabled: bool,
    /// Queue rate (intro requests per second before PoW kicks in)
    pub queue_rate: u32,
    /// Queue burst (max intro requests before PoW required)
    pub queue_burst: u32,
}

impl Default for PowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            queue_rate: 25,
            queue_burst: 200,
        }
    }
}

/// Tor manager for node hidden services
///
/// Note: Nodes do NOT use vanity addresses. Vanity is only for mirrors.
pub struct TorManager {
    control_addr: String,
    cookie_path: String,
    /// Data directory for static hidden service deployment
    data_dir: PathBuf,
    /// PoW configuration
    pow_config: PowConfig,
    /// Counter for hidden service directories
    hs_counter: std::sync::atomic::AtomicUsize,
}

impl TorManager {
    pub fn new(control_addr: String, cookie_path: String) -> Self {
        // Extract data dir from cookie path (typically /path/to/tor/data/control_auth_cookie)
        let data_dir = Path::new(&cookie_path)
            .parent()
            .unwrap_or(Path::new("/tmp/fortify/tor/data"))
            .to_path_buf();

        Self {
            control_addr,
            cookie_path,
            data_dir,
            pow_config: PowConfig::default(),
            hs_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Set PoW configuration
    pub fn with_pow(mut self, config: PowConfig) -> Self {
        self.pow_config = config;
        self
    }

    /// Set data directory for hidden services
    pub fn with_data_dir(mut self, dir: PathBuf) -> Self {
        self.data_dir = dir;
        self
    }

    /// Create a new hidden service for a node listening on the given port
    /// Returns the .onion address
    ///
    /// Note: Node addresses are always random - vanity is for mirrors only.
    ///
    /// Strategy:
    /// 1. Try static file deployment with PoW (preferred)
    /// 2. Fall back to ADD_ONION if static deployment fails
    pub fn create_hidden_service(&self, target_port: u16) -> Result<String, String> {
        // First, try static file deployment with PoW support
        match self.create_static_hidden_service(target_port) {
            Ok(onion) => {
                tracing::info!(
                    "Created static hidden service with PoW for port {}: {}",
                    target_port,
                    onion
                );
                return Ok(onion);
            }
            Err(e) => {
                tracing::warn!(
                    "Static HS deployment failed ({}), falling back to ADD_ONION",
                    e
                );
            }
        }

        // Fallback: Use ADD_ONION (no PoW support in current Tor)
        // Note: When future Tor supports ADD_ONION with PoW, we can add flags here
        self.create_ephemeral_hidden_service(target_port)
    }

    /// Create hidden service using static files (supports PoW)
    /// Note: Nodes always use random addresses - vanity is for mirrors only
    fn create_static_hidden_service(&self, target_port: u16) -> Result<String, String> {
        let hs_num = self
            .hs_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let hs_dir = self.data_dir.join(format!("hs_{}", hs_num));

        // Create hidden service directory
        fs::create_dir_all(&hs_dir).map_err(|e| format!("Failed to create HS dir: {}", e))?;

        // Set proper permissions (700)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&hs_dir, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("Failed to set HS dir permissions: {}", e))?;
        }

        // Signal Tor to load the new hidden service via SETCONF
        // Tor will generate random keys for the node
        self.configure_static_service(&hs_dir, target_port)
    }

    /// Configure Tor to load a static hidden service with PoW
    /// CRITICAL: SETCONF replaces ALL hidden services, so we must include existing ones
    fn configure_static_service(&self, hs_dir: &Path, target_port: u16) -> Result<String, String> {
        let mut stream = TcpStream::connect(&self.control_addr)
            .map_err(|e| format!("Failed to connect to Tor control: {}", e))?;
        stream.set_nodelay(true).ok();

        self.authenticate(&mut stream)?;

        // CRITICAL FIX: Get existing hidden service configurations
        // SETCONF replaces ALL HiddenServiceDir entries, so we must include existing ones
        let getconf_response = self.run_command(&mut stream, "GETCONF HiddenServiceDir")?;

        // Parse existing hidden services from GETCONF response
        // Format: "250 HiddenServiceDir=/path/to/dir" or "250 OK" if none
        let mut existing_services: Vec<(String, Vec<String>)> = Vec::new();

        if !getconf_response.contains("250 OK\r\n") {
            // Parse each line that starts with "250 HiddenServiceDir"
            for line in getconf_response.lines() {
                if let Some(dir) = line.strip_prefix("250 HiddenServiceDir=") {
                    // Remove quotes if present
                    let dir_clean = dir.trim().trim_matches('"').to_string();

                    // Get the port configuration for this service
                    let port_cmd = "GETCONF HiddenServicePort";
                    let port_response = self.run_command(&mut stream, port_cmd)?;

                    let mut ports = Vec::new();
                    for port_line in port_response.lines() {
                        if let Some(port_val) = port_line.strip_prefix("250 HiddenServicePort=") {
                            ports.push(port_val.trim().trim_matches('"').to_string());
                        }
                    }

                    existing_services.push((dir_clean, ports));
                }
            }
        }

        // Build SETCONF command for ALL hidden services (existing + new)
        // Note: Values containing spaces must be quoted in Tor control protocol
        let mut setconf_parts = Vec::new();

        // Add existing hidden services first
        for (dir, ports) in &existing_services {
            setconf_parts.push(format!("HiddenServiceDir=\"{}\"", dir));
            for port in ports {
                setconf_parts.push(format!("HiddenServicePort=\"{}\"", port));
            }
            setconf_parts.push("HiddenServiceVersion=3".to_string());

            // Add DoS defense options (protects against introduction point DoS)
            setconf_parts.push("HiddenServiceEnableIntroDoSDefense=1".to_string());
            setconf_parts.push("HiddenServiceMaxStreams=100".to_string());
            setconf_parts.push("HiddenServiceMaxStreamsCloseCircuit=1".to_string());

            // Add PoW configuration if enabled
            if self.pow_config.enabled {
                setconf_parts.push("HiddenServicePoWDefensesEnabled=1".to_string());
                setconf_parts.push(format!(
                    "HiddenServicePoWQueueRate={}",
                    self.pow_config.queue_rate
                ));
                setconf_parts.push(format!(
                    "HiddenServicePoWQueueBurst={}",
                    self.pow_config.queue_burst
                ));
            }
        }

        // Add the new hidden service
        let hs_dir_str = hs_dir.to_string_lossy();
        setconf_parts.push(format!("HiddenServiceDir=\"{}\"", hs_dir_str));
        setconf_parts.push(format!(
            "HiddenServicePort=\"80 127.0.0.1:{}\"",
            target_port
        ));
        setconf_parts.push("HiddenServiceVersion=3".to_string());

        // Add DoS defense options (protects against introduction point DoS)
        setconf_parts.push("HiddenServiceEnableIntroDoSDefense=1".to_string());
        setconf_parts.push("HiddenServiceMaxStreams=100".to_string());
        setconf_parts.push("HiddenServiceMaxStreamsCloseCircuit=1".to_string());

        // Add PoW configuration if enabled
        if self.pow_config.enabled {
            setconf_parts.push("HiddenServicePoWDefensesEnabled=1".to_string());
            setconf_parts.push(format!(
                "HiddenServicePoWQueueRate={}",
                self.pow_config.queue_rate
            ));
            setconf_parts.push(format!(
                "HiddenServicePoWQueueBurst={}",
                self.pow_config.queue_burst
            ));
        }

        let cmd = format!("SETCONF {}", setconf_parts.join(" "));
        tracing::debug!(
            "Configuring {} hidden services (including new one)",
            existing_services.len() + 1
        );
        let response = self.run_command(&mut stream, &cmd)?;

        if !response.contains("250 OK") {
            return Err(format!("SETCONF failed: {}", response.trim()));
        }

        // Save config to make it persistent
        let save_response = self.run_command(&mut stream, "SAVECONF")?;
        if !save_response.contains("250 OK") {
            tracing::warn!(
                "SAVECONF failed, service may not persist: {}",
                save_response.trim()
            );
        }

        // Wait for hostname file to be created
        let hostname_path = hs_dir.join("hostname");
        for _ in 0..10 {
            if hostname_path.exists() {
                let hostname = fs::read_to_string(&hostname_path)
                    .map_err(|e| format!("Failed to read hostname: {}", e))?;
                return Ok(hostname.trim().to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        Err("Timeout waiting for hostname file".into())
    }

    /// Create hidden service using ADD_ONION (fallback, no PoW in current Tor)
    /// Note: Nodes always use random addresses - vanity is for mirrors only
    fn create_ephemeral_hidden_service(&self, target_port: u16) -> Result<String, String> {
        let mut stream = TcpStream::connect(&self.control_addr)
            .map_err(|e| format!("Failed to connect to Tor control: {}", e))?;
        stream.set_nodelay(true).ok();

        self.authenticate(&mut stream)?;

        // Build ADD_ONION command with random key
        // Note: When future Tor supports PoW with ADD_ONION, add flags here:
        // e.g., "ADD_ONION NEW:ED25519-V3 Port=80,... Flags=Detach,PoW"
        let cmd = format!(
            "ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:{} Flags=Detach",
            target_port
        );

        let response = self.run_command(&mut stream, &cmd)?;
        let service_id = Self::extract_service_id(&response)?;
        let onion = format!("{}.onion", service_id);

        tracing::warn!(
            "Created ephemeral hidden service (NO PoW protection): {}",
            onion
        );
        Ok(onion)
    }

    /// Remove a hidden service by its onion address
    pub fn remove_hidden_service(&self, onion: &str) -> Result<(), String> {
        let mut stream = TcpStream::connect(&self.control_addr)
            .map_err(|e| format!("Failed to connect to Tor control: {}", e))?;
        stream.set_nodelay(true).ok();

        self.authenticate(&mut stream)?;

        let service_id = onion.trim_end_matches(".onion");

        // Try DEL_ONION first (for ephemeral services)
        let cmd = format!("DEL_ONION {}", service_id);
        match self.run_command(&mut stream, &cmd) {
            Ok(response) if response.contains("250 OK") => {
                tracing::info!("Removed ephemeral hidden service: {}", onion);
                return Ok(());
            }
            _ => {}
        }

        // For static services, we need to remove the config
        // This is more complex and may require finding and removing the HiddenServiceDir
        tracing::warn!("DEL_ONION failed for {}, may be a static service", onion);

        Ok(())
    }

    fn authenticate(&self, stream: &mut TcpStream) -> Result<(), String> {
        let cookie =
            fs::read(&self.cookie_path).map_err(|e| format!("Failed to read Tor cookie: {}", e))?;
        let cookie_hex = hex::encode(&cookie);

        let cmd = format!("AUTHENTICATE {}", cookie_hex);
        let response = self.run_command(stream, &cmd)?;

        if !response.contains("250 OK") {
            return Err(format!("Tor authentication failed: {}", response));
        }

        Ok(())
    }

    fn run_command(&self, stream: &mut TcpStream, cmd: &str) -> Result<String, String> {
        let cmd_with_newline = format!("{}\r\n", cmd);
        stream
            .write_all(cmd_with_newline.as_bytes())
            .map_err(|e| format!("Failed to send command: {}", e))?;
        stream
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut response = String::new();

        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("Failed to read response: {}", e))?;

            response.push_str(&line);

            if line.starts_with("250 ") || line.starts_with("5") || line.starts_with("4") {
                break;
            }
            if line.is_empty() {
                break;
            }
        }

        if response.starts_with("5") || response.starts_with("4") {
            return Err(format!("Tor command failed: {}", response.trim()));
        }

        Ok(response)
    }

    fn extract_service_id(response: &str) -> Result<String, String> {
        for line in response.lines() {
            if line.starts_with("250-ServiceID=") {
                return Ok(line.trim_start_matches("250-ServiceID=").to_string());
            }
        }
        Err("Tor ADD_ONION response missing ServiceID".into())
    }
}
