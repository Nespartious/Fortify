# OS Hardening

## Overview

Fortify deployments require a hardened Linux host. This document outlines security configurations applied during installation.

## Kernel Hardening (sysctl)

### Network Stack

```conf
# IP forwarding disabled (not a router)
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# Ignore ICMP redirects
net.ipv4.conf.all.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0

# Do not send ICMP redirects
net.ipv4.conf.all.send_redirects = 0

# Ignore source-routed packets
net.ipv4.conf.all.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0

# SYN cookies (SYN flood protection)
net.ipv4.tcp_syncookies = 1

# TCP hardening
net.ipv4.tcp_max_syn_backlog = 2048
net.ipv4.tcp_synack_retries = 2
net.ipv4.tcp_syn_retries = 5

# Log suspicious packets
net.ipv4.conf.all.log_martians = 1

# Reverse path filtering
net.ipv4.conf.all.rp_filter = 1
```

### Memory Protection

```conf
# Address space layout randomization
kernel.randomize_va_space = 2

# Restrict dmesg access
kernel.dmesg_restrict = 1

# Restrict kernel pointer exposure
kernel.kptr_restrict = 2

# Disable kexec (prevent kernel replacement)
kernel.kexec_load_disabled = 1
```

### Process Restrictions

```conf
# Core dump restrictions
fs.suid_dumpable = 0
kernel.core_uses_pid = 1

# Restrict ptrace (debugging) to same user
kernel.yama.ptrace_scope = 1
```

## Resource Limits

### File: /etc/security/limits.conf

```conf
# Max open files
*    soft nofile 65536
*    hard nofile 65536

# Max processes
*    soft nproc  4096
*    hard nproc  4096

# Core dumps disabled
*    soft core   0
*    hard core   0
```

## Firewall Rules (iptables)

### Default Policy: DROP

```bash
# Default drop all
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT DROP

# Allow loopback
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Allow established/related connections
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow SSH (if needed for management)
iptables -A INPUT -p tcp --dport 22 -m state --state NEW -j ACCEPT

# Allow Tor SOCKS (localhost only)
iptables -A INPUT -i lo -p tcp --dport 9050 -j ACCEPT

# Allow Tor control port (localhost only)
iptables -A INPUT -i lo -p tcp --dport 9051 -j ACCEPT

# Allow outbound Tor connections
iptables -A OUTPUT -p tcp --dport 9001 -j ACCEPT
iptables -A OUTPUT -p tcp --dport 9030 -j ACCEPT

# Allow DNS (for initial setup)
iptables -A OUTPUT -p udp --dport 53 -j ACCEPT

# Rate limiting for new connections
iptables -A INPUT -p tcp --syn -m limit --limit 1/s --limit-burst 3 -j ACCEPT
iptables -A INPUT -p tcp --syn -j DROP
```

## Service Hardening

### SSH (if enabled)
```conf
# /etc/ssh/sshd_config
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
X11Forwarding no
MaxAuthTries 3
ClientAliveInterval 300
ClientAliveCountMax 2
```

### Tor
```conf
# /etc/tor/torrc (base config)
SocksPort 127.0.0.1:9050
ControlPort 127.0.0.1:9051
CookieAuthentication 1
DataDirectory /var/lib/tor
Log notice syslog

# Fortify-specific settings added by install script
```

## File System Permissions

### Fortify directories
```bash
/opt/fortify/           - root:fortify, 750
/opt/fortify/bin/       - root:fortify, 750
/opt/fortify/config/    - root:fortify, 750
/etc/fortify/           - root:fortify, 750
/var/log/fortify/       - fortify:fortify, 750
/var/run/fortify/       - fortify:fortify, 750
```

### Sensitive files
```bash
config/*.toml           - root:fortify, 640
secrets/                - root:fortify, 700
signing keys            - root:fortify, 600
```

## User/Group Isolation

### Fortify user
- Dedicated user: `fortify`
- Dedicated group: `fortify`
- No login shell: `/usr/sbin/nologin`
- No home directory persistence
- Minimal supplementary groups

### Capability restrictions
- CAP_NET_BIND_SERVICE (if binding <1024)
- No other capabilities

## Systemd Service Hardening

```ini
[Service]
Type=simple
User=fortify
Group=fortify

# Filesystem restrictions
PrivateTmp=yes
ProtectHome=yes
ProtectSystem=strict
ReadWritePaths=/var/log/fortify /var/run/fortify
NoNewPrivileges=yes

# Network restrictions
PrivateNetwork=no
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6

# Kernel restrictions
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryMax=2G
CPUQuota=200%

# Security
NoNewPrivileges=yes
SecureBits=noroot-locked
```

## Audit Logging

### auditd rules
```conf
# Monitor Fortify binaries
-w /opt/fortify/bin/ -p x -k fortify_exec

# Monitor config changes
-w /etc/fortify/ -p wa -k fortify_config

# Monitor Tor config
-w /etc/tor/ -p wa -k tor_config
```

## Automatic Updates

### Security patches
- Unattended-upgrades enabled
- Security updates only
- Automatic reboot: 03:00 if required

### Fortify updates
- Manual process
- Changelog review required
- Staged deployment recommended

## Monitoring & Alerting

### Critical alerts
- Failed login attempts >5
- Privilege escalation attempts
- Kernel panics
- Service crashes
- Resource exhaustion

### Log retention
- Syslog: 30 days
- Fortify logs: 7 days
- Audit logs: 90 days

## Compliance Checks

### Post-installation verification
```bash
# Kernel hardening
sysctl -a | grep -E "net.ipv4|kernel" | grep -v "0$"

# Service status
systemctl status fortify-*

# Firewall rules
iptables -L -n -v

# File permissions
find /opt/fortify -type f -ls
find /etc/fortify -type f -ls

# User configuration
id fortify
groups fortify
```

## Phase 1 Implementation Status

### Completed Features
- ✓ Environment detection with minimum requirements validation
- ✓ Comprehensive system checks (CPU, memory, disk, network)
- ✓ OS hardening with sysctl kernel parameters
- ✓ Resource limits configuration
- ✓ Basic firewall rules (iptables)
- ✓ Automatic security updates configuration
- ✓ Tor daemon setup and validation
- ✓ Hidden service directory creation
- ✓ Configuration backup system
- ✓ Service user creation and permissions
- ✓ Systemd service installation
- ✓ Comprehensive error handling and logging

### Verification Steps

After installation, verify the hardening:

```bash
# 1. Check environment detection
cd /path/to/fortify/install
sudo bash detect_env.sh

# 2. Verify kernel parameters
sudo sysctl -a | grep fortify

# 3. Check Tor status
sudo systemctl status tor
sudo journalctl -u tor -n 20

# 4. Verify file permissions
ls -la /opt/fortify/
ls -la /etc/fortify/
ls -la /var/log/fortify/

# 5. Check firewall rules
sudo iptables -L -n -v

# 6. Verify fortify user
id fortify
sudo -u fortify id
```

### Manual Hardening Steps (Post-Install)

The automated installation provides a secure baseline. For production deployments, additionally configure:

1. **Full Firewall Rules**: Implement complete iptables configuration from examples above
2. **SELinux/AppArmor**: Enable and configure mandatory access control
3. **Fail2ban**: Install and configure brute-force protection
4. **SSH Hardening**: Disable password auth, use key-based only
5. **Network Segmentation**: Isolate Fortify host on dedicated VLAN
6. **Monitoring**: Deploy intrusion detection (AIDE, OSSEC, etc.)
7. **Backup**: Configure automated encrypted backups
8. **Audit Logging**: Enable and configure auditd rules

### Installation Logs

Installation creates backups in `/var/backups/fortify-TIMESTAMP/`:
- sysctl.conf.bak
- limits.conf.bak
- iptables.rules.bak

Review these if rollback is needed.

## Known Limitations

- **No SELinux/AppArmor**: Not configured by default (manual setup supported)
- **No encrypted filesystems**: Operator responsibility
- **No TPM/secure boot**: Not required but recommended
- **No DDoS protection**: Requires external infrastructure

## Additional Recommendations

- Deploy on dedicated hardware or VPS
- Use full-disk encryption
- Enable SELinux or AppArmor (manual)
- Regular security audits
- Maintain separate admin access (VPN or jump host)
- Consider using Qubes OS or similar
- Never run other services on same host
