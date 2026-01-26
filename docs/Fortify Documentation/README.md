# Fortify Documentation

**Comprehensive documentation for the Fortify Tor hidden service protection system**

**Last Updated:** January 25, 2026  
**Version:** Beta  
**Status:** Production-Ready Documentation

---

## 📚 Documentation Structure

```
Fortify Documentation/
├── 00-Quick-Start/            ← Get running in 15 minutes
├── 01-Architecture/           ← System design and components
├── 02-Core-Concepts/          ← Trust tiers, behavioral analysis
├── 03-Security-Model/         ← Threat model, attack mitigations
├── 04-Deployment/             ← Installation and setup
├── 05-Configuration/          ← Complete config reference
├── 06-Operations/             ← Monitoring and maintenance
├── 07-Troubleshooting/        ← Common issues and solutions
├── 08-API-Reference/          ← API endpoints and usage
├── 09-ELI5/                   ← Beginner-friendly guide
└── 10-Glossary/               ← Key terms and definitions
```

---

## 🚀 Quick Start

**New to Fortify?** Start here:

1. **[Quick Start Guide](00-Quick-Start/quick-start.md)** - Get running in 15 minutes
2. **[Explain Like I'm 5](09-ELI5/explain-like-im-5.md)** - Complete beginner guide in plain English
3. **[Architecture Overview](01-Architecture/system-overview.md)** - How the system works
4. **[Trust Tiers](02-Core-Concepts/trust-tiers.md)** - Understanding the 5 trust levels
5. **[Threat Model](03-Security-Model/threat-model.md)** - What Fortify protects against

---

## 📖 Documentation by Topic

### Getting Started

| Document | Description | Lines | Audience |
|----------|-------------|-------|----------|
| [⚡ Quick Start](00-Quick-Start/quick-start.md) | Installation to first test in 15 min | 198 | New operators |
| [🎈 ELI5 Guide](09-ELI5/explain-like-im-5.md) | Complete guide in plain English | 504 | Everyone |
| [📖 Glossary](10-Glossary/glossary.md) | Key terms and definitions | 280 | All users |

### Understanding the System

| Document | Description | Lines | Audience |
|----------|-------------|-------|----------|
| [🏗️ System Overview](01-Architecture/system-overview.md) | System design, components, data flow | 516 | Technical |
| [🔄 Component Interaction](01-Architecture/component-interaction.md) | Request flow and communication | 410 | Technical |
| [🔐 Trust Tiers](02-Core-Concepts/trust-tiers.md) | The 5-tier trust system explained | 751 | All users |
| [🎟️ Session Management](02-Core-Concepts/session-management.md) | Tokens, validation, lifecycle | 315 | Operators |
| [🤖 CAPTCHA System](02-Core-Concepts/captcha-system.md) | Challenge types and validation | 405 | Operators |
| [⏱️ Rate Limiting](02-Core-Concepts/rate-limiting.md) | Circuit-based quotas | 278 | Operators |
| [🔄 Mirror Management](02-Core-Concepts/mirror-management.md) | Creation, rotation, burning | 292 | Operators |

### Security & Threats

| Document | Description | Lines | Audience |
|----------|-------------|-------|----------|
| [🛡️ Threat Model](03-Security-Model/threat-model.md) | Attack scenarios and threat actors | 304 | Security researchers |
| [🎯 Attack Mitigations](03-Security-Model/attack-mitigations.md) | Defense strategies per attack type | 479 | Security teams |

### Deployment & Operations

| Document | Description | Lines | Audience |
|----------|-------------|-------|----------|
| [🚀 Deployment Guide](04-Deployment/deployment-guide.md) | Installation, setup, hardening | 337 | Operators |
| [⚙️ Configuration Reference](05-Configuration/configuration-reference.md) | Complete parameter documentation | 544 | Operators |
| [📊 Operations/Monitoring](06-Operations/monitoring.md) | Daily operations, monitoring, backups | 419 | Operators |
| [🔧 Troubleshooting](07-Troubleshooting/common-issues.md) | Solutions to 18+ common problems | 531 | Operators |

### Reference

| Document | Description | Lines | Audience |
|----------|-------------|-------|----------|
| [📡 API Reference](08-API-Reference/api-reference.md) | Complete API documentation | 931 | Developers |

**Total Documentation:** 7,494 lines

---

## 🎯 Documentation by Audience

### For New Operators
- **Start with:** [Quick Start Guide](00-Quick-Start/quick-start.md) (15 minutes)
- **Then read:** [Deployment Guide](04-Deployment/deployment-guide.md)
- **Configure:** [Configuration Reference](05-Configuration/configuration-reference.md)
- **Operate:** [Operations/Monitoring](06-Operations/monitoring.md)

### For End Users
- **Start with:** [ELI5 Guide](09-ELI5/explain-like-im-5.md)
- **Then read:** [Trust Tiers](02-Core-Concepts/trust-tiers.md) to understand your status
- **Reference:** [Glossary](10-Glossary/glossary.md) for terms

### For Operators
- **Start with:** [System Overview](01-Architecture/system-overview.md)
- **Then read:** [Component Interaction](01-Architecture/component-interaction.md)
- **Monitor:** [Operations/Monitoring](06-Operations/monitoring.md)
- **Troubleshoot:** [Common Issues](07-Troubleshooting/common-issues.md)
- **Reference:** [Configuration](05-Configuration/configuration-reference.md)

### For Security Researchers
- **Start with:** [Threat Model](03-Security-Model/threat-model.md)
- **Then read:** [Attack Mitigations](03-Security-Model/attack-mitigations.md)
- **Deep dive:** [System Overview](01-Architecture/system-overview.md)

### For Developers
- **Start with:** [System Overview](01-Architecture/system-overview.md)
- **Reference:** [API Reference](08-API-Reference/api-reference.md)
- **Understand:** [Trust Tiers](02-Core-Concepts/trust-tiers.md) and [Session Management](02-Core-Concepts/session-management.md)

---

## 🔍 Key Concepts

### Trust Tiers (5 Levels)
```
🔴 BURNED (-2)      ← Permanently banned
🟡 SUSPICIOUS (-1)  ← On thin ice, needs 2 CAPTCHAs
⚪ UNKNOWN (0)      ← New user, needs 1 CAPTCHA
🔵 VERIFIED (+1)    ← Proven human, normal access
🟢 TRUSTED (+2)     ← Long-term good behavior, VIP access
```

### Core Security Features
- ✅ **No JavaScript** - Works with Tor Browser Safest mode
- ✅ **CAPTCHA Verification** - 7 types, server-side only
- ✅ **Behavioral Analysis** - Detects bots and attacks
- ✅ **Circuit-Based Rate Limiting** - Per-circuit quotas
- ✅ **Demotion System** - Progressive penalties
- ✅ **Mirror Burn Capability** - CAN replace mirrors if needed
- ✅ **Vanguards** - Guard discovery protection
- ✅ **HMAC-SHA256 Tokens** - Unforgeable session tokens

---

## 📊 Documentation Status

### Complete & Current ✅
- **Quick Start** - 15-minute setup guide
- **Architecture** - System design and component interaction
- **Core Concepts** - Trust tiers, sessions, CAPTCHA, rate limiting, mirrors
- **Security Model** - Threats and mitigations
- **Deployment** - Installation and setup (with TUI wizard TODOs)
- **Configuration** - Complete parameter reference
- **Operations** - Monitoring, maintenance, backups
- **Troubleshooting** - 18+ common issues with solutions
- **API Reference** - Complete API documentation
- **ELI5** - Beginner guide
- **Glossary** - Key terms and definitions

### In Progress 🚧
- **TUI Wizard** - 40% complete (as of January 2026)
- **Prometheus Integration** - Metrics collection system (planned)
- **Session Continuity** - Survive controller restarts (planned)

---

## 🔗 Related Documentation

### In Other Locations
- **[Root README](../../README.md)** - Project overview
- **[ROADMAP](../ROADMAP.md)** - Feature development plan
- **[AUTHENTICATION](../AUTHENTICATION.md)** - Admin panel security
- **[RATE_LIMITING](../RATE_LIMITING.md)** - Circuit-based rate limiting details
- **[Dev Progress](../Dev_Progress/)** - Sprint documentation
- **[Planning](../planning/)** - Future feature planning
- **[Research](../research/)** - Security research and analysis

---

## 💡 How to Use This Documentation

### 1. First Time Reading
Follow this path:
```
Quick Start → ELI5 Guide → System Overview → Trust Tiers
```

### 2. Setting Up Fortify
```
Quick Start → Deployment Guide → Configuration Reference → Operations/Monitoring
```

### 3. Understanding Security
```
Threat Model → Attack Mitigations → System Overview
```

### 4. Troubleshooting Issues
```
Common Issues → Trust Tiers → Session Management → Glossary
```

### 5. Developing/Integrating
```
System Overview → API Reference → Trust Tiers → Session Management
```

---

## 🎓 Learning Path

### Beginner (30 minutes - 1 hour)
1. Read [Quick Start Guide](00-Quick-Start/quick-start.md) - Get hands-on
2. Skim [ELI5 Guide](09-ELI5/explain-like-im-5.md) - Understand concepts
3. Reference [Glossary](10-Glossary/glossary.md) - Learn terminology

**After this:** You can deploy and run Fortify.

### Intermediate (3-5 hours)
4. Study [System Overview](01-Architecture/system-overview.md) - Deep system understanding
5. Read [Trust Tiers](02-Core-Concepts/trust-tiers.md) - Master the trust system
6. Review [Configuration Reference](05-Configuration/configuration-reference.md) - Tune your setup
7. Learn [Operations/Monitoring](06-Operations/monitoring.md) - Maintain the system

**After this:** You can operate and configure Fortify effectively.

### Advanced (5-10 hours)
8. Master [Threat Model](03-Security-Model/threat-model.md) - Understand threats
9. Deep dive [Attack Mitigations](03-Security-Model/attack-mitigations.md) - Know defenses
10. Study [Component Interaction](01-Architecture/component-interaction.md) - Understand internals
11. Reference [API Documentation](08-API-Reference/api-reference.md) - Integrate systems

**After this:** You can contribute to Fortify development and customize deployments.

---

## 📝 Documentation Standards

All Fortify documentation follows these principles:

1. **Plain English First** - Technical terms explained
2. **Examples Included** - Real-world scenarios
3. **Diagrams Used** - Visual explanations where helpful
4. **Privacy Conscious** - No personal data in examples
5. **Tor-Aware** - Acknowledges Tor constraints
6. **Accurate** - Verified against actual code

---

## 🔄 Documentation Updates

This documentation is actively maintained:

- **Last Major Audit:** January 25, 2026 (Sprint 21)
- **Last Major Expansion:** January 25, 2026 (Sprint 22)
- **Security Claims:** All 24 verified against code
- **Accuracy Rate:** 95.8% (1 minor correction applied)
- **Status:** Production-ready for beta release
- **Next Review:** Upon TUI wizard completion (Q2 2026)

### How to Report Documentation Issues
1. Check if information is outdated (compare with code)
2. Look for broken links or unclear explanations
3. Create issue with specific file and line number
4. Suggest correction if possible

---

## 🏆 Documentation Quality

### Metrics
- **Total Lines:** 7,494 lines
- **Documents:** 18 complete files
- **Coverage:** All major features and operations documented
- **Accuracy:** Verified against implementation
- **Readability:** Beginner to advanced content
- **Completeness:** 95% (TUI wizard documentation incomplete pending development)

### Recent Improvements (Sprint 21-22)
- ✅ Created comprehensive threat model (304 lines)
- ✅ Created detailed attack mitigations (479 lines)
- ✅ Created beginner-friendly ELI5 guide (504 lines)
- ✅ Created deployment guide with TUI workflow (337 lines)
- ✅ Created complete configuration reference (544 lines)
- ✅ Created operations/monitoring guide (419 lines)
- ✅ Created troubleshooting guide with 18+ issues (531 lines)
- ✅ Created quick start guide (198 lines)
- ✅ Created comprehensive glossary (280 lines)
- ✅ Corrected trust tier burn threshold
- ✅ Fixed 11 broken links
- ✅ Verified all 24 security claims

---

## 📞 Getting Help

1. **Start with Quick Start:** Fastest path from zero to running - [quick-start.md](00-Quick-Start/quick-start.md)
2. **Check Troubleshooting:** 18+ common issues solved - [common-issues.md](07-Troubleshooting/common-issues.md)
3. **Review ELI5:** Most questions answered - [explain-like-im-5.md](09-ELI5/explain-like-im-5.md)
4. **Check Glossary:** Unfamiliar terms defined - [glossary.md](10-Glossary/glossary.md)
5. **Review Architecture:** System behavior explained - [system-overview.md](01-Architecture/system-overview.md)
6. **Consult API Docs:** Integration help - [api-reference.md](08-API-Reference/api-reference.md)
7. **Security Questions:** Threat model and mitigations - [threat-model.md](03-Security-Model/threat-model.md)

---

**Happy reading! 🎉**

*This documentation set represents comprehensive coverage of Fortify's architecture, security model, operational procedures, and deployment workflows. It is suitable for users ranging from complete beginners to advanced security researchers and system operators.*
