# Mirror PoW Difficulty Strategy for Fortify

**Status:** Planning
**Topic:** Dynamic Proof-of-Work (PoW) for Active/Standby Mirrors

---

## Options Considered

### Option 1: Active=20, Standby=24
- **Normal operation:**
  - Mirror 1 (Active): 20-bit PoW (~1s solve)
  - Mirror 2 (Active): 20-bit PoW
  - Mirror 3 (Standby): 24-bit PoW (~5-20s solve)
  - Mirror 4 (Standby): 24-bit PoW
- **Attack detected:**
  - Route suspicious traffic to standby mirrors (24-bit)
  - Legitimate users stay on fast 20-bit mirrors
- **Pros:**
  - ✅ Attacker cost increases 16x (2^(24-20))
  - ✅ Legit users unaffected in normal operation
  - ✅ Standby mirrors absorb attack cost
  - ✅ Simple to implement
- **Cons:**
  - ⚠️ Weak hardware (RPi, old ARM) may timeout on 24-bit (max ~67s, exceeds 45s default)
  - ⚠️ 24-bit is aggressive, may frustrate attackers or cause botnet escalation
  - ⚠️ Requires reliable threat detection

### Option 2: Active=19, Standby=23
- **Normal operation:**
  - Mirror 1 (Active): 19-bit PoW (~590ms solve)
  - Mirror 2 (Active): 19-bit PoW
  - Mirror 3 (Standby): 23-bit PoW (~6-23s solve)
  - Mirror 4 (Standby): 23-bit PoW
- **Attack detected:**
  - Route suspicious traffic to standby mirrors (23-bit)
- **Pros:**
  - ✅ More balanced: 23-bit is tolerable for weak hardware
  - ✅ Active mirrors are very fast (19-bit)
  - ✅ Still 16x cost multiplier for attackers
  - ✅ Good UX for all but the weakest devices
  - ✅ 4 bits of escalation room
- **Cons:**
  - ⚠️ Active mirrors slightly more vulnerable than 20-bit
  - ⚠️ Standby mirrors at 23-bit won't break all attackers

### 23-Bit Analysis (Sweet Spot)
- Most devices: 2-10s (acceptable)
- Weak ARM: ~22s (tolerable)
- RPi: ~93s (may timeout)
- Attack deterrent: 16x harder than 19-bit

---

## Recommendation

- **Best balance:** 19/23 or 20/23 split
- **If current PoW is 20:**
  - Use 20 for active, 23 for standby (or 24 for max security, but beware weak hardware)
- **Increase GATE_VERIFICATION_TIMEOUT to 75s** if using 23/24-bit for standby

---

## Implementation Strategy

- Set PoW per mirror at deployment:
  - Active mirrors: 19 or 20
  - Standby mirrors: 23 or 24
- On attack detection, route suspicious sessions to standby mirrors
- Threat detection sets session/mirror PoW dynamically

---

## Example Config

```bash
# deploy.sh
ACTIVE_MIRRORS=2
STANDBY_MIRRORS=2

# Active mirrors get fast PoW
for i in 1 2; do
  MIRROR_${i}_POW_DIFFICULTY=20
  # or 19
  done

# Standby mirrors get tough PoW
for i in 3 4; do
  MIRROR_${i}_POW_DIFFICULTY=23
  # or 24
  done

GATE_VERIFICATION_TIMEOUT=75
```

---

## Threat Detection Hook (Rust)

```rust
if mirror.is_standby && threat_detected {
    pow_difficulty = 23; // or 24
} else if mirror.is_standby {
    pow_difficulty = 23; // or 24
} else {
    pow_difficulty = 20; // or 19
}
```

---

## Final Verdict
- **Go with 20/23 or 20/24 split** (decide based on user hardware needs)
- **19/23** is fastest, **20/24** is most secure
- **23-bit** is the sweet spot for attack mirrors
- **24-bit** is only for max security, not for weak hardware

---

## Open Questions
- Should we allow operator to tune PoW per mirror in TUI?
- Should standby mirrors always run high PoW, or only during attack?
- Should we randomize PoW within a range for unpredictability?
