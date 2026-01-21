# 🖥️ Admin Panel Guide

> **Managing Fortify Through the Web Interface**

---

## Accessing the Admin Panel

### URL

```
http://[proxy_address]/ctrl_8f7k3m9x2n4p1q6w5v0b8c/
```

**Default:** `http://127.0.0.1:8082/ctrl_8f7k3m9x2n4p1q6w5v0b8c/`

**Note:** The path is intentionally obscured. Change it in production by modifying `ADMIN_PATH` in `admin.rs`.

---

## Dashboard Overview

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         FORTIFY ADMIN DASHBOARD                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                        SYSTEM STATUS                                 │  │
│  ├─────────────────────────────────────────────────────────────────────┤  │
│  │                                                                      │  │
│  │   Total Sessions: 127        Active Mirrors: 3                      │  │
│  │   ├── Trusted:    12         ├── Mirror 1: ACTIVE                   │  │
│  │   ├── Verified:   89         ├── Mirror 2: ACTIVE                   │  │
│  │   ├── Unknown:    15         └── Mirror 3: STANDBY                  │  │
│  │   ├── Suspicious: 8                                                 │  │
│  │   └── Burned:     3          Nodes: 13 (10 healthy, 3 threat)       │  │
│  │                                                                      │  │
│  │   Violations (24h): 156      Vanguards: RUNNING (12h 34m)           │  │
│  │   Demotions: 23              CPU: 34%  |  Memory: 2.1GB / 8GB       │  │
│  │                                                                      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  NAVIGATION                                                                 │
│  ──────────                                                                 │
│  [Sessions] [Nodes] [Mirrors] [Behavioral] [Captcha] [Traffic]            │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Sessions Page

### URL: `/ctrl_.../sessions`

### Session List View

| Column | Description |
|--------|-------------|
| Session ID | Unique identifier (UUID) |
| Trust Tier | Current tier (with color coding) |
| Requests | Total request count |
| Violations | Total violation count |
| Demotion Count | Times demoted and re-verified |
| Last Activity | Time since last request |
| Current Node | Which node is handling them |
| Status | Normal / Killed / Banned |

### Color Coding

| Tier | Color | Background |
|------|-------|------------|
| Trusted | Cyan | Dark blue |
| Verified | Green | Dark green |
| Unknown | Gray | Dark gray |
| Suspicious | Amber | Dark orange |
| Burned | Red | Dark red |

### Session Actions

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        SESSION ACTIONS                                      │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │  Session: abc123-def456-789...                                       │  │
│  │  Current Tier: VERIFIED                                              │  │
│  │                                                                      │  │
│  │  [Change Tier]  [Ban Session]  [Kill Session]  [View History]       │  │
│  │                                                                      │  │
│  │  Change Tier:                                                        │  │
│  │  ( ) Trusted   ( ) Verified   ( ) Suspicious   ( ) Burned           │  │
│  │  [Apply]                                                             │  │
│  │                                                                      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

| Action | Effect |
|--------|--------|
| **Change Tier** | Set admin override (persists until cleared) |
| **Ban Session** | Mark as banned, serve block page |
| **Unban Session** | Remove ban flag |
| **Kill Session** | Permanent orphan (repeat offender) |
| **View History** | See detailed browsing/event history |

---

## Session Detail Page

### URL: `/ctrl_.../sessions/{session_id}`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        SESSION DETAIL                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  SESSION: abc123-def456-789...                                             │
│  ════════════════════════════════════════════════════════════════════════  │
│                                                                             │
│  OVERVIEW                                                                   │
│  ────────                                                                   │
│  Trust Tier:      SUSPICIOUS                                               │
│  Created:         2024-01-15 14:32:00                                      │
│  Last Activity:   5 minutes ago                                            │
│  Total Requests:  247                                                       │
│  Page Loads:      89                                                        │
│  Violations:      7                                                         │
│  Demotion Count:  2                                                         │
│  Current Node:    threat-0                                                  │
│  Current Mirror:  abc123.onion                                             │
│                                                                             │
│  BEHAVIORAL STATS                                                          │
│  ────────────────                                                          │
│  Unique Paths:        45                                                    │
│  Form Submissions:    12                                                    │
│  Total Payload:       2.3 MB                                               │
│  Severity Score:      12                                                    │
│  Bot Detected:        No                                                    │
│                                                                             │
│  VIOLATIONS BY TYPE                                                         │
│  ──────────────────                                                         │
│  Attack Path Access:      3                                                 │
│  Path Enumeration:        2                                                 │
│  Form Submission Flood:   2                                                 │
│                                                                             │
│  HISTORY (Last 50 Events)                                                  │
│  ════════════════════════════════════════════════════════════════════════  │
│                                                                             │
│  📄 14:32:05  GET /index.html                 200                          │
│  📄 14:32:08  GET /api/users                  200                          │
│  🚨 14:32:12  Violation: Path Enumeration detected                         │
│  📄 14:32:15  GET /page1                      200                          │
│  📄 14:32:16  GET /page2                      200                          │
│  ⚠️ 14:32:20  Auto-Demotion: Threshold exceeded                            │
│  👮 14:32:45  Admin Tier Change: Verified → Suspicious (admin)             │
│  🔓 14:35:00  Captcha Verified (passed 2 captchas)                         │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### History Event Types

| Icon | Type | Description |
|------|------|-------------|
| 📄 | PageRequest | Normal page/API request |
| 👮 | AdminTierChange | Admin changed tier |
| ⚠️ | AutoDemotion | System demoted session |
| 🚫 | SessionBanned | Session banned |
| ✅ | SessionUnbanned | Session unbanned |
| 💀 | SessionKilled | Repeat offender killed |
| 🔓 | CaptchaVerified | Passed verification |
| 🚨 | ViolationDetected | Behavioral violation |

---

## Nodes Page

### URL: `/ctrl_.../nodes`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           NODE STATUS                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  HEALTHY NODES (10)                                                         │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  ID          Address              Status    Requests    Connections        │
│  ──────────────────────────────────────────────────────────────────────    │
│  healthy-0   127.0.0.1:9100      ONLINE    12,456      23                  │
│  healthy-1   127.0.0.1:9101      ONLINE    11,234      18                  │
│  healthy-2   127.0.0.1:9102      ONLINE    13,890      31                  │
│  ...                                                                        │
│                                                                             │
│  THREAT NODES (3)                                                          │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  ID          Address              Status    Requests    Violations         │
│  ──────────────────────────────────────────────────────────────────────    │
│  threat-0    127.0.0.1:9110      ONLINE    2,345       156                 │
│  threat-1    127.0.0.1:9111      ONLINE    1,890       89                  │
│  threat-2    127.0.0.1:9112      ONLINE    2,012       124                 │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Mirrors Page

### URL: `/ctrl_.../mirrors`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           MIRROR MANAGEMENT                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ACTIVE MIRRORS                                                             │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  Onion Address                           Status    Age     Score   Actions │
│  ──────────────────────────────────────────────────────────────────────    │
│  abc123xyz456...onion                    ACTIVE    2h 15m  0.12   [Pause] [Burn]  │
│  def789uvw321...onion                    ACTIVE    1h 45m  0.08   [Pause] [Burn]  │
│                                                                             │
│  STANDBY MIRRORS                                                           │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  ghi012rst654...onion                    PAUSED    4h 30m  0.00   [Activate]      │
│  jkl345mno987...onion                    PAUSED    3h 10m  0.00   [Activate]      │
│                                                                             │
│  BURNED MIRRORS (Last 24h)                                                 │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  old123abc789...onion                    BURNED    6h ago  N/A    [Details]       │
│                                                                             │
│                                                                             │
│  [ Create New Standby ]  [ Manual Rotation ]  [ Emergency Burn All ]       │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Mirror Actions

| Action | Effect |
|--------|--------|
| **Pause** | Stop serving, show redirect page |
| **Activate** | Resume serving traffic |
| **Burn** | Initiate burn sequence |
| **Create Standby** | Spawn new paused mirror |
| **Manual Rotation** | Burn oldest, activate standby |
| **Emergency Burn All** | Burn all mirrors (nuclear option) |

---

## Behavioral Config Page

### URL: `/ctrl_.../behavioral`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        BEHAVIORAL CONFIGURATION                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  DETECTION FEATURES                                                         │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  [✓] User-Agent Analysis                                                   │
│  [✓] Referer Analysis                                                      │
│  [✓] Path Analysis                                                         │
│  [✓] Enumeration Detection                                                 │
│  [✓] Form Tracking                                                         │
│  [✓] Payload Analysis                                                      │
│                                                                             │
│  THRESHOLDS                                                                │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  Max Unique Paths/Min:        [  60  ]                                     │
│  Max Form Submissions/Min:    [  10  ]                                     │
│  Max Payload Size (MB):       [  10  ]                                     │
│  Sequential Path Threshold:   [   5  ]                                     │
│  Demotion Threshold:          [  10  ]  violations                         │
│  Severity Threshold:          [  15  ]  cumulative                         │
│  Kill After Demotions:        [   3  ]                                     │
│                                                                             │
│  ATTACK PATH PATTERNS                                                      │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  [✓] ../         Path traversal                                            │
│  [✓] /.env       Environment file                                          │
│  [✓] /.git       Git directory                                             │
│  [ ] /admin      Admin panel (disabled)                                    │
│  [✓] /wp-admin   WordPress admin                                           │
│  ...                                                                        │
│                                                                             │
│  WHITELIST PATHS                                                           │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  /api/*                                                         [Remove]   │
│  /static/*                                                      [Remove]   │
│  [Add Path: ____________] [Add]                                            │
│                                                                             │
│                                      [Save Configuration]                   │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Captcha Config Page

### URL: `/ctrl_.../captcha`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        CAPTCHA CONFIGURATION                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GATE CAPTCHA (New Users)                                                  │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  Type: (•) BmpText   ( ) Emoji   ( ) Direction   ( ) Sequence              │
│        ( ) WordUnscramble   ( ) ImageRotation   ( ) Silhouette             │
│                                                                             │
│  THREAT CAPTCHA (Demoted Users)                                            │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  [✓] Use different type for threat sessions                                │
│                                                                             │
│  Type: ( ) BmpText   (•) Emoji   ( ) Direction   ( ) Sequence              │
│        ( ) WordUnscramble   ( ) ImageRotation   ( ) Silhouette             │
│                                                                             │
│  RANDOM CYCLING                                                            │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  [ ] Enable random captcha cycling                                         │
│                                                                             │
│  Include in cycle:                                                         │
│  [✓] BmpText   [✓] Emoji   [✓] Direction   [ ] Sequence                   │
│  [ ] WordUnscramble   [ ] ImageRotation   [ ] Silhouette                  │
│                                                                             │
│  TYPE-SPECIFIC SETTINGS                                                    │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  Emoji:                                                                    │
│    Option Count: [  6  ]                                                   │
│    Difficulty:   [  2  ]                                                   │
│                                                                             │
│  Direction:                                                                │
│    Include Diagonals: [ ]                                                  │
│                                                                             │
│                                      [Save Configuration]                   │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Traffic Analytics

### URL: `/ctrl_.../traffic`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        TRAFFIC ANALYTICS                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TIME RANGE: [Last Hour ▼]                                                 │
│                                                                             │
│  REQUESTS                                                                  │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  1200 │                          ╭─╮                                       │
│  1000 │              ╭──╮       │  │    ╭╮                                 │
│   800 │         ╭───╯   ╰──╮   │  │   │ │                                 │
│   600 │    ╭───╯           ╰──╯   ╰──╯  ╰─╮                               │
│   400 │ ──╯                                ╰──                             │
│   200 │                                                                    │
│     0 └────────────────────────────────────────────────────────           │
│       14:00  14:10  14:20  14:30  14:40  14:50  15:00                     │
│                                                                             │
│  SUMMARY                                                                   │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  Total Requests:    45,678            Avg Response Time:   234ms           │
│  Total Bandwidth:   1.2 GB            Violations:          156             │
│  Peak RPS:          1,245             Demotions:           23              │
│                                                                             │
│  BY MIRROR                                                                 │
│  ═══════════════════════════════════════════════════════════════════════   │
│                                                                             │
│  abc123.onion   ████████████████████  42%  19,185 requests                 │
│  def456.onion   █████████████████     38%  17,358 requests                 │
│  ghi789.onion   █████████             20%   9,135 requests                 │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `?` | Show help |
| `s` | Go to Sessions |
| `n` | Go to Nodes |
| `m` | Go to Mirrors |
| `b` | Go to Behavioral |
| `c` | Go to Captcha |
| `t` | Go to Traffic |
| `r` | Refresh page |

---

## API Access

The admin panel exposes these API endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/ctrl_.../api/sessions` | GET | List all sessions |
| `/ctrl_.../api/sessions/{id}` | GET | Get session detail |
| `/ctrl_.../api/sessions/{id}/action` | POST | Session action |
| `/ctrl_.../api/nodes` | GET | List all nodes |
| `/ctrl_.../api/mirrors` | GET | List all mirrors |
| `/ctrl_.../api/mirrors/{id}/action` | POST | Mirror action |
| `/ctrl_.../api/behavioral/config` | GET/POST | Behavioral config |
| `/ctrl_.../api/captcha/config` | GET/POST | Captcha config |
| `/ctrl_.../api/stats` | GET | Global statistics |

---

*All pages work without JavaScript - Pure HTML forms*
