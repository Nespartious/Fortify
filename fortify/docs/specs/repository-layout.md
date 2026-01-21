Fortify — Repository Layout (Scaffold)
fortify/
├── README.md
├── SECURITY.md
├── LICENSE
├── Makefile
├── .gitignore
│
├── docs/
│   ├── architecture.md
│   ├── threat-model.md
│   ├── trust-levels.md
│   ├── community-network.md
│   ├── scaling-model.md
│   └── hardening.md
│
├── install/
│   ├── install.sh
│   ├── detect_env.sh
│   ├── harden_os.sh
│   ├── tor_setup.sh
│   ├── systemd/
│   │   ├── fortify-controller.service
│   │   ├── fortify-orchestrator.service
│   │   ├── fortify-node-healthy.service
│   │   └── fortify-node-threat.service
│   └── templates/
│       ├── sysctl.conf
│       ├── limits.conf
│       └── torrc.template
│
├── config/
│   ├── fortify.example.toml
│   ├── node-healthy.toml
│   ├── node-threat.toml
│   └── community.toml
│
├── scripts/
│   ├── dev-run.sh
│   ├── rotate-orchestrators.sh
│   └── burn-mirror.sh
│
├── crates/
│   ├── fortify-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── errors.rs
│   │       ├── config.rs
│   │       ├── crypto/
│   │       │   ├── mod.rs
│   │       │   ├── tokens.rs
│   │       │   ├── pow.rs
│   │       │   └── hashing.rs
│   │       ├── security/
│   │       │   ├── mod.rs
│   │       │   ├── rate_limits.rs
│   │       │   ├── behavior.rs
│   │       │   └── classification.rs
│   │       ├── resources/
│   │       │   ├── mod.rs
│   │       │   ├── cpu.rs
│   │       │   ├── memory.rs
│   │       │   └── governor.rs
│   │       ├── tor/
│   │       │   ├── mod.rs
│   │       │   ├── control.rs
│   │       │   ├── hidden_service.rs
│   │       │   └── identity.rs
│   │       └── utils/
│   │           ├── mod.rs
│   │           └── time.rs
│
│   ├── fortify-http/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       ├── proxy.rs
│   │       ├── headers.rs
│   │       └── limits.rs
│
│   ├── fortify-gate/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gate.rs
│   │       ├── captcha.rs
│   │       ├── pow.rs
│   │       └── promotion.rs
│
│   ├── fortify-orchestrator/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── router.rs
│   │       ├── mirrors.rs
│   │       ├── load_balancer.rs
│   │       └── health.rs
│
│   ├── fortify-node/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── forwarder.rs
│   │       ├── threat.rs
│   │       ├── healthy.rs
│   │       └── session.rs
│
│   ├── fortify-community/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs
│   │       ├── sync.rs
│   │       ├── seed.rs
│   │       └── verify.rs
│
│   └── fortify-controller/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── lifecycle.rs
│           ├── scaling.rs
│           ├── spawn.rs
│           ├── burn.rs
│           └── status.rs
│
├── assets/
│   ├── html/
│   │   ├── gate.html
│   │   ├── captcha.html
│   │   ├── burned.html
│   │   └── error.html
│   └── images/
│       └── captcha/
│
├── tests/
│   ├── integration/
│   │   ├── gate.rs
│   │   ├── routing.rs
│   │   └── scaling.rs
│   └── security/
│       ├── fuzz.rs
│       └── abuse.rs
│
└── .cargo/
    └── config.toml

Why This Layout Works (Important)
1. Clear Trust Boundaries

fortify-core = pure logic, no I/O

Network-facing code lives outside core

Gate, orchestrator, nodes are isolated binaries

A bug in one does not poison the rest.

2. Replaceable Components

Gate logic can evolve independently

Community system is optional

Nodes can be hardened without touching orchestrator

3. Security Auditing Is Possible

Crypto isolated

Tor logic isolated

No mega-crate doing everything

This matters later.

4. One-Click Deployment Reality

install/ handles OS hardening + Tor setup

fortify-controller owns lifecycle

systemd units are explicit and inspectable

No magic.

Minimal Build Flow (Mental Model)

install.sh

Detect OS

Harden system

Install Tor

Create users

fortify-controller

Reads config

Spawns orchestrators + nodes

Monitors health

fortify-orchestrator

Accepts public traffic

fortify-node

Forwards trusted traffic

fortify-gate

Decides who earns trust

Blunt Truth

This layout:

Is not “startup fast”

Is not “hackathon friendly”

Is production-defense correct

You will thank yourself later.