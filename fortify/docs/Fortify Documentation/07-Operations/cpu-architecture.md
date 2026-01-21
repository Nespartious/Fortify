# 🖥️ 4-Core CPU Architecture

> **Optimized CPU Layout for Maximum Protection**

---

## Overview

Fortify uses a specialized 4-core CPU architecture to maximize protection against state-level botnets (100k+ bots). Each CPU core runs an isolated Tor daemon with specific responsibilities.

---

## Core Layout

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         4-CORE CPU ARCHITECTURE                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Core 0 ──▶ Tor Daemon 0 ──▶ Mirror A + Standby D + Healthy 0-4               │
│   Core 1 ──▶ Tor Daemon 1 ──▶ Mirror B + Standby C + Healthy 5-9               │
│   Core 2 ──▶ Tor Daemon 2 ──▶ FLEX CORE (CAPTCHA pre-gen, overflow)            │
│   Core 3 ──▶ Tor Daemon 3 ──▶ Threat Nodes 0-2 (isolated quarantine)           │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Core 0: Primary Mirror A

| Service | Description |
|---------|-------------|
| Mirror A | Primary public-facing mirror |
| Standby D | Backup for Mirror B (cross-paired) |
| Healthy 0-4 | Verified human traffic routing |

### Core 1: Primary Mirror B

| Service | Description |
|---------|-------------|
| Mirror B | Secondary public-facing mirror |
| Standby C | Backup for Mirror A (cross-paired) |
| Healthy 5-9 | Verified human traffic routing |

### Core 2: Flex Core

The Flex Core adapts to system needs:

| Mode | Description | When Active |
|------|-------------|-------------|
| **Standby** | Pre-generates CAPTCHA images | Default (low CPU) |
| **Emergency Mirror** | Temporary primary mirror | Both Core 0/1 mirrors fail |
| **Healthy Overflow** | Absorbs excess traffic | Core 0/1 healthy nodes at capacity |
| **Threat Overflow** | Temporary quarantine | Core 3 threat nodes saturated |

### Core 3: Threat Quarantine

Completely isolated core for suspicious traffic:

- Threat Nodes 0-2
- All demoted sessions quarantined here
- Cannot affect healthy traffic on other cores

---

## Cross-Paired Standbys

Why standbys are on different cores than their primary:

```
Mirror A (Core 0) ◄──────────────────── Standby C (Core 1)
       │                                        │
       │  If Core 0 fails completely,          │
       │  Standby C is SAFE on Core 1          │
       │                                        │
Mirror B (Core 1) ◄──────────────────── Standby D (Core 0)
       │                                        │
       │  If Core 1 fails completely,          │
       │  Standby D is SAFE on Core 0          │
```

This protects against:
- Core-level hardware failures
- Single-daemon crashes
- Localized attack saturation

---

## CAPTCHA Pre-generation

The Flex Core pre-generates CAPTCHAs during idle periods:

### Pool Configuration

| Setting | Value | Description |
|---------|-------|-------------|
| Target Size | 500 | Ideal pool size |
| Minimum Size | 100 | Triggers urgent refill |
| Maximum Size | 1000 | Prevents unbounded growth |

### Generation Behavior

| Setting | Value | Description |
|---------|-------|-------------|
| Check Interval | 5 seconds | How often to check pool |
| Batch Size | 10 | CAPTCHAs per batch |
| CPU Pause Threshold | 70% | Pause if CPU exceeds |
| Batch Delay | 100ms | Delay between batches |

### Rotation (Anti-Prediction)

| Setting | Value | Description |
|---------|-------|-------------|
| Rotation Interval | 10 days | How often to rotate |
| Rotation Percent | 25% | Oldest CAPTCHAs deleted |

This prevents attackers from caching CAPTCHA solutions.

---

## Benefits

| Benefit | Description |
|---------|-------------|
| **Process Isolation** | One daemon crash doesn't affect others |
| **CPU Cache Utilization** | Each core has dedicated cache |
| **Parallelized PoW** | Multiple cores verify PoW simultaneously |
| **Attack Containment** | Threat traffic isolated to Core 3 |
| **Graceful Failover** | Cross-paired standbys survive core failures |
| **Pre-generated CAPTCHAs** | Eliminates CPU spikes during challenges |

---

## Configuration

```toml
[multi_daemon]
enabled = true
daemons_per_vps = 4  # Match your CPU cores
cpu_affinity = true
base_socks_port = 9050
base_control_port = 9051
health_check_interval_seconds = 30
max_health_failures = 3
auto_restart_daemons = true

[multi_daemon.flex_core]
enabled = true
core_id = 2

[multi_daemon.flex_core.captcha_pregen]
enabled = true
target_pool_size = 500
min_pool_size = 100
max_pool_size = 1000
pause_cpu_threshold = 70.0
batch_size = 10
batch_delay_ms = 100
rotation_percent = 25
rotation_interval_days = 10
```

---

## Monitoring

### CAPTCHA Pool Stats

Available via API or admin panel:

```json
{
  "current_size": 487,
  "target_size": 500,
  "min_size": 100,
  "max_size": 1000,
  "oldest_age_seconds": 432000,
  "newest_age_seconds": 120,
  "needs_refill": false,
  "total_generated": 2500,
  "total_served": 2013,
  "total_expired": 250
}
```

### Daemon Health

Each daemon reports health status:

- **Healthy** - Normal operation
- **Degraded** - Minor issues, still functional
- **Unhealthy** - Needs attention
- **Dead** - Requires restart

Auto-restart is enabled by default for unhealthy daemons.

---

## Scaling Notes

For VPS with different core counts:

| Cores | Recommended Layout |
|-------|-------------------|
| 2 | Core 0: Mirrors + Healthy, Core 1: Threat |
| 4 | Standard layout (as documented) |
| 6 | Add cores 4-5 for additional healthy capacity |
| 8 | Add cores 4-7 for distributed load |

---

## Troubleshooting

### Pool Not Filling

1. Check if CPU usage is below 70%
2. Verify `captcha_pregen.enabled = true`
3. Check logs for generation errors

### Daemon Not Starting

1. Check port availability (`base_socks_port + daemon_id`)
2. Verify Tor binary is installed
3. Check data directory permissions

### High CPU on Flex Core

The Flex Core automatically pauses CAPTCHA generation when CPU exceeds threshold. If you see sustained high CPU:

1. Check for traffic overflow events
2. Consider upgrading to more cores
3. Reduce `batch_size` or increase `batch_delay_ms`
