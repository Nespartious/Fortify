# CAPTCHA Pool Multi-Type Pre-Generation & Mode Switching

## Status: PLANNING (Future Enhancement)
**Priority:** Medium  
**Complexity:** Medium-High  
**Dependencies:** None (standalone enhancement)

---

## Overview

Currently, Fortify pre-generates only **BmpText** (bitmap text) CAPTCHAs at startup. This planning document outlines enhancements to:

1. **Pre-generate multiple CAPTCHA types** with configurable pool sizes per type
2. **Dynamic mode switching** based on system resources and attack conditions
3. **TUI and Control Panel configuration** for granular pool management

---

## Current Implementation

### What Exists Today
- `CaptchaPoolManager` in Gate pre-generates 200 BmpText CAPTCHAs at startup
- Pool is refilled during idle periods
- No support for other CAPTCHA types in the pool
- No dynamic switching based on conditions

### CAPTCHA Types Available
| Type | Difficulty | Resource Usage | Bot Resistance |
|------|------------|----------------|----------------|
| `BmpText` | Medium | Low (pre-rendered) | Medium |
| `Emoji` | Easy | Low (Unicode) | Low |
| `Direction` | Easy | Low (Unicode arrows) | Low |
| `Sequence` | Medium | Low (numbers) | Medium |
| `WordUnscramble` | Hard | Medium (word lookup) | High |
| `ImageRotation` | Medium | High (image gen) | High |
| `Silhouette` | Medium | High (image gen) | High |

---

## Proposed Enhancements

### Phase 1: Multi-Type Pool Configuration

#### Configuration Schema (TOML)
```toml
[captcha.pools]
# Each type can have its own pool size
# Set to 0 to disable pre-generation for that type
bmptext_pool_size = 200       # Default, low resource
emoji_pool_size = 50          # Quick to serve
direction_pool_size = 50      # Quick to serve
sequence_pool_size = 100      # Medium resource
word_unscramble_pool_size = 25 # CPU intensive
image_rotation_pool_size = 10  # Very CPU intensive
silhouette_pool_size = 10      # Very CPU intensive

# Total pool target (sum of all individual pools)
# Pool manager will distribute based on above ratios
total_target = 500

# Minimum pool threshold before refill triggered
min_pool_threshold_percent = 25

# Pool rotation settings
rotation_percent = 25         # Rotate 25% of pool
rotation_interval_days = 10   # Every 10 days
```

#### Implementation Tasks
- [ ] Create `MultiTypeCaptchaPool` struct with per-type pools
- [ ] Update `CaptchaPoolManager` to manage multiple pools
- [ ] Add pool selection logic based on configured ratios
- [ ] Implement pool-specific refill with priority ordering
- [ ] Add metrics for per-type pool usage

---

### Phase 2: Dynamic Mode Switching

#### Trigger Conditions

| Condition | Detection | Response |
|-----------|-----------|----------|
| High CPU usage (>80%) | System metrics | Switch to low-resource types (Emoji, Direction) |
| Active DDoS | Request rate spike | Switch to harder types (WordUnscramble, ImageRotation) |
| Low memory | System metrics | Reduce pool sizes, use on-demand generation |
| Pool depletion | Pool monitor | Prioritize refill, temporary fallback to simple types |
| Attack escalation | Threat score | Progressive difficulty increase |

#### Configuration Schema
```toml
[captcha.mode_switching]
enabled = true

# Thresholds for automatic switching
cpu_threshold_high = 80       # % - switch to easy types above this
cpu_threshold_low = 40        # % - restore normal types below this
memory_threshold_mb = 512     # Minimum free memory to maintain pools

# Attack response
attack_detection_rps = 100    # Requests/sec to trigger attack mode
attack_cooldown_seconds = 300 # Time to wait before returning to normal

# Mode definitions
[captcha.modes.normal]
primary_types = ["BmpText", "Emoji", "Direction"]
weights = [50, 25, 25]        # Percentage distribution

[captcha.modes.under_attack]
primary_types = ["WordUnscramble", "ImageRotation", "Silhouette"]
weights = [40, 30, 30]

[captcha.modes.low_resource]
primary_types = ["Emoji", "Direction", "Sequence"]
weights = [40, 40, 20]
```

#### Implementation Tasks
- [ ] Create `CaptchaMode` enum (Normal, UnderAttack, LowResource, Custom)
- [ ] Add system metrics collection (CPU, memory)
- [ ] Implement mode switching state machine
- [ ] Add mode transition logging and alerts
- [ ] Create fallback chain for pool exhaustion

---

### Phase 3: TUI Integration

#### New TUI Screen: CAPTCHA Management
```
┌─────────────────────────────────────────────────────────────────┐
│  CAPTCHA POOL STATUS                                    [F5] ↻ │
├─────────────────────────────────────────────────────────────────┤
│  Current Mode: NORMAL                          CPU: 23%  MEM: 2.1GB │
│                                                                 │
│  ┌─ Pool Levels ────────────────────────────────────────────┐  │
│  │ BmpText     ████████████████████░░░░  185/200 (93%)      │  │
│  │ Emoji       ████████████░░░░░░░░░░░░   48/50  (96%)      │  │
│  │ Direction   █████████████████░░░░░░░   42/50  (84%)      │  │
│  │ Sequence    ███████████████████░░░░░   95/100 (95%)      │  │
│  │ WordUnscr   ██████████░░░░░░░░░░░░░░   20/25  (80%)      │  │
│  │ ImgRotate   ████████░░░░░░░░░░░░░░░░    8/10  (80%)      │  │
│  │ Silhouette  ██████████░░░░░░░░░░░░░░   10/10  (100%)     │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  [1] Adjust Pool Sizes  [2] Switch Mode  [3] Force Refill      │
│  [4] View Statistics    [5] Export Config                       │
└─────────────────────────────────────────────────────────────────┘
```

#### Implementation Tasks
- [ ] Add CAPTCHA pool widget to TUI
- [ ] Create pool configuration modal
- [ ] Add mode switching controls
- [ ] Implement real-time pool level display
- [ ] Add pool statistics view (usage rates, refill frequency)

---

### Phase 4: Control Panel Web UI

#### Admin Panel Endpoints
```
GET  /admin/captcha/pools          - Get all pool statuses
GET  /admin/captcha/pools/:type    - Get specific pool status
POST /admin/captcha/pools/:type    - Update pool configuration
POST /admin/captcha/mode           - Switch CAPTCHA mode
GET  /admin/captcha/stats          - Get CAPTCHA usage statistics
POST /admin/captcha/refill         - Trigger manual refill
```

#### Control Panel UI Section
- Pool status dashboard with visual indicators
- Per-type configuration forms
- Mode switching buttons with confirmation
- Real-time statistics charts
- Alert configuration for pool thresholds

#### Implementation Tasks
- [ ] Add pool endpoints to admin.rs
- [ ] Create pool management HTML templates
- [ ] Add JavaScript-free statistics display
- [ ] Implement configuration persistence
- [ ] Add audit logging for configuration changes

---

## Database/Storage Considerations

### Pool Persistence (Optional)
- Store pre-generated CAPTCHAs to disk for fast restart recovery
- File format: `captcha_pool_{type}.json`
- Include metadata: created_at, difficulty, checksum
- Validate on load, regenerate if invalid

### Statistics Tracking
```rust
struct CaptchaStats {
    total_served: u64,
    total_solved: u64,
    total_failed: u64,
    avg_solve_time_ms: u64,
    type_distribution: HashMap<CaptchaType, u64>,
    hourly_usage: [u64; 24],
}
```

---

## Security Considerations

1. **Pool Predictability**: Rotate pool contents regularly to prevent pattern detection
2. **Timing Attacks**: Ensure on-demand generation has similar timing to pool serving
3. **Resource Exhaustion**: Limit pool refill rate during attacks
4. **Mode Switching Abuse**: Rate-limit mode changes, require authentication

---

## Migration Path

### Backward Compatibility
- Default configuration matches current behavior (BmpText only)
- New pool types opt-in via configuration
- Existing deployments unaffected until explicitly configured

### Upgrade Steps
1. Deploy new version (pools disabled by default)
2. Test new captcha types individually
3. Enable multi-type pools with conservative sizes
4. Monitor performance and adjust
5. Enable mode switching after baseline established

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Pool hit rate | >95% | Pool serves vs on-demand generation |
| Average solve time | <30s | Time from display to correct answer |
| Bot rejection rate | >99% | Failed attempts from suspicious sources |
| CPU usage during refill | <10% | System metrics during pool maintenance |
| Memory usage per 100 CAPTCHAs | <50MB | Heap measurement |

---

## Timeline Estimate

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1: Multi-Type Pools | 2-3 days | None |
| Phase 2: Mode Switching | 2-3 days | Phase 1 |
| Phase 3: TUI Integration | 1-2 days | Phase 1 |
| Phase 4: Control Panel | 2-3 days | Phase 1 |
| Testing & Documentation | 1-2 days | All phases |

**Total Estimate: 8-13 days**

---

## Open Questions

1. Should we support custom CAPTCHA types via plugins?
2. Should pool configuration be hot-reloadable without restart?
3. How should we handle pool exhaustion during sustained attacks?
4. Should we implement CAPTCHA difficulty progression per session?

---

## Related Documents

- [CAPTCHA-BUG-SPRINT.md](../Dev_Progress/06-CAPTCHA-BUG-SPRINT.md) - Original CAPTCHA implementation
- [HARDENING-SPRINT.md](../Dev_Progress/10-HARDENING-SPRINT.md) - Security hardening details
- [STATIC-CAPTCHA-TEMPLATES-SPRINT.md](../Dev_Progress/11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md) - Template system

---

*Document created: 2026-01-22*  
*Status: Planning - Not yet scheduled for implementation*
