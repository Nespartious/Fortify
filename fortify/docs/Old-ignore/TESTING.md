# Fortify Testing Guide

Comprehensive testing documentation for the Fortify security system.

## Overview

Fortify uses a multi-layered testing approach:
- **Unit Tests**: Component-level validation (inline with code)
- **Integration Tests**: Component interaction validation
- **End-to-End Tests**: Full flow validation
- **Security Tests**: Security invariant verification

## Running Tests

### All Tests
```bash
# Run all unit tests
cargo test

# Run all integration tests
cargo test --test '*'

# Run with verbose output
cargo test -- --nocapture

# Run with logging
RUST_LOG=debug cargo test
```

### Specific Test Suites
```bash
# Component integration tests
cargo test --test integration_test

# Trust flow tests
cargo test --test trust_flow_test

# Security invariant tests
cargo test --test security_test
```

### Individual Tests
```bash
# Run specific test
cargo test test_gate_to_proxy_token_flow

# Run tests matching pattern
cargo test token_
```

## Test Categories

### 1. Unit Tests (Inline)

Located in each crate's `src/` files with `#[cfg(test)]` modules.

**fortify-core** (15+ tests):
- Trust tier transitions
- Token signing and verification
- Session management (create, update, delete)
- HMAC-SHA256 operations
- Base64 encoding/decoding

**fortify-gate** (10+ tests):
- Captcha generation and validation
- Proof-of-work challenge creation
- Verification state machine
- Rate limiting
- Queue management

**fortify-http** (15+ tests):
- Token validation middleware
- Routing strategies (RoundRobin, LeastConnections, WeightedRandom)
- Backpressure control
- Request caps and guards
- Header filtering

**fortify-orchestrator** (20+ tests):
- Mirror state machine
- Tor integration
- Compromise detection
- Rotation strategies
- Burn and replace logic

**fortify-node** (15+ tests):
- Node modes (Healthy/Threat)
- Violation detection
- Session reclassification
- Path validation
- Behavioral analysis

**fortify-controller** (20+ tests):
- Resource monitoring
- Service lifecycle
- Scaling decisions
- Configuration validation

**fortify-community** (25+ tests):
- Cryptographic signatures
- Seed management
- Discovery protocol
- Rate limiting

### 2. Integration Tests

Located in `tests/integration_test.rs` - validates component interactions.

**Key Scenarios:**
- Gate → Proxy token flow
- Session lifecycle management
- Token signature verification
- Node violation detection
- Session demotion on violations
- Routing strategy selection
- Gate verification state machine
- Path traversal detection
- Script injection detection
- Oversized request detection

### 3. Trust Flow Tests

Located in `tests/trust_flow_test.rs` - validates trust tier progression.

**Key Scenarios:**
- Complete tier progression (Unknown → Verified → Trusted)
- Tier demotion sequence
- Burned session handling
- Token round-trip (encode/decode)
- Token tampering detection
- Concurrent session access
- Session cleanup
- Multiple tier transitions
- Session isolation

### 4. Security Tests

Located in `tests/security_test.rs` - validates security invariants.

**Critical Invariants:**
- Discovery never grants automatic trust
- All requests require valid sessions
- Forged tokens are rejected
- Rate limiting prevents abuse
- Path traversal is blocked
- Injection attacks are prevented
- Seed signatures are verified
- Oversized requests are rejected
- Metrics cannot be forged

## Test Coverage by Component

### Core Logic (fortify-core)
- ✅ Trust tier enum and transitions
- ✅ HMAC-SHA256 signing
- ✅ Token encoding/decoding
- ✅ Session CRUD operations
- ✅ Thread-safe access

### Gate (fortify-gate)
- ✅ Captcha generation
- ✅ PoW challenge creation
- ✅ Verification flow
- ✅ Rate limiting
- ✅ State machine

### HTTP Proxy (fortify-http)
- ✅ Token validation
- ✅ Request forwarding
- ✅ Routing strategies
- ✅ Backpressure control
- ✅ Metrics tracking

### Orchestrators (fortify-orchestrator)
- ✅ Mirror lifecycle
- ✅ Tor integration
- ✅ Compromise detection
- ✅ Rotation logic
- ✅ Burn/replace

### Nodes (fortify-node)
- ✅ Healthy/Threat modes
- ✅ Violation detection
- ✅ Session reclassification
- ✅ Path validation
- ✅ Behavioral analysis

### Controller (fortify-controller)
- ✅ Resource monitoring
- ✅ Service spawning
- ✅ Health checking
- ✅ Auto-scaling
- ✅ Configuration

### Community (fortify-community)
- ✅ Ed25519 signatures
- ✅ Seed management
- ✅ Discovery protocol
- ✅ Rate limiting
- ✅ Expiration

## Expected Results

### Successful Test Run
All tests should pass with output similar to:
```
running 100+ tests
test fortify_core::tests::test_trust_tier ... ok
test fortify_gate::tests::test_captcha ... ok
test fortify_http::tests::test_token_validation ... ok
...
test result: ok. 100+ passed; 0 failed
```

### Test Metrics
- **Total Tests**: 120+ across all components
- **Unit Tests**: 95+ inline tests
- **Integration Tests**: 15+ interaction tests
- **Trust Flow Tests**: 10+ progression tests
- **Security Tests**: 10+ invariant tests

## Continuous Integration

For CI/CD pipelines:
```bash
#!/bin/bash
# Run all tests with coverage
cargo test --all-features
cargo test --test '*'

# Check compilation
cargo check --all-features

# Linting
cargo clippy -- -D warnings

# Formatting
cargo fmt -- --check
```

## Known Limitations

1. **No Network Tests**: Integration tests don't start actual servers (would require available ports)
2. **No Tor Tests**: Tor integration tests are simulated (would require Tor daemon)
3. **No Load Tests**: Performance testing requires separate infrastructure
4. **No Fuzzing**: Fuzz testing would catch additional edge cases

## Troubleshooting

### Tests Hang
- Check for deadlocks in async code
- Verify tokio runtime initialization
- Use `--test-threads=1` to isolate issues

### Tests Fail Intermittently
- Check for race conditions
- Verify proper use of `Arc<Mutex<>>`
- Ensure cleanup between tests

### Compilation Errors
- Run `cargo clean`
- Check dependency versions
- Verify workspace configuration

## Security Test Checklist

Before deployment, verify:
- ✅ All tokens require valid signatures
- ✅ Burned sessions cannot access system
- ✅ Rate limiting prevents abuse
- ✅ Path traversal is blocked
- ✅ Injection attacks are prevented
- ✅ Community discovery never bypasses Gate
- ✅ Sessions properly isolated
- ✅ Metrics are read-only

## Contributing Tests

When adding new features:
1. Add unit tests inline with code
2. Add integration tests for interactions
3. Add security tests for invariants
4. Update this documentation
5. Verify all tests pass before PR

## Performance Benchmarks

Not included in test suite but important:
- Gate verification: <5 seconds
- Token validation: <10ms
- Proxy forwarding: <50ms
- Node processing: <20ms
- Scaling decision: <100ms

## Conclusion

Comprehensive testing ensures Fortify's security guarantees hold across all components and scenarios. Run all tests before deployment and after any changes.
