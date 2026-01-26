# Fortify Admin Panel Security Implementation

## Overview
Password protection has been added to all administrative functions in Fortify to prevent unauthorized remote access to mirror and node management operations.

## Changes Made

### 1. Admin Panel Authentication (`fortify-http/src/admin.rs`)

#### Password Protection
- **Password**: `pleaseletmein123`
- **Session Duration**: 24 hours
- **Storage**: Cookie-based sessions with HttpOnly flag

#### New Features
- **Login Page**: `/ctrl_8f7k3m9x2n4p1q6w5v0b8c/login`
- **Logout**: POST to `/ctrl_8f7k3m9x2n4p1q6w5v0b8c/logout`
- **Session Management**: Automatic cleanup of expired sessions
- **Authentication Check**: All admin routes now require valid session cookie

#### Protected Routes
All administrative functions now require authentication:
- Dashboard and session management
- Node creation/modification/deletion
- Mirror creation/pause/resume/destroy
- Behavior settings
- Captcha configuration

### 2. API Token Authentication (`fortify-orchestrator/src/server.rs`)

#### Token-Based Security
- **Auth Header**: `X-Fortify-Admin-Token`
- **Token Generation**: Hash-based derivation from admin password
- **Validation**: Required for all administrative API endpoints

#### Protected Endpoints
- `POST /mirror/create` - Create new mirror
- `POST /mirror/create-standby` - Create standby mirror
- `POST /mirror/activate` - Activate standby mirror
- `POST /mirror/pause` - Pause active mirror
- `POST /mirror/resume` - Resume paused mirror
- `POST /mirror/destroy` - Permanently destroy mirror

#### Public Endpoints (No Auth Required)
- `GET /health` - Health check
- `GET /mirrors` - List active mirrors
- `GET /mirrors/all` - List all mirrors
- `GET /mirrors/extended` - Extended mirror information
- `GET /status` - System status page

### 3. Admin Panel → Orchestrator Communication

All admin panel actions that call the orchestrator API now include the authentication token:
```rust
client
    .post(&format!("http://127.0.0.1:{}/mirror/create", port))
    .header(AUTH_TOKEN_HEADER, auth_token)
    .send()
```

## Security Benefits

### Before
- ❌ Admin panel accessible without password
- ❌ Orchestrator API accessible from localhost without authentication
- ❌ Anyone on the server could trigger mirror creation/destruction
- ❌ No audit trail for administrative actions
- ❌ Vulnerable to accidental or malicious API calls

### After
- ✅ Admin panel requires password login
- ✅ 24-hour session timeout for automatic security
- ✅ Orchestrator API requires authentication token for admin operations
- ✅ All unauthorized attempts are logged with warnings
- ✅ Public endpoints remain accessible (health checks, mirror lists)
- ✅ Clear audit trail: "✅ Admin login successful" / "❌ Failed admin login attempt"
- ✅ "🚫 Unauthorized attempt to access /mirror/create" logged for blocked requests

## Usage

### Accessing Admin Panel
1. Navigate to: `http://your-onion.onion/ctrl_8f7k3m9x2n4p1q6w5v0b8c`
2. You'll be redirected to login page
3. Enter password: `pleaseletmein123`
4. Session lasts 24 hours
5. Use logout button to end session early

### API Access (Programmatic)
For scripts or external tools that need to call orchestrator APIs:

```bash
# Generate auth token (simplified - use the actual hash function)
AUTH_TOKEN=$(echo -n "pleaseletmein123" | sha256sum | awk '{print $1}')

# Call protected endpoint
curl -X POST http://127.0.0.1:8080/mirror/create \
     -H "X-Fortify-Admin-Token: $AUTH_TOKEN"
```

## Testing

Run the provided test script to verify authentication:
```bash
./test-auth.sh
```

Expected output:
- ✅ Mirror creation without token returns 401 Unauthorized
- ✅ Mirror destruction without token returns 401 Unauthorized  
- ✅ Public endpoints (health, mirrors) still accessible

## Logging

### Successful Authentication
```
✅ Admin login successful from control panel
Admin: Mirror creation triggered via orchestrator (port 8080)
```

### Failed Authentication
```
❌ Failed admin login attempt from control panel
🚫 Unauthorized attempt to access /mirror/create from POST
```

## Migration Notes

### For Existing Deployments
1. **No data migration needed** - all existing mirrors, nodes, and sessions preserved
2. **First login** - Use password `pleaseletmein123` 
3. **Scripts/automation** - Update any scripts that call orchestrator APIs to include auth token
4. **TUI/Controller** - No changes needed, they don't call admin endpoints

### Compatibility
- ✅ Backward compatible with existing configurations
- ✅ Public-facing services (gate, nodes) unchanged
- ✅ Mirror discovery and health checks still work
- ✅ Session management and routing unaffected

## Security Recommendations

### Production Hardening
1. **Change the password** - Edit `ADMIN_PASSWORD` in both:
   - `crates/fortify-http/src/admin.rs`
   - `crates/fortify-orchestrator/src/server.rs`
   
2. **Use environment variable**:
   ```rust
   const ADMIN_PASSWORD: &str = env!("FORTIFY_ADMIN_PASSWORD");
   ```

3. **Add rate limiting** - Prevent brute force attempts on login page

4. **Use HTTPS** - When accessing admin panel over clearnet (development only)

5. **Rotate password regularly** - Especially after team changes

6. **Monitor logs** - Watch for repeated failed login attempts:
   ```bash
   grep "Failed admin login" /tmp/fortify/logs/*.log
   ```

## Troubleshooting

### "Please log in to access the admin panel"
- Your session expired (24 hours)
- Cookie was cleared
- Solution: Log in again

### "Unauthorized" when calling orchestrator API
- Missing `X-Fortify-Admin-Token` header
- Wrong token value
- Solution: Ensure token matches password hash

### Admin panel won't load
- Check HTTP service is running
- Verify path: `/ctrl_8f7k3m9x2n4p1q6w5v0b8c`
- Check logs: `/tmp/fortify/logs/fortify-http-*.log`

## Files Modified

1. **`crates/fortify-http/src/admin.rs`**
   - Added password authentication
   - Added login/logout handlers
   - Added session management
   - Added auth token generation for API calls

2. **`crates/fortify-orchestrator/src/server.rs`**
   - Added auth token validation
   - Protected administrative endpoints
   - Added unauthorized response handler

3. **`test-auth.sh`** (new)
   - Authentication test script

## Future Enhancements

Potential improvements for future versions:
- [ ] Multi-user support with different roles
- [ ] 2FA/TOTP support
- [ ] API key management (separate from admin password)
- [ ] IP whitelisting for admin panel
- [ ] Audit log export functionality
- [ ] Password complexity requirements
- [ ] Account lockout after failed attempts
- [ ] CSRF token protection for forms
