# ⚙️ Configuration Reference

> **Complete Configuration Options for All Fortify Components**

---

## Configuration File

**Location:** `/etc/fortify/fortify.toml` (copy from `config/fortify.example.toml`)

---

## Service Configuration

```toml
[service]
# The real hidden service onion address (NEVER exposed publicly)
real_onion_address = "http://xxxxx.onion"

# Port the real service listens on
real_service_port = 80
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `real_onion_address` | String | Required | Your protected hidden service URL |
| `real_service_port` | u16 | 80 | Port your backend listens on |

---

## Controller Configuration

```toml
[controller]
# Internal bind address
bind_address = "127.0.0.1:7000"

# Service pool limits
max_orchestrators = 5
max_healthy_nodes = 10
max_threat_nodes = 5

# Scaling thresholds (0.0 - 1.0)
scale_up_threshold = 0.8
scale_down_threshold = 0.2

# Vanguards addon settings
vanguards_enabled = true
vanguards_layer2_guards = 4
vanguards_layer3_guards = 8
vanguards_circ_max_age_hours = 24
vanguards_circ_max_megabytes = 0  # 0 = unlimited

# Health check interval
health_check_interval_seconds = 30
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | String | "127.0.0.1:7000" | Controller HTTP API address |
| `max_orchestrators` | usize | 5 | Maximum orchestrator processes |
| `max_healthy_nodes` | usize | 10 | Maximum healthy node pool size |
| `max_threat_nodes` | usize | 5 | Maximum threat node pool size |
| `scale_up_threshold` | f32 | 0.8 | CPU/Memory % to trigger scale up |
| `scale_down_threshold` | f32 | 0.2 | CPU/Memory % to trigger scale down |
| `vanguards_enabled` | bool | true | Enable vanguards addon |
| `vanguards_layer2_guards` | u8 | 4 | Number of layer 2 guards |
| `vanguards_layer3_guards` | u8 | 8 | Number of layer 3 guards |

---

## Orchestrator Configuration

```toml
[orchestrator]
# Public-facing bind address
bind_address = "127.0.0.1:8080"
public_bind_address = "127.0.0.1:8080"

# Mirror pool settings
min_mirrors = 2
max_mirrors = 5
standby_mirrors = 2

# Rotation settings
rotation_interval_hours = 24
burn_threshold = 0.7

# Tor control settings
tor_control_addr = "127.0.0.1:9151"
tor_cookie_path = "/tmp/fortify/tor/data/control_auth_cookie"
tor_data_dir = "/tmp/fortify/tor/mirrors"

# Rate limiting triggers
max_connections_per_minute = 100
max_failed_challenges = 50
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | String | "127.0.0.1:8080" | Orchestrator bind address |
| `min_mirrors` | usize | 2 | Minimum active mirrors |
| `max_mirrors` | usize | 5 | Maximum total mirrors |
| `standby_mirrors` | usize | 2 | Paused mirrors ready to activate |
| `rotation_interval_hours` | u64 | 24 | Hours between scheduled rotations |
| `burn_threshold` | f32 | 0.7 | Compromise score to trigger burn |
| `tor_control_addr` | String | "127.0.0.1:9151" | Tor control port address |
| `tor_cookie_path` | String | - | Path to Tor auth cookie |

---

## Gate Configuration

```toml
[gate]
# Internal bind address
bind_address = "127.0.0.1:8081"

# Rate limiting
max_concurrent_verifications = 10
verification_timeout_seconds = 300

# Challenge settings
captcha_difficulty = "medium"  # easy, medium, hard
pow_difficulty = 20            # Leading zero bits required

# Token settings
token_lifetime_seconds = 3600  # 1 hour
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | String | "127.0.0.1:8081" | Gate HTTP server address |
| `max_concurrent_verifications` | usize | 10 | Max simultaneous verifications |
| `verification_timeout_seconds` | u64 | 300 | Seconds before challenge expires |
| `captcha_difficulty` | String | "medium" | Default captcha difficulty |
| `pow_difficulty` | u32 | 20 | PoW difficulty (currently disabled) |
| `token_lifetime_seconds` | u64 | 3600 | Session token validity |

### Captcha Difficulty Levels

| Level | Text Length | Noise | Distortion |
|-------|-------------|-------|------------|
| `easy` | 4 chars | Low | Minimal |
| `medium` | 6 chars | Medium | Moderate |
| `hard` | 8 chars | High | Significant |

---

## HTTP Proxy Configuration

```toml
[http_proxy]
# Public bind address
bind_address = "127.0.0.1:8082"

# Connection limits
max_concurrent_connections = 1000
connection_timeout_seconds = 30
max_request_size_bytes = 10485760  # 10MB

# Backpressure settings
queue_size = 100
reject_when_full = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | String | "127.0.0.1:8082" | Proxy HTTP server address |
| `max_concurrent_connections` | usize | 1000 | Maximum active connections |
| `connection_timeout_seconds` | u64 | 30 | Connection timeout |
| `max_request_size_bytes` | usize | 10MB | Maximum request body size |
| `queue_size` | usize | 100 | Pending request queue size |
| `reject_when_full` | bool | true | Reject when queue is full |

---

## Node Configuration

```toml
[node]
# Base bind address (offset per node)
bind_base = "127.0.0.1:9100"

# Backend (your real service)
backend_address = "http://127.0.0.1:9000"

# Gate for redirects
gate_address = "http://127.0.0.1:8081"

# Limits
max_request_size_bytes = 10485760  # 10MB

# Thresholds
violation_threshold = 3   # Violations before demotion
promotion_threshold = 50  # Clean requests for promotion
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_base` | String | "127.0.0.1:9100" | Base address for node pool |
| `backend_address` | String | Required | Your real service URL |
| `gate_address` | String | "127.0.0.1:8081" | Gate URL for redirects |
| `violation_threshold` | u32 | 3 | Violations to trigger demotion |
| `promotion_threshold` | u32 | 50 | Clean requests for promotion |

### Node Modes

| Mode | Max Req/Min | Timeout | Inspection Level |
|------|-------------|---------|------------------|
| `Healthy` | 20 | 30s | Minimal |
| `Threat` | 10 | 10s | Deep |

---

## Behavioral Analysis Configuration

```toml
[behavioral]
enabled = true

# Detection toggles
ua_analysis_enabled = true
referer_analysis_enabled = true
path_analysis_enabled = true
enumeration_detection_enabled = true
form_tracking_enabled = true
payload_analysis_enabled = true

# Rate limits
max_unique_paths_per_minute = 60
max_form_submissions_per_minute = 10
max_payload_size_bytes = 10485760  # 10MB
min_post_payload_size = 1

# Pattern detection
sequential_path_threshold = 5

# Demotion thresholds
threat_demotion_threshold = 10      # Total violations
threat_severity_threshold = 15      # Cumulative severity

# Kill threshold
max_demotions_before_kill = 3
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | true | Enable behavioral analysis |
| `max_unique_paths_per_minute` | u32 | 60 | Paths before enumeration flag |
| `max_form_submissions_per_minute` | u32 | 10 | POSTs before flood flag |
| `sequential_path_threshold` | u32 | 5 | Sequential paths to detect scan |
| `threat_demotion_threshold` | u32 | 10 | Violations to demote |
| `threat_severity_threshold` | u32 | 15 | Severity score to demote |
| `max_demotions_before_kill` | u32 | 3 | Demotions before permanent ban |

### Violation Type Thresholds

```toml
[behavioral.violation_thresholds]
"Attack Path Access" = 3
"Suspicious User-Agent" = 5
"Path Enumeration" = 3
"Resource Enumeration" = 3
"Form Submission Flood" = 3
"Automated Behavior" = 2
"Suspicious Referer" = 10
"Oversized Payload" = 5
"Undersized Payload" = 10
```

### Custom Whitelists

```toml
[behavioral.whitelists]
custom_paths = [
    "/api/*",
    "/static/*",
    "/assets/*",
]

# Disable specific attack path patterns
disabled_attack_paths = [
    "/admin",      # If you have a legit /admin
    "/test",       # If you have a /test endpoint
]
```

---

## Captcha Configuration

```toml
[captcha]
# Gate (initial verification)
gate_captcha_type = "BmpText"

# Threat (re-verification)
threat_captcha_type = "Emoji"
threat_captcha_enabled = true

# Random cycling
random_cycling = false
cycling_types = ["BmpText", "Emoji", "Direction"]
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `gate_captcha_type` | String | "BmpText" | Type for new users |
| `threat_captcha_type` | String | "Emoji" | Type for demoted users |
| `threat_captcha_enabled` | bool | true | Use different type for threats |
| `random_cycling` | bool | false | Randomly cycle types |
| `cycling_types` | Array | [...] | Types to include in cycling |

### Captcha Types

| Type | Input | Description |
|------|-------|-------------|
| `BmpText` | Text field | Type characters from image |
| `Emoji` | Buttons | Select matching emoji |
| `Direction` | Buttons | Select arrow direction |
| `Sequence` | Buttons | Complete the sequence |
| `WordUnscramble` | Text field | Unscramble the word |
| `ImageRotation` | Buttons | Select upright image |
| `Silhouette` | Buttons | Identify silhouette |

### Per-Type Configuration

```toml
[captcha.types.Emoji]
enabled = true
option_count = 6
difficulty = 2

[captcha.types.Direction]
enabled = true
option_count = 4
difficulty = 1

[captcha.types.Sequence]
enabled = true
option_count = 4
difficulty = 2
```

---

## Community Network Configuration

```toml
[community]
enabled = false  # Opt-in
mode = "standalone"  # standalone, consumer, provider, seed

# Network settings
registry_url = ""
bind_addr = "127.0.0.1:9005"

# Seed management
max_seeds = 100
seed_ttl_days = 7

# Discovery
discovery_enabled = true
max_discovery_hops = 3

# Rate limiting
share_rate_limit = 10  # requests per minute

# Keys
signing_key_path = ""
```

| Mode | Description |
|------|-------------|
| `standalone` | No network participation |
| `consumer` | Receive seeds, don't share |
| `provider` | Share seeds with network |
| `seed` | Act as seed server |

---

## Logging Configuration

```toml
[logging]
level = "info"  # trace, debug, info, warn, error
output = "syslog"  # syslog, file, stdout
log_file = "/var/log/fortify/fortify.log"

# Structured logging
json_format = false
include_timestamp = true
include_thread = false
```

| Level | Description |
|-------|-------------|
| `trace` | Everything (very verbose) |
| `debug` | Detailed debugging info |
| `info` | Normal operation logs |
| `warn` | Warnings and anomalies |
| `error` | Errors only |

---

## Security Configuration

```toml
[security]
# Privilege dropping
drop_privileges = true
run_as_user = "fortify"
run_as_group = "fortify"

# Chroot (optional)
chroot_path = ""

# Memory protection
secure_memory = true

# Secret key for token signing
secret_key_file = "/etc/fortify/secret.key"
# Or generate random on startup:
# secret_key = ""  # Empty = auto-generate
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `drop_privileges` | bool | true | Drop root after binding |
| `secure_memory` | bool | true | Lock memory to prevent swap |
| `secret_key_file` | String | - | Path to signing key file |

---

## Session Continuity Configuration (Planned)

```toml
[session_continuity]
# Enable session continuity for paused VMs
enabled = true

# Maximum days to retain session history
max_age_days = 7

# Storage backend: sqlite, sled, memory
storage_backend = "sqlite"

# Database file path
database_path = "/var/lib/fortify/sessions.db"

[session_continuity.transfer]
# Transfer trust tier to new session
transfer_tier = true

# Transfer demotion count to new session
transfer_demotion_count = true

# Reset violation count (fresh start)
reset_violation_count = true

# Block killed sessions from continuing
deny_if_killed = true

# Block burned sessions from continuing
deny_if_burned = true

[session_continuity.cleanup]
# How often to clean expired records (hours)
run_interval_hours = 24

# SQLite vacuum after cleanup
vacuum_on_cleanup = true
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | true | Enable session continuity |
| `max_age_days` | u32 | 7 | Maximum history retention |
| `storage_backend` | String | "sqlite" | sqlite, sled, or memory |
| `database_path` | String | - | Path to database file |
| `transfer_tier` | bool | true | Transfer trust tier |
| `transfer_demotion_count` | bool | true | Transfer demotion history |
| `reset_violation_count` | bool | true | Reset violations |
| `deny_if_killed` | bool | true | Block killed sessions |
| `deny_if_burned` | bool | true | Block burned sessions |

---

## Dynamic Rate Limiting Configuration

```toml
[rate_limiting.dynamic]
# Enable load-based rate limiting
enabled = true

# How often to check system load (ms)
check_interval_ms = 1000

# CPU thresholds for rate reduction
cpu_threshold_warning = 50    # Start reducing at 50%
cpu_threshold_critical = 85   # Emergency mode at 85%

# Memory thresholds
memory_threshold_warning = 70
memory_threshold_critical = 90

# Connection count threshold
connection_threshold = 5000

# Never go below this multiplier (10% of normal)
min_rate_multiplier = 0.1

# Wait before increasing limits again
recovery_delay_seconds = 30
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | true | Enable dynamic rate limiting |
| `check_interval_ms` | u32 | 1000 | Load check frequency |
| `cpu_threshold_warning` | u8 | 50 | CPU % for 25% reduction |
| `cpu_threshold_critical` | u8 | 85 | CPU % for emergency mode |
| `min_rate_multiplier` | f32 | 0.1 | Minimum rate multiplier |

---

## Bandwidth Throttling Configuration

```toml
[bandwidth_throttling]
enabled = true

[bandwidth_throttling.limits]
# Per-tier bandwidth limits (MB per minute, 0 = unlimited)
trusted_mb_per_min = 0
verified_mb_per_min = 10
unknown_mb_per_min = 5
suspicious_mb_per_min = 1
demoted_kb_per_min = 500

[bandwidth_throttling.delays]
# Additional response delay for low-trust tiers
suspicious_delay_ms = 500
demoted_delay_ms = 1000
cumulative = true           # Delays stack

[bandwidth_throttling.burst]
enabled = true
multiplier = 2.0            # 2x limit for burst
duration_seconds = 10       # Burst window
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `trusted_mb_per_min` | u32 | 0 | Trusted tier limit (0=unlimited) |
| `verified_mb_per_min` | u32 | 10 | Verified tier limit |
| `suspicious_mb_per_min` | u32 | 1 | Suspicious tier limit |
| `suspicious_delay_ms` | u32 | 500 | Extra delay per request |

---

## Honeypot Configuration

```toml
[honeypots]
enabled = true
log_attempts = true
immediate_demotion = true

[[honeypots.endpoints]]
path = "/admin"
type = "admin_trap"
response = "fake_login"
tarpit_seconds = 30

[[honeypots.endpoints]]
path = "/.env"
type = "file_trap"
immediate_burn = true

[[honeypots.endpoints]]
path = "/api/v1/users"
type = "api_trap"
response = "fake_json"
tarpit_seconds = 60
infinite_pagination = true

[honeypots.hidden_fields]
enabled = true
field_name = "website_url"
css_hide = true
fill_action = "demote"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | true | Enable honeypots |
| `log_attempts` | bool | true | Log honeypot access |
| `immediate_demotion` | bool | true | Auto-demote on access |
| `tarpit_seconds` | u32 | 30 | Slow-drip response time |
| `immediate_burn` | bool | false | Burn session on access |

---

## Environment Variables

Override config file settings:

| Variable | Overrides | Example |
|----------|-----------|---------|
| `FORTIFY_CONFIG` | Config file path | `/etc/fortify/fortify.toml` |
| `FORTIFY_LOG_LEVEL` | `logging.level` | `debug` |
| `FORTIFY_SECRET_KEY` | Secret key | `base64_key` |
| `GATE_BIND_ADDR` | `gate.bind_address` | `127.0.0.1:8081` |
| `PROXY_BIND_ADDR` | `http_proxy.bind_address` | `127.0.0.1:8082` |

---

## Configuration Validation

Check configuration:

```bash
# Validate config file
fortify-controller --config /etc/fortify/fortify.toml --validate

# Show effective configuration
fortify-controller --config /etc/fortify/fortify.toml --show-config
```

---

## Runtime Configuration (Admin Panel)

These settings can be changed at runtime via the admin panel:

- ✅ Behavioral analysis toggles
- ✅ Violation thresholds
- ✅ Captcha type selection
- ✅ Whitelist paths
- ✅ Session tier overrides

These require restart:

- ❌ Bind addresses
- ❌ Pool sizes
- ❌ Tor settings
- ❌ Secret key

---

*See [Functions.md](../Functions.md) for complete API reference*
