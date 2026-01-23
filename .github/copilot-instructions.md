# Fortify AI Agent Instructions

These instructions apply to all AI assistants (GitHub Copilot, Claude, etc.) working on this repository.

---

## 🔴 CRITICAL: Sprint Workflow Requirements

### 1. Branch & PR Strategy
- **One sprint = One branch = One PR**
- Branch naming: `feature/sprint-XX-short-description` or `fix/sprint-XX-issue`
- Never commit directly to `main`
- Always create PR for review before merging

### 2. CI/CD Workflow Compliance
- **ALWAYS wait for ALL workflow checks to complete** before considering work done
- If any check fails:
  1. Inspect the failure logs
  2. Fix the issue locally
  3. Commit and push the fix
  4. Wait for new workflow checks to complete
  5. Repeat until ALL checks pass
- **Do NOT merge PRs with failing checks**

### 3. Completion Requirements
Every sprint/task completion MUST include:

#### Summary Report
```markdown
## Sprint Summary
- **Sprint:** [Number and Title]
- **PR:** #[number]
- **Status:** [Merged/Ready for Review]
- **Changes:** [Brief list of changes]
```

#### Testing Instructions
```markdown
## Testing Instructions
1. [Step-by-step testing procedure]
2. [Commands to run]
3. [Expected outcomes]
```

---

## Code Quality Standards

### Before Committing
```bash
cargo fmt                           # Format code
cargo clippy -- -D warnings         # No warnings allowed
cargo test                          # All tests must pass
```

### Commit Messages
Follow conventional commits format:
- `feat(scope): description` - New features
- `fix(scope): description` - Bug fixes
- `docs: description` - Documentation only
- `refactor(scope): description` - Code refactoring
- `test(scope): description` - Test additions
- `chore: description` - Maintenance

---

## Sprint Document Requirements

Each sprint document in `docs/Dev_Progress/` must contain:

1. **Status** - Current phase/completion state
2. **Objective** - Clear goal statement
3. **Implementation Tasks** - Detailed task breakdown with status
4. **Testing Checklist** - What to verify
5. **Success Criteria** - How to know it's complete

---

## Working with This Repository

### Key Directories
- `crates/` - Rust workspace crates
- `docs/Dev_Progress/` - Active sprint documentation
- `docs/Dev_Progress/archive/` - Completed sprint documentation
- `docs/planning/` - Future feature planning
- `.github/workflows/` - CI/CD pipeline definitions

### Before Starting Work
1. Pull latest `main`: `git pull origin main`
2. Create feature branch: `git checkout -b feature/sprint-XX-name`
3. Read the relevant sprint document in `docs/Dev_Progress/`

### After Completing Work
1. Run all checks locally (fmt, clippy, test)
2. Commit with conventional commit message
3. Push and create PR
4. Wait for ALL CI checks to pass
5. Fix any failures and repeat
6. Provide summary report to user
7. **Ask user for permission to merge** once all checks pass
8. Only merge when user approves

### During Development
- **Update the sprint document as you work** - Add useful information, progress notes, and implementation details
- Keep the sprint document current with actual changes made

### After Merging a PR
1. **Archive the sprint document** - Move from `docs/Dev_Progress/` to `docs/Dev_Progress/archive/`
2. **Update Fortify Documentation** - Reflect changes in `docs/Fortify Documentation/`
3. **Update Dev Progress README** - Mark sprint as complete in `docs/Dev_Progress/README.md`

---

## 🔴 CRITICAL: Fortify Documentation Maintenance

The `docs/Fortify Documentation/` directory is the **authoritative system-wide documentation**. It must:
- Fully and clearly explain how Fortify works in plain English
- Include practical examples where helpful
- Be kept in sync with code changes

### Documentation Structure
```
docs/Fortify Documentation/
├── 01-Architecture/     # System design and component relationships
├── 02-Core-Concepts/    # Trust tiers, sessions, CAPTCHA, etc.
└── 08-API-Reference/    # API endpoints and usage
```

### When to Update Fortify Documentation
- **After every merged PR** that changes system behavior
- When adding new features or components
- When modifying existing functionality
- When fixing bugs that affect documented behavior

### Documentation Standards
- Use clear, simple English
- Explain the "why" not just the "what"
- Include code examples for technical concepts
- Keep examples practical and runnable
- Cross-reference related documents

---

## Security Considerations

This is a Tor hidden service protection system. When making changes:
- Never log sensitive data (tokens, keys, IPs in production)
- Handle all untrusted input safely (no unwrap on user input)
- Maintain timeout protections on all network operations
- Prefer safe lock helpers over raw mutex operations

---

## 🔴 CRITICAL: Tor Browser Compatibility

Fortify is a **Tor/onion hidden service** protection system. ALL user-facing components must work with **Tor Browser on the SAFEST security setting**.

### What "Safest" Disables
- ❌ JavaScript (completely disabled)
- ❌ SVG images
- ❌ MathML
- ❌ Some fonts
- ❌ Media auto-play
- ❌ WebGL, WebAudio
- ❌ Most modern web APIs

### Mandatory Requirements
1. **No JavaScript** - All pages must be fully functional without JS
2. **Pure HTML/CSS only** - No client-side scripting of any kind
3. **No external resources** - All assets must be self-hosted (no CDNs)
4. **No XMLHttpRequest/Fetch** - All interactions via form submissions
5. **CSS-only interactivity** - Use `:target`, `:checked`, `:focus` for UI states
6. **Standard image formats** - PNG, JPEG, GIF only (no SVG for content)
7. **Progressive enhancement** - Core functionality works without any enhancements

### HTML Templates Must
- Use `<form>` with `method="POST"` for all user actions
- Provide `<noscript>` alternatives (though JS should never be required)
- Work with cookies disabled (use URL tokens if needed)
- Be accessible without CSS (semantic HTML)

### Testing Requirement
Before merging any UI changes, test in Tor Browser with Security Level set to "Safest"
