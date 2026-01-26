# ⚙️ Configuration Reference

> **Complete Guide to Fortify Configuration Options**

**Status:** ✅ COMPLETE - Extracted from fortify.example.toml  
**Last Updated:** January 25, 2026

---

## Overview

Fortify is configured through a TOML configuration file, typically located at:
- **Development:** `config/fortify.example.toml`
- **Production:** `/etc/fortify/fortify.toml`

---

## Configuration File Structure

```toml
[service]          # Backend service settings
[controller]       # Controller resource management
[orchestrator]     # Mirror and Tor management
[gate]             # CAPTCHA and verification
[http_proxy]       # Request routing and analysis
[node]             # Node pool configuration
[community]        # (Future) Network participation
[logging]          # Log output configuration
[security]         # Security hardening options
```

---

## [service] - Backend Service

Defines the real hidden service that Fortify protects.

```toml
[service]
real_onion_address = "http://your-real-service.onion"
real_service_port = 80
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `real_onion_address` | string | **REQUIRED** | Your actual hidden service .onion address |
| `real_service_port` | integer | 80 | Port your backend service listens on |

**Important:** Never expose this address publicly. Only nodes communicate with it.

---

## [controller] - Resource Management

Controls auto-scaling and resource allocation.

```toml
[controller]
bind_address = "127.0.0.1:7000"
max_orchestrators = 5
max_healthy_nodes = 10
max_threat_nodes = 5
scale_up_threshold = 0.8
scale_down_threshold = 0.2
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_address` | string | "127.0.0.1:7000" | Internal controller API address |
| `max_orchestrators` | integer | 5 | Maximum number of orchestrator instances |
| `max_healthy_nodes` | integer | 10 | Maximum healthy-tier nodes |
| `max_threat_nodes` | integer | 5 | Maximum threat-tier nodes |
| `scale_up_threshold` | float | 0.8 | CPU/memory % to trigger scale-up (0.0-1.0) |
| `scale_down_threshold` | float | 0.2 | CPU/memory % to trigger scale-down (0.0-1.0) |

### Sizing Guidance

**Small deployment:**
```toml
max_orchestrators = 2
max_healthy_nodes = 3
max_threat_nodes = 2
```

**Medium deployment:**
```toml
max_orchestrators = 3
max_healthy_nodes = 5
max_threat_nodes = 3
```

**Large deployment:**
```toml
max_orchestrators = 5
max_healthy_nodes = 10
max_threat_nodes = 5
```

---

## [orchestrator] - Mirror Management

Controls public mirror creation and Tor integration.

```toml
[orchestrator]
bind_address = "127.0.0.1:8080"
public_bind_address = "127.0.0.1:8080"
max_connections_per_minute = 100
max_failed_challenges = 50
rotation_interval_hours = 24
tor_control_port = "127.0.0.1:9051"
tor_socks_port = "127.0.0.1:9050"
tor_control_addr = "127.0.0.1:9151"
tor_cookie_path = "/var/lib/tor/control_auth_cookie"
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_address` | string | "127.0.0.1:8080" | Internal orchestrator API |
| `public_bind_address` | string | "127.0.0.1:8080" | Public-facing address (if different) |
| `max_connections_per_minute` | integer | 100 | Burn threshold: connections/minute |
| `max_failed_challenges` | integer | 50 | Burn threshold: failed CAPTCHAs |
| `rotation_interval_hours` | integer | 24 | Automatic mirror rotation interval |
| `tor_control_port` | string | "127.0.0.1:9051" | Tor control port address |
| `tor_socks_port` | string | "127.0.0.1:9050" | Tor SOCKS proxy port |
| `tor_control_addr` | string | "127.0.0.1:9151" | Alternative control address |
| `tor_cookie_path` | string | "/var/lib/tor/control_auth_cookie" | Path to Tor authentication cookie |

**Note:** Verify `tor_cookie_path` matches your Tor installation.

---

## [gate] - Verification & CAPTCHA

Controls CAPTCHA challenges and token issuance.

```toml
[gate]
bind_address = "127.0.0.1:8081"
max_concurrent_verifications = 10
verification_timeout_seconds = 300
captcha_difficulty = "medium"
pow_difficulty = 20
token_lifetime_seconds = 3600
token_signing_key = "/etc/fortify/gate-signing.key"
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_address` | string | "127.0.0.1:8081" | Gate service bind address |
| `max_concurrent_verifications` | integer | 10 | Max simultaneous CAPTCHA sessions |
| `verification_timeout_seconds` | integer | 300 | CAPTCHA expiration (5 minutes) |
| `captcha_difficulty` | string | "medium" | CAPTCHA difficulty: "easy", "medium", "hard" |
| `pow_difficulty` | integer | 20 | Proof-of-Work leading zero bits (unused currently) |
| `token_lifetime_seconds` | integer | 3600 | Session token TTL (1 hour) |
| `token_signing_key` | string | **REQUIRED** | Path to HMAC signing key file |

### CAPTCHA Difficulty

| Level | Description | Use Case |
|-------|-------------|----------|
| `easy` | Simple challenges | Low-threat environments |
| `medium` | Moderate challenges | Standard protection |
| `hard` | Complex challenges | High-threat or suspicious users |

### Generate Signing Key

```bash
# Generate 256-bit key
sudo openssl rand -hex 32 > /etc/fortify/gate-signing.key
sudo chmod 600 /etc/fortify/gate-signing.key
```

**Security:** Keep this key secret. Compromising it allows token forgery.

---

## [http_proxy] - Request Routing

Controls HTTP proxy behavior and limits.

```toml
[http_proxy]
bind_address = "127.0.0.1:8082"
max_concurrent_connections = 1000
connection_timeout_seconds = 30
max_request_size_bytes = 10485760
queue_size = 100
reject_when_full = true
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_address` | string | "127.0.0.1:8082" | HTTP proxy bind address |
| `max_concurrent_connections` | integer | 1000 | Maximum simultaneous connections |
| `connection_timeout_seconds` | integer | 30 | Connection idle timeout |
| `max_request_size_bytes` | integer | 10485760 | Max request body size (10 MB) |
| `queue_size` | integer | 100 | Request queue depth |
| `reject_when_full` | boolean | true | Reject new requests when queue full |

### Sizing by Traffic

**Low traffic (<100 concurrent users):**
```toml
max_concurrent_connections = 100
queue_size = 50
```

**Medium traffic (100-500 concurrent users):**
```toml
max_concurrent_connections = 500
queue_size = 100
```

**High traffic (>500 concurrent users):**
```toml
max_concurrent_connections = 1000
queue_size = 200
```

---

## [node] - Node Pool

Configures healthy and threat node pools.

```toml
[node]
bind_base = "127.0.0.1:9100"
backend_address = "http://127.0.0.1:9000"
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_base` | string | "127.0.0.1:9100" | Base port for node pool (auto-incremented) |
| `backend_address` | string | **REQUIRED** | Internal address for backend routing |

**Note:** Nodes are assigned sequential ports: 9100, 9101, 9102, etc.

---

## [community] - Network Participation

> 🚧 **FUTURE FEATURE** - Not yet implemented

```toml
[community]
enabled = false
mode = "standalone"
registry_url = ""
update_interval_seconds = 3600
signing_key_path = ""
```

### Planned Modes

| Mode | Description |
|------|-------------|
| `standalone` | Single-instance (current) |
| `consumer` | Use community mirror pool |
| `provider` | Share mirrors with community |
| `seed` | Registry/coordination node |

---

## [logging] - Log Configuration

Controls log output format and destination.

```toml
[logging]
level = "info"
output = "syslog"
log_file = "/var/log/fortify/fortify.log"
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `level` | string | "info" | Log level: "trace", "debug", "info", "warn", "error" |
| `output` | string | "syslog" | Output: "syslog", "file", "stdout" |
| `log_file` | string | "/var/log/fortify/fortify.log" | File path (when output="file") |

### Log Levels

| Level | Use Case | Verbosity |
|-------|----------|-----------|
| `error` | Production, minimal logging | Errors only |
| `warn` | Production, important events | Errors + warnings |
| `info` | Production, standard | Errors + warnings + info |
| `debug` | Development, troubleshooting | All above + debug details |
| `trace` | Development, deep debugging | Everything including traces |

**Recommendation:** Use `info` in production, `debug` for troubleshooting.

---

## [security] - Security Hardening

Additional security options.

```toml
[security]
drop_privileges = true
chroot_path = ""
secure_memory = true
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `drop_privileges` | boolean | true | Drop root privileges after binding ports |
| `chroot_path` | string | "" | Chroot jail path (empty = disabled) |
| `secure_memory` | boolean | true | Lock sensitive data in memory (prevents swapping) |

**Security Best Practices:**
- Always enable `drop_privileges` in production
- Consider `chroot_path` for maximum isolation
- Keep `secure_memory = true` to prevent key leakage

---

## Environment Variables

Additional configuration via environment:

### Vanguards

```bash
export VANGUARDS_ENABLED=true
export VANGUARDS_LAYER2_GUARDS=4
export VANGUARDS_LAYER3_GUARDS=8
export VANGUARDS_CIRC_MAX_AGE_HOURS=24
export VANGUARDS_CIRC_MAX_MEGABYTES=0
```

### Behavioral Analysis

```bash
export BEHAVIORAL_UA_ANALYSIS_ENABLED=true
export BEHAVIORAL_REFERER_ANALYSIS_ENABLED=true
export BEHAVIORAL_PATH_ANALYSIS_ENABLED=true
```

### Rate Limiting

```bash
export RATE_LIMIT_UNKNOWN_TIER=10
export RATE_LIMIT_VERIFIED_TIER=100
export RATE_LIMIT_TRUSTED_TIER=300
```

---

## Complete Example Configuration

```toml
# Production Fortify Configuration

[service]
real_onion_address = "http://your-backend.onion"
real_service_port = 80

[controller]
bind_address = "127.0.0.1:7000"
max_orchestrators = 3
max_healthy_nodes = 5
max_threat_nodes = 3
scale_up_threshold = 0.8
scale_down_threshold = 0.2

[orchestrator]
bind_address = "127.0.0.1:8080"
public_bind_address = "127.0.0.1:8080"
max_connections_per_minute = 100
max_failed_challenges = 50
rotation_interval_hours = 24
tor_control_port = "127.0.0.1:9051"
tor_socks_port = "127.0.0.1:9050"
tor_cookie_path = "/var/lib/tor/control_auth_cookie"

[gate]
bind_address = "127.0.0.1:8081"
max_concurrent_verifications = 10
verification_timeout_seconds = 300
captcha_difficulty = "medium"
pow_difficulty = 20
token_lifetime_seconds = 3600
token_signing_key = "/etc/fortify/gate-signing.key"

[http_proxy]
bind_address = "127.0.0.1:8082"
max_concurrent_connections = 500
connection_timeout_seconds = 30
max_request_size_bytes = 10485760
queue_size = 100
reject_when_full = true

[node]
bind_base = "127.0.0.1:9100"
backend_address = "http://127.0.0.1:9000"

[community]
enabled = false
mode = "standalone"

[logging]
level = "info"
output = "syslog"
log_file = "/var/log/fortify/fortify.log"

[security]
drop_privileges = true
chroot_path = ""
secure_memory = true
```

---

## Configuration Validation

Test your configuration:

```bash
# Dry-run configuration check
./target/release/fortify-controller --config /etc/fortify/fortify.toml --check

# Verbose validation
RUST_LOG=debug ./target/release/fortify-controller --config /etc/fortify/fortify.toml --check
```

---

## Tuning for Different Scenarios

### High Security

```toml
[gate]
captcha_difficulty = "hard"
token_lifetime_seconds = 1800  # 30 minutes

[http_proxy]
max_concurrent_connections = 500  # Lower limit

[security]
drop_privileges = true
chroot_path = "/var/fortify/jail"
secure_memory = true
```

### High Performance

```toml
[controller]
max_healthy_nodes = 15
max_threat_nodes = 5

[http_proxy]
max_concurrent_connections = 2000
queue_size = 500

[gate]
max_concurrent_verifications = 20
```

### Low Resources

```toml
[controller]
max_orchestrators = 2
max_healthy_nodes = 3
max_threat_nodes = 2

[http_proxy]
max_concurrent_connections = 100
queue_size = 50

[gate]
max_concurrent_verifications = 5
```

---

*For operational guidance, see [Operations Guide](../06-Operations/monitoring.md)*
