# 📖 Glossary

> **Key Terms and Definitions**

---

## A

**Admin Panel**  
Web-based management interface for Fortify. Accessed via `/ctrl_xxx` path on any mirror. Requires password authentication. Provides session management, mirror control, and system monitoring.

**Attack Path**  
URL path that matches known malicious patterns (e.g., `/../`, `/.env`, `/wp-admin`). Detected by behavioral analysis and triggers violations.

**Automated Behavior**  
Meta-violation triggered when a session accumulates multiple violations or high severity score, indicating bot-like patterns.

---

## B

**Backend**  
The real hidden service that Fortify protects. Also called "real service" or "protected service". Never exposed directly to public.

**Behavioral Analysis**  
System that monitors request patterns to detect bots and attacks without JavaScript or fingerprinting. Analyzes paths, user-agents, referers, timing, and payload sizes.

**Burn / Burned**  
1. (Session) Permanently banned session that can no longer access the service.
2. (Mirror) Compromised or attacked mirror that CAN be destroyed and replaced, though rarely done in practice.

**Burn Threshold**  
The limit that triggers a burn action. For sessions: 3 demotions. For mirrors: manual operator decision (automated burning rarely used).

---

## C

**CAPTCHA**  
Challenge-Response test to verify human users. Fortify supports 7 types (BMP Text, Emoji, Direction Arrows, Sequence, Word Unscramble, Image Rotation, Silhouette). All server-side, no JavaScript required.

**Circuit**  
In Tor, a path through relay nodes. Fortify uses circuit-based rate limiting, where each circuit gets independent quotas instead of shared IP-based limits.

**Circuit-Based Rate Limiting**  
Rate limiting system that assigns quotas per Tor circuit rather than per IP address. Prevents DDoS attacks from blocking legitimate users.

**Controller**  
Fortify component responsible for resource management, auto-scaling, health checking, and vanguards lifecycle. Controls orchestrators and nodes.

**Compromise Score**  
Metric tracking suspicious activity on a mirror. High score triggers automatic burning.

---

## D

**Demotion**  
Lowering a session's trust tier due to violations. VERIFIED → SUSPICIOUS, or TRUSTED → VERIFIED. After 3 demotions, session is killed (burned).

**Demotion Count**  
Number of times a session has been demoted and re-verified. Tracks repeat offenders. Limit: 3 (configurable).

---

## E

**Enumeration**  
Attack pattern where attacker rapidly accesses many paths (directory scanning) or sequential paths (path enumeration). Detected by behavioral analysis.

---

## F

**Fail Closed**  
Security principle where unknown states result in denial of access, not permission. If Fortify can't determine if a request is safe, it blocks it.

**Form Submission Flood**  
Attack where attacker rapidly submits many POST requests. Detected when exceeding 10 submissions/minute (configurable).

---

## G

**Gate**  
Fortify component that handles CAPTCHA challenges and token issuance. All unverified users must pass through Gate before accessing backend.

**Grace Period**  
24-hour window after a mirror is burned where it serves a "death page" before full destruction. Gives users time to find alternative mirrors.

**Guard Discovery**  
Attack attempting to identify Tor guard nodes by analyzing circuit patterns. Mitigated by vanguards.

**Guard Node**  
First Tor relay in a circuit. Guards are pinned for stability but become targets. Vanguards provides additional protection.

---

## H

**Healthy Node**  
Node pool for verified and trusted sessions. Fast path with minimal security inspection. Contrast with Threat Node.

**HMAC-SHA256**  
Cryptographic signature algorithm used to sign session tokens. Prevents token forgery as only server knows the secret key.

**HTTP Proxy**  
Fortify component that routes requests based on session trust tier. Performs token validation and behavioral analysis.

---

## K

**Kill / Killed**  
Permanent session ban after reaching demotion threshold (3 demotions). Session marked as killed and cannot be recovered.

---

## M

**Mirror**  
Public-facing .onion address that users connect to. Relatively stable, running for months at a time. CAN be burned and replaced if needed, but not commonly done. Fortify runs 3-5 active mirrors by default.

**Mirror Rotation**  
Process of burning old mirrors and creating new ones. Capability exists but rarely used in practice. Can be manual or automated if needed.

---

## N

**Node**  
Backend proxy component that forwards requests to real service. Separated into Healthy and Threat pools for different trust tiers.

**Node Onion**  
Each node has its own .onion address with separate Tor circuit, enabling true circuit isolation when burning compromised nodes.

---

## O

**Orchestrator**  
Fortify component that manages mirrors, communicates with Tor control port, and handles mirror lifecycle (creation, rotation, burning).

**.onion**  
Tor hidden service address. Ends in `.onion` instead of `.com`. Only accessible through Tor network.

---

## P

**Path Enumeration**  
Attack where sequential paths are accessed (e.g., /page1, /page2, /page3). Detected after 5 sequential paths (configurable).

**Path Traversal**  
Attack attempting to access files outside web root using `../` or similar. Immediately detected and flagged.

**Proof-of-Work (PoW)**  
Cryptographic puzzle requiring computational work. Tor supports PoW defenses at protocol level. Fortify enables this feature.

**Promotion**  
Upgrading a session's trust tier due to good behavior. VERIFIED → TRUSTED after 50 clean requests (configurable).

---

## R

**Rate Limiting**  
Restricting number of requests per time period. Fortify uses circuit-based rate limiting with per-tier quotas (Unknown: 10, Verified: 100, Trusted: 300 req/10sec).

**Real Service**  
See Backend.

**Resource Enumeration**  
Rapid access to many unique paths attempting to map site structure. Detected at 60 unique paths/minute (configurable).

---

## S

**Session**  
User's authenticated state tracked by session token. Has trust tier, violation count, demotion count, and behavioral statistics.

**Session Token**  
Cookie containing session ID, trust tier, expiration, and HMAC signature. Issued by Gate after CAPTCHA verification.

**Severity**  
Weight assigned to each violation type (1-3). Higher severity violations more quickly trigger demotion.

**Signing Key**  
Secret key used for HMAC-SHA256 token signing. Must be kept secure as compromise allows token forgery.

**Suspicious**  
Trust tier (-1) for sessions that have violated rules. Requires 2 hard CAPTCHAs to re-verify.

---

## T

**Threat Node**  
Node pool for suspicious sessions. Deep inspection, strict limits, heavy monitoring. Slower than healthy nodes.

**Tier Override**  
Temporary trust tier assignment set by proxy/node when demoting users. Cleared when fresh token is issued.

**Token**  
See Session Token.

**Tor**  
The Onion Router - anonymity network that Fortify protects services on.

**Trust Tier**  
Security level assigned to each session. Five tiers: Burned (-2), Suspicious (-1), Unknown (0), Verified (+1), Trusted (+2).

**TTL (Time To Live)**  
How long a session token remains valid. Default: 1 hour (3600 seconds).

**TUI**  
Text User Interface - interactive terminal-based deployment wizard for Fortify.

---

## U

**Unknown**  
Trust tier (0) for new users without session tokens. Must complete CAPTCHA at Gate before access.

**User-Agent**  
HTTP header identifying client software. Behavioral analysis detects bot patterns (curl, wget, python-requests, etc.).

---

## V

**Vanguards**  
Tor security addon that pins Layer 2 and Layer 3 guards to prevent guard discovery attacks. Strongly recommended for Fortify deployments.

**Verified**  
Trust tier (+1) achieved by solving CAPTCHA. Standard access level with 100 requests/10sec quota.

**Violation**  
Security event triggered by suspicious behavior (attack paths, bot user-agent, enumeration, etc.). Tracked per session.

**Violation Threshold**  
Number of violations of a specific type required to trigger demotion. Default: 3 per type.

---

## Acronyms

**API** - Application Programming Interface  
**CAPTCHA** - Completely Automated Public Turing test to tell Computers and Humans Apart  
**CPU** - Central Processing Unit  
**DDOS** - Distributed Denial of Service  
**HMAC** - Hash-based Message Authentication Code  
**HTTP** - Hypertext Transfer Protocol  
**POW** - Proof-of-Work  
**RAM** - Random Access Memory  
**SHA** - Secure Hash Algorithm  
**TOML** - Tom's Obvious Minimal Language (config format)  
**TUI** - Text User Interface  
**TTL** - Time To Live  
**UA** - User-Agent  
**VIP** - Very Important Person (metaphor for Trusted tier)

---

## Common Abbreviations

**orch** - Orchestrator  
**ctrl** - Controller  
**req** - Request  
**sec** - Second  
**min** - Minute  
**hr** - Hour  

---

## Security Terms

**Attack Surface**  
Total of all possible entry points for attacks. Fortify minimizes this by never exposing the real service.

**Defense in Depth**  
Security strategy using multiple overlapping layers. Fortify implements: CAPTCHA, trust tiers, behavioral analysis, rate limiting, circuit isolation, and vanguards.

**Fail-Safe**  
System design where failures result in safe state (denial rather than exposure).

**Zero Trust**  
Security model where no user is trusted by default. All must prove legitimacy through CAPTCHA.

---

## Technical Terms

**Bind Address**  
IP and port a service listens on. Most Fortify components bind to 127.0.0.1 (localhost only).

**Cookie**  
Small data stored in browser. Fortify uses cookies for session tokens.

**Daemon**  
Background service (e.g., Tor daemon).

**Proxy**  
Intermediary server. Fortify's HTTP Proxy sits between mirrors and backend.

**Queue**  
Line of pending requests. Fortify queues requests when at capacity.

**Regex**  
Regular Expression - pattern matching for text (used in attack path detection).

**Systemd**  
Linux service manager. Fortify can run as systemd services.

---

*For more detailed explanations, see the [ELI5 Guide](../09-ELI5/explain-like-im-5.md)*
