# 🎈 Fortify: Explain Like I'm 5

> **The Complete Guide to Fortify in Plain English**

---

## What is Fortify?

Imagine you run a secret clubhouse on the dark web (a Tor hidden service). Bad people try to attack your clubhouse:
- Bullies who knock on the door a million times (DDoS attacks)
- Robots pretending to be humans (bots)
- Sneaky people trying to find secret passages (hackers)

**Fortify is like a smart bouncer** that protects your clubhouse. It lets good visitors in but keeps the bad guys out.

---

## The 5 Trust Levels: A Story

Think of Fortify's trust system like a video game with 5 levels:

### Level -2: 🔴 BURNED (Game Over)
**What it means:** You're permanently banned. Game over, no respawns.

**How you get here:** You were a bad player who kept breaking rules even after warnings.

**What happens:** You see a "You've been permanently banned" page. That's it.

---

### Level -1: 🟡 SUSPICIOUS (On Thin Ice)
**What it means:** You messed up badly and we're watching you carefully.

**How you get here:** 
- You broke 3 rules while playing normally
- You did something that looked like a robot would do

**What happens:** 
- You have to solve **2 hard puzzles** (CAPTCHAs) to get back in
- If you mess up 3 times total, you're BURNED forever

**Real-world analogy:** It's like getting detention at school - one more problem and you're expelled.

---

### Level 0: ⚪ UNKNOWN (New Player)
**What it means:** You just showed up. We don't know if you're good or bad yet.

**How you get here:** First time visiting, or your pass expired.

**What happens:**
- You have to solve **1 puzzle** (CAPTCHA) to prove you're human
- Once you solve it, you level up to VERIFIED

**Real-world analogy:** It's like showing ID at the door of a club.

---

### Level +1: 🔵 VERIFIED (Good Standing)
**What it means:** You proved you're human and can access everything normally.

**How you get here:** You solved the CAPTCHA puzzle.

**What happens:**
- You can browse the site freely
- You get 100 requests every 10 seconds (plenty for normal browsing)
- If you behave well for a while (50 clean page views), you level up to TRUSTED
- If you break rules (3 strikes), you level down to SUSPICIOUS

**Real-world analogy:** It's like being a regular customer with a membership card.

---

### Level +2: 🟢 TRUSTED (VIP Status)
**What it means:** You've been a model visitor for a long time.

**How you get here:** You made 50 requests without breaking any rules.

**What happens:**
- You get the fastest access (300 requests every 10 seconds)
- Minimal security checks
- Still can level down if you mess up

**Real-world analogy:** It's like being a VIP with backstage access.

---

## How It Actually Works: The Journey

Let's follow "Bob" through his first visit:

### 1. Bob Connects
```
Bob types in the .onion address
      ↓
He connects to a PUBLIC MIRROR
(A fake door that looks like the real clubhouse)
```

### 2. The Bouncer Checks Bob
```
Bouncer (HTTP Proxy): "Do you have a pass?"
Bob: "No, first time here"
Bouncer: "Go see the Gate Guard for verification"
```

### 3. Bob Goes to the Gate
```
Gate Guard: "Are you human?"
Bob: "Yes!"
Gate Guard: "Prove it - solve this puzzle:"
[Shows CAPTCHA: "Type the word you see: BANANA"]
Bob: "B-A-N-A-N-A"
Gate Guard: "Correct! Here's your VERIFIED pass"
```

### 4. Bob Returns to the Bouncer
```
Bob: "I have my pass now!"
Bouncer: *checks pass with secret decoder ring (HMAC)*
Bouncer: "Pass is real. Welcome in!"
```

### 5. Bob Enters the Safe Zone
```
Bob is routed through a HEALTHY NODE
(A safe tunnel that only verified people use)
      ↓
He reaches the REAL CLUBHOUSE
(Your actual hidden service)
```

### 6. Bob Browses Normally
```
Bob visits pages, clicks links, reads content
Each request is analyzed for bad behavior
Everything looks good → Bob's violation count stays at 0
After 50 clean requests → Bob promoted to TRUSTED! 🎉
```

---

## What If Bob Was Actually Evil?

Let's say "Evil Bob" tries to attack:

### Attack Attempt 1: Path Traversal
```
Evil Bob tries: http://site.onion/../../../etc/passwd

Behavioral Analysis: 🚨 ATTACK PATTERN DETECTED
Violation Count: 0 → 1
Action: Warning logged, keep watching
```

### Attack Attempt 2: Directory Scanning
```
Evil Bob rapidly tries: /admin, /login, /backup, /config...

Behavioral Analysis: 🚨 RESOURCE ENUMERATION
Violation Count: 1 → 2
Action: Suspicion rising...
```

### Attack Attempt 3: Bot User-Agent
```
Evil Bob's script sends: User-Agent: python-requests/2.28

Behavioral Analysis: 🚨 BOT DETECTED
Violation Count: 2 → 3
Action: THRESHOLD EXCEEDED - DEMOTE TO SUSPICIOUS
```

### Evil Bob Gets Demoted
```
HTTP Proxy: "You're acting suspicious. Back to the Gate."
Gate Guard: "You need to solve 2 HARD puzzles now"
[Shows 2 difficult CAPTCHAs]
```

### Evil Bob's Bot Can't Solve CAPTCHAs
```
Bot fails the CAPTCHA (no JavaScript to solve it)
Access DENIED
Evil Bob's attack stopped ✅
```

---

## The Secret Sauce: Circuit-Based Rate Limiting

### The Old Problem (IP-Based)
```
Traditional sites: "Allow 100 requests per IP address"

For Tor: Everyone has the SAME IP address (127.0.0.1)!

Attack scenario:
- Attacker makes 100 requests → LIMIT REACHED
- Real user #1 tries to visit → BLOCKED
- Real user #2 tries to visit → BLOCKED
- Real user #3 tries to visit → BLOCKED

Result: Attack works! Real users can't access site 😞
```

### Fortify's Solution (Circuit-Based)
```
Fortify: "Each Tor CIRCUIT gets its own quota"

Attack scenario:
- Attacker Circuit A: makes 10 requests → Circuit A BLOCKED
- Attacker Circuit B: makes 10 requests → Circuit B BLOCKED
- Real User Circuit C: makes 3 requests → ✅ ALLOWED (separate quota)
- Real User Circuit D: makes 5 requests → ✅ ALLOWED (separate quota)

Result: Attack FAILS! Real users unaffected 🎉
```

**Why it works:** Each person using Tor has their own "circuit" (like a private tunnel). Fortify gives each tunnel its own quota, so one bad tunnel can't ruin it for everyone else.

---

## The Architecture: A Castle with Multiple Gates

```
        [INTERNET]
            │
    ┌───────┴───────┐
    ▼               ▼
[Mirror 1]      [Mirror 2]      ← PUBLIC (Disposable fake doors)
.onion           .onion
    │               │
    └───────┬───────┘
            ▼
      [HTTP PROXY]               ← BOUNCER (Checks passes)
            │
    ┌───────┼───────┐
    ▼       ▼       ▼
  [GATE] [HEALTHY] [THREAT]      ← SECURITY LAYERS
          [NODE]   [NODE]
            │       │
            └───┬───┘
                ▼
         [REAL SERVICE]          ← PROTECTED (Your actual site)
         (Hidden .onion)
```

### What Each Part Does:

**Public Mirrors** (The Fake Doors)
- These are the .onion addresses people connect to
- They're DISPOSABLE - if one gets attacked, burn it and make a new one
- You run 3-5 at a time

**HTTP Proxy** (The Bouncer)
- Checks everyone's pass (session token)
- No pass? → Send to Gate
- Valid pass? → Send to appropriate Node
- Analyzes behavior for suspicious patterns

**Gate** (The Puzzle Master)
- Shows CAPTCHAs to new visitors
- Issues passes (session tokens) when solved
- Uses harder puzzles for suspicious people

**Healthy Node** (The VIP Entrance)
- For verified and trusted users
- Fast path with minimal checks
- Connects to real service

**Threat Node** (The Interrogation Room)
- For suspicious users
- Heavy monitoring
- Slower path, strict limits

**Real Service** (The Secret Clubhouse)
- Your actual hidden service
- NEVER exposed to public
- Only accessible through Nodes

---

## The Magic: How Sessions Work

### Session Token = Your Pass

When you solve a CAPTCHA, you get a "session token" - think of it as a backstage pass:

```
Pass Contents:
┌─────────────────────────────────┐
│ Pass #: abc123def456            │  ← Unique ID
│ Trust Level: VERIFIED           │  ← Your current level
│ Issued: 2:00 PM                 │  ← When you got it
│ Expires: 3:00 PM                │  ← When it expires (1 hour)
│                                  │
│ Signature: [secret stamp]       │  ← Proves it's real
└─────────────────────────────────┘
```

### The Secret Stamp (HMAC-SHA256)

The "signature" is like a secret stamp that only the real bouncers know how to make:

```
Creating a pass:
1. Take all the pass info
2. Mix it with a SECRET KEY (only the server knows)
3. Run it through a special formula (HMAC-SHA256)
4. Get a unique code that's impossible to fake

Checking a pass:
1. Take the pass info someone shows you
2. Mix it with the same SECRET KEY
3. Run the same formula
4. If the code matches → REAL PASS ✅
5. If it doesn't match → FAKE PASS ❌ DENIED
```

**Why this matters:** Even if a hacker copies your pass, they can't change the trust level from VERIFIED to TRUSTED because they don't know the secret key to recreate the stamp.

---

## Real-World Examples

### Example 1: Normal User
```
Sarah visits your site for the first time:
1. Gets sent to Gate
2. Solves CAPTCHA: "What's 5 + 3?" → "8"
3. Gets VERIFIED pass
4. Browses 10 pages normally
5. Still VERIFIED (no violations)
6. Browses 40 more pages
7. Promoted to TRUSTED! (50 clean requests)
8. Now gets fastest access
```

### Example 2: Confused User
```
Mike accidentally triggers a false positive:
1. Has VERIFIED pass
2. Accidentally types wrong URL: /admni (typo)
3. Tries /admin, /admon, /administrator (looking for admin page)
4. System sees 4 rapid similar paths
5. Flagged as "path enumeration"
6. Violation count: 0 → 1
7. Warning logged but no demotion yet
8. Mike finds correct URL, browses normally
9. Violation count stays at 1 (no more issues)
10. Mike is fine, still VERIFIED
```

### Example 3: Scraper Bot
```
ScraperBot tries to steal all your content:
1. Gets sent to Gate
2. Can't solve CAPTCHA (no JavaScript)
3. Access DENIED immediately
4. Bot stopped at the door ✅
```

### Example 4: Advanced Attacker
```
Hacker uses CAPTCHA solving service:
1. Pays $1 to solve CAPTCHA
2. Gets VERIFIED pass
3. Starts scraping: /page1, /page2, /page3...
4. Path enumeration detected (5 sequential paths)
5. Violation count: 3 → DEMOTED to SUSPICIOUS
6. Sent back to Gate
7. Needs to solve 2 HARD CAPTCHAs now
8. Pays $2 more to solve both
9. Gets new VERIFIED pass
10. Tries scraping again
11. Detected again → Demotion count: 2
12. Tries THIRD time → Demotion count: 3
13. PERMANENTLY BURNED ❌

Cost to attacker: $9 to get burned
Cost to you: $0
Result: Attack is too expensive to sustain ✅
```

---

## Why No JavaScript?

### The Problem
Most security systems use JavaScript:
- CAPTCHA solving
- Browser fingerprinting
- Complex challenges

### Why That's Bad for Tor
Tor Browser "Safest" mode (most secure setting) disables JavaScript completely.

If your security requires JavaScript:
- Real privacy-conscious users can't access your site
- You're forcing users to lower their security
- Defeats the purpose of using Tor

### Fortify's Solution
Everything works with PURE HTML and CSS:
- Server-side CAPTCHA generation
- Form-based challenges
- CSS-only interactions

Result: Works perfectly with Tor Browser Safest mode ✅

---

## Common Questions

### Q: Can't attackers just get new Tor circuits?
**A:** Yes, but each new circuit still needs to solve a CAPTCHA. If they use bots, they're blocked. If they pay for solving, it becomes expensive. If they solve manually, they can only make so many circuits before it's not worth their time.

### Q: What if someone copies my session token?
**A:** The token is signed with HMAC-SHA256. If they try to modify it (like changing trust level), the signature won't match and it gets rejected. If they use it as-is, their behavior is still monitored - if they do bad things, that token gets demoted/burned.

### Q: How do you know it's a bot?
**A:** Multiple signals:
- User-Agent header (curl, wget, python-requests, etc.)
- Behavior patterns (too fast, too regular, too mechanical)
- Attack attempts (path traversal, directory scanning)
- Missing expected browser headers

### Q: Can't bots solve CAPTCHAs now?
**A:** Some can, but it's expensive. The key is:
1. CAPTCHAs stop simple bots (cheap attacks)
2. For advanced bots using solving services, the demotion system makes it unprofitable
3. After 3 demotions (each requiring 2 CAPTCHAs), the cost becomes prohibitive

### Q: What if I'm doing something legitimate that triggers violations?
**A:** Thresholds are set reasonably high:
- 3 violations before demotion
- 60 unique pages per minute allowed
- 10 form submissions per minute
- Normal browsing won't trigger these

If you have a legitimate use case that hits limits, they're configurable.

---

## The Bottom Line

**Fortify is a smart, multi-layered security system that:**

1. **Verifies humans** with CAPTCHAs (blocks bots)
2. **Watches behavior** to catch sophisticated attackers
3. **Uses trust levels** to route traffic appropriately
4. **Limits attackers** with per-circuit rate limiting
5. **Degrades attackers** through demotion system
6. **Protects your real service** by keeping it hidden
7. **Respects privacy** by working without JavaScript
8. **Fails safely** by denying access when in doubt

All while keeping your real hidden service completely separate and protected.

---

## Visualizing the Flow

```
┌──────────────────────────────────────────────────────────────┐
│                    FORTIFY IN ACTION                          │
└──────────────────────────────────────────────────────────────┘

👤 NEW USER arrives
   │
   ├─ No pass → Route to GATE
   │            │
   │            ├─ Show CAPTCHA
   │            ├─ User solves it ✅
   │            └─ Issue VERIFIED pass
   │
   ├─ Has pass → Validate signature
   │            │
   │            ├─ Valid? ✅ → Analyze behavior
   │            │              │
   │            │              ├─ Good → Route to HEALTHY NODE
   │            │              │          │
   │            │              │          └─ Access REAL SERVICE ✅
   │            │              │
   │            │              └─ Bad → Demote → Route to GATE
   │            │                           │
   │            │                           └─ Show 2 hard CAPTCHAs
   │            │
   │            └─ Invalid? ❌ → Reject → Route to GATE
   │
   └─ Attack? 🤖 → Block → Show error page ❌

```

---

*Now you understand how Fortify keeps the bad guys out while letting the good guys in!* 🎉

**Want more details?** Check out the other documentation:
- [Architecture Overview](../01-Architecture/overview.md) - Technical details
- [Trust Tiers](../02-Core-Concepts/trust-tiers.md) - How the levels work
- [Behavioral Analysis](../02-Core-Concepts/behavioral-analysis.md) - How we detect attacks
- [Threat Model](../03-Security-Model/threat-model.md) - What we protect against
