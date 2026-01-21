# Integration Tests

This directory contains integration tests that validate interactions between Fortify components.

## Test Structure

- `integration_test.rs` - Component interaction tests (Gate → Proxy, Proxy → Node)
- `end_to_end_test.rs` - Full flow validation (Orchestrator → Gate → Proxy → Node → Backend)
- `trust_flow_test.rs` - Trust tier progression and session reclassification
- `security_test.rs` - Security invariant validation

## Running Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific test file
cargo test --test integration_test

# Run with logging
RUST_LOG=debug cargo test --test integration_test
```

## Test Coverage

### Component Integration
- Gate verification and token issuance
- Proxy token validation and request forwarding
- Node violation detection and session reclassification

### End-to-End Flow
- New user verification through Gate
- Token-based access through Proxy
- Request forwarding to backend service
- Session tier progression

### Security Invariants
- All requests pass through Gate first
- Community discovery never bypasses Gate
- Tokens cannot be forged
- Sessions demote on violations
- Sessions promote on good behavior

## Requirements

Tests require:
- Rust toolchain
- All Fortify crates compiled
- Test dependencies (tokio-test, hyper-test, etc.)
