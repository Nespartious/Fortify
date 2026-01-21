# GitHub Issue to Create: TUI Settings Layout

## Title
TUI Settings: Headers and content extending too far right

## Description
The Settings section in the TUI has layout issues where section headers and some panel content extend too far to the right, causing visual overflow.

## Location
- `crates/fortify-tui/src/ui/settings.rs`
- Left-hand side settings panel

## Proposed Solution
Apply the same overhaul pattern used previously:
- Replace extended section headers with compact colored dot indicators
- Constrain content width within panel boundaries
- Ensure proper text wrapping/truncation

## Priority
Low - cosmetic issue

## Related
Similar fix was applied to other TUI sections with colored dot headers.

---
**Create this issue at:** https://github.com/Nespartious/Fortify/issues/new
