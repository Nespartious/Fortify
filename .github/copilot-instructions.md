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
7. Only merge when user approves

---

## Security Considerations

This is a Tor hidden service protection system. When making changes:
- Never log sensitive data (tokens, keys, IPs in production)
- Handle all untrusted input safely (no unwrap on user input)
- Maintain timeout protections on all network operations
- Prefer safe lock helpers over raw mutex operations
