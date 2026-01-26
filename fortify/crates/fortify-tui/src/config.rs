//! Configuration types and management for Fortify TUI

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Available CAPTCHA types for verification challenges.
/// Mirrors the types available in fortify-gate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptchaType {
    /// Traditional text-based BMP image captcha
    #[default]
    BmpText,
    /// Select emoji matching description
    Emoji,
    /// Click arrow pointing in specified direction
    Direction,
    /// Complete the sequence/pattern
    Sequence,
    /// Unscramble letters to form a word
    WordUnscramble,
    /// Select correctly rotated image
    ImageRotation,
    /// Identify silhouette category
    Silhouette,
}

impl CaptchaType {
    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BmpText => "Text Image",
            Self::Emoji => "Emoji Selection",
            Self::Direction => "Arrow Direction",
            Self::Sequence => "Sequence Pattern",
            Self::WordUnscramble => "Word Unscramble",
            Self::ImageRotation => "Image Rotation",
            Self::Silhouette => "Silhouette ID",
        }
    }

    /// All available captcha types
    pub fn all() -> &'static [CaptchaType] {
        &[
            Self::BmpText,
            Self::Emoji,
            Self::Direction,
            Self::Sequence,
            Self::WordUnscramble,
            Self::ImageRotation,
            Self::Silhouette,
        ]
    }

    /// Cycle to next type (for UI selection)
    pub fn next(&self) -> Self {
        match self {
            Self::BmpText => Self::Emoji,
            Self::Emoji => Self::Direction,
            Self::Direction => Self::Sequence,
            Self::Sequence => Self::WordUnscramble,
            Self::WordUnscramble => Self::ImageRotation,
            Self::ImageRotation => Self::Silhouette,
            Self::Silhouette => Self::BmpText,
        }
    }
}

/// Traffic tier for auto-scaling multiple settings based on expected daily users.
/// Selecting a tier adjusts CAPTCHA pool sizes, rate limits, mirror counts, and thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrafficTier {
    /// ~100 users/day - Personal/test site
    Micro,
    /// ~1,000 users/day - Small community (DEFAULT)
    #[default]
    Small,
    /// ~10,000 users/day - Active community
    Medium,
    /// ~100,000 users/day - Popular service
    Large,
    /// ~1,000,000+ users/day - High-traffic platform
    Enterprise,
}

impl TrafficTier {
    /// Returns all available traffic tiers
    pub fn all() -> &'static [TrafficTier] {
        &[
            TrafficTier::Micro,
            TrafficTier::Small,
            TrafficTier::Medium,
            TrafficTier::Large,
            TrafficTier::Enterprise,
        ]
    }

    /// Returns the display name for this tier
    pub fn display_name(&self) -> &'static str {
        match self {
            TrafficTier::Micro => "Micro (~100/day)",
            TrafficTier::Small => "Small (~1K/day)",
            TrafficTier::Medium => "Medium (~10K/day)",
            TrafficTier::Large => "Large (~100K/day)",
            TrafficTier::Enterprise => "Enterprise (~1M+/day)",
        }
    }

    /// Returns the expected daily users for this tier
    pub fn daily_users(&self) -> u64 {
        match self {
            TrafficTier::Micro => 100,
            TrafficTier::Small => 1_000,
            TrafficTier::Medium => 10_000,
            TrafficTier::Large => 100_000,
            TrafficTier::Enterprise => 1_000_000,
        }
    }

    /// Returns the CAPTCHA pool size for this tier (pre-rendered pages)
    /// Memory usage: ~132 KB per page
    /// Micro=128MB, Small=257MB, Medium=645MB, Large=1.3GB, Enterprise=2.6GB
    pub fn pool_size(&self) -> usize {
        match self {
            TrafficTier::Micro => 1_000,      // ~128 MB
            TrafficTier::Small => 2_000,      // ~257 MB
            TrafficTier::Medium => 5_000,     // ~645 MB
            TrafficTier::Large => 10_000,     // ~1.3 GB
            TrafficTier::Enterprise => 20_000, // ~2.6 GB
        }
    }

    /// Returns the minimum CAPTCHA pool size for this tier
    pub fn min_pool_size(&self) -> usize {
        match self {
            TrafficTier::Micro => 100,
            TrafficTier::Small => 200,
            TrafficTier::Medium => 500,
            TrafficTier::Large => 1_000,
            TrafficTier::Enterprise => 2_000,
        }
    }

    /// Returns the maximum CAPTCHA pool size for this tier
    pub fn max_pool_size(&self) -> usize {
        match self {
            TrafficTier::Micro => 2_000,      // 2x pool_size
            TrafficTier::Small => 4_000,      // 2x pool_size
            TrafficTier::Medium => 10_000,    // 2x pool_size
            TrafficTier::Large => 20_000,     // 2x pool_size
            TrafficTier::Enterprise => 40_000, // 2x pool_size
        }
    }

    /// Returns the rate limit (requests per minute) for this tier
    pub fn rate_limit_rpm(&self) -> u32 {
        match self {
            TrafficTier::Micro => 30,
            TrafficTier::Small => 60,
            TrafficTier::Medium => 120,
            TrafficTier::Large => 300,
            TrafficTier::Enterprise => 600,
        }
    }

    /// Returns the DDoS detection threshold (requests per second) for this tier
    pub fn ddos_rps_threshold(&self) -> u32 {
        match self {
            TrafficTier::Micro => 20,
            TrafficTier::Small => 100,
            TrafficTier::Medium => 500,
            TrafficTier::Large => 1_000, // Reduced from 2K for Tor realism
            TrafficTier::Enterprise => 3_000, // Reduced from 10K for Tor realism
        }
    }

    /// Returns the minimum mirrors for this tier
    pub fn min_mirrors(&self) -> u32 {
        match self {
            TrafficTier::Micro => 1,
            TrafficTier::Small => 2,
            TrafficTier::Medium => 3,
            TrafficTier::Large => 5,
            TrafficTier::Enterprise => 10,
        }
    }

    /// Returns the maximum mirrors for this tier
    pub fn max_mirrors(&self) -> u32 {
        match self {
            TrafficTier::Micro => 2,
            TrafficTier::Small => 5,
            TrafficTier::Medium => 10,
            TrafficTier::Large => 12, // Reduced from 20 for Tor realism
            TrafficTier::Enterprise => 20, // Reduced from 50 for Tor realism
        }
    }

    /// Returns the standby mirrors for this tier
    pub fn standby_mirrors(&self) -> u32 {
        match self {
            TrafficTier::Micro => 1,
            TrafficTier::Small => 2,
            TrafficTier::Medium => 3,
            TrafficTier::Large => 4,      // Reduced from 5
            TrafficTier::Enterprise => 6, // Reduced from 10
        }
    }

    /// Returns the temporary ban duration (minutes) for this tier
    pub fn temp_ban_minutes(&self) -> u32 {
        match self {
            TrafficTier::Micro => 60,
            TrafficTier::Small => 30,
            TrafficTier::Medium => 15,
            TrafficTier::Large => 10,
            TrafficTier::Enterprise => 5,
        }
    }

    /// Returns the permanent ban threshold for this tier
    pub fn perm_ban_threshold(&self) -> u32 {
        match self {
            TrafficTier::Micro => 5,
            TrafficTier::Small => 10,
            TrafficTier::Medium => 15,
            TrafficTier::Large => 20,
            TrafficTier::Enterprise => 30,
        }
    }

    // =========================================================================
    // SYSTEM REQUIREMENTS
    // =========================================================================

    /// Returns minimum CPU cores for this tier
    pub fn min_cpu_cores(&self) -> u8 {
        match self {
            TrafficTier::Micro => 1,
            TrafficTier::Small => 2,
            TrafficTier::Medium => 2,
            TrafficTier::Large => 4,
            TrafficTier::Enterprise => 4,
        }
    }

    /// Returns recommended CPU cores for this tier
    pub fn recommended_cpu_cores(&self) -> u8 {
        match self {
            TrafficTier::Micro => 2,
            TrafficTier::Small => 4,
            TrafficTier::Medium => 4,
            TrafficTier::Large => 8,
            TrafficTier::Enterprise => 8,
        }
    }

    /// Returns minimum RAM in MB for this tier (includes ~1GB OS overhead for Ubuntu)
    pub fn min_ram_mb(&self) -> u32 {
        match self {
            TrafficTier::Micro => 2048,      // 1GB OS + 1GB Fortify headroom
            TrafficTier::Small => 2048,      // 1GB OS + 1GB Fortify
            TrafficTier::Medium => 3072,     // 1GB OS + 2GB Fortify
            TrafficTier::Large => 5120,      // 1GB OS + 4GB Fortify
            TrafficTier::Enterprise => 8192, // 1GB OS + 7GB Fortify
        }
    }

    /// Returns recommended RAM in MB for this tier (includes ~1GB OS overhead)
    pub fn recommended_ram_mb(&self) -> u32 {
        match self {
            TrafficTier::Micro => 2048,       // Minimal
            TrafficTier::Small => 3072,       // Comfortable
            TrafficTier::Medium => 4096,      // Production
            TrafficTier::Large => 8192,       // Heavy traffic
            TrafficTier::Enterprise => 16384, // High capacity
        }
    }

    /// Returns minimum disk in MB for this tier
    pub fn min_disk_mb(&self) -> u32 {
        match self {
            TrafficTier::Micro => 1024,       // 1GB
            TrafficTier::Small => 2048,       // 2GB
            TrafficTier::Medium => 3072,      // 3GB
            TrafficTier::Large => 6144,       // 6GB
            TrafficTier::Enterprise => 10240, // 10GB
        }
    }

    /// Returns formatted system requirements string
    pub fn system_requirements(&self) -> String {
        format!(
            "CPU: {}+ cores ({}+ rec) | RAM: {}+ MB ({}+ rec) | Disk: {}+ MB",
            self.min_cpu_cores(),
            self.recommended_cpu_cores(),
            self.min_ram_mb(),
            self.recommended_ram_mb(),
            self.min_disk_mb()
        )
    }

    /// Returns the rate limit multiplier for this tier.
    /// Used by GlobalRateLimiter to scale base limits.
    /// Micro=0.5x, Small=1.0x, Medium=2.0x, Large=3.0x, Enterprise=4.0x
    pub fn rate_limit_multiplier(&self) -> f32 {
        match self {
            TrafficTier::Micro => 0.5,
            TrafficTier::Small => 1.0,
            TrafficTier::Medium => 2.0,
            TrafficTier::Large => 3.0,
            TrafficTier::Enterprise => 4.0,
        }
    }
}

/// Root configuration for a Fortify deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FortifyConfig {
    /// Unique deployment identifier
    pub deployment_id: String,
    /// Traffic tier for auto-scaling settings
    #[serde(default)]
    pub traffic_tier: TrafficTier,
    /// Branding configuration
    pub branding: BrandingConfig,
    /// CAPTCHA settings
    pub captcha: CaptchaConfig,
    /// Threshold and limits configuration
    pub thresholds: ThresholdConfig,
    /// Network settings
    pub network: NetworkConfig,
    /// Mirror configuration
    pub mirrors: MirrorConfig,
    /// Vanity address configuration
    pub vanity: VanityConfig,
    /// Path to config file
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
    /// Whether config has unsaved changes
    #[serde(skip)]
    pub dirty: bool,
}

impl FortifyConfig {
    /// Apply traffic tier settings to all related config sections.
    /// This updates CAPTCHA pool sizes, rate limits, mirror counts, and thresholds.
    pub fn apply_traffic_tier(&mut self) {
        let tier = self.traffic_tier;

        // Update CAPTCHA settings
        self.captcha.pool_size = tier.pool_size();
        self.captcha.min_pool_size = tier.min_pool_size();
        self.captcha.max_pool_size = tier.max_pool_size();

        // Update threshold settings
        self.thresholds.rate_limit_rpm = tier.rate_limit_rpm();
        self.thresholds.ddos_rps_threshold = tier.ddos_rps_threshold();
        self.thresholds.temp_ban_minutes = tier.temp_ban_minutes();
        self.thresholds.perm_ban_threshold = tier.perm_ban_threshold();

        // Update mirror settings (cast u32 to usize)
        self.mirrors.min_mirrors = tier.min_mirrors() as usize;
        self.mirrors.max_mirrors = tier.max_mirrors() as usize;
        self.mirrors.standby_mirrors = tier.standby_mirrors() as usize;

        self.dirty = true;
    }
}

/// Branding configuration for the protected service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrandingConfig {
    /// Display name/title for the service
    pub service_name: String,
    /// Short description
    pub description: String,
    /// Primary brand color (hex format: #RRGGBB)
    pub primary_color: String,
    /// Secondary/accent color (hex format: #RRGGBB)
    pub secondary_color: String,
    /// Welcome message on CAPTCHA page
    pub welcome_message: String,
}

/// CAPTCHA pool and behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptchaConfig {
    /// Enable CAPTCHA challenges
    pub enabled: bool,
    /// Target pool size
    pub pool_size: usize,
    /// Minimum pool before emergency generation
    pub min_pool_size: usize,
    /// Maximum pool size
    pub max_pool_size: usize,
    /// CAPTCHA difficulty (1-10)
    pub difficulty: u8,
    /// Time limit to solve in seconds
    pub timeout_seconds: u64,
    /// Maximum solve attempts
    pub max_attempts: u32,
    /// Rotate pool percentage
    pub rotation_percent: u8,
    /// Rotation interval in days
    pub rotation_interval_days: u32,

    // === CAPTCHA Type Settings (NEW) ===
    /// CAPTCHA type used at the Gate (initial verification)
    pub gate_captcha_type: CaptchaType,
    /// CAPTCHA type used for threat/demotion (re-verification)
    pub threat_captcha_type: CaptchaType,
    /// Whether to use separate threat captcha type (if false, use gate type)
    pub threat_captcha_enabled: bool,
    /// Randomly cycle between captcha types
    pub random_cycling: bool,
    /// Types to include in random cycling
    pub cycling_types: Vec<CaptchaType>,
}

/// Threshold and limit settings for threat detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Requests per minute before rate limiting
    pub rate_limit_rpm: u32,
    /// Failed CAPTCHAs before temporary ban
    pub captcha_fail_limit: u32,
    /// Temporary ban duration in minutes
    pub temp_ban_minutes: u32,
    /// Permanent ban threshold (infractions)
    pub perm_ban_threshold: u32,
    /// Suspicious behavior score threshold
    pub suspicion_threshold: f32,
    /// Threat score threshold for immediate action
    pub threat_threshold: f32,
    /// Mirror burn threshold
    pub burn_threshold: f32,
    /// Enable automatic banning
    pub auto_ban_enabled: bool,
    /// DDoS detection: requests per second threshold
    pub ddos_rps_threshold: u32,
    /// Probe detection sensitivity (1-10)
    pub probe_sensitivity: u8,
    /// Maximum demotions before session is killed (orphaned permanently)
    pub max_demotions_before_kill: u32,
    /// Total violations threshold for threat demotion
    pub threat_demotion_threshold: u32,
}

/// Network and Tor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Backend service address to protect
    pub backend_address: String,
    /// SOCKS proxy port
    pub socks_port: u16,
    /// Control port
    pub control_port: u16,
    /// HTTP proxy bind address
    pub http_bind: String,
    /// Gate bind address  
    pub gate_bind: String,
    /// Enable vanguards addon
    pub vanguards_enabled: bool,
    /// Vanguards layer 2 guards
    pub vanguards_layer2: u8,
    /// Vanguards layer 3 guards
    pub vanguards_layer3: u8,
    /// Data directory
    pub data_dir: PathBuf,
}

/// Mirror management settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    /// Minimum active mirrors
    pub min_mirrors: usize,
    /// Maximum mirrors allowed
    pub max_mirrors: usize,
    /// Standby mirrors to maintain
    pub standby_mirrors: usize,
    /// Rotation interval in seconds
    pub rotation_interval_seconds: u64,
    /// Enable proactive burning
    pub proactive_burn_enabled: bool,
    /// Minimum days before proactive burn
    pub burn_interval_days_min: u32,
    /// Maximum days before proactive burn
    pub burn_interval_days_max: u32,
    /// Retirement page display hours
    pub retirement_page_hours: u32,
}

/// Vanity address generation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VanityConfig {
    /// Enable vanity address generation for all mirrors
    pub enabled: bool,
    /// Prefix to match (max 10 characters)
    pub prefix: String,
    /// Enable safety net timeout
    pub safety_net_enabled: bool,
    /// Maximum seconds to search before reducing prefix
    /// Default: 900 (15 min), Testing: 30
    pub safety_net_timeout_seconds: u64,
    /// Minimum prefix length before giving up
    pub min_prefix_length: usize,
    /// Warn threshold - warn if prefix exceeds this length
    pub warn_threshold: usize,
}

impl Default for FortifyConfig {
    fn default() -> Self {
        Self {
            deployment_id: uuid_short(),
            traffic_tier: TrafficTier::default(),
            branding: BrandingConfig::default(),
            captcha: CaptchaConfig::default(),
            thresholds: ThresholdConfig::default(),
            network: NetworkConfig::default(),
            mirrors: MirrorConfig::default(),
            vanity: VanityConfig::default(),
            config_path: None,
            dirty: false,
        }
    }
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            service_name: "Protected Service".to_string(),
            description: "A Fortify-protected onion service".to_string(),
            primary_color: "#c9a227".to_string(), // Gold - primary brand
            secondary_color: "#a68b5b".to_string(), // Muted gold - accents
            welcome_message: "Please complete the verification to continue.".to_string(),
        }
    }
}

impl BrandingConfig {
    /// Maximum allowed length for service name
    pub const MAX_SERVICE_NAME_LEN: usize = 100;
    /// Maximum allowed length for description
    pub const MAX_DESCRIPTION_LEN: usize = 100;

    /// Validate all branding configuration fields
    pub fn validate(&self) -> Result<(), String> {
        // Validate service name length
        if self.service_name.len() > Self::MAX_SERVICE_NAME_LEN {
            return Err(format!(
                "Service name exceeds {} characters",
                Self::MAX_SERVICE_NAME_LEN
            ));
        }

        // Validate description length
        if self.description.len() > Self::MAX_DESCRIPTION_LEN {
            return Err(format!(
                "Description exceeds {} characters",
                Self::MAX_DESCRIPTION_LEN
            ));
        }

        // Validate hex colors
        for (name, color) in [
            ("primary_color", &self.primary_color),
            ("secondary_color", &self.secondary_color),
        ] {
            if !Self::is_valid_hex_color(color) {
                return Err(format!(
                    "{} is not a valid hex color (expected #RRGGBB): {}",
                    name, color
                ));
            }
        }

        Ok(())
    }

    /// Check if a string is a valid hex color (#RRGGBB format)
    pub fn is_valid_hex_color(color: &str) -> bool {
        color.starts_with('#')
            && color.len() == 7
            && color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pool_size: 500,
            min_pool_size: 100,
            max_pool_size: 1000,
            difficulty: 5,
            timeout_seconds: 120,
            max_attempts: 3,
            rotation_percent: 25,
            rotation_interval_days: 10,
            // CAPTCHA Type defaults
            gate_captcha_type: CaptchaType::BmpText,
            threat_captcha_type: CaptchaType::Emoji,
            threat_captcha_enabled: true,
            random_cycling: false,
            cycling_types: vec![
                CaptchaType::BmpText,
                CaptchaType::Emoji,
                CaptchaType::Direction,
            ],
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            rate_limit_rpm: 60,
            captcha_fail_limit: 5,
            temp_ban_minutes: 30,
            perm_ban_threshold: 10,
            suspicion_threshold: 0.5,
            threat_threshold: 0.7,
            burn_threshold: 0.7,
            auto_ban_enabled: true,
            ddos_rps_threshold: 100,
            probe_sensitivity: 5,
            max_demotions_before_kill: 3, // Kill session after 3 demotion cycles
            threat_demotion_threshold: 10, // 10 total violations triggers demotion
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        // Use ~/.local/share/fortify for runtime data (persists across reboots)
        let data_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("fortify");
            path
        } else {
            PathBuf::from("/tmp/fortify")
        };

        Self {
            backend_address: "http://127.0.0.1:9000".to_string(),
            socks_port: 9150,
            control_port: 9151,
            http_bind: "127.0.0.1:8082".to_string(),
            gate_bind: "127.0.0.1:8081".to_string(),
            vanguards_enabled: true,
            vanguards_layer2: 4,
            vanguards_layer3: 8,
            data_dir,
        }
    }
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            min_mirrors: 2,
            max_mirrors: 5,
            standby_mirrors: 2,
            rotation_interval_seconds: 3600,
            proactive_burn_enabled: true,
            burn_interval_days_min: 60,
            burn_interval_days_max: 120,
            retirement_page_hours: 72,
        }
    }
}

impl Default for VanityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prefix: String::new(),
            safety_net_enabled: true,
            // 30 seconds for testing, change to 900 (15 min) for production
            safety_net_timeout_seconds: 30,
            min_prefix_length: 1,
            warn_threshold: 5,
        }
    }
}

impl FortifyConfig {
    /// Load configuration from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: FortifyConfig = toml::from_str(&content)?;
        config.config_path = Some(path.to_path_buf());
        config.dirty = false;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&mut self) -> Result<()> {
        // Use config_path if set, otherwise use default path
        let path = match self.config_path.as_ref() {
            Some(p) => p.clone(),
            None => {
                let default = Self::default_path();
                self.config_path = Some(default.clone());
                default
            }
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Save to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Mark config as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if config has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get default config path (persistent across reboots)
    pub fn default_path() -> PathBuf {
        // Use ~/.config/fortify for persistent storage (survives reboots)
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("fortify");
            path.push("deployment.toml");
            return path;
        }
        // Fallback to /tmp if HOME not available
        PathBuf::from("/tmp/fortify/config/deployment.toml")
    }

    /// List existing deployments
    pub fn list_deployments() -> Result<Vec<(String, PathBuf)>> {
        let config_dir = if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("fortify");
            path.push("deployments");
            path
        } else {
            PathBuf::from("/tmp/fortify/deployments")
        };
        let mut deployments = Vec::new();

        if config_dir.exists() {
            for entry in std::fs::read_dir(config_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let config_file = path.join("config.toml");
                    if config_file.exists() {
                        if let Ok(config) = Self::load(&config_file) {
                            deployments.push((config.deployment_id.clone(), config_file));
                        }
                    }
                }
            }
        }

        Ok(deployments)
    }
}

/// Configuration change that can be applied or stored
#[derive(Debug, Clone)]
pub struct PendingChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PendingChange {
    /// Check if this change requires a restart (cannot be hot-reloaded)
    pub fn requires_restart(&self) -> bool {
        matches!(
            self.field.as_str(),
            "Pool Size"
                | "Min Pool"
                | "Max Pool"
                | "Min Mirrors"
                | "Max Mirrors"
                | "Standby Mirrors"
                | "Backend Address"
                | "SOCKS Port"
                | "Control Port"
                | "HTTP Bind"
                | "Gate Bind"
                | "Vanguards Enabled"
                | "Vanguards Layers"
                | "Data Directory"
                | "Vanity Enabled"
                | "Prefix"
                | "Safety Net Enabled"
                | "Safety Net Timeout"
        )
    }
}

/// Manages pending configuration changes
#[derive(Debug, Default)]
pub struct ChangeManager {
    pub pending_changes: Vec<PendingChange>,
    pub stored_for_restart: Vec<PendingChange>,
}

impl ChangeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_change(&mut self, field: &str, old: &str, new: &str) {
        self.pending_changes.push(PendingChange {
            field: field.to_string(),
            old_value: old.to_string(),
            new_value: new.to_string(),
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    /// Get changes that can be hot-reloaded
    pub fn hot_reload_changes(&self) -> Vec<&PendingChange> {
        self.pending_changes
            .iter()
            .filter(|c| !c.requires_restart())
            .collect()
    }

    /// Get changes that require restart
    pub fn restart_required_changes(&self) -> Vec<&PendingChange> {
        self.pending_changes
            .iter()
            .filter(|c| c.requires_restart())
            .collect()
    }

    /// Check if any pending changes require restart
    pub fn has_restart_required(&self) -> bool {
        self.pending_changes.iter().any(|c| c.requires_restart())
    }

    pub fn store_for_restart(&mut self) {
        self.stored_for_restart.append(&mut self.pending_changes);
    }

    pub fn clear_pending(&mut self) {
        self.pending_changes.clear();
    }

    /// Clear all changes (pending and stored) - for discarding
    pub fn clear(&mut self) {
        self.pending_changes.clear();
        self.stored_for_restart.clear();
    }

    pub fn apply_all(&mut self) {
        self.pending_changes.clear();
        self.stored_for_restart.clear();
    }
}

/// Generate a short UUID for deployment IDs
fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("deploy-{:x}", now & 0xFFFFFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FortifyConfig::default();
        assert!(config.captcha.enabled);
        assert_eq!(config.mirrors.min_mirrors, 2);
        assert_eq!(config.thresholds.burn_threshold, 0.7);
    }
}
