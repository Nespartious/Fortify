#!/bin/bash
set -e

echo "=== Fortify Tor State Reset ==="
echo "Stopping all Fortify processes..."
pkill -f "fortify-" || true
pkill -f "onion_proxy.py" || true

echo "Stopping Tor daemon..."
if [ -f /tmp/fortify/tor/tor.pid ]; then
    kill "$(cat /tmp/fortify/tor/tor.pid)" 2>/dev/null || true
fi
pkill -f "tor -f /tmp/fortify/tor/torrc" || true
sleep 2

echo "Cleaning Tor data directory (preserving keys in mirrors/)..."
# Preserves keys in /tmp/fortify/tor/mirrors
rm -rf /tmp/fortify/tor/data
rm -f /tmp/fortify/tor/tor.pid
rm -f /tmp/fortify/tor/control_auth_cookie

echo "Done. Please run './scripts/dev-run.sh' to deploy the stack."
