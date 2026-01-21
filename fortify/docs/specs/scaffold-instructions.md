ONE-SHOT AI AGENT PROMPT

Project: Fortify

You are building the initial repository scaffold for a security-first Tor hidden-service protection system called Fortify.

Your task is to generate the full repo layout, minimal Rust crates, placeholder logic, configs, and install scripts exactly as specified below.
This is scaffolding only, not a full implementation.

HARD RULES (NON-NEGOTIABLE)

NO JavaScript anywhere (client or server)

Rust only for application code

Linux server-grade deployment (Ubuntu/Debian)

Defensive system only

Prefer minimal, compiling code over features

Every crate must build

No network attacks, no exploit code

No TODO sprawl — placeholders must be clean and intentional

ROOT REPO NAME
fortify

REQUIRED REPO STRUCTURE

Create this exact folder structure:

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
│   ├── fortify-http/
│   ├── fortify-gate/
│   ├── fortify-orchestrator/
│   ├── fortify-node/
│   ├── fortify-community/
│   └── fortify-controller/
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
│   └── security/
│
└── .cargo/
    └── config.toml

RUST CRATE REQUIREMENTS

Each crate must include:

Cargo.toml

src/

Minimal compiling Rust code

No unused dependencies

Crate Responsibilities
fortify-core

Pure logic only

Config parsing

Token types

Trust tiers

No networking

fortify-http

HTTP server abstraction

Reverse proxy placeholders

Request limits

fortify-gate

Initial gate logic

Captcha + PoW placeholders

Promotion token issuance (stub)

fortify-orchestrator

Public entry binary

Mirror tracking (stub)

Routing decisions (stub)

fortify-node

Node binary

Healthy vs Threat modes

Forwarder placeholder

fortify-community

Registry structures

Seed logic placeholders

Signed registry verification (stub)

fortify-controller

Main lifecycle binary

Spawning logic (stub)

Scaling logic (stub)

IMPLEMENTATION RULES

All binaries must compile

Functions may return unimplemented!() only if unavoidable

Prefer empty structs over fake logic

Use tokio only where async is required

Use tracing for logging (initialized but minimal)

No external services

No JavaScript

No databases

DOCUMENTATION

Populate each .md file with:

A short explanation of its purpose

Bullet-point sections

No marketing language

INSTALL SCRIPTS

Shell scripts only

Safe defaults

No destructive commands

Clearly commented

Assume Debian/Ubuntu

FINAL OUTPUT EXPECTATION

When finished:

The repo builds with cargo build --workspace

No missing files

No placeholder text like “lorem ipsum”

No JavaScript anywhere

Clean, auditable, minimal scaffold

Do not explain what you did.
Just generate the repository.