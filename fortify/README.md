# Fortify

A security-first Tor hidden-service protection system.

## Overview

Fortify is a defensive protection layer for Tor hidden services. It acts as a disposable public entry point that filters hostile traffic, verifies legitimate users, and protects the real service's onion address.

## Key Features

- **No JavaScript**: Pure server-side security
- **Trust Tiers**: Progressive verification system
- **Disposable Orchestrators**: Burn and replace compromised entry points
- **Resource-Aware Scaling**: Adaptive protection within hardware limits
- **Optional Community Discovery**: Decentralized network participation

## Security Model

- Defensive system only
- Availability is secondary to secrecy
- Everything degrades safely
- No offensive tooling or exploit code

## Requirements

- Linux (Ubuntu/Debian)
- Rust toolchain
- Tor daemon
- Root/sudo access for installation

## Installation

```bash
cd install/
sudo ./install.sh
```

## Documentation

See the `docs/` directory for:

- [Architecture](docs/architecture.md)
- [Threat Model](docs/threat-model.md)
- [Trust Levels](docs/trust-levels.md)
- [Community Network](docs/community-network.md)
- [Scaling Model](docs/scaling-model.md)
- [OS Hardening](docs/hardening.md)

## Development & Deployment

### Official Deployment Method: TUI Wizard

**The TUI deployment wizard is the ONLY supported deployment method:**

```bash
# Build the project
cargo build --release --workspace

# Run the TUI deployment wizard
./target/release/fortify
```

The TUI provides:
- Interactive configuration wizard
- Real-time deployment logs
- Mirror status monitoring
- One-click mirror address export
- Health checks and diagnostics

### Quick Start

1. **Build:**
   ```bash
   cargo build --release --workspace
   ```

2. **Deploy:**
   ```bash
   ./target/release/fortify
   ```

3. **Follow the wizard:**
   - Configure network settings
   - Set vanity prefix (optional)
   - Start deployment
   - Monitor logs
   - Export mirror addresses (press 'E')

### Legacy Scripts (Deprecated)

`./scripts/dev-run.sh` - **Deprecated**, use TUI instead. This script is kept for backward compatibility but will be removed in future versions.

## Project Status

### Phase 0: Project Initialization — COMPLETE
- Repository scaffold created
- Rust workspace initialized
- Documentation structure established
- All crates compile

### Phase 1: OS Hardening & Installation — COMPLETE
- Environment detection with requirements validation
- OS hardening scripts (sysctl, limits, firewall)
- Tor daemon setup and validation
- Automated installation with error handling
- Systemd service units
- Configuration backup system

### Phase 2: Core Logic & Trust Model — COMPLETE
- Trust tier system (5 tiers: Burned to Trusted)
- HMAC-SHA256 token signing and verification
- Session state machine with promotion/demotion
- Violation tracking and burn logic
- Thread-safe session management
- Comprehensive unit tests

### Phase 3: Gate System — COMPLETE
- Server-side captcha generation and verification
- Proof-of-work challenge system (SHA-256)
- Multi-step verification state machine
- Rate limiting and queue management
- HTTP server with form handling (no JavaScript)
- Token issuance after verification
- Integration with core trust logic
- Comprehensive unit tests

### Phase 4: HTTP & Proxy Layer — COMPLETE
- Token validation middleware
- Reverse proxy with backend forwarding
- Three routing strategies (RoundRobin, LeastConnections, WeightedRandom)
- Tier-based routing (healthy/threat nodes)
- Backpressure and request caps
- Metrics tracking and monitoring
- Comprehensive unit tests

### Phase 5: Orchestrators & Mirror Rotation — COMPLETE
- Mirror state machine and lifecycle management
- Tor hidden service integration
- Compromise detection with multi-signal analysis
- Automatic burn and replace mechanism
- Three rotation strategies (age, request, risk-based)
- HTTP server with health/status endpoints
- Background monitoring and rotation tasks
- Comprehensive unit tests

### Phase 6: Node System — COMPLETE
- Dual operation modes (Healthy/Threat) with adaptive rate limiting
- Request forwarding to real backend service
- Silent session reclassification (demotion/promotion)
- Violation detection (rate limits, suspicious patterns, invalid paths)
- Behavioral anomaly detection and pattern analysis
- HTTP server accepting from proxy
- Metrics tracking and health endpoints
- Comprehensive unit tests

### Phase 7: Controller & Scaling Logic — COMPLETE
- Resource monitoring (CPU, memory usage tracking)
- Service lifecycle management (spawn, monitor, restart, shutdown)
- Process management for all components
- Automatic health checking and restart
- Resource-aware scaling policy
- Orchestrator and node auto-scaling
- Configuration management with validation
- Graceful shutdown coordination
- Comprehensive unit tests

### Phase 8: Community / Discovery Network — COMPLETE
- Ed25519 cryptographic signatures for seed authenticity
- Seed registry with capacity limits and TTL
- Automatic expiration and cleanup
- Peer discovery with rate limiting
- Multi-hop daisy-chain discovery
- HTTP endpoints for seed sharing
- Opt-in participation (disabled by default)
- Discovery never bypasses Gate verification
- Trust-minimized sharing protocol
- Comprehensive unit tests

### Phase 9: Integration & Testing — COMPLETE
- Integration test suite (15+ tests)
- End-to-end flow validation (10+ tests)
- Security invariant tests (10+ tests)
- Trust tier progression testing
- Token tampering detection
- Path traversal prevention
- Injection attack prevention
- Concurrent access testing
- Comprehensive testing documentation
- Security test checklist

### TUI Deployment Wizard — IN PROGRESS
- Terminal-based deployment wizard with 7-step configuration
- System dependency checking (Tor, mkp224o, Python, vanguards)
- Branding, CAPTCHA, thresholds, network, mirrors configuration
- Vanity .onion address generation for mirrors
- Live mirror address export functionality
- Settings management with hot-reload support
- Log panel with filtering and pause/resume
- Quick Deploy from saved configurations
- Destroy instance with double confirmation

## Project Status

**Core phases complete!** ✅ | **TUI Phase in progress** 🚧

Fortify is a fully implemented security system with:
- 10 Rust crates (including TUI)
- 8,000+ lines of code
- 120+ tests
- Complete documentation
- OS hardening scripts
- Multi-layer defense architecture
- Terminal-based deployment wizard

## Testing

See [docs/TESTING.md](docs/TESTING.md) for comprehensive testing guide.

```bash
# Run all tests
cargo test --all

# Run integration tests
cargo test --test '*'

# Run specific test suite
cargo test --test security_test
```

## Next Steps

1. **Run the TUI**: Launch with `cargo run --bin fortify` for the deployment wizard
2. **Check Dependencies**: First wizard step verifies Tor, mkp224o, Python, vanguards
3. **Configure Deployment**: Follow 7-step wizard to configure your protected service
4. **Deploy**: Start your Fortify-protected onion service
5. **Monitor**: View real-time logs and mirror status in the TUI
6. **Export Addresses**: Press `[E]` to export mirror addresses for sharing

### Manual Deployment (Alternative)

1. **Deploy to Linux**: Transfer repository to Ubuntu/Debian system
2. **Run Installation**: Execute `sudo ./install/install.sh`
3. **Configure Services**: Set environment variables and addresses
4. **Start Controller**: Launch `fortify-controller` to manage all services
5. **Monitor Logs**: Verify all components start successfully
6. **Run Tests**: Execute integration tests on deployed system

## License

See [LICENSE](LICENSE) file.

## Security

See [SECURITY.md](SECURITY.md) for security policy and reporting procedures.
