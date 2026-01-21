# 🛡️ Tor Defensive Capabilities

> **Comprehensive Guide to Fortify's Tor Integration and Defensive Mechanisms**

---

## Overview

Fortify provides multiple layers of Tor-specific defenses:

1. **Mirror Rotation** - Dynamic onion address cycling
2. **Proof-of-Work** - Client puzzle challenges
3. **Vanguards** - Guard discovery protection
4. **Multi-Daemon Architecture** - DoS resilience (Roadmap)

---

## Mirror System

### Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           MIRROR ARCHITECTURE                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│                        ┌─────────────────────┐                               │
│                        │    ORCHESTRATOR     │                               │
│                        │  (Mirror Manager)   │                               │
│                        └──────────┬──────────┘                               │
│                                   │                                          │
│         ┌─────────────────────────┼─────────────────────────┐               │
│         │                         │                         │               │
│    ┌────▼────┐              ┌────▼────┐              ┌────▼────┐           │
│    │ Mirror  │              │ Mirror  │              │ Mirror  │           │
│    │   #1    │              │   #2    │              │   #3    │           │
│    │ ACTIVE  │              │ ACTIVE  │              │ STANDBY │           │
│    └────┬────┘              └────┬────┘              └─────────┘           │
│         │                        │                                          │
│    abc123.onion              def456.onion           ghi789.onion           │
│    (Serving)                 (Serving)              (Paused)               │
│                                                                             │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Mirror States

| State | Can Serve? | Description |
|-------|-----------|-------------|
| `Spawning` | No | Being created, waiting for Tor |
| `Active` | Yes | Serving live traffic |
| `Paused` | No | Admin paused, serves redirect |
| `Suspicious` | Yes | Under observation, still serving |
| `Burning` | No | Being destroyed |
| `Burned` | No | Fully destroyed |

### Mirror Lifecycle

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           MIRROR LIFECYCLE                                  │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────┐    Tor HS Created    ┌─────────┐                            │
│   │SPAWNING │────────────────────►│ ACTIVE  │                            │
│   └─────────┘                      └────┬────┘                            │
│                                         │                                  │
│                    ┌────────────────────┼────────────────────┐            │
│                    │                    │                    │            │
│             Admin Pause          Compromise            Time Rotation      │
│             Score > 0.7          Detected              (Scheduled)        │
│                    │                    │                    │            │
│             ┌──────▼──────┐      ┌──────▼──────┐            │            │
│             │   PAUSED    │      │ SUSPICIOUS  │            │            │
│             │  (Standby)  │      │(Still serve)│            │            │
│             └──────┬──────┘      └──────┬──────┘            │            │
│                    │                    │                    │            │
│             Admin Activate       Score > 0.8                │            │
│                    │                    │                    │            │
│                    └───►┌──────────────┬┘◄───────────────────┘            │
│                         │              │                                   │
│                  ┌──────▼──────┐       │                                   │
│                  │  BURNING    │◄──────┘                                   │
│                  │ (Cleanup)   │                                           │
│                  └──────┬──────┘                                           │
│                         │                                                  │
│                  ┌──────▼──────┐                                           │
│                  │   BURNED    │                                           │
│                  │ (Deleted)   │                                           │
│                  └─────────────┘                                           │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Compromise Detection Signals

| Signal Type | Description | Severity |
|-------------|-------------|----------|
| `UnusualTraffic` | Abnormal request patterns | Variable |
| `TimingAnomaly` | Suspicious timing patterns | High |
| `RepeatedFailures` | High failure rate | Medium |
| `MemoryExhaustion` | Resource exhaustion | High |
| `NetworkAnomaly` | Network-level issues | High |

### Compromise Score Calculation

```rust
// Recent signals (last 5 minutes) contribute to score
let recent_signals: Vec<_> = signals
    .iter()
    .filter(|s| now - s.timestamp < 300)
    .collect();

// Average severity becomes compromise score (0.0 - 1.0)
let total_severity: f32 = recent_signals.iter().map(|s| s.severity).sum();
let compromise_score = (total_severity / recent_signals.len() as f32).min(1.0);

// State transitions
if compromise_score >= 0.7 && state == Active {
    state = Suspicious;
}
if compromise_score >= 0.8 {
    burn();  // Trigger burn
}
```

---

## Tor Hidden Service Creation

### Method 1: ADD_ONION (Tor 0.4.9.2+)

```
ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:8080 Flags=Detach,PoWDefensesEnabled
```

**Response:**
```
250-ServiceID=<service_id>
250-PrivateKey=ED25519-V3:<private_key>
250 OK
```

### Method 2: File-Based (Tor 0.4.8+)

```toml
# /path/to/torrc.inc
HiddenServiceDir /path/to/hs_dir
HiddenServicePort 80 127.0.0.1:8080
HiddenServicePoWDefensesEnabled 1
```

Then signal Tor to reload:
```
SIGNAL RELOAD
```

### Fallback Strategy

```
┌────────────────────────────────────────────────────────────────────────────┐
│                      TOR HS CREATION STRATEGY                               │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   1. Try ADD_ONION with PoWDefensesEnabled                                 │
│      │                                                                      │
│      ├── Success (Tor 0.4.9.2+) ──► Real HS with PoW                       │
│      │                                                                      │
│      └── Error 512/552 (older Tor) ──┐                                     │
│                                       │                                     │
│   2. Fall back to file-based HS      │                                     │
│      │◄──────────────────────────────┘                                     │
│      │                                                                      │
│      ├── Create HiddenServiceDir                                           │
│      ├── Write torrc.inc with PoW config                                   │
│      ├── Append %include to main torrc                                     │
│      └── SIGNAL RELOAD                                                     │
│                                                                             │
│   3. If no Tor control port ──► Placeholder .onion (testing)               │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Proof-of-Work Defense

### Tor Native PoW (0.4.8+)

Tor's built-in PoW defense makes clients solve puzzles before connections are accepted:

```
HiddenServicePoWDefensesEnabled 1
```

**Benefits:**
- Defense at Tor layer (before HTTP)
- Automatic difficulty adjustment
- No JavaScript required (solved by Tor client)

### Fortify Application-Level PoW

```rust
pub struct ProofOfWorkChallenge {
    pub challenge_id: String,
    pub challenge: Vec<u8>,     // 32 random bytes
    pub difficulty: u32,        // Leading zero bits
    pub created_at: u64,
}
```

**Verification:**
```rust
fn verify(&self, nonce: u64) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(&self.challenge);
    hasher.update(&nonce.to_le_bytes());
    let hash = hasher.finalize();
    
    // Check leading zeros
    self.count_leading_zeros(&hash) >= self.difficulty
}
```

**Note:** Currently disabled at Gate because JavaScript is banned. Tor native PoW handles this layer.

---

## Vanguards Integration

### What is Vanguards?

Vanguards is a Tor addon that provides additional guard layers to protect against:
- Guard discovery attacks
- Traffic analysis
- Circuit correlation attacks

### Architecture with Vanguards

```
┌────────────────────────────────────────────────────────────────────────────┐
│                      CIRCUIT WITH VANGUARDS                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   [Client] ──► [Guard] ──► [L2 Guard] ──► [L3 Guard] ──► [HS Intro] ──► HS │
│                               │               │                             │
│                          Vanguards        Vanguards                        │
│                          Managed          Managed                          │
│                                                                             │
│   Layer 2: 4 guards (default), rotated every 1-30 days                     │
│   Layer 3: 8 guards (default), rotated every 1-48 hours                    │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Vanguards Configuration

```ini
[Vanguards]
num_layer2_guards = 4
min_layer2_lifetime_days = 1
max_layer2_lifetime_days = 30
num_layer3_guards = 8
min_layer3_lifetime_hours = 1
max_layer3_lifetime_hours = 48

[Bandguards]
circ_max_age_hours = 24
circ_max_megabytes = 0
circ_max_dropped_cells = 30

[Rendguard]
rend_use_count = 16
rend_use_global_start_count = 1000
rend_use_relay_start_count = 100
rend_use_scale_at_count = 10000
```

### Installation Detection

```rust
fn find_vanguards_path() -> Option<(String, Vec<String>)> {
    // Check common locations
    let paths = [
        "vanguards",
        "/usr/local/bin/vanguards",
        "/usr/bin/vanguards",
        "~/.local/bin/vanguards",       // pip --user
        "/tmp/fortify/venv/bin/vanguards",  // Fortify venv
    ];
    
    // Try as Python module
    // python3 -m vanguards --help
    
    // Check /opt/vanguards (git clone)
    // /opt/vanguards/src/vanguards.py
}
```

### Controller Integration

```rust
// Start vanguards if enabled and available
async fn start_vanguards(&self) -> Result<()> {
    if !self.config.vanguards_enabled {
        return Ok(());
    }

    if !VanguardsManager::is_available() {
        tracing::warn!("Vanguards not found. Install with: pip3 install vanguards");
        return Ok(());  // Continue without vanguards
    }

    let mut vanguards = self.vanguards_manager.lock().await;
    vanguards.start()?;
}
```

---

## Multi-Daemon Architecture (Roadmap)

### Problem: Single Tor Daemon Bottleneck

```
┌────────────────────────────────────────────────────────────────────────────┐
│                    SINGLE DAEMON LIMITATION                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   All mirrors ──► Single Tor Daemon ──► Single point of DoS failure        │
│                                                                             │
│   • One daemon handles all circuits                                         │
│   • CPU-bound encryption                                                    │
│   • Memory pressure under load                                              │
│   • Single process crash = all mirrors down                                │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Solution: One Tor Per Core

```
┌────────────────────────────────────────────────────────────────────────────┐
│                     MULTI-DAEMON ARCHITECTURE                               │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│   │ Tor Daemon  │  │ Tor Daemon  │  │ Tor Daemon  │  │ Tor Daemon  │      │
│   │   Core 0    │  │   Core 1    │  │   Core 2    │  │   Core 3    │      │
│   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
│          │                │                │                │              │
│     Mirror A,B       Mirror C,D       Mirror E,F       Mirror G,H         │
│                                                                             │
│   Benefits:                                                                │
│   • Parallel crypto processing                                              │
│   • Process isolation (crash doesn't affect others)                        │
│   • Better CPU utilization                                                 │
│   • Distributed DoS handling                                               │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Planned Implementation

```rust
// Future: TorDaemonPool
pub struct TorDaemonPool {
    daemons: Vec<TorDaemon>,
    assignment_strategy: AssignmentStrategy,
}

impl TorDaemonPool {
    // Round-robin mirror assignment
    fn assign_mirror(&self, mirror: &Mirror) -> &TorDaemon {
        let idx = mirror.id.hash() % self.daemons.len();
        &self.daemons[idx]
    }
    
    // Health-based assignment
    fn assign_healthiest(&self) -> &TorDaemon {
        self.daemons
            .iter()
            .min_by_key(|d| d.circuit_count())
            .unwrap()
    }
}
```

---

## Tor Control Commands Used

| Command | Purpose | When Used |
|---------|---------|-----------|
| `AUTHENTICATE` | Authenticate to control port | Connection |
| `ADD_ONION` | Create hidden service | Mirror creation |
| `DEL_ONION` | Delete hidden service | Mirror burn |
| `SIGNAL NEWNYM` | New identity/circuits | After burn |
| `SIGNAL RELOAD` | Reload configuration | File-based HS |
| `GETINFO` | Get Tor status | Health checks |

---

## Security Considerations

### Mirror Isolation

- Each mirror has its own data directory
- Private keys are stored separately
- Burning deletes all associated data

### Rate Limiting

- Per-mirror request tracking
- Failure rate monitoring
- Automatic burn on threshold

### Key Management

```
/tmp/fortify/tor/mirrors/
├── mirror-001/
│   ├── hostname          # abc123...onion
│   ├── private_key       # ED25519 key
│   └── torrc.inc        # HS config
├── mirror-002/
│   └── ...
```

**Permissions:** `chmod 700` on all directories (Tor requirement)

---

## Debugging Tor Issues

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "Authentication failed" | Wrong cookie path | Check `tor_cookie_path` |
| "512 Unrecognized flag" | Old Tor version | Use file-based PoW |
| "Permission denied" | Wrong directory perms | `chmod 700` |
| "No control port" | Tor not configured | Add `ControlPort 9051` |

### Logging

```rust
// PoW status logging (once per startup)
if !POW_STATUS_LOGGED.swap(true, Ordering::Relaxed) {
    tracing::info!("✅ Tor PoW enabled via ADD_ONION (Tor 0.4.9.2+)");
}

if !POW_FILE_LOGGED.swap(true, Ordering::Relaxed) {
    tracing::info!("ADD_ONION PoW not supported, using file-based HS with PoW");
}
```

---

*See [Functions.md](../Functions.md) for complete Tor API reference*
