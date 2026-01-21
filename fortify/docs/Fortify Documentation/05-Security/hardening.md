# 🛡️ Security Hardening Guide

> **Production-Ready Security Configuration**

---

## Security Checklist

```
┌────────────────────────────────────────────────────────────────────────────┐
│                    PRODUCTION SECURITY CHECKLIST                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  REQUIRED                                                                   │
│  ════════                                                                   │
│  [ ] Run as dedicated non-root user                                        │
│  [ ] Enable Proof-of-Work for Tor PoW abuse protection                     │
│  [ ] Configure Vanguards addon                                             │
│  [ ] Set restrictive file permissions                                      │
│  [ ] Enable kernel hardening (sysctl)                                      │
│  [ ] Configure resource limits (ulimit)                                    │
│  [ ] Use systemd sandboxing                                                │
│  [ ] Setup log rotation                                                    │
│                                                                             │
│  RECOMMENDED                                                                │
│  ═══════════                                                                │
│  [ ] Network namespace isolation                                           │
│  [ ] SELinux/AppArmor profiles                                             │
│  [ ] Separate user per component                                           │
│  [ ] Encrypted swap                                                        │
│  [ ] Disable core dumps                                                    │
│  [ ] Memory-only temporary files                                           │
│  [ ] Firewall rules (iptables/nftables)                                   │
│                                                                             │
│  PARANOID                                                                   │
│  ════════                                                                   │
│  [ ] Air-gapped key generation                                             │
│  [ ] Reproducible builds from source                                       │
│  [ ] Full disk encryption                                                  │
│  [ ] Hardware security module (HSM)                                        │
│  [ ] Multiple geographic instances                                         │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## User & Permission Setup

### Create Dedicated User

```bash
# Create fortify user (no login shell)
sudo useradd -r -s /usr/sbin/nologin fortify

# Create service directories
sudo mkdir -p /opt/fortify/{bin,config,data,logs}
sudo mkdir -p /var/lib/fortify
sudo mkdir -p /var/log/fortify

# Set ownership
sudo chown -R fortify:fortify /opt/fortify
sudo chown -R fortify:fortify /var/lib/fortify
sudo chown -R fortify:fortify /var/log/fortify

# Restrictive permissions
sudo chmod 750 /opt/fortify
sudo chmod 700 /opt/fortify/config
sudo chmod 700 /var/lib/fortify
sudo chmod 750 /var/log/fortify
```

### Permission Matrix

| Path | Owner | Mode | Purpose |
|------|-------|------|---------|
| `/opt/fortify/bin/` | root | 755 | Executables |
| `/opt/fortify/config/` | fortify | 700 | Sensitive configs |
| `/var/lib/fortify/` | fortify | 700 | Runtime data |
| `/var/log/fortify/` | fortify | 750 | Log files |
| `*.toml` | fortify | 600 | Config files |

---

## Kernel Hardening

### sysctl Configuration

Save to `/etc/sysctl.d/99-fortify.conf`:

```ini
# Network security
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.icmp_ignore_bogus_error_responses = 1
net.ipv4.tcp_timestamps = 0

# IPv6 (disable if not needed)
net.ipv6.conf.all.disable_ipv6 = 1
net.ipv6.conf.default.disable_ipv6 = 1

# Memory protection
kernel.randomize_va_space = 2
kernel.dmesg_restrict = 1
kernel.kptr_restrict = 2
kernel.yama.ptrace_scope = 2

# Disable core dumps
kernel.core_pattern = |/bin/false
fs.suid_dumpable = 0

# File descriptor limits
fs.file-max = 2097152
fs.nr_open = 2097152

# Network buffer tuning
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.core.netdev_max_backlog = 65535
net.core.somaxconn = 65535
```

Apply changes:

```bash
sudo sysctl -p /etc/sysctl.d/99-fortify.conf
```

---

## Resource Limits

### limits.conf

Save to `/etc/security/limits.d/fortify.conf`:

```ini
# File descriptor limits
fortify         soft    nofile          65535
fortify         hard    nofile          131072

# Process limits
fortify         soft    nproc           4096
fortify         hard    nproc           8192

# Memory limits (KB)
fortify         soft    memlock         unlimited
fortify         hard    memlock         unlimited

# Core dumps disabled
fortify         soft    core            0
fortify         hard    core            0

# Address space (unlimited for Tor)
fortify         soft    as              unlimited
fortify         hard    as              unlimited
```

---

## Systemd Sandboxing

### Controller Service

Save to `/etc/systemd/system/fortify-controller.service`:

```ini
[Unit]
Description=Fortify Controller Service
After=network.target tor.service
Requires=tor.service

[Service]
Type=simple
User=fortify
Group=fortify
ExecStart=/opt/fortify/bin/fortify-controller --config /opt/fortify/config/fortify.toml

# Sandboxing
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallArchitectures=native

# Allow write to specific paths
ReadWritePaths=/var/lib/fortify /var/log/fortify /run/tor

# Capabilities
CapabilityBoundingSet=
AmbientCapabilities=

# Resource limits
LimitNOFILE=65535
LimitNPROC=4096

# Restart policy
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Gate Service (Similar)

```ini
[Unit]
Description=Fortify Gate Service
After=network.target fortify-controller.service

[Service]
Type=simple
User=fortify
Group=fortify
ExecStart=/opt/fortify/bin/fortify-gate --config /opt/fortify/config/fortify.toml

# Same sandboxing as controller
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallArchitectures=native

ReadWritePaths=/var/lib/fortify /var/log/fortify

CapabilityBoundingSet=
AmbientCapabilities=

LimitNOFILE=65535
LimitNPROC=4096

Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## Tor Hardening

### torrc Security Settings

```
# Authentication
CookieAuthentication 1
CookieAuthFileGroupReadable 1

# Restrict access
ControlPort 127.0.0.1:9051
SocksPort 127.0.0.1:9050

# Directory server anonymity
UseBridges 0
FetchHidServDescriptors 1

# Sandbox (if supported)
Sandbox 1

# Logging
Log notice file /var/log/tor/notices.log
SafeLogging 1

# Resource limits
HardwareAccel 1
NumCPUs 2
MaxMemInQueues 512 MB

# Connection limits
ConnLimit 4096

# Onion service security
HiddenServiceDir /var/lib/tor/fortify
HiddenServicePort 80 127.0.0.1:8081
HiddenServiceVersion 3
HiddenServiceNumIntroductionPoints 6
```

### Vanguards Configuration

```yaml
# vanguards.yaml
control_port: 9051
control_socket: 
state_file: /var/lib/vanguards/vanguards.state

# Layer rotation
layer2_rotation_time: 1080  # 18 hours
layer3_rotation_time: 3600  # 1 hour

# Guard selection
min_layer2_guards: 4
max_layer2_guards: 8
min_layer3_guards: 8
max_layer3_guards: 16

# Anomaly detection
enable_bandguards: true
enable_rendguard: true
enable_cbtverify: true

# Logging
log_level: INFO
log_file: /var/log/vanguards/vanguards.log
```

---

## Firewall Rules

### iptables Configuration

```bash
#!/bin/bash
# /opt/fortify/firewall.sh

# Flush existing rules
iptables -F
iptables -X
iptables -t nat -F
iptables -t nat -X

# Default policies
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT DROP

# Loopback
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Established connections
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow Tor traffic (outbound only)
iptables -A OUTPUT -p tcp -m tcp --dport 9001 -j ACCEPT  # OR port
iptables -A OUTPUT -p tcp -m tcp --dport 9030 -j ACCEPT  # Dir port
iptables -A OUTPUT -p tcp -m tcp --dport 443 -j ACCEPT   # Bridges

# Allow local services
iptables -A INPUT -p tcp -s 127.0.0.1 --dport 8081 -j ACCEPT  # Gate
iptables -A INPUT -p tcp -s 127.0.0.1 --dport 8082 -j ACCEPT  # Proxy
iptables -A INPUT -p tcp -s 127.0.0.1 --dport 9100 -j ACCEPT  # Node

# Rate limiting
iptables -A INPUT -p tcp --dport 8081 -m limit --limit 100/sec --limit-burst 200 -j ACCEPT
iptables -A INPUT -p tcp --dport 8081 -j DROP

# Log dropped packets
iptables -A INPUT -j LOG --log-prefix "FORTIFY_DROP: " --log-level 4
iptables -A INPUT -j DROP

# Save rules
iptables-save > /etc/iptables/rules.v4
```

---

## Network Isolation

### Network Namespace Setup

```bash
#!/bin/bash
# Create isolated network namespace for Fortify

# Create namespace
ip netns add fortify

# Create veth pair
ip link add veth-fortify type veth peer name veth-host
ip link set veth-fortify netns fortify

# Configure host side
ip addr add 10.200.200.1/24 dev veth-host
ip link set veth-host up

# Configure namespace side
ip netns exec fortify ip addr add 10.200.200.2/24 dev veth-fortify
ip netns exec fortify ip link set veth-fortify up
ip netns exec fortify ip link set lo up

# Add default route in namespace
ip netns exec fortify ip route add default via 10.200.200.1

# Enable NAT for outbound traffic
iptables -t nat -A POSTROUTING -s 10.200.200.0/24 -j MASQUERADE
echo 1 > /proc/sys/net/ipv4/ip_forward

# Run Fortify in namespace
ip netns exec fortify /opt/fortify/bin/fortify-controller --config /opt/fortify/config/fortify.toml
```

---

## Memory Protection

### Disable Swap (or Encrypt)

```bash
# Option 1: Disable swap entirely
sudo swapoff -a
sudo sed -i '/swap/d' /etc/fstab

# Option 2: Encrypted swap
sudo cryptsetup -d /dev/urandom create cryptswap /dev/sdXN
sudo mkswap /dev/mapper/cryptswap
sudo swapon /dev/mapper/cryptswap
```

### Memory Locking

Add to Fortify startup:

```rust
// In main.rs, before any sensitive operations
use libc::{mlockall, MCL_CURRENT, MCL_FUTURE};

unsafe {
    if mlockall(MCL_CURRENT | MCL_FUTURE) != 0 {
        eprintln!("Warning: Could not lock memory");
    }
}
```

---

## Log Security

### Log Rotation

Save to `/etc/logrotate.d/fortify`:

```
/var/log/fortify/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 640 fortify fortify
    sharedscripts
    postrotate
        systemctl reload fortify-controller 2>/dev/null || true
    endscript
}
```

### Secure Logging

```bash
# Set log directory permissions
sudo chmod 750 /var/log/fortify
sudo chown fortify:fortify /var/log/fortify

# No world-readable logs
sudo chmod 640 /var/log/fortify/*.log

# Append-only logs (optional, paranoid)
sudo chattr +a /var/log/fortify/*.log
```

---

## Production Configuration

### Recommended Config for Production

```toml
[global]
environment = "production"
log_level = "warn"

[security]
# Strong session secret (generate with: openssl rand -hex 32)
session_secret = "your-64-char-hex-secret-here"

# Enable all protections
enable_pow = true
pow_difficulty = 20
enable_vanguards = true
vanguards_config = "/opt/fortify/config/vanguards.yaml"

[behavioral]
# Aggressive detection
enable_user_agent_analysis = true
enable_referer_analysis = true
enable_path_analysis = true
enable_enumeration_detection = true
enable_form_tracking = true
enable_payload_analysis = true

# Strict thresholds
max_unique_paths_per_minute = 30
max_form_submissions_per_minute = 5
max_payload_size_bytes = 5242880
enumeration_detection_threshold = 3
demotion_threshold = 5
severity_demotion_threshold = 10
kill_after_demotions = 2

[mirrors]
# Aggressive rotation
rotation_interval_seconds = 3600
max_age_hours = 24
compromise_threshold = 0.15
standby_count = 5
```

---

## Security Monitoring

```
┌────────────────────────────────────────────────────────────────────────────┐
│                    SECURITY MONITORING CHECKLIST                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  DAILY                                                                      │
│  ═════                                                                      │
│  [ ] Review demotion logs                                                  │
│  [ ] Check for repeated violations                                         │
│  [ ] Monitor bandwidth anomalies                                           │
│  [ ] Verify mirror rotation occurred                                       │
│                                                                             │
│  WEEKLY                                                                     │
│  ══════                                                                     │
│  [ ] Review killed sessions                                                │
│  [ ] Analyze attack patterns                                               │
│  [ ] Check Vanguards status                                                │
│  [ ] Update attack path patterns                                           │
│  [ ] Review admin panel access logs                                        │
│                                                                             │
│  MONTHLY                                                                    │
│  ═══════                                                                    │
│  [ ] Rotate session secret                                                 │
│  [ ] Update Tor to latest version                                          │
│  [ ] Review and update firewall rules                                      │
│  [ ] Security audit of configurations                                      │
│  [ ] Test backup and recovery procedures                                   │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

*For emergency procedures, see [Incident Response](incident-response.md)*
