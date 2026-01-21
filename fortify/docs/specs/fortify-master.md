Fortify — Updated Architecture Write-Up
Project Name

Fortify

Purpose

A defensive, quick-deploy protection layer for Tor hidden services that:

Keeps the real onion address permanently secret

Absorbs and filters hostile traffic

Scales safely within hardware limits

Optionally participates in a decentralized discovery network

Core Design Principle (Unchanged)

The real hidden service is never directly exposed, never public, and never connected to by any public-facing address.

Everything else exists to preserve that rule.

Address Topology (Updated)
1. Public Entry Layer (Mirrored Orchestrators)

Instead of a single public orchestrator, Fortify may generate multiple public entry addresses (mirrors).

Why this is worth it

Onion addresses do get burned under sustained attack

Tor circuits can degrade or stall

Attackers often pin traffic to a known onion

Rotatable entry points materially improve survivability.

Behavior

Multiple Public Orchestrator Addresses exist at once

All orchestrators are functionally identical

Each orchestrator:

Reverse proxies traffic

Load balances to nodes

Classifies sessions

Never connects to the real hidden service

2. Orchestrator Death & Rotation Logic

If an orchestrator mirror becomes:

Overloaded

Targeted by sustained abuse

Functionally degraded

Then Fortify may:

Mark the mirror as “burned”

Gracefully drain or hard-drop traffic

Spawn a replacement mirror onion

Update internal routing tables

User Experience During Rotation

Users arriving at a dead mirror see:

A hard captcha challenge

A message indicating the mirror is no longer valid

On successful captcha:

Session is migrated to a healthy orchestrator

New mirror address is issued for that session

This prevents automated migration and forces human verification.

Traffic Handling (Clarified)
Traffic Classes

Unknown / First-Time

Verified / Healthy

Degraded / Suspicious

Flow

User connects to any public orchestrator

Orchestrator classifies session

Routing decision:

Unknown → Threat Nodes

Verified → Healthy Nodes

Only Healthy Nodes forward traffic to the real hidden service

Node Types (Unchanged, clarified)
A. Healthy Nodes

Proxy trusted sessions

Forward traffic to the real hidden service

Load-balanced

Scaled based on traffic and hardware constraints

B. Threat Nodes

Handle:

First-time visitors

Reclassified sessions

Migration from dead mirrors

Enforce:

Captcha on every page load

No long-lived trust

Promotion only after successful challenges

Scaling Model (Same idea, clearer framing)

Scaling is adaptive and capped.

Example targets (illustrative, not fixed):

~1k visits/day → 5 healthy / 2 threat

~5k visits/day → 10 healthy / 4 threat

~25k visits/day → 50 healthy / 10 threat

Rules:

Node ratios configurable (e.g., 70/30 healthy/threat)

Hard cap at ~75% system resource utilization

Excess traffic is throttled, not accepted

Community / Discovery Network (Updated)
Core Idea

A daisy-chained, decentralized discovery network with no central authority.

This is powerful — and risky if done sloppily — so boundaries matter.

Seed Addresses (New Concept)

Each discoverable Fortify instance may expose:

One or more Seed Addresses

A seed address allows:

Joining that member’s network

Automatic discovery of the entire connected network

Network Behavior

Any user opting into discoverability:

Hosts a /Community page

Shares network registry data

Networks are transitive:

If A trusts B

And B trusts C

Then A, B, and C become part of the same network

This creates an ever-expanding, daisy-chained network graph.

Community Page Behavior

Every network member hosts the same community listing

Lists:

Member onion addresses (public entry only)

Optional metadata (operator-defined)

Visiting any member exposes the entire connected network

This is:

Explicit

Opt-in

Irreversible once joined (by design)

Security Constraints

Community participation is never default

Community traffic never bypasses Fortify protections

Real hidden service addresses are never shared, even internally

Network registry is signed and verified to prevent poisoning

What This Adds (Honest Assessment)
Multiple Public Orchestrators

Yes, worth it

Improves uptime

Enables burn-and-replace strategy

Makes automated attacks more expensive

Daisy-Chained Community Network

Powerful but dangerous

Discovery grows fast

One bad actor pollutes the graph

Needs trust scoring, revocation, or segmentation later

For v1: acceptable if framed as experimental and opt-in only.