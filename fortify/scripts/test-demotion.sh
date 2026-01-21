#!/bin/bash
# Test Violation/Demotion System
# This script fires rapid requests to trigger rate limiting
# and watches the Node logs for violation detection

set -e

# Configuration
MIRROR="${1:-yjbsvhvt3chprhtu3vd7wwaiwjzqs3uqpel23xqjk4ofcvtbmumaiead.onion}"
TOKEN="$2"

if [ -z "$TOKEN" ]; then
    echo "Usage: $0 <mirror> <fortify_session_token>"
    echo ""
    echo "Example:"
    echo "  $0 yjbsvhvt3chprhtu3vd7wwaiwjzqs3uqpel23xqjk4ofcvtbmumaiead.onion 'eyJzZXNzaW9uX2lkIjo...'"
    echo ""
    echo "To get a token:"
    echo "  1. Visit a mirror in Tor Browser"
    echo "  2. Solve the captcha"
    echo "  3. Open Developer Tools (F12)"
    echo "  4. Go to Storage > Cookies"
    echo "  5. Copy the 'fortify_session' cookie value"
    exit 1
fi

echo "=== Fortify Demotion Test ==="
echo "Mirror: $MIRROR"
echo "Token (first 50 chars): ${TOKEN:0:50}..."
echo ""

# Rate limit is now 20 requests/minute in Healthy mode
# We need >20 requests to trigger rate limiting
# Each rate limit violation = 1 violation
# Threshold = 3 violations for demotion

echo "Step 1: Checking Node metrics before test..."
echo "----------------------------------------"
curl -s http://127.0.0.1:9100/metrics 2>/dev/null || echo "(Node 9100 metrics unavailable)"
echo ""

echo "Step 2: Firing 30 rapid requests (rate limit is 20/minute)..."
echo "----------------------------------------"
echo "This should trigger ~10 rate limit violations..."
echo ""

for i in {1..30}; do
    # Use synchronous requests to be more predictable
    HTTP_CODE=$(curl -s --max-time 5 --socks5-hostname 127.0.0.1:9150 \
        -b "fortify_session=$TOKEN" \
        -o /dev/null -w "%{http_code}" \
        "http://$MIRROR/" 2>/dev/null || echo "000")
    
    echo "Request $i: HTTP $HTTP_CODE"
    
    # Stop if we get a 403 (demotion response)
    if [ "$HTTP_CODE" = "403" ]; then
        echo ""
        echo "*** GOT 403 FORBIDDEN - DEMOTION TRIGGERED! ***"
        break
    fi
done

echo ""
echo "Step 3: Checking Node metrics after test..."
echo "----------------------------------------"
curl -s http://127.0.0.1:9100/metrics 2>/dev/null || echo "(Node 9100 metrics unavailable)"
echo ""

echo "Step 4: Try one more request to see the warning page..."
echo "----------------------------------------"
echo "Response body (first 500 chars):"
curl -s --max-time 10 --socks5-hostname 127.0.0.1:9150 \
    -b "fortify_session=$TOKEN" \
    "http://$MIRROR/" 2>/dev/null | head -c 500
echo ""
echo ""

echo "=== Test Complete ==="
echo ""
echo "What to check:"
echo "  1. Look at the dev-run.sh terminal for WARN messages about violations"
echo "  2. Look for 'RATE LIMITED' and 'VIOLATION recorded' messages"
echo "  3. Look for 'redirecting to Gate' messages"
echo "  4. In your browser, refresh the mirror - you should see the warning page"
echo "     and your cookie should be cleared"
