# Security Policy

## Scope

Fortify is a defensive protection system for Tor hidden services. This policy covers:

- Security vulnerabilities in Fortify code
- Misconfigurations that compromise protection
- Design flaws that enable bypass
- Information disclosure risks

## Reporting a Vulnerability

**Do NOT open public issues for security vulnerabilities.**

### Reporting Process

1. **Contact**: Create a security advisory via GitHub or contact maintainers privately
2. **Information**: Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested mitigation (if any)
3. **Response Time**: Expect acknowledgment within 72 hours
4. **Disclosure**: Coordinate disclosure timeline with maintainers

## Security Considerations

### By Design

- **No JavaScript**: Eliminates entire class of client-side attacks
- **Defensive Only**: No offensive capabilities to misuse
- **Degradation**: System fails closed, never open
- **Trust Tiers**: Progressive verification limits blast radius

### Out of Scope

- DoS attacks (availability is explicitly secondary)
- Tor protocol vulnerabilities
- Operating system vulnerabilities
- Social engineering attacks

### Known Limitations

- Resource exhaustion possible under sustained load
- Mirror burn detection is heuristic-based
- Community network requires trust in seed operators

## Security Updates

Security fixes are prioritized above all other changes. Updates will:

- Be released as soon as safely possible
- Include clear migration paths
- Preserve backward compatibility when feasible

## Threat Model

See [docs/threat-model.md](docs/threat-model.md) for detailed threat analysis.

## Hardening

See [docs/hardening.md](docs/hardening.md) for deployment hardening guidelines.
