# Critical Issue #1: Comprehensive Timeout Strategy

**Priority:** 🔴 CRITICAL (Beta Blocker)  
**Estimated Effort:** 2-3 days  
**Status:** Not Started

---

## Overview

**Problem:** Slow-loris attacks can exhaust connection pools by holding connections open indefinitely with slow reads/writes. This bypasses PoW defenses (which only protect the Tor introduction layer) and can make the entire service unresponsive.

**Goal:** Add explicit timeouts to all async network operations to prevent resource exhaustion from slow clients.

**Success Criteria:**
- [ ] All async network calls have explicit timeout configuration
- [ ] Timeout strategy documented with rationale for each value
- [ ] Slow-loris simulation test passes (service remains responsive)
- [ ] No false positives for legitimate slow Tor users during testing
- [ ] Graceful degradation (timeouts return 408 Request Timeout, not crash)

---

## Timeout Strategy

| Operation Type | Timeout Value | Rationale |
|----------------|---------------|-----------|
| TCP Connection (handshake) | 10 seconds | Tor circuits establish quickly; 10s is generous |
| Read (per chunk) | 30 seconds | Accommodate Tor latency variability |
| Write (per flush) | 30 seconds | Accommodate slow Tor relays |
| Request (end-to-end) | 60 seconds | Total request processing time |
| Idle (keep-alive) | 300 seconds | WebSocket/admin connections |
| Tor Control Operations | 15 seconds | ADD_ONION, DEL_ONION commands |
| Health Checks | 5 seconds | Fast failure detection |

---

## Implementation Steps

### Phase 1: Audit Current Async Operations

**Status:** ⬜ Not Started

**Task 1.1:** Identify all `.await` calls in network-facing crates

```bash
# Files to audit
crates/fortify-http/src/**/*.rs
crates/fortify-gate/src/**/*.rs
crates/fortify-orchestrator/src/**/*.rs
crates/fortify-node/src/**/*.rs
crates/fortify-controller/src/**/*.rs
```

**Command:**
```bash
cd /home/shadowbox/Fortify/Fortify/fortify
grep -rn "\.await" crates/fortify-http/src/ crates/fortify-gate/src/ crates/fortify-orchestrator/src/ crates/fortify-node/src/ crates/fortify-controller/src/ | grep -v test > /tmp/await_audit.txt
wc -l /tmp/await_audit.txt
```

**Expected Output:** List of all async operations needing timeout review

**Deliverable:** 
- [ ] Create `/tmp/await_audit.txt` with all async call locations
- [ ] Categorize each by operation type (network I/O, computation, lock, etc.)
- [ ] Mark network I/O calls as HIGH PRIORITY

---

**Task 1.2:** Check existing timeout implementations

**Files to review:**
- ✅ `crates/fortify-gate/src/main.rs` - GATE_VERIFICATION_TIMEOUT (45s default)
- ✅ `crates/fortify-controller/src/health.rs` - Health check timeout (30s)
- ✅ `crates/fortify-controller/src/mirror_health.rs` - Mirror health (2s)
- ✅ `crates/fortify-tui/src/mirror_health.rs` - TUI health check (30s)

**Command:**
```bash
grep -rn "timeout\|Duration::from_secs" crates/fortify-*/src/*.rs | grep -v test
```

**Deliverable:**
- [ ] Document existing timeout values in spreadsheet
- [ ] Identify gaps where timeouts are missing

---

### Phase 2: Tor Control Socket Timeouts (HIGHEST PRIORITY)

**Status:** ⬜ Not Started  
**Why Critical:** ADD_ONION commands during mirror creation can hang indefinitely if Tor is unresponsive

**Task 2.1:** Add timeout wrapper to `TorService::run_command()`

**File:** `crates/fortify-orchestrator/src/tor.rs`

**Location:** Around line 160-200 (in `create_via_control_port()` and related functions)

**Current Code Pattern:**
```rust
let pow_cmd = format!(
    "ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:{} Flags=Detach,PoWDefensesEnabled",
    target_port
);
match self.run_command(&mut stream, &pow_cmd) {
    Ok(response) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

**Required Change:**
```rust
use tokio::time::{timeout, Duration};

// Wrap run_command with timeout
match timeout(
    Duration::from_secs(15),
    self.run_command_async(&mut stream, &pow_cmd)
).await {
    Ok(Ok(response)) => { /* success */ }
    Ok(Err(e)) => { /* command error */ }
    Err(_) => {
        log::error!("Tor control command timed out after 15s");
        return Err(OrchestratorError::TorTimeout);
    }
}
```

**Steps:**
1. [ ] Convert `TorService::run_command()` to async (currently blocking I/O)
2. [ ] Add `tokio::time::timeout` wrapper around all Tor control calls
3. [ ] Add `TorTimeout` error variant to `OrchestratorError`
4. [ ] Test with unresponsive Tor daemon (kill -STOP on tor process)

**Files to modify:**
- [ ] `crates/fortify-orchestrator/src/tor.rs` - run_command() method
- [ ] `crates/fortify-orchestrator/src/lib.rs` - OrchestratorError enum

**Test Command:**
```bash
# Start Tor, then pause it
sudo systemctl start tor
PID=$(pgrep tor)
sudo kill -STOP $PID

# Try to create mirror (should timeout after 15s, not hang)
./target/debug/fortify-orchestrator

# Resume Tor
sudo kill -CONT $PID
```

---

**Task 2.2:** Add timeout to `SIGNAL RELOAD` command

**File:** `crates/fortify-orchestrator/src/tor.rs`

**Location:** Line ~290 (in `create_file_based_pow_service()`)

**Current Code:**
```rust
match self.run_command(&mut stream, "SIGNAL RELOAD") {
    Ok(_) => tracing::debug!("Tor reloaded configuration"),
    Err(e) => tracing::warn!("Failed to signal Tor reload: {}", e),
}
```

**Required Change:**
```rust
match timeout(
    Duration::from_secs(10),
    self.run_command_async(&mut stream, "SIGNAL RELOAD")
).await {
    Ok(Ok(_)) => tracing::debug!("Tor reloaded configuration"),
    Ok(Err(e)) => tracing::warn!("Failed to signal Tor reload: {}", e),
    Err(_) => tracing::warn!("Tor reload timed out after 10s"),
}
```

**Steps:**
- [ ] Apply same timeout pattern as Task 2.1
- [ ] Use 10s timeout (RELOAD is faster than ADD_ONION)
- [ ] Log timeout as warning (not fatal, mirror can still work)

---

### Phase 3: HTTP Proxy Timeouts

**Status:** ⬜ Not Started

**Task 3.1:** Add explicit request timeout to Hyper server

**File:** `crates/fortify-http/src/main.rs`

**Location:** Server builder configuration

**Current Code Pattern:**
```rust
let server = Server::bind(&addr)
    .serve(make_svc)
    .await?;
```

**Required Change:**
```rust
use hyper::server::conn::Http;

let server = Server::bind(&addr)
    .http1()
    .http1_header_read_timeout(Duration::from_secs(30))
    .http1_max_buf_size(16 * 1024) // 16KB max header size
    .serve(make_svc)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

**Steps:**
- [ ] Add header read timeout (30s per chunk)
- [ ] Add header size limit (prevent header bloat attacks)
- [ ] Test with slow HTTP client
- [ ] Verify legitimate Tor users not affected

**Test Command:**
```bash
# Slow HTTP request test
python3 << 'EOF'
import socket, time
s = socket.create_connection(("localhost", 8082))
s.send(b"GET / HTTP/1.1\r\n")
time.sleep(5)
s.send(b"Host: test.onion\r\n")
time.sleep(5)
s.send(b"\r\n")
# Should timeout after 30s total
EOF
```

---

**Task 3.2:** Add timeout to backend proxying

**File:** `crates/fortify-http/src/routing.rs` or `crates/fortify-http/src/proxy.rs`

**Location:** Where HTTP Proxy forwards requests to Healthy/Threat Nodes

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

**Steps:**
- [ ] Find backend proxying code
- [ ] Add 60s end-to-end timeout
- [ ] Return 504 Gateway Timeout on timeout
- [ ] Log timeout events for monitoring

**Note:** Even though connection from Node → Real Hidden Service is in "safe space", timeouts provide defense-in-depth and prevent resource exhaustion if backend becomes unresponsive.

---

### Phase 4: Orchestrator API Timeouts

**Status:** ⬜ Not Started

**Task 4.1:** Add timeout to orchestrator HTTP client calls

**File:** `crates/fortify-controller/src/orchestrator.rs` or similar

**Location:** Where Controller/Admin Panel makes API calls to Orchestrators

**Current Code Pattern:**
```rust
let response = client.get(format!("{}/mirrors", orchestrator_url))
    .send()
    .await?;
```

**Required Change:**
```rust
let response = timeout(
    Duration::from_secs(10),
    client.get(format!("{}/mirrors", orchestrator_url))
        .send()
).await
    .map_err(|_| Error::OrchestratorTimeout)?
    .map_err(Error::Network)?;
```

**Steps:**
- [ ] Audit all orchestrator API client calls
- [ ] Add 10s timeout (internal API should be fast)
- [ ] Return clear error message to admin panel
- [ ] Test with unreachable orchestrator

---

### Phase 5: WebSocket/Admin Panel Timeouts

**Status:** ⬜ Not Started

**Task 5.1:** Add idle timeout to admin panel WebSocket connections

**File:** `crates/fortify-http/src/admin.rs` or WebSocket handler

**Location:** WebSocket connection setup

**Required Change:**
```rust
let config = WebSocketConfig {
    max_frame_size: Some(1024 * 1024), // 1MB
    max_message_size: Some(1024 * 1024),
    max_write_buffer_size: 128 * 1024,
    accept_unmasked_frames: false,
};

// Add ping/pong heartbeat with 5-minute idle timeout
let (mut ws_sender, mut ws_receiver) = ws_stream.split();
let mut interval = tokio::time::interval(Duration::from_secs(60));

loop {
    tokio::select! {
        _ = interval.tick() => {
            // Send ping every 60s
            if ws_sender.send(Message::Ping(vec![])).await.is_err() {
                break; // Connection dead
            }
        }
        msg = ws_receiver.next() => {
            // Handle message or timeout after 300s idle
        }
        _ = tokio::time::sleep(Duration::from_secs(300)) => {
            log::warn!("WebSocket idle timeout");
            break;
        }
    }
}
```

**Steps:**
- [ ] Implement ping/pong heartbeat (60s interval)
- [ ] Close idle connections after 5 minutes
- [ ] Log idle timeout events
- [ ] Test with inactive admin panel tab

---

### Phase 6: Documentation

**Status:** ⬜ Not Started

**Task 6.1:** Create timeout configuration documentation

**File:** `docs/TIMEOUT_CONFIGURATION.md`

**Content Required:**
```markdown
# Timeout Configuration

## Environment Variables
- GATE_VERIFICATION_TIMEOUT (default: 45)
- HTTP_REQUEST_TIMEOUT (default: 60)
- TOR_CONTROL_TIMEOUT (default: 15)
- BACKEND_PROXY_TIMEOUT (default: 60)
- WEBSOCKET_IDLE_TIMEOUT (default: 300)

## Tuning Guidelines
- Increase timeouts if legitimate users report errors
- Monitor timeout events in logs
- Balance between DoS prevention and user experience
```

**Steps:**
- [ ] Document all timeout values and their rationale
- [ ] Create tuning guidelines for different deployment scenarios
- [ ] Add monitoring/alerting recommendations

---

**Task 6.2:** Update prof_review.md status

**File:** `docs/research/prof_review.md`

**Change:** Mark "Lack of Async Deadlines / Timeouts" section as `valid-addressed` after implementation

---

### Phase 7: Testing

**Status:** ⬜ Not Started

**Task 7.1:** Create slow-loris attack simulation

**File:** `tests/slowloris_test.py`

**Test Script:**
```python
#!/usr/bin/env python3
"""
Slow-loris attack simulation for Fortify timeout testing
Sends very slow HTTP requests to exhaust connection pool
"""
import socket
import time
import argparse
from concurrent.futures import ThreadPoolExecutor

def slow_request(target_host, target_port, delay):
    """Send one very slow HTTP request"""
    try:
        s = socket.create_connection((target_host, target_port), timeout=120)
        
        # Send headers very slowly
        s.send(b"GET / HTTP/1.1\r\n")
        time.sleep(delay)
        s.send(b"Host: test.onion\r\n")
        time.sleep(delay)
        s.send(b"User-Agent: SlowLoris\r\n")
        time.sleep(delay)
        s.send(b"Connection: keep-alive\r\n")
        time.sleep(delay)
        # Never send final \r\n - keep connection open
        
        # Wait for timeout
        time.sleep(120)
    except Exception as e:
        print(f"Connection failed (expected): {e}")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="localhost")
    parser.add_argument("--port", type=int, default=8082)
    parser.add_argument("--connections", type=int, default=100)
    parser.add_argument("--delay", type=float, default=5.0)
    args = parser.parse_args()
    
    print(f"Starting slow-loris attack simulation:")
    print(f"  Target: {args.host}:{args.port}")
    print(f"  Connections: {args.connections}")
    print(f"  Delay between chunks: {args.delay}s")
    print("Service should remain responsive and timeout connections after ~30s")
    
    with ThreadPoolExecutor(max_workers=args.connections) as executor:
        futures = [
            executor.submit(slow_request, args.host, args.port, args.delay)
            for _ in range(args.connections)
        ]
        
        # Wait for all to complete
        for f in futures:
            f.result()

if __name__ == "__main__":
    main()
```

**Steps:**
- [ ] Create test script
- [ ] Run against HTTP Proxy (should timeout connections, remain responsive)
- [ ] Verify legitimate requests still work during attack
- [ ] Monitor connection pool utilization

**Test Command:**
```bash
# Terminal 1: Start Fortify
./target/debug/fortify

# Terminal 2: Run slow-loris simulation
python3 tests/slowloris_test.py --connections 50

# Terminal 3: Verify service still responsive
while true; do
    curl -sS http://localhost:8082 -o /dev/null && echo "OK" || echo "FAIL"
    sleep 1
done
```

**Expected Result:**
- Slow connections timeout after 30-60 seconds
- New legitimate requests continue to be served
- Service does not exhaust connection pool
- No panics or crashes in logs

---

**Task 7.2:** Integration test for all timeout scenarios

**File:** `tests/timeout_integration_test.rs`

**Test Cases:**
```rust
#[tokio::test]
async fn test_tor_control_timeout() {
    // Simulate unresponsive Tor daemon
    // Verify ADD_ONION times out after 15s
}

#[tokio::test]
async fn test_http_request_timeout() {
    // Send very slow HTTP request
    // Verify timeout after 30s
}

#[tokio::test]
async fn test_backend_proxy_timeout() {
    // Mock slow backend server
    // Verify 504 Gateway Timeout after 60s
}

#[tokio::test]
async fn test_websocket_idle_timeout() {
    // Open WebSocket, send no data
    // Verify close after 300s idle
}
```

**Steps:**
- [ ] Write integration tests for each timeout scenario
- [ ] Use mock servers for controlled testing
- [ ] Verify timeouts fire at expected intervals
- [ ] Add to CI/CD pipeline

---

## Completion Checklist

**Phase 1: Audit**
- [ ] All `.await` calls cataloged
- [ ] Existing timeouts documented
- [ ] Gaps identified

**Phase 2: Tor Control**
- [ ] run_command() converted to async
- [ ] ADD_ONION timeout (15s)
- [ ] SIGNAL RELOAD timeout (10s)
- [ ] TorTimeout error added

**Phase 3: HTTP Proxy**
- [ ] Hyper server request timeout (60s)
- [ ] Header read timeout (30s)
- [ ] Backend proxy timeout (60s)

**Phase 4: Orchestrator API**
- [ ] Orchestrator client timeout (10s)

**Phase 5: WebSocket**
- [ ] WebSocket idle timeout (300s)
- [ ] Ping/pong heartbeat

**Phase 6: Documentation**
- [ ] TIMEOUT_CONFIGURATION.md created
- [ ] Environment variables documented
- [ ] Tuning guidelines written

**Phase 7: Testing**
- [ ] Slow-loris simulation script
- [ ] Integration tests written
- [ ] All tests passing

**Final Validation:**
- [ ] Service remains responsive under slow-loris attack
- [ ] No false positives for legitimate Tor users
- [ ] All timeouts logged for monitoring
- [ ] Documentation complete
- [ ] Ready for Beta release

---

## Questions to Answer

1. **Q:** Should backend proxy timeout be lower since it's in "safe space"?
   **A:** Keep it at 60s for defense-in-depth. If backend becomes unresponsive for any reason, we don't want proxies to hang indefinitely.

2. **Q:** What if legitimate Tor users have slower connections than our timeouts?
   **A:** Our timeouts (30s read, 60s request) are very generous for Tor. Monitor timeout events in production and adjust if false positives occur.

3. **Q:** Should we make all timeouts configurable via environment variables?
   **A:** Yes, for operational flexibility. Default values should work for 95% of deployments.

4. **Q:** How do we handle cascading timeouts (request timeout vs read timeout)?
   **A:** Request timeout is outer bound (60s total), read timeout is per-chunk (30s). First to fire wins.

---

## Risk Assessment

**Implementation Risks:** 🟢 LOW
- Timeouts are defensive, non-breaking changes
- Can be tuned per deployment
- Existing code mostly unchanged (wrapping with timeout())

**False Positive Risk:** 🟡 MEDIUM
- Very slow Tor users might get timed out
- Mitigation: Generous timeout values, monitoring, configurability

**Security Impact:** 🟢 HIGH POSITIVE
- Prevents slow-loris DoS attacks
- Prevents resource exhaustion
- Complements existing rate limiting and PoW defenses

**Operational Impact:** 🟢 POSITIVE
- Better resource utilization
- Faster failure detection
- Clearer error messages

---

**Status Legend:**
- ⬜ Not Started
- 🟦 In Progress  
- ✅ Complete
- ⚠️ Blocked
