# Threat Model

## Adversary Profiles

### Script Kiddie
- **Capability**: Automated tools, known exploits
- **Motivation**: Opportunistic disruption
- **Mitigation**: Gate challenges eliminate automated attacks

### Motivated Attacker
- **Capability**: Custom tools, sustained effort
- **Motivation**: Targeted disruption or deanonymization
- **Mitigation**: 
  - Orchestrator rotation limits reconnaissance
  - Resource caps prevent exhaustion
  - No client-side code to exploit

### State-Level Actor
- **Capability**: Traffic analysis, infrastructure compromise
- **Motivation**: Deanonymization, service shutdown
- **Mitigation**:
  - Tor network provides base anonymity
  - Fortify adds defense-in-depth
  - Burn-on-compromise limits exposure window

## Threat Categories

### 1. Service Discovery
**Threat**: Attacker discovers real onion address

**Scenarios**:
- Traffic correlation
- Timing analysis
- Infrastructure compromise

**Mitigations**:
- Real address never transmitted publicly
- Orchestrators are disposable buffers
- Node forwarding obscures patterns

**Residual Risk**: HIGH - requires Tor network security

### 2. Denial of Service
**Threat**: Exhaust resources, make service unavailable

**Scenarios**:
- Connection flooding
- Slow loris attacks
- Computational exhaustion

**Mitigations**:
- Gate rate limiting
- Connection caps
- Resource governor
- Backpressure handling

**Residual Risk**: MEDIUM - availability is secondary to secrecy

### 3. Bypass/Escalation
**Threat**: Attacker bypasses Gate without verification

**Scenarios**:
- Token forgery
- Logic bugs
- Protocol confusion

**Mitigations**:
- Signed tokens with expiry
- No JavaScript eliminates client-side bypass
- Minimal attack surface
- Defense-in-depth layers

**Residual Risk**: LOW - multiple validation layers

### 4. Information Disclosure
**Threat**: Leak implementation details or real service info

**Scenarios**:
- Error messages
- Timing side-channels
- Header leakage

**Mitigations**:
- Generic error pages
- Minimal response headers
- Constant-time operations where feasible

**Residual Risk**: LOW-MEDIUM - requires careful implementation

### 5. Deanonymization
**Threat**: Link service operator to real identity

**Scenarios**:
- Traffic correlation
- Operational security failures
- Infrastructure fingerprinting

**Mitigations**:
- Tor network provides base anonymity
- Minimal logging
- No external dependencies

**Residual Risk**: HIGH - depends on operator OpSec

## Attack Scenarios

### Scenario 1: Mass Connection Flood
1. Attacker opens thousands of connections
2. Gate enforces per-IP rate limits
3. Computational challenges slow attack
4. Resource governor drops connections
5. Service remains available to legitimate users

**Outcome**: Degraded but functional

### Scenario 2: Orchestrator Compromise
1. Attacker compromises orchestrator host
2. Burn detection triggers
3. Controller spawns new mirror
4. Old orchestrator serves migration page
5. Real service address remains unknown

**Outcome**: Temporary disruption, no disclosure

### Scenario 3: Token Forgery Attempt
1. Attacker attempts to forge promotion token
2. Signature validation fails
3. Session demoted to threat mode
4. Forced through Gate again

**Outcome**: Attack detected and mitigated

## Out of Scope Threats

- **Physical access**: Assumed secure infrastructure
- **Tor protocol vulnerabilities**: Assumed Tor network security
- **Social engineering**: Operator responsibility
- **Zero-day exploits**: Patching responsibility

## Trust Boundaries

1. **Network → Orchestrator**: Untrusted, hostile
2. **Orchestrator → Gate**: Semi-trusted, monitored
3. **Gate → HTTP Proxy**: Verified sessions only
4. **HTTP Proxy → Node**: Token-authenticated
5. **Node → Real Service**: Trusted internal

## Assumptions

- Linux kernel is not compromised
- Tor daemon is not backdoored
- System clock is accurate
- Operator follows security guidelines
- No JavaScript execution environment exists
