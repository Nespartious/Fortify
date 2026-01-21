# Integration Quick Start Guide

This guide shows how to wire up the defensive features in your Controller startup code.

## Overview

The implementation uses callback functions to connect components:
1. **Nodes → Controller**: Report demotions for blacklist tracking
2. **Controller → HTTP Proxy**: Check if sessions are blacklisted

## Step 1: Controller Modifications

### Location
`crates/fortify-controller/src/lib.rs` - in the `start()` or initialization method

### Code to Add

```rust
use std::sync::Arc;

impl Controller {
    pub async fn start(&self) -> Result<(), ControllerError> {
        // ... existing service startup code ...
        
        // STEP 1: Connect Node Demotion Callbacks
        // This allows nodes to report demotions to the controller's blacklist
        let self_arc = Arc::new(self.clone()); // Or however you get Arc<Self>
        
        for node in &self.nodes {
            let controller_ref = Arc::clone(&self_arc);
            node.lock().await.set_demotion_callback(move |session_id, demotion_count| {
                controller_ref.add_to_blacklist(session_id, demotion_count);
                tracing::info!("Node reported demotion for session (count: {})", demotion_count);
            });
        }
        
        // STEP 2: Connect HTTP Proxy Blacklist Check
        // This allows the proxy to check if sessions are blacklisted before routing
        let controller_ref = Arc::clone(&self_arc);
        self.http_proxy.lock().await.set_blacklist_check(move |session_id| {
            controller_ref.is_blacklisted(session_id)
        });
        
        // ... continue with existing start_monitoring(), etc ...
        self.start_monitoring().await;
        
        Ok(())
    }
}
```

## Alternative: Lazy Initialization

If you can't easily get `Arc<Self>` in the start method, you can defer callback setup:

```rust
impl Controller {
    pub fn set_callbacks(&self) {
        // Get Arc reference however your codebase handles it
        let controller_arc = /* your Arc<Controller> here */;
        
        // Node callbacks
        for node in &self.nodes {
            let ctrl = Arc::clone(&controller_arc);
            node.lock().unwrap().set_demotion_callback(move |sid, count| {
                ctrl.add_to_blacklist(sid, count);
            });
        }
        
        // Proxy callback
        let ctrl = Arc::clone(&controller_arc);
        self.http_proxy.lock().unwrap().set_blacklist_check(move |sid| {
            ctrl.is_blacklisted(sid)
        });
    }
}

// Then call it after Controller is constructed:
let controller = Arc::new(Controller::new(...));
controller.set_callbacks();
controller.start().await?;
```

## Step 2: Verify Blacklist Cleanup is Running

The blacklist cleanup task should already be running if you completed Phase 3. Check your logs for:

```
[DEBUG] Cleaned blacklist: 150 -> 120 entries
[WARN] Blacklist exceeded 10K entries, removed oldest 2000
```

If you don't see these, verify `start_monitoring()` is being called in your `start()` method.

## Step 3: Testing the Integration

### Test 1: Demotion Reporting
```bash
# Cause a session to be demoted (3 violations)
# Check logs for:
[WARN] Session abc-123 reached violation threshold, demoting
[INFO] Node reported demotion for session (count: 1)
[INFO] Session abc-123 blacklisted for 60 seconds (demotion #1)
```

### Test 2: Blacklist Check
```bash
# Attempt to use a blacklisted session token
# Check logs for:
[WARN] Session abc-123 is blacklisted, redirecting to gate
```

### Test 3: Blacklist Cleanup
```bash
# Wait 60 seconds after demotion
# Check logs for:
[DEBUG] Cleaned blacklist: 1 -> 0 entries
```

## Common Issues

### Issue 1: "demotion_callback is None"
**Cause:** Callbacks not set during initialization  
**Fix:** Ensure `set_demotion_callback()` is called for each node

### Issue 2: "blacklist_check is None"
**Cause:** Proxy callback not set  
**Fix:** Ensure `set_blacklist_check()` is called for the proxy

### Issue 3: Compilation errors with Arc<Self>
**Cause:** Controller might not implement Clone or might need different Arc handling  
**Fix:** Use alternative lazy initialization pattern (see above)

### Issue 4: Blacklist never cleans up
**Cause:** `start_monitoring()` not running  
**Fix:** Verify `self.start_monitoring().await` is called in `start()`

## Architecture Diagram

```
┌─────────────┐
│   Node      │────(violation)───> record_violation()
│             │                           │
│             │                           │ (3 violations)
│             │                           ▼
│             │                    check_demotion()
│             │                           │
│             │                           │ (demotion_callback)
│             │                           ▼
└─────────────┘              ┌────────────────────────┐
                             │  Controller            │
                             │  - session_blacklist   │
                             │  - add_to_blacklist()  │
       ┌─────────────────────┤  - is_blacklisted()    │
       │                     │  - cleanup_blacklist() │
       │                     └────────────────────────┘
       │                                  ▲
       │ (blacklist_check)                │ (every 30s cleanup)
       │                                  │
       ▼                                  ▼
┌─────────────┐                    [Monitoring Loop]
│ HTTP Proxy  │
│             │───(valid token)──> is_blacklisted?
│             │                           │
│             │                    No ────┤
│             │                           │ Yes
│             │                           ▼
│             │                    redirect_to_gate()
└─────────────┘
```

## Verification Checklist

- [ ] Callbacks set for all nodes in Controller.start()
- [ ] Callback set for HTTP proxy in Controller.start()
- [ ] start_monitoring() running (check for cleanup logs)
- [ ] Test demotion triggers blacklist entry (check logs)
- [ ] Test blacklisted session gets redirected (check logs)
- [ ] Test blacklist cleanup after 60 seconds (check logs)
- [ ] Full build succeeds: `cargo build --release`

## Next Steps

Once integration is complete:
1. Run Attack #5 simulation
2. Verify CPU usage drops from 348% → ~90%
3. Verify real user response time <5 seconds
4. Monitor blacklist size stays under 10,000 entries

## Support

If you encounter integration issues:
1. Check logs for callback invocation messages
2. Verify Arc references are valid
3. Ensure mutex locks aren't deadlocking
4. Run unit tests: `cargo test`

---

**Quick Reference:**
- Node callback: `node.set_demotion_callback(|sid, count| {...})`
- Proxy callback: `proxy.set_blacklist_check(|sid| {...})`
- Blacklist methods: `add_to_blacklist()`, `is_blacklisted()`, `cleanup_blacklist()`
