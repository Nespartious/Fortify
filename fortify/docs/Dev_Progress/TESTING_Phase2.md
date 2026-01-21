# Manual Testing Guide - Phase 2: Session Protection

**Date:** January 20, 2026  
**Status:** Ready for Testing  
**Deployment:** Use `./target/release/fortify` for manual testing

---

## Prerequisites

1. Build the release version:
   ```bash
   cd /home/shadowbox/Fortify/Fortify/fortify
   cargo build --release
   ```

2. Ensure you have:
   - Gate service (port 8081)
   - HTTP proxy service (port 8080)
   - At least one backend node configured

---

## Test Suite

### Test 1: Normal User Flow (Happy Path)

**Purpose:** Verify the full token upgrade flow works for legitimate users

**Steps:**
1. Start Fortify:
   ```bash
   ./target/release/fortify
   ```

2. Open a browser (Tor Browser recommended) and navigate to:
   ```
   http://127.0.0.1:8080/
   ```

3. You should be redirected to the Gate CAPTCHA page

4. Solve the CAPTCHA

5. After solving, check your browser cookies (F12 → Application → Cookies):
   - Should see `fortify_verification` cookie (60s expiry)
   - Should NOT see `fortify_session` cookie yet

6. Navigate to any page on the site (e.g., `/test`)

7. Check cookies again:
   - `fortify_verification` cookie should be DELETED
   - `fortify_session` cookie should now be present (24h expiry)

8. Refresh the page multiple times
   - Should access site normally
   - No CAPTCHA prompts

**Expected Logs:**
```
[fortify-gate] Generated CAPTCHA for session <sid>
[fortify-gate] CAPTCHA verified for session <sid>
[fortify-gate] Created verification token <token_id>
[fortify-http] Found verification token, attempting upgrade
[fortify-gate] Upgraded verification token <token_id> to session
[fortify-http] Successfully upgraded verification token to session <sid>
[fortify-http] HEALTHY PATH: Routing Verified user to backend
```

**Pass Criteria:**
- ✅ User solves CAPTCHA once
- ✅ Verification token issued
- ✅ First request upgrades to session token
- ✅ Subsequent requests use session token
- ✅ No CAPTCHA re-prompts

---

### Test 2: Verification Token Expiry

**Purpose:** Verify verification tokens expire after 60 seconds

**Steps:**
1. Start Fortify
2. Navigate to `http://127.0.0.1:8080/`
3. Solve CAPTCHA
4. Check cookies - `fortify_verification` cookie present
5. **WAIT 65 seconds** (do not navigate)
6. After 65 seconds, try to navigate to any page

**Expected Behavior:**
- Token expired → redirected back to CAPTCHA page
- Must solve CAPTCHA again

**Expected Logs:**
```
[fortify-gate] Verification token expired: <token_id>
[fortify-http] Failed to upgrade verification token
[fortify-http] THREAT PATH: Proxying Unknown user to Gate
```

**Pass Criteria:**
- ✅ Token expires after 60 seconds
- ✅ User redirected to CAPTCHA
- ✅ No token upgrade allowed

---

### Test 3: Verification Token Replay Attack

**Purpose:** Verify verification tokens are single-use only

**Steps:**
1. Start Fortify
2. Open browser and navigate to `http://127.0.0.1:8080/`
3. Solve CAPTCHA
4. Extract `fortify_verification` cookie value (F12 → Application → Cookies)
5. Navigate to `/test` (this uses the token once)
6. Open a NEW incognito/private window
7. Manually set the same `fortify_verification` cookie in new window (use browser console):
   ```javascript
   document.cookie = "fortify_verification=<PASTE_TOKEN_HERE>; path=/";
   ```
8. Try to navigate to `/test` in the new window

**Expected Behavior:**
- Second use fails → "Token already used" error
- Redirected to CAPTCHA page

**Expected Logs:**
```
[fortify-gate] Verification token already used: <token_id>
[fortify-http] Failed to upgrade verification token
```

**Pass Criteria:**
- ✅ First use succeeds
- ✅ Second use fails with "already used" error
- ✅ Attacker cannot reuse stolen verification token

---

### Test 4: User-Agent Mismatch Detection

**Purpose:** Verify tokens are bound to User-Agent

**Steps:**
1. Start Fortify
2. Use `curl` to get a verification token:
   ```bash
   # Get CAPTCHA page
   curl -c cookies.txt http://127.0.0.1:8080/
   
   # Solve CAPTCHA manually via browser, copy the verification token
   ```

3. Try to use the token with a DIFFERENT User-Agent:
   ```bash
   curl -b "fortify_verification=<TOKEN>" \
        -H "User-Agent: DifferentBrowser/1.0" \
        http://127.0.0.1:8080/test
   ```

**Expected Behavior:**
- Upgrade fails → "User-Agent mismatch" error
- Redirected to CAPTCHA

**Expected Logs:**
```
[fortify-gate] User-Agent mismatch for token <token_id>
[fortify-http] Failed to upgrade verification token
```

**Pass Criteria:**
- ✅ Token upgrade fails with different User-Agent
- ✅ Attacker cannot use token from different device/browser

---

### Test 5: Session Token User-Agent Validation

**Purpose:** Verify session tokens validate User-Agent on every request

**Steps:**
1. Start Fortify
2. Get a valid session token (complete Test 1)
3. Extract `fortify_session` cookie value
4. Try to use it with a different User-Agent:
   ```bash
   curl -b "fortify_session=<SESSION_TOKEN>" \
        -H "User-Agent: AttackerBrowser/1.0" \
        http://127.0.0.1:8080/test
   ```

**Expected Behavior:**
- Request fails → "User-Agent mismatch"
- User redirected to CAPTCHA for re-verification

**Expected Logs:**
```
[fortify-http] User-Agent mismatch for session <sid>: token requires different UA
[fortify-http] THREAT PATH: Proxying Unknown user to Gate
```

**Pass Criteria:**
- ✅ Session token rejected with wrong User-Agent
- ✅ User must re-verify with correct browser

---

### Test 6: Session Cloning Detection

**Purpose:** Verify rapid concurrent requests trigger cloning detection

**Steps:**
1. Start Fortify
2. Get a valid session token (complete Test 1)
3. Use a script to send rapid concurrent requests:
   ```bash
   # Save session token
   SESSION="<paste_your_session_token>"
   
   # Send 10 requests in parallel (< 100ms apart)
   for i in {1..10}; do
     curl -b "fortify_session=$SESSION" \
          http://127.0.0.1:8080/test &
   done
   wait
   ```

**Expected Logs:**
```
[fortify-http] CLONING DETECTED: Session <sid> made requests <X>ms apart
```

**Pass Criteria:**
- ✅ Cloning warning logged for requests < 100ms apart
- ✅ Requests still succeed (non-blocking detection)
- ✅ Admin can review logs to identify cloned sessions

**Note:** This is a detection mechanism, not blocking. Future enhancement can auto-demote cloned sessions.

---

### Test 7: Token Cleanup Task

**Purpose:** Verify expired verification tokens are cleaned from cache

**Steps:**
1. Start Fortify
2. Solve 5-10 CAPTCHAs (generate verification tokens)
3. Wait 90 seconds (let tokens expire)
4. Check logs for cleanup activity

**Expected Logs (every 30 seconds):**
```
[fortify-gate] Cleaned up X expired verification tokens
[fortify-gate] Verification token cache: Y tokens remaining
```

**Pass Criteria:**
- ✅ Cleanup task runs every 30 seconds
- ✅ Expired tokens removed from cache
- ✅ Memory doesn't grow unbounded

---

### Test 8: Backend Load Testing

**Purpose:** Verify the system handles normal load without issues

**Steps:**
1. Start Fortify
2. Get a valid session token
3. Send sustained requests (simulate normal browsing):
   ```bash
   SESSION="<paste_session_token>"
   
   # 100 requests over 60 seconds (normal user pace)
   for i in {1..100}; do
     curl -b "fortify_session=$SESSION" \
          http://127.0.0.1:8080/test
     sleep 0.6
   done
   ```

**Expected Behavior:**
- All requests succeed
- No CAPTCHA re-prompts
- Stable response times

**Expected Logs:**
```
[fortify-http] HEALTHY PATH: Routing Verified user to backend
[fortify-http] Valid session token for session <sid>
```

**Pass Criteria:**
- ✅ 100% success rate
- ✅ No unexpected errors
- ✅ Session persists throughout test

---

## Attack Simulation Tests

### Attack Test 1: CAPTCHA Farming Prevention

**Scenario:** Attacker solves 1 CAPTCHA and tries to distribute verification token to 100 bots

**Steps:**
1. Solve CAPTCHA, get verification token
2. Try to use the same token 100 times:
   ```bash
   TOKEN="<paste_verification_token>"
   
   for i in {1..100}; do
     echo "Request $i:"
     curl -b "fortify_verification=$TOKEN" \
          http://127.0.0.1:8080/test
   done
   ```

**Expected Behavior:**
- Request 1: Success → token upgraded to session
- Requests 2-100: Fail → "Token already used"

**Pass Criteria:**
- ✅ Only 1 bot can use the verification token
- ✅ 99 bots blocked
- ✅ CAPTCHA farming ineffective

---

### Attack Test 2: Session Cloning Prevention

**Scenario:** Attacker copies session token to 100 bot instances (like Jan 19 attack)

**Steps:**
1. Get valid session token
2. Try to clone it across different User-Agents:
   ```bash
   SESSION="<paste_session_token>"
   
   for i in {1..100}; do
     curl -b "fortify_session=$SESSION" \
          -H "User-Agent: Bot${i}/1.0" \
          http://127.0.0.1:8080/test
   done
   ```

**Expected Behavior:**
- All requests fail → User-Agent mismatch
- Cloning detection triggered

**Pass Criteria:**
- ✅ 100% of clone attempts blocked
- ✅ Attacker must solve CAPTCHA per bot (100 CAPTCHAs)
- ✅ Attack cost increased 100x

---

## Success Metrics

### Overall System Health

After all tests, verify:

1. **No Memory Leaks:**
   ```bash
   # Check cache sizes in logs
   grep "tokens remaining" logs/fortify-gate.log
   ```

2. **No Performance Degradation:**
   - Response times < 100ms for healthy path
   - CAPTCHA solve time < 2 seconds

3. **Security Effectiveness:**
   - Verification tokens: Single-use enforced
   - Session tokens: User-Agent bound
   - Cloning detection: Logs suspicious activity

---

## Troubleshooting

### Issue: "Token not found in cache"

**Cause:** Verification token expired or cleaned up  
**Solution:** Normal behavior - user must solve CAPTCHA again

---

### Issue: "User-Agent mismatch" on legitimate requests

**Cause:** User changed browser or User-Agent header  
**Solution:** User must re-verify with CAPTCHA (security working as designed)

---

### Issue: Session token rejected after service restart

**Cause:** Signature keys regenerated on restart  
**Solution:** Expected - users re-verify automatically via gate flow

---

## Deployment Checklist

Before deploying to production:

- [ ] All 8 tests pass
- [ ] Attack simulations blocked correctly
- [ ] No memory leaks observed
- [ ] Logs show cleanup tasks running
- [ ] Performance metrics acceptable
- [ ] Backup configuration files
- [ ] Document deployment timestamp

---

## Next Steps After Testing

1. If all tests pass:
   - Deploy to production using deployment wizard
   - Monitor logs for 24 hours
   - Track session cloning incidents (should be 0)

2. If issues found:
   - Document specific failures
   - Check error logs
   - Report findings for debugging

3. Future enhancements (after Phase 2 stable):
   - Implement Phase 1 (CAPTCHA HTML serving from fortify-http)
   - Add auto-demotion for cloned sessions
   - Implement rate limiting per verification token

---

**Test Conducted By:** _________________  
**Test Date:** _________________  
**Test Result:** ☐ PASS  ☐ FAIL  ☐ NEEDS REVIEW  
**Notes:**
