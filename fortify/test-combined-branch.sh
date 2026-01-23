#!/bin/bash
# =============================================================================
# Fortify Combined Branch Test Script
# Branch: feature/combined-templates-branding
# Testing: Static Templates Engine + HTML Branding Sprint
# =============================================================================

set -e
cd "$(dirname "$0")"

echo "=============================================="
echo "  FORTIFY COMBINED BRANCH TEST"
echo "  Branch: feature/combined-templates-branding"
echo "=============================================="
echo ""

# Check current branch
BRANCH=$(git branch --show-current)
echo "📍 Current branch: $BRANCH"
echo ""

# Show what's being tested
echo "🧪 FEATURES BEING TESTED:"
echo "   1. Static Templates Engine (compile-time HTML loading)"
echo "   2. Pre-rendered CAPTCHA pages with base64 images"
echo "   3. HTML branding placeholders ({{SERVICE_NAME}}, colors, etc.)"
echo "   4. busy.html with CSS-only 20-second delay (NO JavaScript)"
echo ""

# Kill any existing Fortify processes
echo "🔄 Stopping any existing Fortify processes..."
pkill -f "fortify" 2>/dev/null || true
pkill -f "fortify-tui" 2>/dev/null || true
sleep 1

# Run the TUI
echo "🚀 Launching Fortify TUI..."
echo ""
echo "=============================================="
echo "  MANUAL TEST CHECKLIST:"
echo "=============================================="
echo ""
echo "  [ ] TUI launches without errors"
echo "  [ ] Can navigate menus with arrow keys"
echo "  [ ] Status displays correctly"
echo "  [ ] No JavaScript in any served pages"
echo "  [ ] Branding colors show correctly"
echo ""
echo "  Press 'q' to quit the TUI when done testing"
echo "=============================================="
echo ""

# Launch the TUI
./target/release/fortify
