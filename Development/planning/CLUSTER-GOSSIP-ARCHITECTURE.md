# Fortify Cluster Architecture - Gossip Protocol Design

**Status:** Planning  
**Priority:** High  
**Complexity:** Large  
**Target:** v0.4.0+

---

## Overview

Enable multiple Fortify instances to work together as a unified cluster, sharing session state, distributing load, and providing redundancy. Users verify once on any node and can seamlessly access the protected service through any other node.

## Goals

1. **High Availability** - Survive individual node failures
2. **Horizontal Scaling** - Handle more traffic than one machine can process
3. **Attack Distribution** - Spread DDoS load across multiple machines
4. **Session Continuity** - Verify once, access everywhere
5. **Simple Setup** - One token to join a cluster

## Non-Goals (This Phase)

- Geographic load balancing (routing to nearest node)
- Automatic scaling (spin up/down nodes based on load)
- Cross-cluster federation (clusters talking to other clusters)

---

## Architecture

### Network Topology

```
┌────────────────────────────────────────────────────────────────┐
│                    GOSSIP LAYER (Encrypted)                     │
│                                                                  │
│    ┌──────────┐         WireGuard          ┌──────────┐         │
│    │Fortify A │◄──────────────────────────►│Fortify B │         │
│    │ US East  │                            │ EU West  │         │
│    └────┬─────┘                            └────┬─────┘         │
│         │              ╲           ╱             │              │
│         │               ╲         ╱              │              │
│         │                ╲       ╱               │              │
│         │                 ╲     ╱                │              │
│         │            ┌──────────┐                │              │
│         └───────────►│Fortify C │◄───────────────┘              │
│                      │  Asia    │                               │
│                      └────┬─────┘                               │
└────────────────────────────────────────────────────────────────┘
                            │
                     Each node has its
                     own .onion address
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
       abc123.onion   def456.onion   ghi789.onion
```

### Transport Modes

| Mode | Latency | Privacy | Recommended For |
|------|---------|---------|-----------------|
| **WireGuard** | ~5ms | Operators know each other's IPs | Production, attack defense |
| **Tor Gossip** | ~200ms | Fully anonymous between operators | Privacy-focused, smaller scale |

> ⚠️ **Tor Gossip Warning:** Onion-mode clustering has significantly higher latency. Not recommended for high-traffic attack defense scenarios. Use WireGuard mode for production deployments.

---

## Data Structures

### Gossip Sync Payload

```rust
/// Data synchronized across the cluster
struct ClusterSync {
    /// Active sessions with verification status
    sessions: HashMap<SessionId, SessionState>,
    
    /// Known cluster peers and their health
    peers: HashMap<NodeId, PeerInfo>,
    
    /// Cluster-wide banned sessions
    bans: HashSet<BannedSession>,
    
    /// Sync metadata
    vector_clock: VectorClock,
    origin_node: NodeId,
    timestamp: u64,
}

/// Session state shared across nodes (~80 bytes)
struct SessionState {
    session_id: [u8; 16],      // 128-bit session ID
    trust_level: TrustTier,    // New=0, Verified=1, Trusted=2
    verified_at: u64,          // Unix timestamp
    expires_at: u64,           // Unix timestamp  
    origin_node: NodeId,       // Which node verified this
    captchas_solved: u8,       // For threat-mode tracking
}

/// Peer health information (~100 bytes)
struct PeerInfo {
    node_id: NodeId,           // Unique node identifier
    onion_address: String,     // abc123.onion
    gossip_addr: GossipAddr,   // IP:port or onion:port
    last_seen: u64,            // Unix timestamp
    load_percent: u8,          // 0-100 current load
    pool_available: u32,       // CAPTCHA pages available
    pool_target: u32,          // CAPTCHA pool target size
    is_healthy: bool,          // Passing health checks
    version: String,           // Fortify version
}

/// Gossip address - clearnet or onion
enum GossipAddr {
    WireGuard { ip: IpAddr, port: u16, pubkey: [u8; 32] },
    Tor { onion: String, port: u16 },
}

/// Banned session record (~60 bytes)
struct BannedSession {
    session_id: [u8; 16],
    reason: BanReason,
    banned_at: u64,
    banned_by: NodeId,
    expires_at: u64,           // Bans can expire
}

enum BanReason {
    FailedCaptcha { attempts: u8 },
    RateLimited,
    Suspicious,
    ManualBan,
}
```

### Cluster Configuration

```rust
/// Cluster settings in fortify.toml
struct ClusterConfig {
    /// Enable clustering
    enabled: bool,
    
    /// Transport mode
    mode: ClusterMode,  // WireGuard or Tor
    
    /// Gossip listen address
    listen_addr: SocketAddr,  // 0.0.0.0:9090
    
    /// Cluster shared secret (derived from join token)
    cluster_key: [u8; 32],
    
    /// Known peers (learned via gossip, persisted)
    bootstrap_peers: Vec<GossipAddr>,
    
    /// Sync intervals
    delta_sync_ms: u64,       // 100ms default
    full_sync_secs: u64,      // 60s default
    
    /// Health check settings
    health_check_secs: u64,   // 5s default
    peer_timeout_secs: u64,   // 30s before marking unhealthy
}

enum ClusterMode {
    WireGuard,
    Tor,
}
```

---

## User Flows

### Cluster Initialization (First Node)

```bash
$ fortify cluster init --mode wireguard
# or: fortify cluster init --mode tor

✓ Generated cluster key
✓ WireGuard interface created: fortify0
✓ Listening for peers on 0.0.0.0:9090

Cluster Join Token (share with other nodes):
┌─────────────────────────────────────────────────────────────┐
│  FTY1.wg.MTAzLjQ1LjY3Ljg5OjkwOTAuYWJjMTIzLi4u              │
└─────────────────────────────────────────────────────────────┘

Your node ID: NodeA
Your onion: abc123.onion
Cluster size: 1 node

Token format: FTY1.<mode>.<base64(addr + cluster_key)>
```

### Joining a Cluster

```bash
$ fortify cluster join FTY1.wg.MTAzLjQ1LjY3Ljg5OjkwOTAuYWJjMTIzLi4u

✓ Parsed join token
✓ Connecting to 103.45.67.89:9090...
✓ WireGuard tunnel established
✓ Authenticated with cluster key
✓ Discovered 2 existing peers: NodeA, NodeB
✓ Full sync complete: 1,234 sessions
✓ Your node registered: NodeC (ghi789.onion)
✓ Cluster now has 3 nodes

You can share the same token to add more nodes.
```

### Session Verification Flow

```
1. User visits abc123.onion (NodeA)
2. No session cookie → Show CAPTCHA
3. User solves CAPTCHA
4. NodeA:
   a. Creates session: XYZ123, trust=Verified
   b. Sets cookie: fortify_session=XYZ123.<signature>
   c. Gossips to cluster: "XYZ123 verified"
5. User clicks link to def456.onion (NodeB)  
6. NodeB receives request with cookie
7. NodeB validates signature, extracts XYZ123
8. NodeB checks local session cache → Found (from gossip)!
9. NodeB passes request to backend (no CAPTCHA)
```

### Cookie Format

```
fortify_session=<session_id>.<timestamp>.<hmac_signature>

Example:
fortify_session=abc123def456.1706300000.a1b2c3d4e5f6

- session_id: 16 bytes, base64url encoded
- timestamp: Unix timestamp when issued
- signature: HMAC-SHA256(session_id + timestamp, cluster_key)
```

Signature uses cluster key, so valid across all nodes.

---

## Landing Page - Cluster Mode

### Standalone Mode (Current)
Shows 2 mirror links for redundancy.

### Cluster Mode (New)
Shows 1 link per cluster node in header bar.

```html
<!-- Cluster mode header -->
<header class="cluster-nav">
  <span class="brand">🛡️ Protected Service</span>
  <nav class="entry-points">
    <span class="label">Entry Points:</span>
    <!-- Server-rendered based on peer health -->
    <a href="http://abc123.onion" class="node healthy">Node1</a>
    <a href="http://def456.onion" class="node healthy">Node2</a>
    <a href="http://ghi789.onion" class="node degraded">Node3</a>
  </nav>
</header>

<style>
.entry-points { display: flex; gap: 8px; }
.node { 
  padding: 4px 12px; 
  border-radius: 4px;
  text-decoration: none;
  font-size: 0.85rem;
}
.node.healthy { background: #1a3d1a; color: #4ade80; }
.node.healthy::before { content: "● "; }
.node.degraded { background: #3d3d1a; color: #fbbf24; }
.node.degraded::before { content: "○ "; }
.node.offline { background: #3d1a1a; color: #f87171; opacity: 0.5; }
</style>
```

Health indicators:
- **● Green** - Healthy, low load
- **○ Yellow** - High load (>70%) or degraded
- **Dimmed** - Offline (still shown for awareness)

---

## TUI Integration

### Main Menu Addition

```
┌─ Fortify ─────────────────────────┐
│                                   │
│  [D] Deploy                       │
│  [J] Join Community Network       │
│  [C] Cluster Settings      ← NEW  │
│  [V] View System Settings         │
│  [M] Modify System Settings       │
│  [X] Destroy Instance             │
│  [Q] Quit                         │
│                                   │
└───────────────────────────────────┘
```

### Cluster Settings Submenu

```
┌─ Cluster Settings ─────────────────────────────────┐
│                                                    │
│  Status: Connected                                 │
│  Mode: WireGuard                                   │
│  Peers: 3 healthy, 0 unhealthy                     │
│                                                    │
│  [I] Initialize new cluster                        │
│  [J] Join existing cluster                         │
│  [L] Leave cluster                                 │
│  [P] View peer status                              │
│  [T] Show join token                               │
│  [B] Back                                          │
│                                                    │
└────────────────────────────────────────────────────┘
```

### Peer Status View

```
┌─ Cluster Peers ────────────────────────────────────┐
│                                                    │
│  NODE         ONION              LOAD   POOL  PING │
│  ──────────────────────────────────────────────── │
│  NodeA (you)  abc123.onion       45%   1800   -   │
│  NodeB        def456.onion       32%   1950   8ms │
│  NodeC        ghi789.onion       78%   1200  12ms │
│                                                    │
│  Sessions synced: 2,341                            │
│  Last full sync: 12 seconds ago                    │
│                                                    │
│  [R] Refresh   [B] Back                            │
│                                                    │
└────────────────────────────────────────────────────┘
```

---

## Gossip Protocol Details

### Message Types

```rust
enum GossipMessage {
    /// Initial handshake
    Hello {
        node_id: NodeId,
        version: String,
        cluster_key_hash: [u8; 8],  // First 8 bytes for quick reject
    },
    
    /// Response to Hello
    Welcome {
        node_id: NodeId,
        peers: Vec<PeerInfo>,       // Full peer list for discovery
    },
    
    /// Delta sync - just changes since last sync
    Delta {
        since_clock: VectorClock,
        sessions_added: Vec<SessionState>,
        sessions_removed: Vec<SessionId>,
        bans_added: Vec<BannedSession>,
    },
    
    /// Full sync request/response
    FullSync {
        sessions: Vec<SessionState>,
        bans: Vec<BannedSession>,
        peers: Vec<PeerInfo>,
    },
    
    /// Health ping
    Ping { load: u8, pool_available: u32 },
    Pong { load: u8, pool_available: u32 },
    
    /// Node leaving gracefully
    Goodbye { node_id: NodeId },
}
```

### Sync Strategy

| Event | Action | Latency |
|-------|--------|---------|
| Session verified | Delta broadcast to all peers | <50ms |
| Session banned | Delta broadcast to all peers | <50ms |
| Peer health change | Ping/Pong exchange | Every 5s |
| Node joins | Full sync from one peer | Once |
| Periodic consistency | Full sync with random peer | Every 60s |

### Conflict Resolution

**Last-write-wins with node priority:**
1. Compare timestamps
2. If equal, higher node_id wins
3. Bans always override verifications (security wins)

### Scaling Limits

| Nodes | Connections | Bandwidth | Notes |
|-------|-------------|-----------|-------|
| 2 | 1 | ~1 KB/s | Trivial |
| 5 | 10 | ~10 KB/s | Easy |
| 10 | 45 | ~50 KB/s | Target design |
| 20 | 190 | ~200 KB/s | Max full mesh |
| 50+ | - | - | Needs hierarchy (future) |

For >20 nodes, would need super-node architecture (future enhancement).

---

## Security Considerations

### Cluster Key Protection
- 256-bit key generated at cluster init
- Stored encrypted in node's secure storage
- Never transmitted in plaintext (only key hash for validation)

### WireGuard Security
- Modern cryptography (ChaCha20, Poly1305, Curve25519)
- Perfect forward secrecy
- Automatic key rotation

### Tor Mode Security
- Traffic analysis resistant
- Operator IP anonymity
- Higher latency trade-off

### Session Cookie Security
- HMAC-SHA256 signature with cluster key
- Timestamp prevents replay beyond session lifetime
- Signature invalid if cluster key changes

---

## Implementation Phases

### Phase 1: Core Gossip (MVP)
- [ ] `fortify-cluster` crate with gossip protocol
- [ ] WireGuard tunnel management (via `wireguard-rs` or shell)
- [ ] Session state sync
- [ ] Peer discovery
- [ ] CLI: `fortify cluster init/join/leave`

### Phase 2: TUI Integration
- [ ] Cluster menu in TUI
- [ ] Peer status display
- [ ] Join token display/input
- [ ] Cluster health in dashboard

### Phase 3: Landing Page
- [ ] Multi-node links in header
- [ ] Health-based styling
- [ ] Session cookie validation
- [ ] Pre-render pages include cluster info

### Phase 4: Tor Gossip Mode
- [ ] Onion service for gossip
- [ ] Latency-aware sync tuning
- [ ] Warning UI for onion mode
- [ ] Connection retry logic for Tor

### Phase 5: Attack Coordination (See Next Section)
- [ ] Coordinated rate limiting
- [ ] Attack pattern sharing
- [ ] Load-based traffic steering
- [ ] Distributed CAPTCHA pool

---

## Attack Defense Strategies

*To be designed - how cluster coordination improves attack resilience.*

See: [CLUSTER-ATTACK-DEFENSE.md](./CLUSTER-ATTACK-DEFENSE.md)

---

## Open Questions

1. **Peer authentication:** Should nodes verify each other beyond shared key? (mTLS?)
2. **Key rotation:** How to rotate cluster key without downtime?
3. **Partial mesh:** For large clusters, should nodes only connect to subset?
4. **Cross-DC latency:** Special handling for geographically distant peers?

---

## References

- [WireGuard Protocol](https://www.wireguard.com/protocol/)
- [SWIM Gossip Protocol](https://www.cs.cornell.edu/projects/Quicksilver/public_pdfs/SWIM.pdf)
- [Vector Clocks](https://en.wikipedia.org/wiki/Vector_clock)
