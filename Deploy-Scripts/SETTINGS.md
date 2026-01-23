# Fortify Settings Reference

This document explains all configurable settings in Fortify, their purposes, and safe value ranges.

## Table of Contents

- [Traffic Tiers](#traffic-tiers)
- [System Requirements by Tier](#system-requirements-by-tier)
- [CAPTCHA Pool Settings](#captcha-pool-settings)
- [Rate Limiting](#rate-limiting)
- [Mirror Settings](#mirror-settings)
- [Ban Thresholds](#ban-thresholds)
- [CAPTCHA Settings](#captcha-settings)
- [Backend Configuration](#backend-configuration)
- [Network Settings](#network-settings)
- [Trust Tier System](#trust-tier-system)

---

## Traffic Tiers

Traffic tiers automatically configure all settings based on your expected daily user count. Choose the tier closest to your traffic volume:

| Tier | Daily Users | Use Case |
|------|-------------|----------|
| **Micro** | ~100 | Personal sites, testing, development |
| **Small** | ~1,000 | Small community sites (DEFAULT) |
| **Medium** | ~10,000 | Active community sites |
| **Large** | ~100,000 | Popular services |
| **Enterprise** | ~1,000,000+ | High-traffic platforms |

### Quick Selection Guide

- **Just testing?** → Micro
- **Personal project or small community?** → Small
- **Growing community with regular traffic?** → Medium
- **Popular service with high traffic?** → Large
- **Major platform or marketplace?** → Enterprise

---

## System Requirements by Tier

### Hardware Cheat Sheet

| Tier | CPU Cores | RAM | Disk | Expected Load |
|------|-----------|-----|------|---------------|
| **Micro** | 1-2 cores | 512MB-1GB | 500MB | Minimal |
| **Small** | 2-4 cores | 1-2GB | 1GB | Light |
| **Medium** | 4 cores | 4GB | 5GB | Moderate |
| **Large** | 8+ cores | 8-16GB | 20GB SSD | Heavy |
| **Enterprise** | 16+ cores | 32GB+ | 100GB+ SSD | Maximum |

### Recommended VPS/Server Types

| Tier | VPS Examples |
|------|--------------|
| Micro | Any $5/mo VPS, Raspberry Pi 4 |
| Small | $10-20/mo VPS (2 vCPU) |
| Medium | $40-60/mo VPS (4 vCPU) |
| Large | Dedicated server or high-end VPS |
| Enterprise | Multiple dedicated servers |

### Minimum Requirements (Any Tier)

- **OS**: Linux (Debian 12+, Ubuntu 22.04+, Fedora 38+)
- **Tor**: Version 0.4.7+
- **Network**: Stable internet connection

---

## CAPTCHA Pool Settings

The CAPTCHA pool maintains pre-generated challenges for instant delivery.

### Settings

| Setting | Description | Safe Range |
|---------|-------------|------------|
| `POOL_SIZE` | Target number of CAPTCHAs to maintain | 50-10,000 |
| `MIN_POOL_SIZE` | Emergency refill threshold | 10-2,000 (10-25% of POOL_SIZE) |
| `MAX_POOL_SIZE` | Maximum pool size for burst capacity | 100-20,000 (2x POOL_SIZE) |

### How It Works

1. Pool maintains `POOL_SIZE` CAPTCHAs ready to serve
2. When pool drops below `MIN_POOL_SIZE`, emergency generation kicks in
3. Pool never exceeds `MAX_POOL_SIZE` to conserve memory

### Tier Defaults

| Tier | Pool Size | Min Pool | Max Pool |
|------|-----------|----------|----------|
| Micro | 50 | 10 | 100 |
| Small | 500 | 100 | 1,000 |
| Medium | 2,000 | 500 | 5,000 |
| Large | 5,000 | 1,000 | 10,000 |
| Enterprise | 10,000 | 2,000 | 20,000 |

### Memory Impact

Each CAPTCHA uses approximately 50-100KB in memory:

| Pool Size | Memory Usage |
|-----------|--------------|
| 50 | ~5MB |
| 500 | ~50MB |
| 2,000 | ~200MB |
| 5,000 | ~500MB |
| 10,000 | ~1GB |

---

## Rate Limiting

Rate limits control how many requests a single Tor circuit can make.

### Settings

| Setting | Description | Safe Range |
|---------|-------------|------------|
| `RATE_LIMIT_RPM` | Requests per minute per circuit | 30-600 |
| `DDOS_RPS_THRESHOLD` | Global RPS to trigger DDoS mode | 20-10,000 |

### Tier Defaults

| Tier | Rate Limit (RPM) | DDoS Threshold (RPS) |
|------|------------------|---------------------|
| Micro | 30 | 20 |
| Small | 60 | 100 |
| Medium | 120 | 500 |
| Large | 300 | 2,000 |
| Enterprise | 600 | 10,000 |

### When to Adjust

- **Too many legitimate users blocked?** → Increase `RATE_LIMIT_RPM`
- **Attackers slipping through?** → Decrease `RATE_LIMIT_RPM`
- **False DDoS alerts?** → Increase `DDOS_RPS_THRESHOLD`
- **DDoS not detected?** → Decrease `DDOS_RPS_THRESHOLD`

---

## Mirror Settings

Mirrors provide redundancy and load distribution for your onion service.

### Settings

| Setting | Description | Safe Range |
|---------|-------------|------------|
| `MIN_MIRRORS` | Minimum active mirrors | 1-10 |
| `MAX_MIRRORS` | Maximum mirrors to scale to | 2-50 |
| `STANDBY_MIRRORS` | Mirrors kept on standby | 1-10 |

### Tier Defaults

| Tier | Min Mirrors | Max Mirrors | Standby |
|------|-------------|-------------|---------|
| Micro | 1 | 2 | 1 |
| Small | 2 | 5 | 2 |
| Medium | 3 | 10 | 3 |
| Large | 5 | 20 | 5 |
| Enterprise | 10 | 50 | 10 |

### Notes

- More mirrors = better availability but more resource usage
- Standby mirrors activate during failover
- Each mirror requires its own Tor circuit

---

## Ban Thresholds

Ban settings control how aggressively to block abusive circuits.

### Settings

| Setting | Description | Safe Range |
|---------|-------------|------------|
| `TEMP_BAN_MINUTES` | Duration of temporary bans | 5-120 minutes |
| `PERM_BAN_THRESHOLD` | Infractions before permanent ban | 3-30 |

### Tier Defaults

| Tier | Temp Ban | Perm Ban Threshold |
|------|----------|-------------------|
| Micro | 60 min | 5 infractions |
| Small | 30 min | 10 infractions |
| Medium | 15 min | 15 infractions |
| Large | 10 min | 20 infractions |
| Enterprise | 5 min | 30 infractions |

### Rationale

- **Low traffic** → Longer bans, fewer infractions (can afford to be strict)
- **High traffic** → Shorter bans, more infractions (more false positives)

---

## CAPTCHA Settings

Configure the CAPTCHA challenge behavior.

### Settings

| Setting | Description | Safe Range |
|---------|-------------|------------|
| `CAPTCHA_DIFFICULTY` | Complexity level (1-10) | 3-7 |
| `CAPTCHA_TIMEOUT_SECONDS` | Time to solve CAPTCHA | 60-300 |
| `CAPTCHA_MAX_ATTEMPTS` | Attempts before circuit ban | 2-5 |

### Defaults

| Setting | Default | Notes |
|---------|---------|-------|
| `CAPTCHA_DIFFICULTY` | 5 | Balanced human/bot discrimination |
| `CAPTCHA_TIMEOUT_SECONDS` | 120 | 2 minutes to solve |
| `CAPTCHA_MAX_ATTEMPTS` | 3 | 3 tries before ban |

### Adjustment Tips

- **Accessibility concerns?** → Decrease difficulty, increase timeout
- **Bot problem?** → Increase difficulty, decrease attempts
- **Tor Browser slow?** → Increase timeout to 180-300s

---

## Backend Configuration

Configure connection to your protected service.

### Settings

| Setting | Description | Example |
|---------|-------------|---------|
| `BACKEND_ADDRESS` | URL of your backend | `http://127.0.0.1:9000` |
| `SERVICE_NAME` | Display name | `My Onion Service` |
| `SERVICE_DESCRIPTION` | Short description | `A privacy-focused service` |
| `PRIMARY_COLOR` | Hex color for branding | `#c9a227` |
| `SECONDARY_COLOR` | Secondary hex color | `#a68b5b` |

### Security Notes

- Always use `127.0.0.1` for backend (never expose to network)
- Backend should only accept connections from Fortify

---

## Network Settings

Configure network ports and bindings.

### Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `SOCKS_PORT` | Tor SOCKS proxy port | 9150 |
| `CONTROL_PORT` | Tor control port | 9151 |
| `HTTP_BIND` | HTTP server binding | 127.0.0.1:8082 |
| `GATE_BIND` | Gate server binding | 127.0.0.1:8081 |

### Important

- All bindings should use `127.0.0.1` (localhost only)
- Never bind to `0.0.0.0` unless in a container

---

## Trust Tier System

The trust tier system automatically classifies visitors based on behavior.

### Tiers

| Tier | Rate Limit | Description |
|------|------------|-------------|
| **Trusted** | 120 RPM | Verified users with good history |
| **Verified** | 60 RPM | Passed CAPTCHA, building trust |
| **Stranger** | 30 RPM | New visitors, requires CAPTCHA |
| **Suspicious** | 10 RPM | Failed verification or rate limited |
| **Hostile** | 0 RPM | Banned circuits |

### Automatic Progression

1. New visitor → Stranger (must solve CAPTCHA)
2. CAPTCHA solved → Verified
3. Consistent good behavior → Trusted
4. Rate limit exceeded → Suspicious
5. Multiple violations → Hostile (banned)

---

## Quick Reference Card

### Choosing Your Tier

```
Personal/Testing    → Micro    (1 core, 512MB)
Small Community     → Small    (2 cores, 1GB)   ← DEFAULT
Active Community    → Medium   (4 cores, 4GB)
Popular Service     → Large    (8 cores, 8GB)
Major Platform      → Enterprise (16+ cores, 32GB)
```

### Deploy Commands

```bash
cd Deploy-Scripts/

# Choose your tier:
./deploy-micro.sh       # ~100 users/day
./deploy-small.sh       # ~1,000 users/day (DEFAULT)
./deploy-medium.sh      # ~10,000 users/day
./deploy-large.sh       # ~100,000 users/day
./deploy-enterprise.sh  # ~1,000,000+ users/day
```

### Emergency Adjustments

If you're under attack:
```bash
# Reduce rate limits
export RATE_LIMIT_RPM=15
export DDOS_RPS_THRESHOLD=50

# Increase CAPTCHA difficulty
export CAPTCHA_DIFFICULTY=8
export CAPTCHA_MAX_ATTEMPTS=2
```

If legitimate users are blocked:
```bash
# Increase rate limits
export RATE_LIMIT_RPM=120
export DDOS_RPS_THRESHOLD=500

# Decrease CAPTCHA difficulty
export CAPTCHA_DIFFICULTY=3
export CAPTCHA_MAX_ATTEMPTS=5
```
