# 📚 Fortify Documentation

> **Comprehensive Developer Reference for the Fortify Tor Hidden Service Protection Layer**

```
    ╔═══════════════════════════════════════════════════════════════════╗
    ║                                                                   ║
    ║                  ███████╗ ██████╗ ██████╗ ████████╗██╗███████╗██╗ ║
    ║                  ██╔════╝██╔═══██╗██╔══██╗╚══██╔══╝██║██╔════╝██║ ║
    ║                  █████╗  ██║   ██║██████╔╝   ██║   ██║█████╗  ██║ ║
    ║                  ██╔══╝  ██║   ██║██╔══██╗   ██║   ██║██╔══╝  ╚═╝ ║
    ║                  ██║     ╚██████╔╝██║  ██║   ██║   ██║██║     ██╗ ║
    ║                  ╚═╝      ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝╚═╝     ╚═╝ ║
    ║                                                                   ║
    ║              DECENTRALIZED TOR HIDDEN SERVICE PROTECTION          ║
    ╚═══════════════════════════════════════════════════════════════════╝
```

## 📁 Documentation Structure

```
Fortify Documentation/
├── README.md                      # This file - Documentation index
├── Functions.md                   # Complete function/API reference (1500+ lines)
│
├── 01-Architecture/
│   └── overview.md                # System architecture with ASCII diagrams ✓
│
├── 02-Core-Concepts/
│   ├── trust-tiers.md             # Trust tier system (5 tiers) ✓
│   └── behavioral-analysis.md     # Behavioral engine (9 violation types) ✓
│
├── 03-TOR-Integration/
│   └── onion-services.md          # Tor integration, PoW, mirrors ✓
│
├── 04-Components/
│   └── crate-reference.md         # All 7 crates documented ✓
│
├── 05-Security/
│   └── hardening.md               # Production security hardening ✓
│
├── 06-Configuration/
│   └── config-reference.md        # Complete config file reference ✓
│
├── 07-Operations/
│   ├── quickstart.md              # 10-minute quick start guide ✓
│   └── monitoring.md              # Admin panel guide ✓
│
└── 08-API-Reference/
    └── api-reference.md           # REST API documentation ✓
```

## 🚀 Quick Start

1. **New to Fortify?** Start with [Quick Start Guide](07-Operations/quickstart.md)
2. **Understanding the system?** Read [Architecture Overview](01-Architecture/overview.md)
3. **Looking for functions?** See [Functions.md](Functions.md)
4. **Deploying to production?** Check [Security Hardening](05-Security/hardening.md)

## 📊 Project Statistics

| Metric | Value |
|--------|-------|
| **Total Lines of Code** | ~20,000+ |
| **Documentation Lines** | ~7,000+ |
| **Number of Crates** | 7 |
| **Captcha Types** | 7 |
| **Trust Tiers** | 5 |
| **Violation Types** | 9 (behavioral) |
| **Attack Patterns** | 25+ |
| **Fast-Pass Tiers (Planned)** | 2 (Squire, Knight) |

## 🔗 Quick Links

| Document | Description |
|----------|-------------|
| [Functions.md](Functions.md) | Complete function/struct/enum reference |
| [Quick Start](07-Operations/quickstart.md) | Get running in 10 minutes |
| [Trust Tiers](02-Core-Concepts/trust-tiers.md) | 5-tier trust system + Fast-Pass (planned) |
| [Behavioral Analysis](02-Core-Concepts/behavioral-analysis.md) | Threat detection engine |
| [Config Reference](06-Configuration/config-reference.md) | All configuration options |
| [Admin Panel](07-Operations/monitoring.md) | Web interface guide |
| [API Reference](08-API-Reference/api-reference.md) | REST API documentation |
| [Security Hardening](05-Security/hardening.md) | Production deployment |
| [Crate Reference](04-Components/crate-reference.md) | Individual component docs |

## 🚧 Planned Features

| Feature | Status | Priority |
|---------|--------|----------|
| **Fast-Pass System** | Planned | Low |

The Fast-Pass system will provide PGP-based persistent identity for returning users:
- **Squire (Free)**: PGP identity, start at Verified, 1 easy captcha per session
- **Knight (Paid via XMR)**: Full captcha bypass, start at Trusted, vouching privileges

See [Trust Tiers](02-Core-Concepts/trust-tiers.md#fast-pass-identity-system-future-feature) for full documentation.

---
*Documentation based on current codebase analysis | Fortify v0.1.0*
