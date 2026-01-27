# Kernel and Tor Guardrails: Immediate Implementation Plan

## Purpose

This document provides a step-by-step plan to harden a Tor onion service host using kernel-level (sysctl, iptables/nftables) and Tor daemon configuration guardrails. It includes rationale, recommended values, and actionable before/after code blocks for each change. No application code changes are required, but code/config locations are identified for clarity.

---

## 1. TCP / Socket Guardrails (sysctl)

### Recommended sysctl Settings

```
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 4096
net.ipv4.tcp_synack_retries = 3
net.ipv4.tcp_syn_retries = 3
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_tw_reuse = 1
net.ipv4.ip_local_port_range = 10000 65535
```

**Why:**
- Faster failure on half-open connections
- Prevent TIME_WAIT buildup
- Avoid local port exhaustion under load

#### Before (e.g., /etc/sysctl.conf):
```ini
# ...existing code...
```

#### After:
```ini
# ...existing code...
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 4096
net.ipv4.tcp_synack_retries = 3
net.ipv4.tcp_syn_retries = 3
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_tw_reuse = 1
net.ipv4.ip_local_port_range = 10000 65535
```

---

## 2. Connection Tracking Limits (sysctl)

```
net.netfilter.nf_conntrack_max = 262144
net.netfilter.nf_conntrack_tcp_timeout_established = 600
net.netfilter.nf_conntrack_tcp_timeout_syn_recv = 30
```

**Why:**
- Prevent conntrack table overflow (which kills the box)
- Short-lived junk connections get evicted quickly

#### Before (e.g., /etc/sysctl.conf):
```ini
# ...existing code...
```

#### After:
```ini
# ...existing code...
net.netfilter.nf_conntrack_max = 262144
net.netfilter.nf_conntrack_tcp_timeout_established = 600
net.netfilter.nf_conntrack_tcp_timeout_syn_recv = 30
```

---

## 3. iptables / nftables Guardrails

### SYN Rate Limiting (Tor-friendly)

**Concept:** Allow bursts, clamp sustained floods.

#### Example iptables rule:
```bash
iptables -A INPUT -p tcp --syn --dport <onion_port> -m limit --limit 30/second --limit-burst 200 -j ACCEPT
iptables -A INPUT -p tcp --syn --dport <onion_port> -j DROP
```

#### Before (iptables rules):
```bash
# ...existing rules...
```

#### After:
```bash
# ...existing rules...
iptables -A INPUT -p tcp --syn --dport <onion_port> -m limit --limit 30/second --limit-burst 200 -j ACCEPT
iptables -A INPUT -p tcp --syn --dport <onion_port> -j DROP
```

### Concurrent Connection Cap

#### Example iptables rule (connlimit):
```bash
iptables -A INPUT -p tcp --dport <onion_port> -m connlimit --connlimit-above 5000 -j REJECT
```

#### Before:
```bash
# ...existing rules...
```

#### After:
```bash
# ...existing rules...
iptables -A INPUT -p tcp --dport <onion_port> -m connlimit --connlimit-above 5000 -j REJECT
```

---

## 4. Dynamic Kernel Blocking (Emergency Only)

- Use temporary blocklists for emergency flood mitigation or known-bad patterns.
- TTL should be seconds to minutes, never for long-lived reputation.

#### Example (temporary block):
```bash
iptables -I INPUT -s <bad_ip> -j DROP
# Remove after a short TTL
```

---

## 5. Tor Daemon Tuning

### Onion-Service–Specific Limits (torrc)

```
MaxStreamsPerCircuit 4
MaxClientCircuitsPending 32
CircuitBuildTimeout 30
```

#### Before (e.g., /etc/tor/torrc):
```ini
# ...existing code...
```

#### After:
```ini
# ...existing code...
MaxStreamsPerCircuit 4
MaxClientCircuitsPending 32
CircuitBuildTimeout 30
```

### Stream / Connection Timeouts

```
ClientIdleTimeout 60
ClientTransportPluginTimeout 30
```

#### Before (e.g., /etc/tor/torrc):
```ini
# ...existing code...
```

#### After:
```ini
# ...existing code...
ClientIdleTimeout 60
ClientTransportPluginTimeout 30
```

### Intro Point Pressure Relief

```
NumIntroductionPoints 3
```

#### Before (e.g., /etc/tor/torrc):
```ini
# ...existing code...
```

#### After:
```ini
# ...existing code...
NumIntroductionPoints 3
```

### Logging (Minimal and Buffered)
- Avoid excessive logging under attack.
- Use minimal, buffered logging in torrc:

#### Example:
```ini
Log notice file /var/log/tor/notices.log
```

---

## 6. Code/Config Locations to Review

- `/etc/sysctl.conf` (or `/etc/sysctl.d/` custom file)
- `/etc/tor/torrc`
- iptables/nftables rules (typically in `/etc/iptables/rules.v4` or managed by firewall scripts)

No application code changes are required, but review any deployment scripts (e.g., `install/harden_os.sh`, `install/tor_setup.sh`) to ensure these settings are applied automatically.

---

## Summary

Apply these guardrails immediately for a hardened, production-ready Tor onion service. Use the before/after code blocks above to update your configs and scripts. For dynamic kernel blocking, only use as a last resort and always with a short TTL.
