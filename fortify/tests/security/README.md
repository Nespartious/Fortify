# Security Tests

Security-focused tests for Fortify.

## Test Categories

- **Token validation**: Test token forgery detection
- **Rate limiting**: Test rate limit enforcement
- **Input validation**: Test malicious input handling
- **Bypass attempts**: Test gate bypass prevention
- **Resource exhaustion**: Test DoS resistance

## Running Security Tests

```bash
cargo test --workspace --features security-tests
```

## Test Placeholders

Security tests will be implemented in future phases.

## Fuzzing

Fuzz testing targets:
- Configuration parsing
- Token deserialization
- HTTP request parsing
- Captcha validation
