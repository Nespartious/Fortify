# Phase 6: Deployment TUI - Terminal User Interface

**Version:** 1.0  
**Started:** January 16, 2026  
**Status:** 🔄 In Progress  
**Crate:** `fortify-tui`

---

## Overview

Phase 6 implements a full-screen Terminal User Interface (TUI) for Fortify deployment and management. Built with Ratatui (the modern Rust TUI framework), this interface provides:

- **Split-screen layout**: Left panel for controls, right panel for live logs
- **Deployment wizard**: Step-by-step configuration before launch
- **Live log streaming**: Real-time filtered log viewing
- **Hot configuration**: Apply changes while running or store for restart
- **Branding customization**: Service name, logo, colors
- **Comprehensive settings**: CAPTCHA, thresholds, network, mirrors

---

## Architecture

```
┌─────────────────────────────────┬──────────────────────────────────┐
│  🏰 FORTIFY CONTROL CENTER      │  ▶ LIVE LOGS                     │
│                                 │                                  │
│  [D]eploy                       │  16:45:32 INFO Mirror spawned    │
│  [J]oin Community Network       │  16:45:33 INFO Tor circuit built │
│  [S]ettings                     │  16:45:34 WARN CPU 85%           │
│  [T]atus                        │  16:45:35 INFO CAPTCHA served    │
│  [X] Destroy Instance           │  16:45:36 DEBUG Session created  │
│  [Q]uit                         │  16:45:37 INFO Request proxied   │
│                                 │                                  │
│  ─────────────────────          │                                  │
│  Active Mirrors: 2/4            │                                  │
│  Standby: 2 │ Burned: 0         │                                  │
│  CAPTCHA Pool: 487/500          │                                  │
│  Sessions: 12 active            │                                  │
│                                 │                                  │
│  [Tab] Switch Panel             │  [F] Filter  [P] Pause  [C] Clear│
└─────────────────────────────────┴──────────────────────────────────┘
```

---

## Progress Checklist

### 6.1 Core Framework
- [x] Create `fortify-tui` crate with Cargo.toml
- [x] Set up Ratatui + Crossterm dependencies
- [x] Implement main App struct with state management
- [x] Create split-screen layout (50/50 horizontal)
- [x] Implement keyboard event handling
- [x] Add focus management (Menu/Settings/Logs/Dialog)

### 6.2 Configuration System
- [x] `FortifyConfig` root configuration struct
- [x] `BrandingConfig` - service name, description, logo, colors
- [x] `CaptchaConfig` - pool size, difficulty, timeout, attempts
- [x] `ThresholdConfig` - rate limits, bans, burn thresholds
- [x] `NetworkConfig` - addresses, ports, vanguards
- [x] `MirrorConfig` - counts, rotation, proactive burn
- [x] TOML serialization/deserialization
- [x] `ChangeManager` for tracking modifications
- [x] Hot-reload detection (dirty flag)

### 6.3 Views & Screens
- [x] Home screen with ASCII logo and menu
- [x] Deployment wizard (7 steps: Deps, Branding, CAPTCHA, Thresholds, Network, Mirrors, Review)
- [x] Settings panel with tabbed interface
- [x] Running deployment view
- [x] Resume deployment selection
- [x] Join network placeholder (Phase 5 integration)
- [x] System status view
- [x] Dependency check and installation step

### 6.4 Settings Tabs
- [x] Branding tab (name, description, welcome, color, logo)
- [x] CAPTCHA tab (enabled, pool, difficulty, timeout, attempts)
- [x] Thresholds tab (rate limit, bans, burn, DDoS)
- [x] Network tab (backend, binds, ports, vanguards)
- [x] Mirrors tab (min, max, standby, rotation, burn interval)
- [x] Vanity tab (enabled, prefix, safety net, timeout)

### 6.5 Dialogs
- [x] Confirm dialog (Yes/No)
- [x] Apply changes dialog (Apply Now/Store for Later)
- [x] Text input dialog
- [x] Error dialog
- [x] Info dialog

### 6.6 Log Panel
- [x] `LogEntry` struct with level, source, message, timestamp
- [x] `LogBuffer` circular buffer (5000 entries)
- [x] Level filtering (Trace/Debug/Info/Warn/Error)
- [x] Pause/resume functionality
- [x] Clear buffer
- [x] Scroll support (PageUp/PageDown)
- [x] Log line parsing from child processes

### 6.7 Vanity Address Generation
- [x] `VanityConfig` struct in TUI config
- [x] `VanityConfig` struct in orchestrator tor.rs
- [x] Prefix-only matching (up to 10 characters)
- [x] Warn threshold for long prefixes (>7 chars)
- [x] Safety net timeout system (auto-shorten prefix)
- [x] Vanity tab in settings UI
- [x] Vanity step in deployment wizard
- [x] mkp224o integration for generation (orchestrator)
- [x] Vanity forwarded from TUI → Controller → Orchestrator
- [x] Nodes (healthy/threat) use random addresses only
- [x] Mirrors use vanity when enabled
- [ ] Progressive prefix reduction on timeout

### 6.8 Mirror Status Display
- [x] `MirrorStatus` struct with address and state
- [x] `MirrorStatusState` enum (Pending/Verifying/Live/Failed/Generating)
- [x] Colored status indicators (Yellow/Orange/Green/Red/Magenta)
- [x] Status symbols (● live, ◐ pending, ○ failed)
- [x] Running view shows all mirror addresses
- [x] Active/Standby counts and summary
- [ ] Self-verification of .onion addresses
- [ ] Auto-update status from orchestrator

### 6.9 Deployment Manager
- [x] `DeploymentState` enum (Stopped/Starting/Running/Stopping/Error)
- [x] Start deployment with config
- [x] Stop deployment (graceful shutdown)
- [x] Child process management
- [x] Stdout/stderr capture to log buffer
- [x] Configuration reload support

### 6.10 Integration
- [x] Add to workspace Cargo.toml
- [x] Build verification
- [ ] Integration with fortify-controller
- [ ] Test deployment workflow

---

## File Structure

```
crates/fortify-tui/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public exports
│   ├── main.rs             # Entry point
│   ├── app.rs              # Main application state & loop
│   ├── config.rs           # Configuration types
│   ├── deployment.rs       # Deployment lifecycle
│   ├── events.rs           # Event types
│   ├── logging.rs          # Log entry & buffer
│   ├── settings.rs         # Settings utilities
│   ├── widgets.rs          # Custom widgets
│   └── ui/
│       ├── mod.rs          # UI module & main draw
│       ├── home.rs         # Home screen
│       ├── logs.rs         # Log panel
│       ├── settings.rs     # Settings panels
│       ├── wizard.rs       # Deployment wizard
│       ├── running.rs      # Running view
│       └── dialogs.rs      # Dialog overlays
```

---

## Deployment Wizard Steps

The deployment wizard now has 7 steps:

| Step | Name | Description |
|------|------|-------------|
| 0 | **Deps** | Check system dependencies (tor, mkp224o, python3, vanguards) |
| 1 | **Branding** | Service name, description, colors, logo |
| 2 | **CAPTCHA** | Pool size, difficulty, timeout, max attempts |
| 3 | **Thresholds** | Rate limits, ban thresholds, burn triggers |
| 4 | **Network** | Backend address, ports, vanguards config |
| 5 | **Mirrors** | Min/max mirrors, standby, rotation, vanity |
| 6 | **Review** | Final review and deploy confirmation |

### Dependency Check (Step 0)

The first wizard step checks system dependencies before deployment:

```
┌─ System Dependencies ─────────────────────────────────────┐
│                                                           │
│   Required: 2/2 installed                                 │
│   Optional: 2/3 installed                                 │
│                                                           │
│   ✓ tor (required) - Tor anonymity network                │
│   ✓ python3 (required) - Python interpreter for Vanguards │
│   ✓ mkp224o (optional) - Vanity .onion address generator  │
│   ○ vanguards (optional) - Tor guard node protection      │
│       Install: pip3 install vanguards                     │
│   ✓ libsodium-dev (optional) - Cryptographic library      │
│                                                           │
│   ✓ All required dependencies met. Press [→] to continue │
│   Optional: Press [I] to install missing dependencies    │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

#### Dependencies Tracked

| Dependency | Required | Check Command | Description |
|------------|----------|---------------|-------------|
| tor | ✓ | `which tor` | Tor anonymity network daemon |
| python3 | ✓ | `which python3` | Python 3 for vanguards |
| mkp224o | ○ | `which mkp224o` | Vanity .onion generator |
| vanguards | ○ | `pip3 show vanguards` | Guard node protection |
| libsodium-dev | ○ | `pkg-config --exists libsodium` | Crypto library |

#### Installation Process

When [I] is pressed:
1. Dialog confirms installation (may require sudo)
2. Each missing dependency is installed:
   - apt-get packages: `sudo apt-get install -y <package>`
   - pip packages: `pip3 install <package>`
   - mkp224o: Built from source (git clone, autogen, configure, make, install)
3. Results dialog shows installed/failed counts
4. Dependency list refreshes

---

## Configuration Schema

### FortifyConfig (deployment.toml)

```toml
deployment_id = "deploy-a1b2c3d4"

[branding]
service_name = "My Protected Service"
description = "A Fortify-protected onion service"
welcome_message = "Please complete verification to continue."
primary_color = "#6B46C1"
logo_path = "/path/to/logo.png"   # Optional, max 256x256

[captcha]
enabled = true
pool_size = 500
min_pool_size = 100
max_pool_size = 1000
difficulty = 5                     # 1-10 scale
timeout_seconds = 120
max_attempts = 3
audio_enabled = false
rotation_percent = 25
rotation_interval_days = 10

[thresholds]
rate_limit_rpm = 60
captcha_fail_limit = 5
temp_ban_minutes = 30
perm_ban_threshold = 10
suspicion_threshold = 0.5
threat_threshold = 0.7
burn_threshold = 0.7
auto_ban_enabled = true
ddos_rps_threshold = 100
probe_sensitivity = 5              # 1-10 scale

[network]
backend_address = "127.0.0.1:8080"
socks_port = 9150
control_port = 9151
http_bind = "127.0.0.1:8082"
gate_bind = "127.0.0.1:8081"
vanguards_enabled = true
vanguards_layer2 = 4
vanguards_layer3 = 8
data_dir = "/tmp/fortify"

[mirrors]
min_mirrors = 2
max_mirrors = 5
standby_mirrors = 2
rotation_interval_seconds = 3600
proactive_burn_enabled = true
burn_interval_days_min = 60
burn_interval_days_max = 120
retirement_page_hours = 72

[vanity]
enabled = false
prefix = ""                        # Up to 10 chars, lowercase alphanumeric
safety_net_enabled = true
safety_net_timeout_seconds = 30    # 30s for testing, 900 (15 min) for production
min_prefix_length = 1              # Stop shortening at this length
warn_threshold = 7                 # Warn if prefix > this
```

---

## Mirror Status Display

When deployed, the left panel shows all mirror addresses with status indicators:

```
┌─ Active Mirrors ──────────────────────┐
│ ● fortify1...xyz.onion   [LIVE]       │  ← Green dot
│ ◐ fortify2...uvw.onion   [VERIFYING]  │  ← Orange dot
│ ◐ fortify3...rst.onion   [PENDING]    │  ← Yellow dot
│ ○ fortify4...opq.onion   [FAILED]     │  ← Red dot
│ ◐ (generating...)        [GENERATING] │  ← Magenta dot
│                                       │
│ Active: 1/4  │  Standby: 2           │
│ Vanity Prefix: fortify               │
└───────────────────────────────────────┘
```

### Status States

| State | Color | Symbol | Description |
|-------|-------|--------|-------------|
| `PENDING` | Yellow | ◐ | Tor daemon starting, not yet announced |
| `VERIFYING` | Orange | ◐ | Announced, self-verification in progress |
| `LIVE` | Green | ● | Verified accessible from network |
| `FAILED` | Red | ○ | Failed to create or publish |
| `GENERATING` | Magenta | ◐ | Vanity address being generated |

---

## Vanity Address Generation

### Architecture

**IMPORTANT**: Vanity addresses are for **MIRRORS ONLY**, not for nodes.

```
┌──────────────┐    VANITY_*    ┌──────────────┐    VANITY_*    ┌──────────────┐
│  Fortify TUI │ ──────────────▶│  Controller  │ ──────────────▶│ Orchestrator │
└──────────────┘                └──────────────┘                └──────────────┘
                                       │                               │
                                       │                               ▼
                                       │                        ┌─────────────┐
                                       │                        │ TorService  │
                                       │                        │ (mkp224o)   │
                                       │                        └─────────────┘
                                       │                               │
                                       │                               ▼
                                       │                        ┌─────────────┐
                                       │                        │   Mirrors   │
                                       │                        │ sigil*.onion│
                                       │                        └─────────────┘
                                       │
                                       ▼
                                ┌─────────────┐
                                │ TorManager  │
                                │ (random)    │
                                └─────────────┘
                                       │
                                       ▼
                                ┌─────────────┐
                                │   Nodes     │
                                │ random.onion│
                                └─────────────┘
```

### Component Responsibilities

| Component | Handles | Vanity? | Address Type |
|-----------|---------|---------|--------------|
| TUI | User config | Config only | N/A |
| Controller | Node allocation | NO | Random .onion |
| Orchestrator | Mirror creation | YES | Vanity .onion |

### Configuration Flow

1. **TUI** reads `config.vanity.*` from user
2. **TUI** passes `VANITY_ENABLED`, `VANITY_PREFIX`, `VANITY_TIMEOUT` to Controller
3. **Controller** stores in `ControllerConfig` and forwards to `OrchestratorEnvFactory`
4. **OrchestratorEnvFactory** passes VANITY_* env vars when spawning orchestrators
5. **Orchestrator** reads env vars, configures `TorService` with `VanityConfig`
6. **TorService** uses mkp224o to generate vanity addresses for mirrors

### Why Nodes Don't Use Vanity

- **Nodes are internal infrastructure** - not user-facing
- **Nodes may be frequently rotated** - vanity generation is expensive
- **Attackers shouldn't identify nodes** - random addresses provide better anonymity
- **Mirrors are public entry points** - vanity helps users verify authenticity

### Safety Net System

The safety net prevents infinite generation attempts:

1. User sets prefix "fortify" (7 chars)
2. System attempts to find match within 30 seconds (testing) / 15 minutes (production)
3. If timeout: prefix shortened to "fortif" (6 chars)
4. Process repeats until match found or min_prefix_length reached
5. If min reached: random address generated

### Estimated Generation Times

| Prefix Length | Approx. Time (single core) |
|---------------|---------------------------|
| 1-4 | Instant to seconds |
| 5 | ~30 seconds |
| 6 | ~15 minutes |
| 7 | ~8 hours |
| 8 | ~10 days |
| 9 | ~1 year |
| 10 | ~32 years |

Multi-core reduces times proportionally.

---

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Ctrl+C` | Stop deployment / Quit |
| `Ctrl+Q` | Quit (with confirmation if running) |
| `Tab` | Switch focus between panels |

### Home Menu
| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate menu |
| `Enter` | Select item |
| `D` | Deploy (with Quick Deploy if existing config) |
| `J` | Join Network |
| `S` | Settings |
| `T` | Status |
| `X` | Destroy Instance (double confirmation) |
| `Q` | Quit |

**Note**: The menu now uses "Deploy" instead of separate "Deploy New" and "Resume". 
When an existing configuration is found, a "Quick Deploy" option appears in a dialog.

### Settings
| Key | Action |
|-----|--------|
| `←/→` or `h/l` | Switch tab |
| `↑/↓` or `j/k` | Navigate fields |
| `Enter` | Edit field |
| `Esc` | Back (prompts if dirty) |

### Running View
| Key | Action |
|-----|--------|
| `E` | Export mirror addresses to file |
| `P` | Pause/Resume log streaming |
| `S` | Open Settings |
| `Esc` | Stop deployment (with confirmation) |

### Log Panel
| Key | Action |
|-----|--------|
| `P` | Pause/Resume |
| `C` | Clear buffer |
| `F` | Cycle filter level |
| `PageUp/Down` | Scroll |

### Dialogs
| Key | Action |
|-----|--------|
| `Y` | Confirm |
| `N` | Cancel |
| `A` | Apply Now (changes dialog) |
| `L` | Store for Later |
| `Esc` | Cancel |

---

## Hot Configuration Behavior

When configuration is modified while deployment is running:

1. **Change Detected**: Field value updated, `dirty` flag set
2. **On Exit/Tab Change**: Dialog prompts user:
   - **[A] Apply Now**: Save config, signal reload to running services
   - **[L] Store for Later**: Save config, will apply on next restart
   - **[Esc] Cancel**: Discard changes
3. **If Not Running**: Changes apply immediately on save

### Reload Support
Some settings can be hot-reloaded:
- ✅ Thresholds (rate limits, bans)
- ✅ CAPTCHA pool size
- ⚠️ Network binds (requires restart)
- ⚠️ Mirror counts (gradual adjustment)
- ❌ Branding (requires restart)

---

## Logo Constraints

| Property | Requirement |
|----------|-------------|
| Format | PNG, JPG, or GIF |
| Max Width | 256 pixels |
| Max Height | 256 pixels |
| Max File Size | 100 KB |
| Recommended | 128x128 or 64x64 |

Logo is stored as base64 in the gate HTML templates.

---

## Export Feature

Press `[E]` in the Running View to export all mirror addresses to a file.

### Export Format

```
# Fortify Mirror Addresses
# Exported: 2025-01-XX XX:XX:XX
# Service: My Protected Service
# Backend: 127.0.0.1:8080

## CONTROL PANEL:
http://xyzab1234567890abcd.onion/ctrl_8f7k3m9x2n4p1q6w5v0b8c

## LIVE MIRRORS (3):
http://xyzab1234567890abcd.onion  # Live
http://mnopq5678901234efgh.onion  # Live
http://rstuv9012345678ijkl.onion  # Live

## STANDBY MIRRORS (2):
http://abcde3456789012wxyz.onion  # Live [STANDBY]
http://fghij7890123456opqr.onion  # Pending [STANDBY]

# Plain addresses (for easy copying):

# Live:
http://xyzab1234567890abcd.onion
http://mnopq5678901234efgh.onion
http://rstuv9012345678ijkl.onion

# Standby:
http://abcde3456789012wxyz.onion
http://fghij7890123456opqr.onion
```

### Export Details
- **Location**: `/tmp/fortify/mirror-addresses.txt`
- **Auto-Open**: Attempts to open with `xdg-open` (Linux default text editor)
- **Live Mirrors**: Currently active and receiving traffic
- **Standby Mirrors**: Ready to activate on demotion/burn
- **Control Panel**: Admin interface URL with secret path
- **Plain Addresses Section**: Easy copy/paste for sharing
- **Vanity Prefixes**: If vanity was enabled, addresses will start with configured prefix

---

## Dependencies

```toml
[dependencies]
# TUI framework
ratatui = "0.29"
crossterm = "0.28"

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
dirs = "5.0"
image = "0.25"
base64 = "0.22"
unicode-width = "0.2"

# Internal crates
fortify-core = { path = "../fortify-core" }
fortify-orchestrator = { path = "../fortify-orchestrator" }
fortify-controller = { path = "../fortify-controller" }
```

---

## Usage

### Running the TUI

```bash
# From workspace root
cargo run --bin fortify

# Or with release build
cargo build --release
./target/release/fortify
```

### First Launch

1. TUI starts with Home screen
2. Press `D` or navigate to "Deploy New Instance"
3. Step through wizard:
   - **Step 1**: Branding configuration
   - **Step 2**: CAPTCHA settings
   - **Step 3**: Security thresholds
   - **Step 4**: Network configuration
   - **Step 5**: Review and deploy
4. Press `Enter` on final step to deploy
5. View transitions to Running with live logs

### Resume Existing

1. Press `R` on Home screen
2. Select from list of saved deployments
3. Configuration is loaded
4. Deployment starts automatically

---

## Testing

### Manual Testing Checklist

- [ ] Launch TUI, verify split-screen renders correctly
- [ ] Navigate menu with arrow keys and hotkeys
- [ ] Open Settings, switch between all tabs
- [ ] Edit a field, verify input dialog works
- [ ] Make changes, try to exit, verify Apply dialog appears
- [ ] Complete deployment wizard, verify all steps
- [ ] Start deployment, verify logs appear in right panel
- [ ] Test log filtering (F key)
- [ ] Test log pause (P key)
- [ ] Stop deployment (Esc → confirm)
- [ ] Resume existing deployment
- [ ] Test Ctrl+C handling

---

## Future Enhancements

### Phase 6.1 (Planned)
- [ ] Real-time mirror status display
- [ ] Session count and statistics
- [ ] CAPTCHA pool level indicator
- [ ] Network traffic graphs (ASCII)

### Phase 6.2 (Planned)
- [ ] Mouse support for navigation
- [ ] Copy log entries to clipboard
- [ ] Search within logs
- [ ] Export configuration

---

## Related Documentation

- [Fortify Documentation/06-Interfaces/deployment-tui.md](../Fortify%20Documentation/06-Interfaces/deployment-tui.md) - User guide
- [Phase 4: Resilience & Recovery](04-PHASE-4.md) - Underlying infrastructure
- [Phase 5: Community Network](05-PHASE-5.md) - "Join Network" integration

---

## Changelog

### 2026-01-16
- Initial Phase 6 implementation
- Created `fortify-tui` crate structure
- Implemented core TUI framework with Ratatui
- Added all configuration types (Branding, CAPTCHA, Thresholds, Network, Mirrors)
- Built deployment wizard with 5 steps
- Implemented settings panels with tabbed interface
- Added log buffer with filtering and scroll
- Created dialog system for confirmations and input
- Implemented deployment manager with process lifecycle
