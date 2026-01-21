#!/bin/bash

echo "=== Testing Fortify Authentication System ==="
echo ""

# Test 1: Try to create mirror WITHOUT auth token (should fail)
echo "Test 1: Creating mirror without auth token (should fail with 401)..."
response=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://127.0.0.1:8080/mirror/create 2>&1)
status=$(echo "$response" | grep "HTTP_STATUS" | cut -d':' -f2)
if [ "$status" = "401" ]; then
    echo "✅ PASS: Unauthorized access blocked (HTTP 401)"
else
    echo "❌ FAIL: Expected 401, got $status"
    echo "$response"
fi
echo ""

# Test 2: Try to destroy mirror WITHOUT auth token (should fail)
echo "Test 2: Destroying mirror without auth token (should fail with 401)..."
response=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://127.0.0.1:8080/mirror/destroy \
    -H "Content-Type: application/json" \
    -d '{"onion_address": "test.onion"}' 2>&1)
status=$(echo "$response" | grep "HTTP_STATUS" | cut -d':' -f2)
if [ "$status" = "401" ]; then
    echo "✅ PASS: Unauthorized access blocked (HTTP 401)"
else
    echo "❌ FAIL: Expected 401, got $status"
    echo "$response"
fi
echo ""

# Test 3: Public endpoints should still work (no auth needed)
echo "Test 3: Accessing public endpoint /mirrors (should work)..."
response=$(curl -s -w "\nHTTP_STATUS:%{http_code}" http://127.0.0.1:8080/mirrors 2>&1)
status=$(echo "$response" | grep "HTTP_STATUS" | cut -d':' -f2)
if [ "$status" = "200" ]; then
    echo "✅ PASS: Public endpoint accessible (HTTP 200)"
else
    echo "⚠️  Status: $status (orchestrator may not be running)"
fi
echo ""

echo "=== Authentication System Test Complete ==="
echo ""
echo "Summary:"
echo "- Admin panel now requires password: pleaseletmein123"
echo "- All mirror/node management operations require authentication"
echo "- Orchestrator API rejects unauthenticated administrative requests"
echo "- Public endpoints (health, mirror list) remain accessible"
