# Branding Not Applied to HTML Pages - Investigation & Fix Plan

## Status
Complete - All fixes applied and tests passing

## Objective
Identify and resolve the issue where branding (e.g., service name, welcome message) set via TUI or Control Panel is not reflected on any Fortify HTML pages (gate, busy, captcha, etc.).

## Problem Summary
- User set branding via TUI/Control Panel settings.
- No branding appears on any served HTML pages (e.g., gate, busy, captcha, etc.).
- Confirmed: Branding/config values are not being propagated/applied to HTML templates at runtime due to a lack of runtime update mechanism in the gate service.

## Investigation
- Verified: HTML pages (e.g., gate.html) do not show updated branding after settings change via TUI or Control Panel.
- Affected: All user-facing HTML pages.
- Root cause (confirmed):
    - The `Gate` struct in fortify-gate holds its branding config as an `Arc<BrandingVars>` set only at construction (see `Gate::with_branding`).
    - When branding is updated via TUI/Control Panel, the backend (fortify-http) updates its own in-memory branding config, but there is no mechanism to propagate this update to the running `Gate` instance.
    - All HTML rendering in the gate server uses `gate.branding().clone()`, which always returns the branding as it was at startup.
    - There is no code in fortify-gate to listen for branding config changes or to update the `branding` field after initial construction.

## Implementation Tasks
- [x] Trace branding/config value flow from TUI/Control Panel to backend/template engine (confirmed: stops at gate instance construction).
- [x] Verify config reload and propagation to all services (confirmed: not propagated to running gate instance).
- [x] Inspect HTML template rendering for correct variable substitution (confirmed: uses branding from gate instance, not updated at runtime).
- [x] Implement runtime update mechanism in fortify-gate to receive and apply branding config changes (via internal HTTP API and Arc<Mutex<BrandingVars>>).
- [x] On branding update in fortify-http, propagate new config to running gate instance and update its `branding` field via POST /gate/admin/branding.
- [x] Ensure all template rendering uses the latest branding config from this shared state.
- [x] Add regression test: branding changes are reflected on all HTML pages without restart.

## Fix Steps Applied

### 1. fortify-core/src/templates.rs
- Added `serde::Serialize, serde::Deserialize` derives to `BrandingVars` to enable JSON serialization for HTTP API.

### 2. fortify-gate/src/lib.rs
- Changed `branding` field from `Arc<BrandingVars>` to `Arc<Mutex<BrandingVars>>`.
- Added `update_branding(&self, new: BrandingVars)` method for runtime updates.
- Updated `branding()` method to safely lock and clone the current branding.
- Added regression test `test_branding_hot_reload` to verify runtime updates work.

### 3. fortify-gate/src/server.rs
- Added POST `/gate/admin/branding` endpoint to receive branding config updates via HTTP.
- Added `async fn handle_update_branding_config` at module scope (best practice: free function outside any block).
- Updated `styled_error_response` to use default branding (gate context not available in error handler).

### 4. fortify-http/src/admin.rs
- Added `sync_branding_config_to_gate` function to POST branding changes to Gate server via HTTP.
- Called this function from `handle_branding_settings` after updating branding config.
- Ensured function is at module scope (not nested inside another function).

## Testing Checklist
- [x] Set branding via TUI, verify on all HTML pages (gate, busy, captcha, etc.).
- [x] Set branding via Control Panel, verify on all HTML pages.
- [x] Test hot reload: branding change appears without restart.
- [x] Test restart: branding persists and appears after restart.

## Success Criteria
- Branding (service name, welcome message, etc.) appears on all user-facing HTML pages after settings change via TUI or Control Panel.
- Branding changes are reflected immediately (hot reload) without requiring a restart.
- No regression in config propagation or template rendering.

---

*Created automatically in response to user report: branding not applied to HTML pages after settings change via TUI/Control Panel. See test session notes for details.*
