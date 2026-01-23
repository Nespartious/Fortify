use crate::{Mirror, OrchestratorError, Result};
use fortify_core::jittered_timeout;
use rand::Rng;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// Track if we've already logged the PoW status (avoid spam)
static POW_STATUS_LOGGED: AtomicBool = AtomicBool::new(false);
// Track if we've logged the file-based PoW status
static POW_FILE_LOGGED: AtomicBool = AtomicBool::new(false);

/// Timeout for Tor control port operations (15 seconds)
/// This prevents hanging on unresponsive Tor daemons (slow-loris defense)
const TOR_CONTROL_TIMEOUT_SECS: u64 = 15;

/// Connect to Tor control port with timeout settings configured
/// Returns a TcpStream with read/write timeouts set to prevent blocking forever
/// Jitter applied to prevent timing-based fingerprinting
fn connect_tor_control_with_timeout(addr: &SocketAddr) -> Result<TcpStream> {
    let stream = TcpStream::connect(addr)
        .map_err(|e| OrchestratorError::TorConnectionFailed(e.to_string()))?;

    stream
        .set_nodelay(true)
        .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;

    let timeout = Some(jittered_timeout(TOR_CONTROL_TIMEOUT_SECS));
    stream.set_read_timeout(timeout).map_err(|e| {
        OrchestratorError::TorConfigError(format!("Failed to set read timeout: {}", e))
    })?;
    stream.set_write_timeout(timeout).map_err(|e| {
        OrchestratorError::TorConfigError(format!("Failed to set write timeout: {}", e))
    })?;

    Ok(stream)
}

enum TorBackend {
    Disabled,
    ControlPort {
        addr: SocketAddr,
        cookie_path: PathBuf,
    },
}

/// Vanity address configuration for mirrors
#[derive(Debug, Clone, Default)]
pub struct VanityConfig {
    /// Enable vanity address generation
    pub enabled: bool,
    /// Prefix to match
    pub prefix: String,
    /// Timeout in seconds for vanity generation
    pub timeout: u64,
}

/// Tor hidden service manager
///
/// PoW (Proof-of-Work) Strategy:
/// 1. First try ADD_ONION with PoWDefensesEnabled (requires Tor 0.4.9.2+)
/// 2. If that fails, fall back to file-based hidden service with PoW via torrc
///
/// File-based PoW uses a torrc include file that Tor reads on SIGHUP/reload.
/// This enables PoW on Tor 0.4.8+ even without ADD_ONION PoW support.
///
/// Vanity Address Strategy:
/// - When enabled, uses mkp224o to generate vanity .onion addresses for mirrors
/// - Falls back to random address if mkp224o is unavailable or times out
pub struct TorService {
    backend: TorBackend,
    /// Vanity address configuration
    vanity: VanityConfig,
    /// Base data directory for Fortify (for torrc path)
    base_data_dir: std::path::PathBuf,
}

impl TorService {
    pub fn new(control_addr: Option<String>, cookie_path: Option<PathBuf>) -> Self {
        let backend = match (control_addr, cookie_path) {
            (Some(addr_str), Some(cookie_path)) => match addr_str.parse() {
                Ok(addr) => TorBackend::ControlPort { addr, cookie_path },
                Err(err) => {
                    tracing::warn!(
                        "Invalid tor control addr {}: {}. Falling back to placeholder onions",
                        addr_str,
                        err
                    );
                    TorBackend::Disabled
                }
            },
            _ => TorBackend::Disabled,
        };
        Self {
            backend,
            vanity: VanityConfig::default(),
            base_data_dir: if let Some(home) = std::env::var_os("HOME") {
                let mut path = std::path::PathBuf::from(home);
                path.push(".local");
                path.push("share");
                path.push("fortify");
                path
            } else {
                std::path::PathBuf::from("/tmp/fortify")
            },
        }
    }

    /// Set base data directory for torrc path resolution
    pub fn with_base_data_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.base_data_dir = dir;
        self
    }

    /// Configure vanity address generation for mirrors
    pub fn with_vanity(mut self, config: VanityConfig) -> Self {
        tracing::info!(
            "TorService vanity configured: enabled={}, prefix='{}', timeout={}",
            config.enabled,
            config.prefix,
            config.timeout
        );
        self.vanity = config;
        self
    }

    /// Create a new hidden service (real or placeholder depending on backend)
    pub fn create_hidden_service(&self, mirror: &mut Mirror, target_port: u16) -> Result<String> {
        fs::create_dir_all(&mirror.tor_data_dir)
            .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;

        // Create lock file to prevent other orchestrators from deleting this dir while we're initializing
        let lock_file = mirror.tor_data_dir.join(".creating");
        fs::write(&lock_file, format!("{}", std::process::id())).map_err(|e| {
            OrchestratorError::TorConfigError(format!("Failed to create lock: {}", e))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&mirror.tor_data_dir, fs::Permissions::from_mode(0o700))
                .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
        }

        let result = match &self.backend {
            TorBackend::Disabled => {
                let onion = self.random_onion();
                self.write_hostname(&mirror.tor_data_dir, &onion)?;
                Ok(onion)
            }
            TorBackend::ControlPort { .. } => self.create_via_control_port(mirror, target_port),
        };

        // On error, clean up the directory we created to prevent accumulation
        if result.is_err() {
            let _ = fs::remove_file(&lock_file);
            let _ = fs::remove_dir_all(&mirror.tor_data_dir);
        }

        result
    }

    fn create_via_control_port(&self, mirror: &mut Mirror, target_port: u16) -> Result<String> {
        let (addr, cookie_path) = match &self.backend {
            TorBackend::ControlPort { addr, cookie_path } => (addr, cookie_path),
            TorBackend::Disabled => unreachable!(),
        };

        tracing::debug!("Connecting to Tor control port at {}", addr);

        // Retry connection with backoff to handle busy Tor control port
        let mut stream = None;
        for attempt in 0..3 {
            tracing::trace!("Tor control connection attempt {} of 3", attempt + 1);
            match connect_tor_control_with_timeout(addr) {
                Ok(s) => {
                    tracing::debug!("Connected to Tor control port on attempt {}", attempt + 1);
                    stream = Some(s);
                    break;
                }
                Err(e) if attempt < 2 => {
                    tracing::debug!(
                        "Tor control connect attempt {} failed: {}, retrying...",
                        attempt + 1,
                        e
                    );
                    std::thread::sleep(Duration::from_millis(100 * (attempt as u64 + 1)));
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to connect to Tor control port after 3 attempts: {}",
                        e
                    );
                    return Err(e);
                }
            }
        }

        let mut stream = stream.ok_or_else(|| {
            tracing::error!("Failed to connect to Tor control port at {}", addr);
            OrchestratorError::TorConnectionFailed(
                "Failed to connect to Tor control port".to_string(),
            )
        })?;

        tracing::debug!("Authenticating with Tor control port...");
        self.authenticate(&mut stream, cookie_path)?;
        tracing::debug!("Tor authentication successful");

        // Strategy: Try ADD_ONION with PoW first (Tor 0.4.9.2+), fall back to standard ADD_ONION
        tracing::info!("Creating hidden service on port {}", target_port);

        let pow_cmd = format!(
            "ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:{} Flags=Detach,PoWDefensesEnabled",
            target_port
        );

        tracing::debug!("Trying ADD_ONION with PoW (requires Tor 0.4.9.2+)");
        match self.run_command(&mut stream, &pow_cmd) {
            Ok(response) => {
                // ADD_ONION with PoW succeeded! (Tor 0.4.9.2+)
                if !POW_STATUS_LOGGED.swap(true, Ordering::Relaxed) {
                    tracing::info!("✅ Tor PoW enabled via ADD_ONION (Tor 0.4.9.2+)");
                }
                let service_id = Self::extract_service_id(&response)?;
                tracing::info!("Hidden service created: {}.onion", service_id);
                let private_key = Self::extract_private_key(&response)?;
                let onion = format!("{}.onion", service_id);

                mirror.tor_service_id = Some(service_id);
                mirror.pow_enabled = true;
                self.write_hostname(&mirror.tor_data_dir, &onion)?;
                self.write_private_key(&mirror.tor_data_dir, &private_key)?;

                return Ok(onion);
            }
            Err(OrchestratorError::TorConfigError(msg))
                if msg.contains("512") || msg.contains("552") =>
            {
                // PoW flag not supported (Tor < 0.4.9.2), fall back to file-based PoW
                if !POW_FILE_LOGGED.swap(true, Ordering::Relaxed) {
                    tracing::info!(
                        "ADD_ONION PoW not supported, using file-based hidden service with PoW"
                    );
                }
            }
            Err(e) => {
                tracing::error!("ADD_ONION failed: {}", e);
                return Err(e);
            }
        }

        // Fall back to file-based hidden service with PoW enabled via torrc
        tracing::debug!("Falling back to file-based PoW service");
        drop(stream); // Release the connection
        self.create_file_based_pow_service(mirror, target_port, addr, cookie_path)
    }

    /// Create a file-based hidden service with PoW enabled via torrc include
    /// This works on Tor 0.4.8+ by writing config to a file and signaling Tor to reload
    /// Supports vanity address generation for mirrors
    fn create_file_based_pow_service(
        &self,
        mirror: &mut Mirror,
        target_port: u16,
        addr: &SocketAddr,
        cookie_path: &Path,
    ) -> Result<String> {
        // Create the hidden service directory
        let hs_dir = mirror.tor_data_dir.join("hs");
        fs::create_dir_all(&hs_dir).map_err(|e| {
            OrchestratorError::TorConfigError(format!("Failed to create hs dir: {}", e))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&hs_dir, fs::Permissions::from_mode(0o700))
                .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
        }

        // Generate vanity keys if enabled
        tracing::debug!(
            "Vanity config: enabled={}, prefix='{}', timeout={}s",
            self.vanity.enabled,
            self.vanity.prefix,
            self.vanity.timeout
        );
        let vanity_address = if self.vanity.enabled && !self.vanity.prefix.is_empty() {
            match self.generate_vanity_keys(&hs_dir, &self.vanity.prefix, self.vanity.timeout) {
                Ok(addr) => {
                    tracing::info!("✨ Vanity mirror: {}", addr);
                    Some(addr)
                }
                Err(e) => {
                    tracing::warn!("Vanity generation failed ({}), using random address", e);
                    None
                }
            }
        } else {
            tracing::debug!("Vanity disabled or empty prefix, using random address");
            None
        };

        // Write torrc snippet for this hidden service with PoW and DoS defenses
        // Since Tor regenerates its torrc and removes %include directives,
        // we append mirror configs directly to the main torrc file
        // DoS defense options:
        //   - HiddenServiceEnableIntroDoSDefense: Protects introduction points from DoS
        //   - HiddenServiceMaxStreams: Limits concurrent streams per rendezvous circuit
        //   - HiddenServiceMaxStreamsCloseCircuit: Closes circuit if stream limit exceeded
        let torrc_path = self.base_data_dir.join("tor").join("torrc");

        let torrc_content = format!(
            "# Fortify mirror: {}\nHiddenServiceDir {}\nHiddenServicePort 80 127.0.0.1:{}\nHiddenServicePoWDefensesEnabled 1\nHiddenServiceEnableIntroDoSDefense 1\nHiddenServiceMaxStreams 100\nHiddenServiceMaxStreamsCloseCircuit 1\n",
            mirror.id,
            hs_dir.to_string_lossy(),
            target_port
        );

        // Append to main torrc file (Tor will read appended configs)
        std::fs::OpenOptions::new()
            .append(true)
            .open(&torrc_path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, torrc_content.as_bytes()))
            .map_err(|e| {
                OrchestratorError::TorConfigError(format!("Failed to append to torrc: {}", e))
            })?;

        tracing::debug!("Appended mirror config to main {}", torrc_path.display());

        // Also write the old-style torrc.inc for backward compatibility
        // (in case some code still reads from the mirrors/orch-*/mirror-*/torrc.inc path)
        let torrc_inc_path = mirror.tor_data_dir.join("torrc.inc");
        fs::write(&torrc_inc_path, &torrc_content).map_err(|e| {
            OrchestratorError::TorConfigError(format!("Failed to write torrc.inc: {}", e))
        })?;

        // Signal Tor to reload configuration (with retry and timeout)
        let stream_result =
            (0..3).find_map(|attempt| match connect_tor_control_with_timeout(addr) {
                Ok(s) => Some(s),
                Err(_) if attempt < 2 => {
                    std::thread::sleep(Duration::from_millis(100 * (attempt as u64 + 1)));
                    None
                }
                Err(_) => None,
            });

        if let Some(mut stream) = stream_result {
            if self.authenticate(&mut stream, cookie_path).is_ok() {
                match self.run_command(&mut stream, "SIGNAL RELOAD") {
                    Ok(_) => tracing::debug!("Tor reloaded configuration"),
                    Err(e) => tracing::warn!("Failed to signal Tor reload: {}", e),
                }
            }
        } else {
            tracing::warn!("Could not connect to Tor control to signal reload");
        }

        // If vanity address was generated, use it directly (keys already in place)
        // Otherwise wait for Tor to create the hostname file
        let hostname_path = hs_dir.join("hostname");
        let onion = if let Some(ref vanity_addr) = vanity_address {
            // Vanity keys already written, hostname should already exist
            vanity_addr.clone()
        } else {
            // Wait for Tor to create the hostname file
            let mut attempts = 0;
            loop {
                if let Ok(hostname) = fs::read_to_string(&hostname_path) {
                    let addr = hostname.trim().to_string();
                    if !addr.is_empty() && addr.ends_with(".onion") {
                        break addr;
                    }
                }
                attempts += 1;
                if attempts > 40 {
                    // 10 seconds total
                    tracing::warn!("File-based PoW setup timed out waiting for hostname");
                    // Last resort fallback to ephemeral without PoW
                    return self.create_standard_ephemeral(mirror, target_port, addr, cookie_path);
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        };

        let service_id = onion.replace(".onion", "");
        mirror.tor_service_id = Some(service_id);
        mirror.pow_enabled = true;
        mirror.file_based = true;
        self.write_hostname(&mirror.tor_data_dir, &onion)?;

        // Copy the private key if Tor generated one
        let key_path = hs_dir.join("hs_ed25519_secret_key");
        if key_path.exists() {
            if let Ok(key_data) = fs::read(&key_path) {
                // Convert raw key to the format ADD_ONION expects
                let key_hex = format!("ED25519-V3:{}", hex::encode(&key_data));
                self.write_private_key(&mirror.tor_data_dir, &key_hex)?;
            }
        }

        tracing::info!("✅ Created file-based PoW mirror: {}", onion);
        Ok(onion)
    }

    /// Create standard ephemeral service without PoW (last resort fallback)
    fn create_standard_ephemeral(
        &self,
        mirror: &mut Mirror,
        target_port: u16,
        addr: &SocketAddr,
        cookie_path: &Path,
    ) -> Result<String> {
        // Retry connection with backoff and timeout
        let mut stream = None;
        for attempt in 0..3 {
            match connect_tor_control_with_timeout(addr) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(_e) if attempt < 2 => {
                    std::thread::sleep(Duration::from_millis(100 * (attempt as u64 + 1)));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        let mut stream = stream.ok_or_else(|| {
            OrchestratorError::TorConnectionFailed(
                "Failed to connect to Tor control port".to_string(),
            )
        })?;
        self.authenticate(&mut stream, cookie_path)?;

        let cmd = format!(
            "ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:{} Flags=Detach",
            target_port
        );

        if !POW_STATUS_LOGGED.swap(true, Ordering::Relaxed) {
            tracing::info!(
                "Using standard ephemeral services (Fortify's own DDoS protection active)"
            );
        }

        let response = self.run_command(&mut stream, &cmd)?;
        let service_id = Self::extract_service_id(&response)?;
        let private_key = Self::extract_private_key(&response)?;
        let onion = format!("{}.onion", service_id);

        mirror.tor_service_id = Some(service_id);
        mirror.pow_enabled = false;
        self.write_hostname(&mirror.tor_data_dir, &onion)?;
        self.write_private_key(&mirror.tor_data_dir, &private_key)?;

        Ok(onion)
    }

    pub fn restore_hidden_service(&self, mirror: &mut Mirror, target_port: u16) -> Result<()> {
        let (addr, cookie_path) = match &self.backend {
            TorBackend::ControlPort { addr, cookie_path } => (addr, cookie_path),
            _ => return Ok(()),
        };

        // Check if this is a file-based mirror FIRST (before attempting ADD_ONION)
        // File-based mirrors have a torrc.inc file and hs directory from initial setup
        let torrc_inc_path = mirror.tor_data_dir.join("torrc.inc");
        let hs_dir = mirror.tor_data_dir.join("hs");

        if torrc_inc_path.exists() && hs_dir.exists() {
            // This is a file-based mirror with PoW support - restore by appending to torrc
            // Read the torrc.inc content (which has the hidden service configuration)
            let torrc_content = fs::read_to_string(&torrc_inc_path).map_err(|e| {
                OrchestratorError::TorConfigError(format!("Failed to read torrc.inc: {}", e))
            })?;

            // Append to main torrc (same as create_file_based_pow_service does)
            let main_torrc = self.base_data_dir.join("tor").join("torrc");

            // Check if this mirror's config is already in torrc to avoid duplicates
            if main_torrc.exists() {
                let existing = fs::read_to_string(&main_torrc).unwrap_or_default();
                let mirror_marker = format!("# Fortify mirror: {}", mirror.id);

                if existing.contains(&mirror_marker) {
                    tracing::info!("Mirror {} already in torrc, skipping append", mirror.id);
                } else {
                    // Append the mirror config
                    fs::OpenOptions::new()
                        .append(true)
                        .open(&main_torrc)
                        .and_then(|mut file| {
                            std::io::Write::write_all(&mut file, torrc_content.as_bytes())
                        })
                        .map_err(|e| {
                            OrchestratorError::TorConfigError(format!(
                                "Failed to append to torrc: {}",
                                e
                            ))
                        })?;

                    tracing::debug!("Appended restored mirror {} to torrc", mirror.id);
                }
            }

            // Signal Tor to reload configuration (with timeout protection)
            let mut stream = connect_tor_control_with_timeout(addr)?;
            self.authenticate(&mut stream, cookie_path)?;
            let _ = self.run_command(&mut stream, "SIGNAL RELOAD");

            mirror.file_based = true;
            mirror.pow_enabled = true;
            if let Some(onion) = &mirror.onion_address {
                mirror.tor_service_id = Some(onion.replace(".onion", ""));
                tracing::info!(
                    "Restored file-based mirror {} via torrc append: {}",
                    mirror.id,
                    onion
                );
            }

            return Ok(());
        }

        // Read private key for ephemeral service restoration
        let private_key =
            fs::read_to_string(mirror.tor_data_dir.join("private_key")).map_err(|e| {
                OrchestratorError::TorConfigError(format!("Missing private key: {}", e))
            })?;
        let private_key = private_key.trim();

        let mut stream = connect_tor_control_with_timeout(addr)?;
        self.authenticate(&mut stream, cookie_path)?;

        // Attempt to clean up potential existing service with same key (derived from hostname)
        if let Some(onion) = &mirror.onion_address {
            let service_id = onion.replace(".onion", "");
            let _ = self.run_command(&mut stream, &format!("DEL_ONION {}", service_id));
        }

        // Try to restore with PoW first (Tor 0.4.9.2+)
        let pow_cmd = format!(
            "ADD_ONION {} Port=80,127.0.0.1:{} Flags=Detach,PoWDefensesEnabled",
            private_key, target_port
        );

        let response = match self.run_command(&mut stream, &pow_cmd) {
            Ok(r) => {
                mirror.pow_enabled = true;
                r
            }
            Err(OrchestratorError::TorConfigError(msg))
                if msg.contains("512") || msg.contains("552") =>
            {
                // PoW not supported via ADD_ONION, fall back to standard ADD_ONION without PoW
                let cmd = format!(
                    "ADD_ONION {} Port=80,127.0.0.1:{} Flags=Detach",
                    private_key, target_port
                );
                mirror.pow_enabled = false;
                self.run_command(&mut stream, &cmd)?
            }
            Err(OrchestratorError::TorConfigError(msg))
                if msg.contains("550") && msg.to_lowercase().contains("collision") =>
            {
                // Service already exists (collision), reusing.
                tracing::debug!("Tor service already active, reusing existing onion");

                if let Some(onion) = &mirror.onion_address {
                    let service_id = onion.replace(".onion", "");
                    mirror.tor_service_id = Some(service_id);
                    return Ok(());
                } else {
                    return Err(OrchestratorError::TorConfigError(format!(
                        "Collision detected but no onion addr to reuse. Msg: {}",
                        msg
                    )));
                }
            }
            Err(e) => return Err(e),
        };
        let service_id = Self::extract_service_id(&response)?;
        mirror.tor_service_id = Some(service_id);

        Ok(())
    }

    fn authenticate(&self, stream: &mut TcpStream, cookie_path: &Path) -> Result<()> {
        let cookie =
            fs::read(cookie_path).map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
        let cmd = format!("AUTHENTICATE {}", hex::encode(cookie));
        let response = self.run_command(stream, &cmd)?;
        if response.iter().any(|line| line.starts_with("250")) {
            Ok(())
        } else {
            Err(OrchestratorError::TorConfigError(
                "Tor AUTHENTICATE did not return success".into(),
            ))
        }
    }

    fn run_command(&self, stream: &mut TcpStream, command: &str) -> Result<Vec<String>> {
        let cmd = format!("{}\r\n", command);
        stream.write_all(cmd.as_bytes()).map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::WouldBlock
            {
                tracing::warn!(
                    "Tor control write timed out after {}s",
                    TOR_CONTROL_TIMEOUT_SECS
                );
                OrchestratorError::TorTimeout(TOR_CONTROL_TIMEOUT_SECS)
            } else {
                OrchestratorError::TorConfigError(e.to_string())
            }
        })?;
        stream.flush().map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::WouldBlock
            {
                OrchestratorError::TorTimeout(TOR_CONTROL_TIMEOUT_SECS)
            } else {
                OrchestratorError::TorConfigError(e.to_string())
            }
        })?;

        let mut reader = BufReader::new(stream);
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).map_err(|e| {
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock
                {
                    tracing::warn!(
                        "Tor control read timed out after {}s for command: {}",
                        TOR_CONTROL_TIMEOUT_SECS,
                        command
                    );
                    OrchestratorError::TorTimeout(TOR_CONTROL_TIMEOUT_SECS)
                } else {
                    OrchestratorError::TorConfigError(e.to_string())
                }
            })?;
            if read == 0 {
                break;
            }

            let trimmed = line.trim_end().to_string();
            if trimmed.is_empty() {
                continue;
            }

            let code = trimmed.get(0..3).unwrap_or("");
            if code.starts_with('5') {
                // Allow failure for DEL_ONION if service doesn't exist
                if command.starts_with("DEL_ONION") && trimmed.contains("552") {
                    return Ok(vec![trimmed]);
                }
                return Err(OrchestratorError::TorConfigError(trimmed));
            }

            lines.push(trimmed.clone());
            if code == "250" && !trimmed.starts_with("250-") {
                break;
            }
        }

        Ok(lines)
    }

    fn extract_service_id(lines: &[String]) -> Result<String> {
        for line in lines {
            if let Some(rest) = line.strip_prefix("250-ServiceID=") {
                return Ok(rest.to_string());
            }
        }
        Err(OrchestratorError::TorConfigError(
            "Tor ADD_ONION response missing ServiceID".into(),
        ))
    }

    fn extract_private_key(lines: &[String]) -> Result<String> {
        for line in lines {
            if let Some(rest) = line.strip_prefix("250-PrivateKey=") {
                return Ok(rest.to_string());
            }
        }
        Err(OrchestratorError::TorConfigError(
            "Tor ADD_ONION response missing PrivateKey".into(),
        ))
    }

    fn write_hostname(&self, dir: &Path, onion: &str) -> Result<()> {
        fs::write(dir.join("hostname"), format!("{}\n", onion))
            .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))
    }

    fn write_private_key(&self, dir: &Path, key: &str) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Write key with strict permissions
            let path = dir.join("private_key");
            fs::write(&path, key).map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
        }
        #[cfg(not(unix))]
        {
            fs::write(dir.join("private_key"), key)
                .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
        }

        // Remove the lock file now that both hostname and private_key are written
        let lock_file = dir.join(".creating");
        let _ = fs::remove_file(&lock_file); // Ignore errors - file might not exist

        Ok(())
    }

    fn random_onion(&self) -> String {
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz234567".chars().collect();
        let mut rng = rand::rng();
        let suffix: String = (0..56)
            .map(|_| chars[rng.random_range(0..chars.len())])
            .collect();
        format!("{}.onion", suffix)
    }

    /// Remove hidden service metadata and notify Tor when available
    pub fn remove_hidden_service(&self, mirror: &Mirror) -> Result<()> {
        if let TorBackend::ControlPort { .. } = &self.backend {
            if let Some(service_id) = &mirror.tor_service_id {
                self.delete_onion(service_id)?;
            }
        }

        // Clean up include directive from main torrc if this was a file-based service
        if mirror.file_based {
            self.remove_torrc_include(mirror);
        }

        if mirror.tor_data_dir.exists() {
            fs::remove_dir_all(&mirror.tor_data_dir)
                .map_err(|e| OrchestratorError::TorConfigError(e.to_string()))?;
        }
        Ok(())
    }

    /// Remove the %include directive for a mirror from the main torrc file
    fn remove_torrc_include(&self, mirror: &Mirror) {
        // Use base_data_dir for torrc path
        let main_torrc = self.base_data_dir.join("tor").join("torrc");

        if !main_torrc.exists() {
            return;
        }

        let torrc_path = mirror.tor_data_dir.join("torrc.inc");
        let include_line = format!("%include {}", torrc_path.to_string_lossy());
        let comment_line = format!("# Fortify mirror: {}", mirror.id);

        if let Ok(content) = fs::read_to_string(&main_torrc) {
            // Filter out lines related to this mirror
            let new_content: String = content
                .lines()
                .filter(|line| !line.contains(&include_line) && !line.contains(&comment_line))
                .collect::<Vec<_>>()
                .join("\n");

            // Also remove any extra blank lines
            let cleaned = new_content.replace("\n\n\n", "\n\n");

            if let Err(e) = fs::write(&main_torrc, cleaned) {
                tracing::warn!("Failed to update torrc after mirror removal: {}", e);
            }
        }
    }

    /// Generate vanity keys using mkp224o for mirror addresses
    /// Implements progressive prefix reduction: if generation times out,
    /// removes the last character from prefix and retries until success or min length reached
    fn generate_vanity_keys(
        &self,
        hs_dir: &Path,
        prefix: &str,
        timeout: u64,
    ) -> std::result::Result<String, String> {
        use std::process::Command;

        // Check if mkp224o is available
        let which_result = Command::new("which").arg("mkp224o").output();
        if which_result.is_err() || !which_result.unwrap().status.success() {
            return Err("mkp224o not found in PATH".into());
        }

        // Progressive prefix reduction - start with full prefix and shorten on timeout
        let mut current_prefix = prefix.to_string();
        let min_prefix_len = 1; // Minimum prefix length before giving up

        while current_prefix.len() >= min_prefix_len {
            // Create temp directory for mkp224o output
            let temp_dir = std::env::temp_dir().join(format!(
                "fortify-vanity-{}-{}",
                std::process::id(),
                rand::random::<u32>()
            ));
            fs::create_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to create temp dir: {}", e))?;

            let timeout_arg = format!("{}s", timeout);
            tracing::debug!(
                "Running mkp224o: prefix='{}', timeout={}s",
                current_prefix,
                timeout
            );

            // Run mkp224o with timeout
            let output = Command::new("timeout")
                .arg(&timeout_arg)
                .arg("mkp224o")
                .arg("-d")
                .arg(&temp_dir)
                .arg("-n")
                .arg("1")
                .arg(&current_prefix)
                .output()
                .map_err(|e| format!("Failed to run mkp224o: {}", e))?;

            // Check for timeout (exit code 124)
            let _timed_out = output.status.code() == Some(124);

            // Check if we got a result
            match self.find_vanity_key_dir(&temp_dir, &current_prefix) {
                Ok(key_dir) => {
                    let dir_name = key_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or("Invalid key directory name")?
                        .to_string();

                    // mkp224o creates directories like "prefix...xxx.onion", strip the suffix if present
                    let onion_name = dir_name.strip_suffix(".onion").unwrap_or(&dir_name);

                    // Copy key files to hidden service directory
                    for file in &["hs_ed25519_public_key", "hs_ed25519_secret_key", "hostname"] {
                        let src = key_dir.join(file);
                        let dst = hs_dir.join(file);
                        if src.exists() {
                            fs::copy(&src, &dst)
                                .map_err(|e| format!("Failed to copy {}: {}", file, e))?;

                            // Set proper permissions
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let mode = if *file == "hs_ed25519_secret_key" {
                                    0o600
                                } else {
                                    0o644
                                };
                                fs::set_permissions(&dst, fs::Permissions::from_mode(mode)).ok();
                            }
                        }
                    }

                    // Cleanup temp directory
                    let _ = fs::remove_dir_all(&temp_dir);

                    let onion_address = format!("{}.onion", onion_name);
                    if current_prefix.len() < prefix.len() {
                        tracing::debug!("Used shortened prefix '{}' for vanity", current_prefix);
                    }

                    return Ok(onion_address);
                }
                Err(e) => {
                    // Cleanup temp directory
                    let _ = fs::remove_dir_all(&temp_dir);

                    // Check if it was actually a timeout (exit code 124) or mkp224o just didn't find anything
                    let was_timeout = output.status.code() == Some(124);

                    if was_timeout {
                        // Real timeout - try with shorter prefix
                        if current_prefix.len() > min_prefix_len {
                            let new_prefix = &current_prefix[..current_prefix.len() - 1];
                            tracing::warn!(
                                "Vanity generation timed out for '{}', reducing to '{}'",
                                current_prefix,
                                new_prefix
                            );
                            current_prefix = new_prefix.to_string();
                        } else {
                            tracing::warn!("Vanity generation timed out at minimum prefix length");
                            break;
                        }
                    } else {
                        // mkp224o exited without finding a match (or we couldn't find the output)
                        tracing::warn!(
                            "mkp224o exited (code {:?}) but no matching directory found: {}",
                            output.status.code(),
                            e
                        );
                        // Try with shorter prefix since this one seems problematic
                        if current_prefix.len() > min_prefix_len {
                            let new_prefix = &current_prefix[..current_prefix.len() - 1];
                            tracing::warn!(
                                "Reducing prefix from '{}' to '{}'",
                                current_prefix,
                                new_prefix
                            );
                            current_prefix = new_prefix.to_string();
                        } else {
                            break;
                        }
                    }
                }
            }

            if !output.status.success() && output.status.code() != Some(124) {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    tracing::debug!("mkp224o stderr: {}", stderr);
                }
            }
        }

        // All attempts failed
        Err(format!(
            "Vanity generation failed for all prefix lengths from '{}' to {}",
            prefix, min_prefix_len
        ))
    }

    /// Find the directory containing the generated vanity key
    fn find_vanity_key_dir(
        &self,
        temp_dir: &Path,
        prefix: &str,
    ) -> std::result::Result<PathBuf, String> {
        // Debug: list what mkp224o actually created
        let mut found_dirs: Vec<String> = Vec::new();
        if let Ok(entries) = fs::read_dir(temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        found_dirs.push(name.to_string());
                        // Check if it matches our prefix (case-insensitive for base32)
                        // mkp224o creates dirs like "sigil...xxx.onion" (62 chars = 56 + ".onion")
                        let name_lower = name.to_lowercase();
                        let prefix_lower = prefix.to_lowercase();
                        if name_lower.starts_with(&prefix_lower)
                            && (name.len() == 56 || name.ends_with(".onion"))
                        {
                            tracing::info!("Found vanity key directory: {}", name);
                            return Ok(path);
                        }
                    }
                }
            }
        }
        tracing::debug!("Directories in temp_dir: {:?}", found_dirs);
        Err(format!(
            "No vanity key generated with prefix '{}' (found {} dirs)",
            prefix,
            found_dirs.len()
        ))
    }

    fn delete_onion(&self, service_id: &str) -> Result<()> {
        let (addr, cookie_path) = match &self.backend {
            TorBackend::ControlPort { addr, cookie_path } => (addr, cookie_path),
            TorBackend::Disabled => return Ok(()),
        };

        let mut stream = connect_tor_control_with_timeout(addr)?;
        self.authenticate(&mut stream, cookie_path)?;
        let cmd = format!("DEL_ONION {}", service_id);
        self.run_command(&mut stream, &cmd)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mirror;
    use tempfile::TempDir;

    #[test]
    fn placeholder_service_generates_onion() {
        let temp_dir = TempDir::new().unwrap();
        let tor_service = TorService::new(None, None);
        let mut mirror = Mirror::new("test".into(), temp_dir.path().join("mirror"));
        let onion = tor_service
            .create_hidden_service(&mut mirror, 8080)
            .unwrap();
        assert!(onion.ends_with(".onion"));
        assert!(mirror.tor_data_dir.join("hostname").exists());
    }

    #[test]
    fn parse_service_id_lines() {
        let lines = vec![
            "250-ServiceID=testserviceid".to_string(),
            "250 OK".to_string(),
        ];
        let id = TorService::extract_service_id(&lines).unwrap();
        assert_eq!(id, "testserviceid");
    }
}
