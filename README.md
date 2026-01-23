# 🏰 Fortify

> **Advanced DDoS Protection for Tor Hidden Services**

<!-- Primary Workflow Badges -->
[![CI](https://github.com/Nespartious/Fortify/actions/workflows/ci.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/ci.yml)
[![Security Audit](https://github.com/Nespartious/Fortify/actions/workflows/security.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/security.yml)
[![Tor Alignment](https://github.com/Nespartious/Fortify/actions/workflows/tor-alignment.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/tor-alignment.yml)
[![Code Quality](https://github.com/Nespartious/Fortify/actions/workflows/code-quality.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/code-quality.yml)
[![Coverage](https://github.com/Nespartious/Fortify/actions/workflows/coverage.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/coverage.yml)
[![SBOM](https://github.com/Nespartious/Fortify/actions/workflows/sbom.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/sbom.yml)
[![Dependency Review](https://github.com/Nespartious/Fortify/actions/workflows/dependency-review.yml/badge.svg)](https://github.com/Nespartious/Fortify/actions/workflows/dependency-review.yml)

<!-- Static Badges -->
[![Status](https://img.shields.io/badge/Status-Alpha-orange)](https://github.com/Nespartious/Fortify)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

---

### 🔒 Security Verification

<details>
<summary><b>What does "Security Audit Passed" mean?</b></summary>

Our Security Audit workflow runs **6 independent security checks**:

| Check | Tool | What It Validates |
|-------|------|-------------------|
| **Vulnerability Scan** | `cargo-audit` | Checks all dependencies against [RustSec Advisory Database](https://rustsec.org/) for known CVEs |
| **License Compliance** | `cargo-deny` | Ensures all dependencies use approved licenses (MIT, Apache-2.0, BSD) |
| **SAST Analysis** | `Semgrep` | Static analysis for security anti-patterns, injection risks, unsafe patterns |
| **Unsafe Code Audit** | `cargo-geiger` | Reports all `unsafe` blocks in codebase and dependencies |
| **Secrets Detection** | `Gitleaks` | Scans for accidentally committed API keys, tokens, credentials |
| **Supply Chain** | `cargo-vet` | Verifies dependency integrity and trusted sources |

Runs **daily** and on every push to main. [View workflow →](.github/workflows/security.yml)

</details>

<details>
<summary><b>What does "CI Passed" mean?</b></summary>

| Check | Description |
|-------|-------------|
| **Build** | Compiles on stable Rust with all features |
| **Tests** | 131+ unit and integration tests pass |
| **Clippy** | Zero warnings from Rust linter |
| **Format** | Code follows `rustfmt` standards |

</details>

<details>
<summary><b>What does "Tor Alignment Passed" mean?</b></summary>

Ensures Fortify is fully compatible with Tor Browser's privacy requirements:

| Check | What It Validates |
|-------|-------------------|
| **No JavaScript** | HTML contains no `<script>` tags (Tor Browser disables JS by default) |
| **No External URLs** | No CDN, Google Fonts, or external resource loading (prevents deanonymization) |
| **No Tracking** | No tracking pixels, analytics, or inline event handlers |
| **Privacy Patterns** | Flags IP extraction and User-Agent logging for review |
| **Tor Headers** | Reports on Onion-Location and security header implementation |

Runs on every push/PR. [View workflow →](.github/workflows/tor-alignment.yml)

</details>

**Fortify** is a sophisticated multi-layered defense system that protects Tor hidden services from DDoS attacks while maintaining access for legitimate users. Built in Rust for performance and security, Fortify acts as a proxy shield between attackers and your real hidden service.

---

## 🎯 Project Status: **Alpha**

**Production-Ready Core Protection** ✅  
Battle-tested DDoS defense for Tor hidden services. Blocks attack traffic while maintaining access for legitimate users.

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
- **CPU:** 4 cores recommended (2 cores minimum)
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
| **Trusted** | ✅ Yes | 120 req/min | Consistent good behavior |
| **Verified** | ✅ Yes | 60 req/min | Solve CAPTCHA |
| **Stranger** | ❌ No | 30 req/min | New visitor (requires CAPTCHA) |
| **Suspicious** | ⚠️ Limited | 10 req/min | Failed verification or rate limited |
| **Hostile** | ❌ Never | Banned | Multiple violations |

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

**Defense Capabilities:**
- Per-circuit isolation prevents quota exhaustion
- Independent quotas for each Tor circuit
- CAPTCHA paths always accessible (no rate limit)
- Attack traffic blocked at circuit level
- Zero impact on legitimate users during attacks

**Traffic Tiers:**

Fortify auto-scales based on your expected traffic:

| Tier | Daily Users | CPU | RAM |
|------|-------------|-----|-----|
| Micro | ~100 | 1-2 cores | 512MB |
| Small | ~1,000 | 2-4 cores | 1-2GB |
| Medium | ~10,000 | 4 cores | 4GB |
| Large | ~100,000 | 8+ cores | 8-16GB |
| Enterprise | ~1M+ | 16+ cores | 32GB+ |

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
