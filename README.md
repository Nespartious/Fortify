# 🏰 Fortify

> **Advanced DDoS Protection for Tor Hidden Services**

<!-- Workflow Status Badges -->
[![CI](https://github.com/Nespartious/Fortify/actions/workflows/ci.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/ci.yml)
[![Security Audit](https://github.com/Nespartious/Fortify/actions/workflows/security.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/security.yml)
[![Code Quality](https://github.com/Nespartious/Fortify/actions/workflows/code-quality.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/code-quality.yml)

<!-- Static Badges -->
[![Status](https://img.shields.io/badge/Status-Alpha-orange)](https://github.com/Nespartious/Fortify)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)
[![Attack Tested](https://img.shields.io/badge/Attack%20Tested-65K%2B%20requests-green)](docs/Dev_Progress/Alpha_Review.md)

**Fortify** is a sophisticated multi-layered defense system that protects Tor hidden services from DDoS attacks while maintaining access for legitimate users. Built in Rust for performance and security, Fortify acts as a proxy shield between attackers and your real hidden service.

---

## 🎯 Project Status: **Alpha**

**Production-Ready Core Protection** ✅  
Successfully defended against **65,576 attack requests** over 3 hours while maintaining access for **280 legitimate users**.

**Current Capabilities:**
- ✅ Per-circuit rate limiting (prevents IP-based blocking on Tor)
- ✅ Single-use verification tokens (prevents CAPTCHA farming)
- ✅ Trust tier system with behavioral analysis
- ✅ Session protection with User-Agent binding
- ✅ 7 CAPTCHA types (JavaScript-free, Tor-compatible)
- ✅ Automatic quota clearing after verification

**In Development:**
- 🔄 TUI deployment wizard (40% complete)
- ⏳ Mirror management automation (Phase 4)
- ⏳ Auto-scaling system (Phase 4)
- ⏳ Cluster support (Phase 5)

[View detailed progress →](fortify/docs/Dev_Progress/Alpha_Review.md)

---

## 🚀 Quick Start

### Installation

```bash
# 1. Update system and install build dependencies
sudo apt update && sudo apt upgrade -y
sudo apt install -y git build-essential pkg-config libssl-dev

# 2. Clone the repository
git clone https://github.com/Nespartious/Fortify.git
cd Fortify/fortify

# 3. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# 4. Build the project
cargo build --release

# 5. Run the TUI
./target/release/fortify
```

### Deployment via TUI

**Fortify uses an interactive TUI (Terminal User Interface) for deployment and management.**

```bash
# Launch the Fortify TUI
./target/release/fortify
```

The TUI provides:
- **Deployment Wizard** - Step-by-step setup for new deployments
- **Live Status Monitoring** - Real-time component health and metrics
- **Configuration Management** - Hot-reload settings without restart
- **Log Streaming** - Multi-component log aggregation
- **Mirror Management** - View and manage .onion addresses

**Navigation:**
- Arrow keys to navigate
- Enter to select
- Tab to switch panels
- Esc to go back
- 'q' to quit

### Headless Deployment (Scripted)

**For automated/headless server deployments without interactive TUI.**

Fortify includes a comprehensive deployment script that configures everything from a single file. Perfect for:
- Automated server provisioning
- Infrastructure-as-code setups
- Headless server environments
- Reproducible deployments

```bash
# 1. Clone the repository
git clone https://github.com/Nespartious/Fortify.git
cd Fortify

# 2. Edit the deployment script configuration
#    Open deploy.sh and modify the settings at the top of the file.
#    Each setting has comments explaining what it does.
nano deploy.sh

# 3. Review your configuration
#    Make sure BACKEND_ADDRESS points to your real service
#    Adjust CAPTCHA, mirror, and security settings as needed

# 4. Run the deployment (requires root)
sudo ./deploy.sh
```

The script will:
- ✅ Install all system dependencies (Tor, Rust, build tools)
- ✅ Apply OS hardening (sysctl, file limits)
- ✅ Build Fortify from source
- ✅ Generate configuration from your settings
- ✅ Install and enable systemd services
- ✅ Start all components

**Key Configuration Sections in deploy.sh:**
| Section | Description |
|---------|-------------|
| Backend Settings | Your protected service address and branding |
| Mirror Settings | Number of mirrors and rotation behavior |
| Vanity Settings | Custom .onion address prefixes |
| CAPTCHA Settings | Pool size, difficulty, timeouts |
| Rate Limiting | Request limits and ban thresholds |
| Vanguards | Tor circuit protection settings |
| Auto-Scaling | Dynamic resource allocation |

### Manual Component Control (Development Only)

For development/debugging, individual components can be run:

```bash
# Controller (resource monitoring)
./target/release/fortify-controller

# Orchestrator (mirror management)
SECRET_KEY="your-secret" ./target/release/fortify-orchestrator

# Gate (CAPTCHA verification)
SECRET_KEY="your-secret" ./target/release/fortify-gate

# HTTP Proxy (main traffic handler)
SECRET_KEY="your-secret" ./target/release/fortify-http

# Nodes (backend services)
NODE_MODE="healthy" ./target/release/fortify-node
NODE_MODE="threat" ./target/release/fortify-node
```

⚠️ **Note:** Manual deployment is complex and error-prone. Use the TUI for reliable deployment.

**Requirements:**
- **Rust:** 1.88 or higher (MSRV)
- **Tor:** Latest stable version
- **OS:** Linux (tested on Ubuntu 22.04/24.04)
- **RAM:** 2GB minimum (4GB recommended for production)
- **Disk:** 1GB for binaries + logs

---

## 📊 How It Works

### Basic User Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    NEW USER JOURNEY                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  User → Mirror.onion                                            │
│    │                                                             │
│    ├─ No token? → Redirect to CAPTCHA                           │
│    │                                                             │
│    ▼                                                             │
│  Gate CAPTCHA Challenge                                         │
│    │                                                             │
│    ├─ SOLVED ──────────────┐                                    │
│    │                        │                                    │
│    ▼                        ▼                                    │
│  Verification Token    [FAILED → Try Again]                     │
│    (60s, single-use)                                            │
│    │                                                             │
│    ▼                                                             │
│  First Request → Upgrade to Session Token                       │
│    │              (24hr, reusable)                               │
│    │                                                             │
│    ▼                                                             │
│  ✓ Access Real Site (Verified tier: 100 req/10s)               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Attack Defense Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    DDOS ATTACK SCENARIO                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Attack Bot #1 ──┐                                              │
│  Attack Bot #2 ──┤                                              │
│  Attack Bot #3 ──┼─► Mirror.onion                               │
│  Attack Bot #N ──┘    │                                         │
│                       │                                          │
│                       ▼                                          │
│                  Rate Limiter (per-circuit)                      │
│                       │                                          │
│                       ├─ Bot #1: 10 req/10s → BLOCKED ❌        │
│                       ├─ Bot #2: 10 req/10s → BLOCKED ❌        │
│                       ├─ Bot #3: 10 req/10s → BLOCKED ❌        │
│                       └─ Each bot isolated, can't exhaust quota │
│                                                                  │
│  Real User ─────────────────────────────────┐                   │
│                                             │                   │
│                                             ▼                   │
│                                        Rate Limiter              │
│                                             │                   │
│                                             ├─ Independent quota│
│                                             ├─ 3 requests → ✓  │
│                                             │                   │
│                                             ▼                   │
│                                        CAPTCHA Challenge        │
│                                             │                   │
│                                             ├─ Solved → ✓      │
│                                             │                   │
│                                             ▼                   │
│                                        Access Site ✓            │
│                                                                  │
│  Result: Bots blocked, real users unaffected                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Demotion & Re-verification Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                BEHAVIORAL VIOLATION FLOW                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Verified User → Suspicious Behavior Detected                   │
│    │              (path scanning, bot UA, etc.)                 │
│    │                                                             │
│    ├─ Violation Count: 1 ──► Warning logged                     │
│    ├─ Violation Count: 2 ──► Warning logged                     │
│    ├─ Violation Count: 3 ──► DEMOTE to Suspicious ⚠️           │
│    │                                                             │
│    ▼                                                             │
│  Redirect to Gate                                               │
│    │                                                             │
│    ▼                                                             │
│  Re-verification Challenge (2x CAPTCHAs)                        │
│    │                                                             │
│    ├─ Both Solved ──────┐                                       │
│    │                     │                                       │
│    ▼                     ▼                                       │
│  Re-issue Token     [FAILED → BURNED 🔥]                        │
│    │                  (permanent ban)                            │
│    │                                                             │
│    ▼                                                             │
│  ✓ Return to Site (Verified tier restored)                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        FORTIFY LAYERS                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  EXTERNAL (Untrusted)                                       │    │
│  │  • Tor Network                                              │    │
│  │  • Mirror Onion Addresses                                   │    │
│  │  • User Connections                                         │    │
│  └──────────────────────┬──────────────────────────────────────┘    │
│                         │                                            │
│                         ▼                                            │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  BOUNDARY LAYER                                             │    │
│  │  ┌──────────────────┐  ┌────────────┐  ┌─────────────┐    │    │
│  │  │   HTTP PROXY     │  │    GATE    │  │  BEHAVIORAL │    │    │
│  │  │ Token Validation │  │  CAPTCHA   │  │  ANALYSIS   │    │    │
│  │  │ Rate Limiting    │  │ Verification│  │  Detection  │    │    │
│  │  └──────────────────┘  └────────────┘  └─────────────┘    │    │
│  └──────────────────────┬──────────────────────────────────────┘    │
│                         │                                            │
│                         ▼                                            │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  INTERNAL (Trusted)                                         │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │    │
│  │  │  CONTROLLER  │  │ ORCHESTRATOR │  │    NODES     │     │    │
│  │  │   Service    │  │    Mirror    │  │ Healthy/     │     │    │
│  │  │  Management  │  │  Management  │  │   Threat     │     │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │    │
│  └──────────────────────┬──────────────────────────────────────┘    │
│                         │                                            │
│                         ▼                                            │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  PROTECTED (Your Service)                                   │    │
│  │  • Real Hidden Service                                      │    │
│  │  • Backend Application                                      │    │
│  │  • Real Onion Address                                       │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Components:**
- **Controller**: Service lifecycle management, health monitoring
- **Orchestrator**: Mirror creation and rotation
- **HTTP Proxy**: Request routing, token validation, rate limiting
- **Gate**: CAPTCHA challenges and verification
- **Nodes**: Backend proxying with behavioral analysis

---

## 🔐 Security Features

### Trust Tier System

| Tier | Access | Rate Limit | How to Achieve |
|------|--------|------------|----------------|
| **Trusted** | ✅ Yes | 300 req/10s | Consistent good behavior |
| **Verified** | ✅ Yes | 100 req/10s | Solve CAPTCHA |
| **Unknown** | ❌ No | 10 req/10s | New visitor |
| **Suspicious** | ❌ No | 2x CAPTCHA | 3+ violations |
| **Burned** | ❌ Never | Banned | 10+ violations |

### Behavioral Analysis

**Detected Violations:**
- Path traversal attempts (`../`, `/.env`)
- Bot User-Agents (curl, wget, python-requests)
- Path enumeration (sequential scanning)
- Resource enumeration (rapid unique paths)
- Form submission floods
- Attack path access (admin panels, config files)

**Severity Levels:**
- **Level 3**: Attack path access, automated behavior
- **Level 2**: Bot UA, enumeration, form floods
- **Level 1**: Suspicious referer, payload anomalies

### CAPTCHA System

**7 JavaScript-Free Types:**
1. **BmpText**: Traditional text image
2. **Emoji**: Select matching emoji
3. **Direction**: Click correct arrow
4. **Sequence**: Complete the pattern
5. **WordUnscramble**: Unscramble letters
6. **ImageRotation**: Select upright image
7. **Silhouette**: Identify silhouette

**Features:**
- No JavaScript required (Tor Browser "Safest" mode compatible)
- Single-use verification tokens (60s lifetime)
- User-Agent binding (prevents token sharing)
- Atomic check-and-mark (prevents race conditions)

---

## 📈 Performance

**Verified Attack Defense (January 20, 2026):**
- **Duration**: 3 hours (17:54 - 20:49)
- **Attack Requests**: 65,576 total
- **Attack Traffic Blocked**: 58,461 (89.1%)
- **Legitimate Users Served**: 280
- **CAPTCHA Completions**: 54
- **Zero Downtime**: ✅

**Rate Limiting Efficiency:**
- Per-circuit isolation prevents quota exhaustion
- Independent quotas for each Tor circuit
- CAPTCHA paths always accessible (no rate limit)
- Attack traffic stopped at 10 req/10s per circuit

---

## 📚 Documentation

**Core Documentation:**
- [Alpha Review](fortify/docs/Dev_Progress/Alpha_Review.md) - Project status and roadmap
- [Architecture Overview](fortify/docs/Fortify%20Documentation/01-Architecture/overview.md) - System design
- [Trust Tiers](fortify/docs/Fortify%20Documentation/02-Core-Concepts/trust-tiers.md) - Trust system details
- [Behavioral Analysis](fortify/docs/Fortify%20Documentation/02-Core-Concepts/behavioral-analysis.md) - Violation detection
- [API Reference](fortify/docs/Fortify%20Documentation/08-API-Reference/api-reference.md) - Gate and Admin APIs

**Specialized Guides:**
- [Rate Limiting](fortify/docs/RATE_LIMITING.md) - Circuit-based rate limiting system
- [Authentication](fortify/docs/AUTHENTICATION.md) - Admin authentication
- [Security Audit](fortify/docs/SECURITY_AUDIT.md) - Vulnerability assessment

**Research:**
- [Tor Hidden Service Attacks](fortify/docs/research/or%20Hidden%20Service%20Attacks%20&%20Defensive%20Methods.md) - Attack vectors and defenses

---

## 🛠️ Development

### Project Structure

```
fortify/
├── crates/
│   ├── fortify-core/       # Shared types, trust system, behavioral analysis
│   ├── fortify-controller/ # Service lifecycle management
│   ├── fortify-orchestrator/ # Mirror management
│   ├── fortify-http/       # HTTP proxy, rate limiting, routing
│   ├── fortify-gate/       # CAPTCHA verification
│   ├── fortify-node/       # Backend proxy nodes
│   ├── fortify-community/  # Community network (Phase 7)
│   └── fortify-tui/        # Deployment TUI (40% complete)
├── config/                 # Example configurations
├── docs/                   # Documentation
├── install/                # Installation scripts
├── scripts/                # Utility scripts
└── tests/                  # Integration tests
```

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run specific crate
cargo run -p fortify-controller

# Build documentation
cargo doc --no-deps --open
```

---

## 🎯 Roadmap

### Phase 4: Resilience & Recovery (0%)
- Mirror discovery bar and retirement system
- Auto-scaling based on resource usage
- Session behavioral analysis enhancements
- Cleanup automation

### Phase 5: Cluster System (0%)
- Multi-VPS coordination
- Distributed mirror management
- Cross-cluster session sharing
- Cluster-wide health monitoring

### Phase 6: Deployment TUI (40%)
- ✅ Core framework and configuration system
- ✅ Deployment wizard (7 steps)
- ✅ Live log streaming
- ⏳ Controller integration
- ⏳ Testing and deployment

### Phase 7: Community Network (0%)
- P2P mirror discovery
- Community node registration
- Trust verification system
- Reputation tracking

### Phase 8: Advanced Capabilities (0%)
- Machine learning threat detection
- Predictive scaling algorithms
- Advanced behavioral analysis
- Traffic pattern recognition

[View detailed roadmap →](fortify/docs/ROADMAP.md)

---

## 🤝 Contributing

Fortify is currently in Alpha development. Contributions are welcome!

**Areas needing help:**
- Testing and bug reports
- Documentation improvements
- CAPTCHA type implementations
- Performance optimizations

**Before contributing:**
1. Read the [Security Audit](fortify/docs/SECURITY_AUDIT.md)
2. Review the [Alpha Review](fortify/docs/Dev_Progress/Alpha_Review.md)
3. Check existing issues and PRs

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## ⚠️ Disclaimer

Fortify is provided as-is for educational and defensive purposes. The authors are not responsible for misuse. Always comply with applicable laws when deploying hidden services.

---

## 🙏 Acknowledgments

- Built with Rust 🦀
- Tor Project for the anonymity network
- The security research community

---

**Built by Nespartious** | [GitHub](https://github.com/Nespartious/Fortify) | Alpha Release

*Protecting hidden services, one CAPTCHA at a time.* 🏰
