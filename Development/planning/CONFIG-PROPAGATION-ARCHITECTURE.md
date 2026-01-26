# Configuration Propagation Architecture

**Created**: 2026-01-24  
**Status**: PLANNING  
**Priority**: HIGH  

## Problem Statement

### Current Issues
1. **Gate starts with default branding** - Controller spawns Gate without branding config
2. **Race condition on sync** - TUI tries to sync branding before services are ready
3. **Fragmented config** - Settings scattered across:
   - `~/.config/fortify/deployment.toml` (TUI config)
   - Environment variables (passed at spawn)
   - Runtime HTTP API updates
4. **No single source of truth** - Different deployment methods can get out of sync

### User Requirements
- All settings must apply correctly (branding, captcha, thresholds, etc.)
- Live updates where possible (hot reload)
- Interchangeable deployment methods (TUI, Control Panel, Scripts)
- If TUI deploys, scripts should produce identical results with same config

---

## Current Architecture

```
┌─────────────────┐
│      TUI        │  ← deployment.toml
│ (fortify-tui)   │
└────────┬────────┘
         │ spawns (via Command)
         ▼
┌─────────────────┐
│   Controller    │  ← ControllerConfig (env vars only)
│(fortify-ctrl)   │  ← NO branding, NO captcha config
└────────┬────────┘
         │ spawns (via ServiceManager)
         ▼
┌────────┴────────┬────────────────┬──────────────┐
│                 │                │              │
▼                 ▼                ▼              ▼
Gate          HTTP Proxy      Orchestrator     Nodes
(defaults)    (has branding)   (has vanity)   (no config)
```

### Why Branding Fails
1. TUI has `deployment.toml` with branding settings
2. TUI spawns Controller with env vars (no branding included)
3. Controller spawns Gate with `GATE_BIND_ADDR` and `SECRET_KEY` only
4. Gate initializes with `BrandingVars::default()` ("Protected Service")
5. TUI tries to sync via HTTP API... but timing is fragile

---

## Proposed Architecture

### Option A: Environment Variables at Spawn (Recommended)

**Pass branding as env vars when Controller spawns Gate:**

```rust
// fortify-controller/src/lib.rs
fn gate_env(&self) -> Vec<String> {
    vec![
        format!("GATE_BIND_ADDR={}", self.config.gate_bind_addr),
        format!("SECRET_KEY={}", self.config.secret_key),
        // NEW: Branding config
        format!("BRANDING_SERVICE_NAME={}", self.config.branding.service_name),
        format!("BRANDING_DESCRIPTION={}", self.config.branding.description),
        format!("BRANDING_WELCOME_MESSAGE={}", self.config.branding.welcome_message),
        format!("BRANDING_PRIMARY_COLOR={}", self.config.branding.primary_color),
        format!("BRANDING_SECONDARY_COLOR={}", self.config.branding.secondary_color),
    ]
}
```

**Gate reads env vars at startup:**

```rust
// fortify-gate/src/main.rs
let branding = BrandingVars {
    service_name: env::var("BRANDING_SERVICE_NAME").unwrap_or_else(|_| "Protected Service".into()),
    description: env::var("BRANDING_DESCRIPTION").unwrap_or_else(|_| "A Fortify-protected onion service".into()),
    // ...
};
let gate = Gate::with_branding(..., branding);
```

**Pros:**
- No race condition - branding is available at Gate startup
- Works with any deployment method (TUI, scripts, systemd)
- Controller is the single source of truth

**Cons:**
- Lots of env vars (can get messy)
- Need to update Controller to accept branding config

---

### Option B: Shared Config File

**All services read from `~/.local/share/fortify/config.toml`:**

```
TUI writes → config.toml ← Controller reads
                         ← Gate reads
                         ← HTTP Proxy reads
                         ← Orchestrator reads
```

**Pros:**
- Single source of truth (the file)
- Easy to inspect/debug
- Scripts just need to write the file

**Cons:**
- File locking complexity
- Services need to watch for changes (inotify or polling)
- Need to handle stale reads

---

### Option C: Controller as Config Server

**Controller exposes config API, services fetch on startup:**

```
┌─────────────────┐
│   Controller    │ ← POST /api/config (from TUI)
│                 │ ← GET  /api/config (from services)
└────────┬────────┘
         │
         ▼
    All services fetch config from Controller on startup
```

**Pros:**
- Controller is authoritative
- Easy to add new config fields
- Hot reload via WebSocket/polling

**Cons:**
- Controller must start first (dependency ordering)
- Network dependency for config
- More complex than env vars

---

### Option D: Hybrid (Recommended)

Combine Options A and B:

1. **At spawn**: Pass essential config via env vars (fast startup)
2. **Config file**: Detailed config in shared file for hot reload
3. **HTTP API**: For runtime updates (hot reload)

```
deployment.toml ─┬─► TUI spawns Controller with env vars
                 │
                 └─► Controller reads file for full config
                          │
                          ├─► Spawns Gate with branding env vars
                          ├─► Spawns HTTP Proxy with branding env vars
                          └─► Spawns Orchestrator with captcha/vanity config
```

---

## Implementation Plan

### Phase 1: Add Branding to ControllerConfig

```rust
// fortify-controller/src/config.rs
pub struct ControllerConfig {
    // ... existing fields ...
    
    // Branding configuration (forwarded to Gate and HTTP Proxy)
    pub branding_service_name: String,
    pub branding_description: String,
    pub branding_welcome_message: String,
    pub branding_primary_color: String,
    pub branding_secondary_color: String,
}
```

### Phase 2: TUI Passes Branding to Controller

```rust
// fortify-tui/src/deployment.rs - when starting controller
cmd.env("BRANDING_SERVICE_NAME", &config.branding.service_name);
cmd.env("BRANDING_DESCRIPTION", &config.branding.description);
// ...
```

### Phase 3: Controller Passes Branding to Gate

```rust
// fortify-controller/src/lib.rs - gate_env()
fn gate_env(&self) -> Vec<String> {
    vec![
        // ... existing ...
        format!("BRANDING_SERVICE_NAME={}", self.config.branding_service_name),
        // ...
    ]
}
```

### Phase 4: Gate Reads Branding from Env

```rust
// fortify-gate/src/main.rs
let branding = BrandingVars::from_env();
let gate = Gate::with_branding(..., branding);
```

### Phase 5: Hot Reload Still Works

The HTTP API hot reload continues to work for runtime updates:
- POST /Fortify/settings/branding → Admin panel → Gate API → Update branding

---

## Script Deployment Compatibility

### Current Script Pattern
```bash
# Deploy-Scripts/deploy-medium.sh
export TRAFFIC_TIER=medium
./fortify-controller
```

### Proposed Script Pattern
```bash
# Deploy-Scripts/deploy.sh
source ~/.config/fortify/deployment.env  # Generated from deployment.toml
./fortify-controller
```

### Auto-Generated Scripts from TUI

**Add to TUI**: "Export Headless Deploy Script" button

```rust
// fortify-tui/src/config.rs
impl FortifyConfig {
    pub fn export_deploy_script(&self, path: &Path) -> Result<()> {
        let script = format!(r#"#!/bin/bash
# Auto-generated by Fortify TUI on {}
# Deployment ID: {}

export BRANDING_SERVICE_NAME="{}"
export BRANDING_DESCRIPTION="{}"
export BRANDING_PRIMARY_COLOR="{}"
# ... all config as env vars ...

cd "$(dirname "$0")"
./fortify-controller
"#, chrono::Local::now(), self.deployment_id, 
   self.branding.service_name, self.branding.description, 
   self.branding.primary_color);
        
        std::fs::write(path, script)?;
        // chmod +x
        Ok(())
    }
}
```

---

## Future Considerations

### TODO: Add to Planning
- [ ] Auto-generate headless deploy scripts from TUI settings
- [ ] Export/import configuration between TUI and scripts
- [ ] Configuration validation (ensure all required fields present)
- [ ] Configuration migration (version upgrades)
- [ ] Encrypted config for sensitive fields (API keys, secrets)

---

## Files to Modify

| File | Changes |
|------|---------|
| `fortify-controller/src/config.rs` | Add branding fields |
| `fortify-controller/src/lib.rs` | Update `gate_env()` and `proxy_env()` |
| `fortify-gate/src/main.rs` | Read branding from env vars |
| `fortify-core/src/templates.rs` | Add `BrandingVars::from_env()` |
| `fortify-tui/src/deployment.rs` | Pass branding env vars to controller |

---

## Success Criteria

- [ ] Fresh deployment shows custom branding immediately (no sync delay)
- [ ] Hot reload from TUI settings still works
- [ ] Hot reload from Control Panel still works
- [ ] Script deployment produces identical results to TUI
- [ ] No race conditions or timing dependencies
- [ ] All config fields propagate correctly (branding, captcha, thresholds)
