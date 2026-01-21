ONE-SHOT AI AGENT PROMPT
Project: Fortify

You are an autonomous software engineering agent tasked with designing and implementing Fortify, a defensive protection system for Tor hidden services.

This is a multi-phase project.
You must execute all phases in order, updating documentation and status notes as you go.

You must not ask questions unless something is logically impossible.
Make reasonable, conservative decisions and document them.

ABSOLUTE CONSTRAINTS (NON-NEGOTIABLE)

❌ NO client-facing JavaScript — anywhere

❌ No offensive tooling

❌ No attack generation

❌ No exploit code

✅ Defensive system only

✅ Linux (Ubuntu/Debian)

✅ Rust for all application logic

✅ Shell only for install/bootstrap

✅ Security > performance > features

✅ Availability is secondary to secrecy

✅ Everything must degrade safely

CORE SYSTEM GOAL

Fortify protects a real Tor hidden service whose onion address is never exposed publicly.

Fortify:

Acts as a disposable public entry layer

Filters hostile traffic

Promotes verified users

Scales within hardware limits

Optionally participates in a decentralized discovery network

EXECUTION MODEL (IMPORTANT)

You will:

Create a git-style repo structure

Implement minimal but compiling Rust code

Leave stubs where logic is intentionally deferred

Update documentation files with:

Design notes

Assumptions

Open risks

Completion status

Each phase must:

Mark itself COMPLETE / PARTIAL / BLOCKED

Write a short summary into docs/architecture.md

PHASED EXECUTION PLAN
PHASE 0 — Project Initialization
Objectives

Create the Fortify repository

Establish structure, tooling, and documentation

No business logic yet

Tasks

Create the full repo scaffold (as previously specified)

Initialize a Rust workspace

Ensure cargo build --workspace succeeds

Populate all .md files with purpose + outline

Add a top-level STATUS section to README.md

Deliverables

Compiling empty system

Clear documentation skeleton

Status Output

Update README.md with:

Phase 0: COMPLETE

PHASE 1 — OS Hardening & Installation System
Objectives

One-click deployment foundation

Defensive host posture

Tasks

Implement environment detection scripts

Implement OS hardening scripts:

sysctl

limits

permissions

Prepare systemd unit files

Write install.sh orchestration script

Document all changes in docs/hardening.md

Constraints

No destructive defaults

Clearly commented shell scripts

Assume minimal VPS

Status Output

Mark Phase 1 in docs/hardening.md

Update README.md

PHASE 2 — Core Logic & Trust Model
Objectives

Define trust tiers

Define session lifecycle

No networking yet

Tasks

Implement fortify-core

Define:

Trust levels

Session tokens (signed, short-lived)

Promotion/demotion rules (logic only)

No Tor, no HTTP

Documentation

Update docs/trust-levels.md

Update docs/threat-model.md

Status Output

Phase 2 summary in architecture.md

PHASE 3 — Gate System (Slow, Brutal Entry)
Objectives

Initial user verification

Time-expensive checks allowed

Tasks

Implement fortify-gate

Server-side captcha generation

Proof-of-Work placeholder

Promotion token issuance

Static HTML templates only

Constraints

No JS

HTML + forms only

Intentional delay acceptable

Status Output

Mark Phase 3 in docs/architecture.md

PHASE 4 — HTTP & Proxy Layer
Objectives

Fast path for verified users

Minimal inspection

Tasks

Implement fortify-http

Reverse proxy stubs

Request caps

Backpressure handling

Constraints

No buffering large bodies

No unbounded queues

PHASE 5 — Orchestrators & Mirror Rotation
Objectives

Disposable public entry points

Burn & replace logic

Tasks

Implement fortify-orchestrator

Mirror registry

Burn detection (heuristic stub)

Migration challenge page

Documentation

Update docs/architecture.md

Update docs/scaling-model.md

PHASE 6 — Node System (Healthy / Threat)
Objectives

Traffic separation

Silent demotion

Tasks

Implement fortify-node

Healthy mode forwarding stub

Threat mode enforcement stub

Session reclassification hooks

PHASE 7 — Controller & Scaling Logic
Objectives

Lifecycle ownership

Resource-aware scaling

Tasks

Implement fortify-controller

Spawn/kill logic

Resource governor hooks

Safe shutdown paths

PHASE 8 — Community / Discovery Network (Optional)
Objectives

Daisy-chained discovery

Opt-in only

Tasks

Implement fortify-community

Seed registry

Signed membership records

/Community static page

Constraints

Discovery ≠ trust

Never bypass gate

PHASE 9 — Integration, Testing, Sanity Checks
Objectives

Ensure system coherence

Catch obvious design flaws

Tasks

Integration test stubs

Abuse scenario notes

Fuzz test placeholders

Final doc updates

FINAL REQUIREMENTS

When complete:

cargo build --workspace passes

No JavaScript exists

No offensive tooling exists

Docs clearly explain:

What works

What is stubbed

What is risky

README.md contains a phase checklist

OUTPUT RULES

Do not narrate your thinking

Do not ask questions

Do not add features beyond scope

Document assumptions instead of asking

FAILURE HANDLING

If something cannot be safely implemented:

Mark the phase as PARTIAL

Explain why in documentation

Continue to next phase

BEGIN EXECUTION

Start with PHASE 0 and proceed sequentially.