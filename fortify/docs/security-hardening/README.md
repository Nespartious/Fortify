# Security Hardening Tasks

**Purpose:** Critical security issues that must be addressed before Beta release.

**Status:** 🔴 Not Started  
**Priority:** Beta Blockers  
**Timeline:** 5-8 days estimated

---

## Overview

This directory contains detailed implementation plans for critical security hardening tasks identified during Alpha review and external security audit. Both issues are classified as **Beta Blockers** - the system should not be released to production until these are complete.

---

## Critical Issues

### [01-timeout-strategy.md](01-timeout-strategy.md) - Async Timeout Strategy
**Priority:** 🔴 CRITICAL  
**Estimated Effort:** 2-3 days  
**Status:** ⬜ Not Started

**Problem:** Slow-loris attacks can exhaust connection pools by holding connections open indefinitely. This bypasses PoW defenses (which only protect Tor introduction layer) and can make entire service unresponsive.

**Impact:**
- Service DoS via resource exhaustion
- Proven attack vector against Tor hidden services
- Application-layer attack (PoW doesn't help)

**Implementation Areas:**
1. Tor control socket operations (ADD_ONION, SIGNAL RELOAD)
2. HTTP Proxy request/response handling
3. Backend node proxying
4. Orchestrator API calls
5. WebSocket/admin panel connections

**Key Deliverables:**
- [ ] All async operations have explicit timeouts
- [ ] Comprehensive timeout configuration documentation
- [ ] Slow-loris simulation test passes
- [ ] Service remains responsive under attack

---

### [02-panic-audit.md](02-panic-audit.md) - Comprehensive Panic Audit
**Priority:** 🔴 CRITICAL  
**Estimated Effort:** 3-5 days  
**Status:** ⬜ Not Started

**Problem:** `unwrap()` and `expect()` calls on attacker-controlled input can cause panics, resulting in instant DoS. Lock poisoning can cascade failures across all threads.

**Impact:**
- One panic on malformed input = entire service crashes
- Panics in async contexts poison task executors
- Lock poisoning cascades to all future requests
- Exploitable attack vector

**Implementation Areas:**
1. HTTP header/cookie parsing
2. Token deserialization
3. Tor control response parsing
4. WebSocket message handling
5. Lock operations (Mutex/RwLock)

**Key Deliverables:**
- [ ] Zero unwraps on untrusted input
- [ ] Lock poisoning handled gracefully
- [ ] Clippy lints enforced: `#![deny(clippy::unwrap_used)]`
- [ ] Fuzzing infrastructure operational
- [ ] Malformed input test suite passing

---

## Implementation Workflow

Each task document follows this structure:

### 1. **Overview**
- Problem statement
- Goal definition
- Success criteria

### 2. **Implementation Steps**
Organized into phases:
- **Phase 1:** Audit (find all occurrences)
- **Phase 2-5:** Fix by priority area
- **Phase 6:** Documentation
- **Phase 7:** Testing

### 3. **Task Format**
Each task includes:
- ⬜ Status indicator
- Specific files to modify
- Code examples (before/after)
- Test commands
- Expected results

### 4. **Completion Checklist**
Final validation steps before marking complete

---

## How to Use These Documents

### For Implementation:

1. **Open task document** (01 or 02)
2. **Start with Phase 1** (Audit)
3. **Work through each task sequentially**
4. **Mark tasks complete** with ✅ as you finish
5. **Run tests** after each phase
6. **Update status** in Alpha_Review.md when done

### Task Status Indicators:
- ⬜ Not Started
- 🟦 In Progress  
- ✅ Complete
- ⚠️ Blocked

### For Review:

Each task has:
- **Command examples** to verify implementation
- **Test cases** to validate fixes
- **Expected results** for validation

---

## Dependencies

### Before Starting:
- [ ] Fortify builds successfully
- [ ] All existing tests pass (59/62)
- [ ] Git working directory clean (commit checkpoint)

### Tools Needed:
- [ ] Rust/Cargo (1.88+)
- [ ] cargo-clippy
- [ ] cargo-fuzz (for panic audit)
- [ ] Python 3 (for slow-loris test)
- [ ] curl (for HTTP testing)

---

## Testing Strategy

### Continuous Testing (During Implementation):
- Run unit tests after each file modification
- Run integration tests after each phase
- Verify service starts and responds after each change

### Final Validation:
- **Timeout Strategy:** Slow-loris simulation (service stays responsive)
- **Panic Audit:** Fuzzing for 4+ hours (no panics discovered)
- **Both:** Malformed input test suite (all tests pass)

---

## Timeline Estimate

| Phase | Task | Effort | Dependencies |
|-------|------|--------|--------------|
| 1 | Timeout Audit | 0.5 days | None |
| 2 | Tor Control Timeouts | 1 day | Phase 1 |
| 3 | HTTP Proxy Timeouts | 0.5 days | Phase 1 |
| 4 | Orchestrator/WebSocket Timeouts | 0.5 days | Phase 1 |
| 5 | Timeout Testing | 0.5 days | Phases 2-4 |
| 6 | Panic Audit | 1 day | None |
| 7 | Critical Path Panic Fixes | 1.5 days | Phase 6 |
| 8 | Lock Operation Fixes | 1 day | Phase 6 |
| 9 | Clippy Lints & Fuzzing | 1 day | Phases 7-8 |
| 10 | Integration Testing | 0.5 days | All phases |
| **Total** | | **8 days** | |

**Note:** Can parallelize timeout and panic work if multiple developers available.

---

## Progress Tracking

Update this section as work progresses:

### Overall Status: ⬜ Not Started

**Timeout Strategy:** ⬜ 0% Complete (0/7 phases)
- [ ] Phase 1: Audit
- [ ] Phase 2: Tor Control
- [ ] Phase 3: HTTP Proxy
- [ ] Phase 4: Orchestrator API
- [ ] Phase 5: WebSocket
- [ ] Phase 6: Documentation
- [ ] Phase 7: Testing

**Panic Audit:** ⬜ 0% Complete (0/7 phases)
- [ ] Phase 1: Audit
- [ ] Phase 2: Critical Path Fixes
- [ ] Phase 3: Lock Operations
- [ ] Phase 4: Clippy Lints
- [ ] Phase 5: Fuzzing
- [ ] Phase 6: Integration Tests
- [ ] Phase 7: Documentation

---

## Questions & Blockers

Use this section to track issues during implementation:

### Open Questions:
- (None yet - add as they arise)

### Blocked Tasks:
- (None yet - add if blocked)

### Decisions Needed:
- (None yet - add if decisions needed)

---

## Related Documentation

- [Alpha_Review.md](../Dev_Progress/Alpha_Review.md) - Overall project status
- [prof_review.md](../research/prof_review.md) - External security audit
- [TESTING.md](../TESTING.md) - Testing guidelines
- [README.md](../../README.md) - Project overview

---

## Contact

Questions about these tasks? Review the detailed task documents first. If still unclear, refer to:
- Implementation examples in task documents
- Test commands for validation
- Expected results for each step

---

**Last Updated:** January 21, 2026  
**Created By:** Security Hardening Initiative  
**Review Status:** Ready for Implementation
