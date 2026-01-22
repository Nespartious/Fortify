# Sprint: TUI Deployment Wizard Completion

**Sprint ID:** FEAT-001  
**Priority:** 🟡 MEDIUM  
**Estimated Effort:** 3-5 days  
**Status:** ⬜ Not Started (40% Complete Overall)  
**Created:** January 22, 2026

---

## Objective

Complete the remaining 60% of the TUI deployment wizard to enable fully interactive deployment without manual configuration.

## Current State (40% Complete)

### ✅ Completed
- Core framework, keyboard events, focus management
- Configuration system with TOML serialization
- Views: Home, deployment wizard (7 steps), settings, status
- Settings tabs: Branding, CAPTCHA, Thresholds, Network, Mirrors, Vanity
- Dialogs: Confirm, apply changes, text input, error, info
- Log panel with filtering (5000 entries, level filtering, pause/resume)
- Vanity generation: Prefix-only matching, mkp224o integration
- Mirror status: Display with colored indicators
- Deployment manager: State management, process control

### ❌ Remaining
- Progressive prefix reduction on timeout
- Self-verification of .onion addresses
- Auto-update status from orchestrator
- Integration with fortify-controller
- End-to-end deployment workflow testing

---

## Implementation Tasks

### Task 1: Progressive Prefix Reduction
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**File:** `crates/fortify-tui/src/vanity.rs`

**Problem:** If vanity generation takes too long, user is stuck waiting.

**Solution:** Automatically reduce prefix length after timeout.

```rust
pub struct VanityConfig {
    pub prefix: String,
    pub timeout_seconds: u64,
    pub min_prefix_length: usize,
}

async fn generate_vanity_with_fallback(config: VanityConfig) -> Result<String> {
    let mut current_prefix = config.prefix.clone();
    
    loop {
        match timeout(
            Duration::from_secs(config.timeout_seconds),
            generate_vanity(&current_prefix)
        ).await {
            Ok(Ok(address)) => return Ok(address),
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timeout - reduce prefix
                if current_prefix.len() <= config.min_prefix_length {
                    // Give up on vanity, use random
                    return generate_random_address();
                }
                current_prefix.pop();
                log::info!("Reducing vanity prefix to: {}", current_prefix);
            }
        }
    }
}
```

**Sub-tasks:**
- [ ] Implement timeout wrapper for vanity generation
- [ ] Add progress UI showing current prefix attempt
- [ ] Add user prompt: "Reduce prefix or continue waiting?"
- [ ] Log prefix reduction events

---

### Task 2: Self-Verification of .onion Addresses
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**File:** `crates/fortify-tui/src/verification.rs`

**Problem:** Generated .onion addresses may not be reachable.

**Solution:** Verify address is accessible via Tor before marking complete.

```rust
async fn verify_onion_address(address: &str, tor_proxy: &str) -> Result<bool> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(tor_proxy)?)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    match client.get(format!("http://{}", address)).send().await {
        Ok(resp) => {
            if resp.status().is_success() || resp.status() == 302 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(_) => Ok(false)
    }
}
```

**Sub-tasks:**
- [ ] Implement verification function
- [ ] Add retry logic (3 attempts with backoff)
- [ ] Show verification status in UI
- [ ] Handle verification failure gracefully

---

### Task 3: Auto-Update Status from Orchestrator
**Status:** ⬜ Not Started  
**Estimated Time:** 3 hours  
**File:** `crates/fortify-tui/src/status.rs`

**Problem:** TUI doesn't reflect real-time system state.

**Solution:** Poll orchestrator API for current status.

```rust
async fn poll_orchestrator_status(
    orchestrator_url: &str,
    auth_token: &str,
    interval: Duration,
) -> Result<SystemStatus> {
    loop {
        let status = reqwest::Client::new()
            .get(format!("{}/status", orchestrator_url))
            .header("X-Fortify-Admin-Token", auth_token)
            .send()
            .await?
            .json::<SystemStatus>()
            .await?;
        
        // Update UI
        tx.send(Message::StatusUpdate(status))?;
        
        tokio::time::sleep(interval).await;
    }
}
```

**Sub-tasks:**
- [ ] Create status polling task
- [ ] Parse orchestrator /status endpoint
- [ ] Update TUI views with real-time data
- [ ] Handle connection failures gracefully
- [ ] Add connection status indicator

---

### Task 4: Integration with fortify-controller
**Status:** ⬜ Not Started  
**Estimated Time:** 4 hours  
**File:** `crates/fortify-tui/src/controller.rs`

**Problem:** TUI can't manage running Fortify system.

**Solution:** Add controller integration for runtime management.

**Features to Add:**
- [ ] Start/stop fortify-controller from TUI
- [ ] View controller logs in real-time
- [ ] Send commands to controller (burn mirror, pause node, etc.)
- [ ] Display controller health status

```rust
pub struct ControllerClient {
    pub url: String,
    pub auth_token: String,
}

impl ControllerClient {
    async fn burn_mirror(&self, mirror_id: &str) -> Result<()> {
        self.post("/mirror/burn", json!({ "id": mirror_id })).await
    }
    
    async fn pause_node(&self, node_id: &str) -> Result<()> {
        self.post("/node/pause", json!({ "id": node_id })).await
    }
    
    async fn get_health(&self) -> Result<HealthStatus> {
        self.get("/health").await
    }
}
```

---

### Task 5: End-to-End Deployment Testing
**Status:** ⬜ Not Started  
**Estimated Time:** 4 hours

**Test Scenarios:**

1. **Fresh Deployment**
   - [ ] Start TUI on clean system
   - [ ] Complete wizard (all 7 steps)
   - [ ] Verify Tor starts
   - [ ] Verify mirrors created
   - [ ] Verify service accessible

2. **Configuration Change**
   - [ ] Modify settings
   - [ ] Apply changes
   - [ ] Verify hot-reload works
   - [ ] Verify no service interruption

3. **Mirror Management**
   - [ ] Create new mirror via TUI
   - [ ] Pause mirror via TUI
   - [ ] Resume mirror via TUI
   - [ ] Destroy mirror via TUI

4. **Error Handling**
   - [ ] Test with Tor unavailable
   - [ ] Test with invalid config
   - [ ] Test with port conflicts
   - [ ] Verify graceful error messages

**Test Command:**
```bash
# Run TUI in test mode
./target/release/fortify --test-mode

# Or with specific config
./target/release/fortify --config test-config.toml
```

---

### Task 6: User Documentation
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

**Create:** `docs/TUI-GUIDE.md`

**Sections:**
- [ ] Quick start
- [ ] Wizard walkthrough (with screenshots)
- [ ] Keyboard shortcuts reference
- [ ] Troubleshooting common issues
- [ ] Configuration file reference

---

## Completion Checklist

- [ ] Progressive prefix reduction implemented
- [ ] Onion address self-verification working
- [ ] Auto-update from orchestrator functional
- [ ] Controller integration complete
- [ ] All 4 E2E test scenarios passing
- [ ] User documentation complete

---

## Dependencies

- Requires Tor to be running for verification
- Requires orchestrator API to be accessible
- Requires controller to be running for management

---

## References

- Current TUI code: `crates/fortify-tui/`
- Orchestrator API: `crates/fortify-orchestrator/src/server.rs`
- Controller API: `crates/fortify-controller/src/http.rs`
