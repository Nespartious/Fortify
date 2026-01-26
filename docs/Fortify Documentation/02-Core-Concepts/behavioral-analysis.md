# 📊 Behavioral Analysis Engine

> **Intelligent Bot Detection Without JavaScript or Fingerprinting**

---

## Overview

Fortify's behavioral analysis engine detects suspicious patterns in request behavior without requiring:
- ❌ JavaScript execution
- ❌ Browser fingerprinting
- ❌ Third-party services
- ❌ Client-side code

This makes it fully compatible with **Tor Browser "Safest" mode**.

---

## Violation Types

### Attack Path Access (Severity: 3)

Detects access to known malicious or sensitive paths.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        ATTACK PATH PATTERNS                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CATEGORY         PATTERN              DESCRIPTION                         │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Traversal        ../                  Path traversal attempt              │
│                   ..\\                 Windows traversal                   │
│                                                                             │
│  Config Files     /.env                Environment file                    │
│                   /.git                Git directory                       │
│                   /.svn                SVN directory                       │
│                   /.htaccess           Apache config                       │
│                   /.htpasswd           Password file                       │
│                   /config.             Config files                        │
│                                                                             │
│  CMS Probing      /wp-admin            WordPress admin                     │
│                   /wp-login            WordPress login                     │
│                   /wp-content          WordPress content                   │
│                   /phpmyadmin          Database admin                      │
│                   /admin               Admin panel                         │
│                   /administrator       Joomla admin                        │
│                                                                             │
│  Sensitive        /backup              Backup files                        │
│                   /.sql                SQL dumps                           │
│                   /dump                Data dumps                          │
│                                                                             │
│  Debug            /debug               Debug endpoints                     │
│                   /test                Test endpoints                      │
│                   /phpinfo             PHP info                            │
│                   /server-status       Apache status                       │
│                                                                             │
│  Exploit          /shell               Shell access                        │
│                   /cmd                 Command execution                   │
│                   /eval                Code evaluation                     │
│                   /exec                Execution attempt                   │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Suspicious User-Agent (Severity: 2)

Detects non-browser User-Agent strings.

**Bot Patterns Detected:**
```
curl, wget, python-requests, python-urllib, httpie,
scrapy, bot, crawler, spider, scraper,
googlebot, bingbot, yandex, baidu, duckduck,
facebookexternalhit, twitterbot, linkedinbot,
java/, perl, ruby, go-http-client, axios,
node-fetch, undici, libwww, lwp-,
mechanize, httpclient, okhttp, apache-httpclient
```

**Note:** Missing User-Agent is NOT flagged (common in Tor safest mode)

### Suspicious Referer (Severity: 1)

Detects impossible/suspicious referer headers.

**Flagged Referers:**
- Search engines (google.com, bing.com, etc.)
- Social media (facebook.com, twitter.com)
- Injection attempts (`<script>`, `javascript:`, `data:`)

**Note:** Missing Referer is NOT flagged (normal for Tor)

### Path Enumeration (Severity: 2)

Detects sequential path scanning.

```
┌────────────────────────────────────────────────────────────────────────────┐
│                       PATH ENUMERATION DETECTION                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Request 1: /page1    ─┐                                                   │
│  Request 2: /page2    ─┤                                                   │
│  Request 3: /page3    ─┼─► Sequential pattern detected!                    │
│  Request 4: /page4    ─┤                                                   │
│  Request 5: /page5    ─┘                                                   │
│                                                                             │
│  Threshold: 5 sequential paths (configurable)                              │
│                                                                             │
│  Algorithm:                                                                │
│  1. Extract trailing numbers from paths                                    │
│  2. Check if numbers are sequential (±1)                                   │
│  3. Flag if count >= threshold                                             │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Resource Enumeration (Severity: 2)

Detects rapid unique path access (directory scanning).

**Default Threshold:** 60 unique paths per minute

**Detection Logic:**
```rust
// Clean timestamps older than 1 minute
path_timestamps.retain(|&ts| current_time - ts <= 60);

// Check rate
if path_timestamps.len() > max_unique_paths_per_minute {
    // Flag as enumeration
}
```

### Form Submission Flood (Severity: 2)

Detects rapid POST requests.

**Default Threshold:** 10 form submissions per minute

**Detection Logic:**
```rust
// Track POST timestamps
form_timestamps.push(current_time);

// Clean old timestamps
form_timestamps.retain(|&ts| current_time - ts <= 60);

// Check rate
if form_timestamps.len() > max_form_submissions_per_minute {
    // Flag as flood
}
```

### Oversized Payload (Severity: 1)

Detects abnormally large request bodies.

**Default Threshold:** 10MB

### Undersized Payload (Severity: 1)

Detects suspiciously empty POST requests.

**Trigger:** POST with 0-byte body

### Automated Behavior (Severity: 3)

Meta-detection based on accumulated patterns.

**Trigger Conditions:**
- 5+ total violations, OR
- Severity score >= 10

---

## Detection Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│                      BEHAVIORAL ANALYSIS FLOW                               │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Request arrives                                                          │
│        │                                                                    │
│        ▼                                                                    │
│   ┌────────────────────┐                                                   │
│   │  Extract Metadata  │                                                   │
│   │  - Path            │                                                   │
│   │  - Method          │                                                   │
│   │  - User-Agent      │                                                   │
│   │  - Referer         │                                                   │
│   │  - Content-Length  │                                                   │
│   └─────────┬──────────┘                                                   │
│             │                                                               │
│             ▼                                                               │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    PARALLEL ANALYSIS                                 │  │
│   ├─────────────────────────────────────────────────────────────────────┤  │
│   │                                                                      │  │
│   │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │  │
│   │  │  UA Analysis │  │ Referer Chk  │  │ Path Analysis│              │  │
│   │  │  (if enabled)│  │ (if enabled) │  │ (if enabled) │              │  │
│   │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │  │
│   │         │                 │                 │                       │  │
│   │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │  │
│   │  │ Enumeration  │  │ Form Flood   │  │ Payload Size │              │  │
│   │  │  Detection   │  │  Detection   │  │  Analysis    │              │  │
│   │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │  │
│   │         │                 │                 │                       │  │
│   └─────────┼─────────────────┼─────────────────┼───────────────────────┘  │
│             │                 │                 │                          │
│             └─────────────────┼─────────────────┘                          │
│                               ▼                                            │
│                    ┌───────────────────┐                                   │
│                    │ Collect Violations │                                  │
│                    └─────────┬─────────┘                                   │
│                              │                                             │
│                              ▼                                             │
│                    ┌───────────────────┐                                   │
│                    │  Record in Stats  │                                   │
│                    └─────────┬─────────┘                                   │
│                              │                                             │
│            ┌─────────────────┼─────────────────┐                          │
│            │                 │                 │                          │
│            ▼                 ▼                 ▼                          │
│     [No Violations]   [Below Threshold]   [Above Threshold]              │
│            │                 │                 │                          │
│            ▼                 ▼                 ▼                          │
│       Continue          Log Warning       DEMOTE SESSION                  │
│                                                │                          │
│                                                ▼                          │
│                                       Set fortify_demoted=1               │
│                                                │                          │
│                                                ▼                          │
│                                       Redirect to Gate                    │
│                                       (2 captchas required)               │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Statistics Tracking

### Per-Session Stats

```rust
pub struct BehaviorStats {
    pub requests_analyzed: u64,
    pub violations_by_type: HashMap<String, u64>,
    pub recent_violations: VecDeque<BehaviorViolation>,  // Last 50
    pub unique_paths_count: u64,
    pub form_submissions: u64,
    pub total_payload_bytes: u64,
    pub suspicious_ua_detected: bool,
    pub last_activity: u64,
}
```

### Global Stats

```rust
pub struct GlobalBehaviorStats {
    pub total_requests_analyzed: u64,
    pub total_violations: u64,
    pub violations_by_type: HashMap<String, u64>,
    pub sessions_flagged_automated: u64,
}
```

---

## Demotion Thresholds

### Method 1: Total Violations

```rust
if stats.total_violations() >= threat_demotion_threshold {
    demote_to_suspicious();
}
```

**Default:** 10 total violations

### Method 2: Severity Score

```rust
let score: u64 = recent_violations.iter().map(|v| v.severity as u64).sum();
if score >= threat_severity_threshold {
    demote_to_suspicious();
}
```

**Default:** 15 cumulative severity

### Method 3: Type-Specific Thresholds

```rust
for (violation_type, count) in &stats.violations_by_type {
    if let Some(&threshold) = thresholds.get(violation_type) {
        if count >= threshold {
            demote_to_suspicious();
            break;
        }
    }
}
```

**Defaults:**
| Violation Type | Threshold |
|----------------|-----------|
| Attack Path Access | 3 |
| Suspicious User-Agent | 5 |
| Path Enumeration | 3 |
| Resource Enumeration | 3 |
| Form Submission Flood | 3 |
| Automated Behavior | 2 |
| Suspicious Referer | 10 |
| Oversized Payload | 5 |
| Undersized Payload | 10 |

---

## Configuration

### Enable/Disable Features

```toml
[behavioral]
ua_analysis_enabled = true
referer_analysis_enabled = true
path_analysis_enabled = true
enumeration_detection_enabled = true
form_tracking_enabled = true
payload_analysis_enabled = true
```

### Custom Whitelists

```toml
# Paths that won't trigger attack path violations
custom_whitelist_paths = [
    "/api/*",      # API prefix (wildcard)
    "/static/*",   # Static files
    "/admin",      # If you have a legit /admin
]

# Disable specific attack patterns
disabled_attack_paths = [
    "/test",       # If you have a /test endpoint
    "/debug",      # If you have debug pages
]
```

### Rate Limits

```toml
max_unique_paths_per_minute = 60
max_form_submissions_per_minute = 10
max_payload_size = 10485760  # 10MB
min_post_payload_size = 1
sequential_path_threshold = 5
```

---

## Admin Panel Integration

### Behavioral Config Page

```
/ctrl_xxx/behavioral
```

**Features:**
- Toggle each detection feature
- Adjust thresholds
- View global statistics
- Enable/disable attack paths
- Manage whitelists

### Session Detail View

```
/ctrl_xxx/sessions/{session_id}
```

**Displays:**
- Violation history timeline
- Violation counts by type
- Severity score
- Demotion count
- Automated behavior flag

---

## Code Examples

### Analyzing a Request

```rust
use fortify_core::{BehaviorAnalyzer, BehaviorConfig, RequestMeta};

// Create analyzer with config
let config = BehaviorConfig::default();
let mut analyzer = BehaviorAnalyzer::new(config);

// Create request metadata
let req = RequestMeta::new(
    "/api/users".to_string(),
    "GET".to_string(),
    Some("Mozilla/5.0 (Windows NT 10.0; rv:102.0) Firefox/102.0".to_string()),
    None,
    0,
);

// Analyze and get violations
let violations = analyzer.analyze("session-123", &req);

// Check if should demote
let stats = analyzer.get_session_stats("session-123").unwrap();
if config.should_demote_to_threat(stats) {
    // Demote session
}
```

### Checking Whitelist

```rust
let config = BehaviorConfig::default();

// Check if path is whitelisted
if config.is_custom_whitelisted("/api/users") {
    // Skip attack path analysis
}

// Check if attack pattern is enabled
if config.is_attack_path_enabled("../") {
    // Check for path traversal
}
```

---

## False Positive Mitigation

### Normal Tor Browser Behavior

These are NOT flagged:
- Missing User-Agent (common in safest mode)
- Missing Referer (normal for direct navigation)
- Standard browsing patterns

### Tuning Recommendations

| Scenario | Adjustment |
|----------|------------|
| High false positives on UAs | Lower `ua_analysis_enabled` or increase threshold |
| API-heavy site | Add `/api/*` to whitelist |
| Many form submissions | Increase `max_form_submissions_per_minute` |
| Deep site structure | Increase `max_unique_paths_per_minute` |

---

## Testing Behavioral Analysis

### Test Demotion Script

```bash
#!/bin/bash
# scripts/test-demotion.sh

# Trigger path enumeration
for i in {1..10}; do
    curl -s "http://localhost:8082/page$i" -H "Cookie: fortify_session=$TOKEN"
done

# Trigger attack path
curl -s "http://localhost:8082/../etc/passwd" -H "Cookie: fortify_session=$TOKEN"

# Trigger bot UA
curl -s "http://localhost:8082/" -H "User-Agent: python-requests/2.28"
```

---

*See [Functions.md](../Functions.md) for complete API reference*
