# Sprint: Async Timeout Strategy

**Sprint ID:** BETA-001  
**Priority:** 🔴 CRITICAL (Beta Blocker)  
**Estimated Effort:** 2-3 days  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Objective

Implement comprehensive timeout handling across all async network operations to prevent slow-loris and resource exhaustion attacks.

## Success Criteria

- [ ] All async network calls have explicit timeout configuration
- [ ] Slow-loris simulation test passes (service remains responsive)
- [ ] No false positives for legitimate slow Tor users
- [ ] Graceful degradation (timeouts return 408, not crash)

---

## Timeout Values

| Operation Type | Timeout | Rationale |
|----------------|---------|-----------|
| TCP Connection (handshake) | 10s | Tor circuits establish quickly |
| Read (per chunk) | 30s | Accommodate Tor latency |
| Write (per flush) | 30s | Accommodate slow relays |
| Request (end-to-end) | 60s | Total request processing |
| Idle (keep-alive) | 300s | WebSocket/admin connections |
| Tor Control Operations | 15s | ADD_ONION, SIGNAL commands |
| Health Checks | 5s | Fast failure detection |

---

## Implementation Tasks

### Task 1: Audit Current Async Operations
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min

**Command to run:**
```bash
cd /home/shadowbox/Fortify/Fortify/fortify
grep -rn "\.await" crates/fortify-http/src/ crates/fortify-gate/src/ \
  crates/fortify-orchestrator/src/ crates/fortify-node/src/ \
  crates/fortify-controller/src/ | grep -v test | wc -l
```

**Deliverables:**
- [ ] List all `.await` calls needing timeout review
- [ ] Categorize by operation type (network I/O, computation, lock)
- [ ] Mark network I/O calls as HIGH PRIORITY

---

### Task 2: Tor Control Socket Timeouts
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**File:** `crates/fortify-orchestrator/src/tor.rs`

**Problem:** ADD_ONION commands can hang indefinitely if Tor is unresponsive.

**Current Code Pattern:**
```rust
match self.run_command(&mut stream, &pow_cmd) {
    Ok(response) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

**Required Change:**
```rust
use tokio::time::{timeout, Duration};

match timeout(
    Duration::from_secs(15),
    self.run_command_async(&mut stream, &pow_cmd)
).await {
    Ok(Ok(response)) => { /* success */ }
    Ok(Err(e)) => { /* command error */ }
    Err(_) => {
        tracing::error!("Tor control command timed out after 15s");
        return Err(OrchestratorError::TorTimeout);
    }
}
```

**Sub-tasks:**
- [ ] Convert `run_command()` to async (currently blocking I/O)
- [ ] Add `TorTimeout` error variant to `OrchestratorError`
- [ ] Wrap ADD_ONION calls (15s timeout)
- [ ] Wrap SIGNAL RELOAD calls (10s timeout)
- [ ] Wrap DEL_ONION calls (10s timeout)

**Test Command:**
```bash
# Start Tor, pause it, verify timeout fires
PID=$(pgrep tor)
sudo kill -STOP $PID
./target/debug/fortify-orchestrator  # Should timeout after 15s
sudo kill -CONT $PID
```

---

### Task 3: HTTP Server Request Timeout
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour  
**File:** `crates/fortify-http/src/lib.rs`

**Current:** Using default hyper 1.x settings.

**Required:** Add explicit request timeout configuration.

```rust
use hyper::server::conn::http1;

// In connection handling loop
http1::Builder::new()
    .header_read_timeout(Duration::from_secs(30))
    .max_buf_size(16 * 1024)  // 16KB max header size
    .serve_connection(io, service)
    .await?;
```

**Sub-tasks:**
- [ ] Add header read timeout (30s)
- [ ] Add header size limit (16KB - prevent bloat attacks)
- [ ] Test with slow HTTP client
- [ ] Verify legitimate Tor users not affected

---

### Task 4: Backend Proxy Timeout
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour  
**File:** `crates/fortify-http/src/proxy.rs` or routing.rs

**Current Code Pattern:**
```rust
let response = client.request(backend_req).await?;
```

**Required Change:**
```rust
let response = timeout(
    Duration::from_secs(60),
    client.request(backend_req)
).await
    .map_err(|_| ProxyError::BackendTimeout)?
    .map_err(ProxyError::Backend)?;
```

**Sub-tasks:**
- [ ] Add 60s end-to-end timeout
- [ ] Return 504 Gateway Timeout on timeout
- [ ] Log timeout events for monitoring

---

### Task 5: Orchestrator API Timeouts
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min  
**Files:** Controller/Admin API calls to orchestrator

**Required Change:**
```rust
let response = timeout(
    Duration::from_secs(10),
    client.get(format!("{}/mirrors", orchestrator_url)).send()
).await
    .map_err(|_| Error::OrchestratorTimeout)?
    .map_err(Error::Network)?;
```

**Sub-tasks:**
- [ ] Find all orchestrator API client calls
- [ ] Add 10s timeout (internal API should be fast)
- [ ] Return clear error message to admin panel

---

### Task 6: WebSocket Idle Timeout
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour  
**File:** `crates/fortify-http/src/admin.rs`

**Required:** Add ping/pong heartbeat with idle timeout.

```rust
let mut interval = tokio::time::interval(Duration::from_secs(60));

loop {
    tokio::select! {
        _ = interval.tick() => {
            // Send ping every 60s
            if ws_sender.send(Message::Ping(vec![])).await.is_err() {
                break; // Connection dead
            }
        }
        _ = tokio::time::sleep(Duration::from_secs(300)) => {
            tracing::warn!("WebSocket idle timeout");
            break;
        }
        msg = ws_receiver.next() => {
            // Handle message
        }
    }
}
```

**Sub-tasks:**
- [ ] Implement ping/pong heartbeat (60s interval)
- [ ] Close idle connections after 5 minutes
- [ ] Log idle timeout events

---

### Task 7: Create Test Suite
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour  
**File:** `tests/timeout_test.py`

**Slow-loris simulation script:**
```python
#!/usr/bin/env python3
import socket, time
from concurrent.futures import ThreadPoolExecutor

def slow_request(host, port, delay):
    s = socket.create_connection((host, port), timeout=120)
    s.send(b"GET / HTTP/1.1\r\n")
    time.sleep(delay)
    s.send(b"Host: test.onion\r\n")
    time.sleep(delay)
    # Never send final \r\n - keep connection open
    time.sleep(120)

# Run 50 slow connections simultaneously
with ThreadPoolExecutor(max_workers=50) as executor:
    futures = [executor.submit(slow_request, "localhost", 8082, 5.0) for _ in range(50)]
```

**Sub-tasks:**
- [ ] Create slow-loris simulation script
- [ ] Create integration tests for each timeout scenario
- [ ] Verify service remains responsive during attack
- [ ] Add to CI/CD pipeline

---

### Task 8: Documentation
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min

**Deliverables:**
- [ ] Document all timeout values and rationale
- [ ] Create tuning guidelines for different deployment scenarios
- [ ] Add environment variable configuration options:
  - `FORTIFY_TOR_TIMEOUT` (default: 15)
  - `FORTIFY_REQUEST_TIMEOUT` (default: 60)
  - `FORTIFY_WEBSOCKET_IDLE` (default: 300)

---

## Completion Checklist

- [ ] All Phase 1-6 tasks complete
- [ ] Slow-loris simulation test passes
- [ ] No false positives with real Tor traffic
- [ ] All timeouts logged for monitoring
- [ ] Documentation complete
- [ ] CI tests passing

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| False positives for slow Tor | Low | Medium | Generous timeout values, monitoring |
| Breaking changes to API | Low | Low | Timeouts add error cases, not change behavior |
| Test coverage gaps | Medium | Medium | Add integration tests for each timeout |

---

## References

- [hyper 1.x Timeout Configuration](https://docs.rs/hyper/latest/hyper/server/conn/http1/struct.Builder.html)
- [tokio::time::timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)
- Previous document: `security-hardening/01-timeout-strategy.md` (archived)
