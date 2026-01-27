# Fortify Cluster Attack Defense Strategies

**Status:** Planning  
**Priority:** High  
**Related:** CLUSTER-GOSSIP-ARCHITECTURE.md

---

## Overview

This document details how Fortify clusters coordinate to absorb, deflect, and mitigate attacks using shared intelligence, resource-based load balancing, and automatic user routing. All strategies are designed for Tor Browser compatibility (no JS), high resilience, and minimal false positives.

---

## 1. Health-Based Auto-Routing

- **Mechanism:**
  - Each node shares health metrics (CPU, RAM, pool%) every 1-2 seconds via gossip.
  - Each node computes a "health score" (lower is better):
    - `score = weighted(cpu, ram, pool)`
  - If `my_score > 40` and another node is at least 5 points better, new users are 302-redirected to the healthiest node.
  - If all nodes are unhealthy, show a meta-refresh "queue" page.
- **Benefits:**
  - Fast, automatic, user-transparent
  - No JavaScript required
  - Prevents overload and spreads attack traffic

---

## 2. False Positive Prevention

- **Principle:** Only share and act on explicit, unambiguous signals:
  - Session bans (multiple failed CAPTCHAs, manual operator ban)
  - Node health metrics
  - Attack alerts (rate, not patterns)
- **No auto-blocking** based on heuristics or patterns—intelligence is shared, but each node acts independently.

---

## 3. Resource-Based Load Balancing

- **Metrics:**
  - CPU usage, RAM usage, CAPTCHA pool fill, requests/sec, blocks/sec
- **Routing Logic:**
  - If a node's health score > 40 and another node is 5+ points better, redirect new sessions to the healthier node.
  - Traffic split is proportional to health gap (not all-or-nothing).
  - Tank mode triggers if node is >90% load for >9 seconds: node redirects 90%+ of new traffic away, absorbs attack, and recovers when load drops.

---

## 4. Proof-of-Work (PoW) Live Adjustment

- **Default:** PoW difficulty is set to 20 (current default).
- **Escalation:**
  - When attack detected, new pages are generated at higher difficulty (e.g., 24).
  - Pool gradually rotates to higher difficulty as old pages are served.
  - Difficulty returns to normal as attack subsides.
- **Why not always high?**
  - Higher PoW = more user friction, slower solves, more support issues.
  - Default is tuned for best balance of security and usability.

---

## 5. Queue System (Pure HTML)

- **When triggered:** All nodes are at high load.
- **Behavior:**
  - User sees a static "queue" page with meta-refresh (no JS).
  - Queue position is cluster-wide.
  - When user's turn, server issues 302 redirect to CAPTCHA.
  - Optionally, user can solve a simple challenge (e.g., CAPTCHA) to move up in line (prevents bots from idling in queue).

---

## 6. Manual Overrides (TUI)

- **Operator controls:**
  - Force redirect all traffic away from this node
  - Manually set attack mode (tank, normal, etc.)
  - Force accept all traffic (bypass auto-redirects)
- **Purpose:**
  - Human-in-the-loop for emergencies, testing, or special events

---

## 7. Tank Mode

- **Integrated with load balancing:**
  - Node enters tank mode if >90% load for >9 seconds
  - Redirects most new sessions away, absorbs attack traffic
  - Exits tank mode when load normalizes

---

## Open Questions

- Should queue position be visible to user? (privacy vs. transparency)
- Should queue challenge always be a CAPTCHA, or can it be a simple math question?
- Should PoW difficulty be cluster-wide or node-local?

---

## Next Steps
- Finalize queue challenge design
- Implement health score calculation and routing logic
- Integrate TUI manual override controls
- Test PoW escalation and tank mode transitions
