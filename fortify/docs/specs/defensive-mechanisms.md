Fortify — Defensive Mechanisms Specification
Core Constraints (Enforced)

❌ No client-side JavaScript. Ever.

✅ All heavy checks occur only at initial gate

✅ Post-verification browsing must be fast and low-latency

✅ Legit users are never punished unless behavior proves hostile

✅ Defensive-only system

✅ Linux server-grade OS (Ubuntu/Debian)

1. OS & Host-Level Hardening (First Thing Fortify Does)

This happens before Tor services are even started.

1.1 Environment Detection

Detect:

Bare metal vs VM vs container

Hypervisor hints

Disk type

CPU limits

RAM ceilings

Adjust hardening profile accordingly

1.2 Mandatory OS Hardening

Minimum baseline:

Disable unused services

Lock down SSH:

Key-only auth

Rate-limited

Non-standard port (optional)

Harden sysctl:

TCP SYN protections

ICMP rate limiting

File descriptor limits

Mount options:

noexec, nosuid, nodev where applicable

Restrict /proc, /sys, /dev visibility

Tighten cgroup limits to prevent fork bombs / memory exhaustion

Enforce strict file permissions on Tor and Fortify directories

1.3 Damage Containment

Each Fortify component runs as:

Separate user

Separate permissions

No component has:

Root after install

Direct access to other components’ secrets

Compromise of one node ≠ compromise of system

2. Tor-Level Defensive Configuration
2.1 Hidden Service Isolation

Separate Tor instances (or identities) for:

Public Orchestrators

Healthy Nodes

Threat Nodes

Separate keys, separate data dirs

No shared circuits between classes

2.2 Circuit Hygiene

Short-lived circuits for threat nodes

Longer-lived circuits for healthy sessions

Aggressive circuit renewal on suspicious behavior

2.3 Connection Limits

Per-onion:

Connection caps

Bandwidth ceilings

Hard drop behavior under overload (fail closed, not open)

3. Public Orchestrator Defense

These are disposable by design.

3.1 Stateless First Contact

No trust on first request

No expensive backend calls until classification

Minimal surface:

Simple HTTP responses

Static templates

No dynamic processing yet

3.2 Mirror Burn Strategy

Track:

Connection flood patterns

Request entropy

Resource exhaustion trends

If a mirror is abused:

Mark as “burned”

Stop advertising it internally

Spawn replacement

Old mirror only serves captcha migration page

4. Initial Gate (Primary Defense Wall)

This is where Fortify is allowed to be slow, brutal, and expensive.

4.1 Proof-of-Work (PoW)

Server-issued challenge

CPU-bound or memory-hard (configurable)

Difficulty adaptive based on load

No JS:

Uses pure HTML + HTTP challenge/response

Can rely on form submission and server validation

4.2 Captcha (Hard Mode)

Image or logic-based

Regenerated on every failure

No reuse

No audio fallback (Tor reality)

4.3 Time-as-a-Weapon

Intentional delay between challenge steps

Humans tolerate it

Bots suffer

4.4 One-Time Promotion Token

On success:

Issue short-lived, signed session token

Token maps to:

Trust tier

Assigned node pool

Token never grants permanent trust

5. Post-Verification Speed Path (Critical Requirement)

Once verified:

No PoW

No captcha

No artificial delays

5.1 Fast Path Characteristics

Minimal request inspection

Pre-approved routing

Cached policy decisions

Direct proxy to healthy nodes

Latency here should approach:

Tor baseline + proxy overhead only

6. Behavioral Threat Detection (Silent & Surgical)
6.1 What Is Monitored

Request rate anomalies

Page traversal patterns

Session entropy

Concurrent connections per session

Resource usage per identity

6.2 What Is NOT Done

No fingerprinting

No JS-based tracking

No invasive client probing

6.3 Reclassification Rules

If behavior crosses threshold:

Session silently demoted

Routed back to threat nodes

Forced re-verification

No warning

No error messages

No feedback loop for attackers

7. Node-Level Defense
7.1 Healthy Nodes

Strict request ceilings

Fast fail on overload

No direct public access

7.2 Threat Nodes

Intentionally slower

Lower concurrency limits

Aggressive timeouts

Captcha on every page load

Threat nodes are meant to be hostile environments for automation.

8. Resource Exhaustion Protection
8.1 Global Resource Governor

CPU, RAM, disk IO budgets enforced

Never exceed ~75% system capacity

Hard cutoffs over graceful degradation

8.2 Priority Rules

OS survival

Orchestrators

Healthy nodes

Threat nodes (first to suffer)

9. Community Network Safety (Defensive Only)

Community traffic never bypasses gate

Discovery does not equal trust

Network registry data is:

Signed

Rate-limited

Validated

One poisoned member cannot push configs or code

10. Fail-Safe Philosophy

When something goes wrong:

Drop traffic

Burn mirrors

Force re-verification

Preserve secrecy

Availability is secondary to secrecy. Always.

Bottom Line (Straight Talk)

What Fortify defends well against:

Layer 7 floods

Connection exhaustion

Enumeration

Blind traffic abuse

Low-cost bot attacks

What it will not magically solve:

Massive Tor-wide saturation

Global adversaries with Tor-level leverage

Compromised host OS (after root)

That’s reality — and this design is honest about it.