# Sprint 17 Test Session Guide

**Date:** January 23, 2026  
**Sprint:** Settings Hot Reload & Missing Config Fields  
**PR:** #44 (Merged)

---

## Quick Reference: What Changed

### Menu Changes
| Old | New |
|-----|-----|
| `Settings` | `View System Settings` + `Modify System Settings` |
| `System Status` | **Removed** (merged into Live TUI Monitor) |
| Hotkey `S` | `V` (View) + `M` (Modify) |
| Hotkey `T` (Status) | **Removed** |

### New CAPTCHA Config Fields
| Field | Type | Default |
|-------|------|---------|
| Gate CAPTCHA Type | Enum (7 types) | BmpText |
| Threat CAPTCHA Type | Enum (7 types) | Emoji |
| Threat CAPTCHA Enabled | Boolean | false |
| Random Cycling | Boolean | false |
| Cycling Types | List | [BmpText, Emoji] |

### CAPTCHA Types Available
1. BmpText - Text-based BMP image
2. Emoji - Emoji selection
3. Direction - Arrow direction
4. Sequence - Number sequence
5. WordUnscramble - Word puzzle
6. ImageRotation - Rotate image
7. Silhouette - Shape matching

---

## TUI Testing Checklist

### 1. Menu Navigation ✅
- [ ] Launch TUI: `cargo run --release`
- [ ] Verify menu shows:
  - Deploy
  - Join Community Network
  - **View System Settings** (new)
  - **Modify System Settings** (new)
  - Destroy Instance
  - Quit
- [ ] Verify NO "System Status" menu item exists
- [ ] Press `V` - should open View Settings (read-only)
- [ ] Press `M` - should open Modify Settings (editable)

### 2. View Settings (Read-Only Mode)
- [ ] From menu, select "View System Settings"
- [ ] Verify title shows: "👁 View Settings (Read-Only)"
- [ ] Verify footer shows: `[←→] Tab [↑↓] Scroll [Esc] Done`
- [ ] Navigate tabs with ←/→ arrows
- [ ] Verify Tier tab is the **default** (first shown)
- [ ] Press Enter - should NOT allow editing
- [ ] Press Esc - should return to Home menu

### 3. Modify Settings (Edit Mode)
- [ ] From menu, select "Modify System Settings"
- [ ] Verify title shows: "⚙ Modify Settings"
- [ ] Verify footer shows: `[←→] Tab [↑↓] Select [Enter] Edit [Esc] Back`
- [ ] Navigate to CAPTCHA tab

### 4. CAPTCHA Tab - New Fields
Navigate to CAPTCHA tab and verify these fields exist:

| # | Field | Expected UI | Test Action |
|---|-------|-------------|-------------|
| 1 | Pool Size | Shows number, ⚠️Restart label | Enter → type new value |
| 2 | Min Pool | Shows number, ⚠️Restart label | Enter → type new value |
| 3 | Max Pool | Shows number, ⚠️Restart label | Enter → type new value |
| 4 | Difficulty | Shows 1-10 | Enter → type 5 |
| 5 | Timeout Seconds | Shows number | Enter → type 120 |
| 6 | Max Attempts | Shows number | Enter → type 5 |
| 7 | **Gate CAPTCHA Type** | Shows type name, ↵ | Press Enter to cycle |
| 8 | **Threat CAPTCHA Enabled** | Yes/No, ↵ | Press Enter to toggle |
| 9 | **Threat CAPTCHA Type** | Shows type name, ↵ | Press Enter to cycle |
| 10 | **Random Cycling** | Yes/No, ↵ | Press Enter to toggle |
| 11 | **Cycling Types** | Shows list | (display only) |
| 12 | Warmup Enabled | Yes/No | Press Enter to toggle |
| 13 | Warmup Target | Shows number | Enter → type value |

### 5. CAPTCHA Type Cycling
For Gate CAPTCHA Type and Threat CAPTCHA Type:
- [ ] Press Enter to cycle through types
- [ ] Verify it cycles: BmpText → Emoji → Direction → Sequence → WordUnscramble → ImageRotation → Silhouette → BmpText

### 6. Tier Tab
- [ ] Navigate to Tier tab (should be default)
- [ ] Verify current tier is shown
- [ ] Change tier with Enter/Up/Down
- [ ] Verify ⚠️ CAPTCHA Pool field shows restart warning

### 7. Apply Changes Dialog
After making changes:
- [ ] Press Esc to exit settings
- [ ] Verify "Apply Changes?" dialog appears
- [ ] Verify dialog shows TWO sections:
  - ✓ Green: "Can apply immediately" (hot-reload changes)
  - ⚠ Yellow: "Requires restart" (restart-required changes)

Test each button:
- [ ] Press `A` (Apply) - applies hot-reload changes only
- [ ] Press `R` (Restart) - triggers restart for all changes
- [ ] Press `C` (Cancel) - discards all changes

### 8. Post-Apply Navigation
After pressing Apply:
- [ ] If deployed: Should return to Running view (Live TUI Monitor)
- [ ] If not deployed: Should return to Home menu
- [ ] Verify status message shows confirmation

### 9. Shortcuts from Running View
After deploying:
- [ ] Press `V` - should open View Settings
- [ ] Press `M` - should open Modify Settings
- [ ] Verify Esc returns to Running view

---

## Script Testing Checklist

### 1. Build and Run
```bash
cd /home/shadowbox/Fortify/Fortify/fortify
cargo build --release
./target/release/fortify
```

### 2. Config File Verification
After making changes in TUI, verify config file:
```bash
cat config/fortify.toml | grep -A 20 "\[captcha\]"
```

Expected new fields in config:
```toml
[captcha]
# ... existing fields ...
gate_captcha_type = "BmpText"
threat_captcha_type = "Emoji"
threat_captcha_enabled = false
random_cycling = false
cycling_types = ["BmpText", "Emoji"]
```

### 3. Hot Reload Test (requires deployment)
1. Deploy the service
2. Open Modify Settings
3. Change branding (Service Name, Primary Color)
4. Press Esc → Apply
5. Verify changes applied without restart
6. Refresh any connected browser page
7. Verify new branding visible

### 4. Restart-Required Test
1. Deploy the service
2. Open Modify Settings
3. Change Pool Size (requires restart)
4. Press Esc
5. Verify dialog shows Pool Size in yellow "Requires restart" section
6. Press R to restart
7. Verify services restart

---

## Settings Classification Reference

### ✅ HOT RELOAD (Apply Immediately)
- Service Name
- Description
- Welcome Message
- Primary Color
- Secondary Color
- Traffic Tier
- Rate Limit RPM
- Suspicion Threshold
- Threat Threshold
- Temp Ban Duration
- CAPTCHA Difficulty
- CAPTCHA Timeout
- CAPTCHA Max Attempts
- Gate CAPTCHA Type
- Threat CAPTCHA Type
- Threat CAPTCHA Enabled
- Random Cycling
- Cycling Types

### ⚠️ RESTART REQUIRED
- Pool Size
- Min Pool Size
- Max Pool Size
- Min Mirrors
- Max Mirrors
- Standby Mirrors
- Backend Address
- SOCKS Port
- Control Port
- HTTP Bind
- Gate Bind
- Vanguards Enabled
- Vanguards Layers
- Data Directory
- Vanity Enabled
- Vanity Prefix
- Safety Net Enabled

---

## Bug Report Template

If you find issues, note:
- [ ] Current view/screen
- [ ] Steps to reproduce
- [ ] Expected behavior
- [ ] Actual behavior
- [ ] Error messages (if any)
- [ ] Log output (if relevant)

---

## Sign-Off

| Test Area | Tester | Date | Pass/Fail |
|-----------|--------|------|-----------|
| Menu Navigation | AI + User | Jan 23, 2026 | ✅ PASS |
| View Settings | AI + User | Jan 23, 2026 | ✅ PASS |
| Modify Settings | AI + User | Jan 23, 2026 | ✅ PASS |
| CAPTCHA Fields | AI + User | Jan 23, 2026 | ✅ PASS |
| Apply Dialog | AI + User | Jan 23, 2026 | ✅ PASS |
| Hot Reload | | | |
| Restart Flow | | | |
| Shortcuts | | | |

### Test Session Notes (January 23, 2026)

**TUI Testing - Fresh Install:**
- Menu shows View/Modify System Settings correctly
- System Status removed as expected
- View Settings read-only mode works
- Tier tab is default
- All new CAPTCHA fields present and functional
- Apply Changes dialog appears correctly when deployed
- Note: Dialog only appears with active deployment (expected behavior)
