---
id: CHORE-010
title: "Implement §2's 'degraded at window ≥ warn %': [probe] warn_pct = 90 (steer bucket 2)"
type: chore
status: provisioning
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, design]
depends_on: []
---

# Implement §2's 'degraded at window ≥ warn %': [probe] warn_pct = 90 (steer bucket 2)

Found in the EXP-004 review... correction: in the EXP-002 review (`reviews/EXP-002-r1.md` F2, PR #12). DESIGN.md §2's state table lists "a window ≥ warn %" under `degraded`, but no `warn %` config or code exists, so a subscription window at 97 % reads `ok`.

**Decided by steer, bucket 2 (2026-10-08, open to the Captain's veto; basis in the comment):** option (a), `[probe] warn_pct`, default **90**.

## Definition of Done
1. Config: `[probe] warn_pct` (a number, 0 < x ≤ 100), default 90. An out-of-range value is refused with an error that names the key.
2. A row whose state would otherwise be `ok` reads `degraded` when ANY of its windows (`headroom.windows[].used_pct`, or `headroom.window_pct`) is ≥ `warn_pct`. Known answers: 89.9 → ok, 90 → degraded, 97 → degraded. A control: with `warn_pct = 100`, 97 reads ok. A worse state (quota_exhausted, rate_limited…) is never softened to degraded.
3. `select` refuses such a row (it already refuses degraded; a test pins it). `alert` treats it as any bad state until SIG-010 is ruled.
4. DESIGN §2/§3 and `examples/quotabus.toml` document `warn_pct`, citing this item and the steer decision. `make check` is green.

## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 20:15)

Kept in harbor: it lacks a default warn % (or a ruling to drop the clause). No ruling names one; the question is for the Captain.

### M5-MBP-2/s-72a67d16 (2026-10-08 21:58)

[steer] bucket-2: option (a). Implement §2's clause with `[probe] warn_pct = 90` (configurable): any window whose used_pct is >= warn_pct reads `degraded`, for subscription `headroom.windows[]` and API `window_pct`.
Basis:
- G2 and DESIGN §1 Goal: 'does it work right now, and how much is left?'. The motivating failure: 'the Copilot subscription ran out and was found only when a review failed' (§1). A near-limit state is exactly what that needed.
- G1: `degraded` is never `ok`, and select already refuses it, so this cannot produce a false all-clear.
- G3: one key, already designed in §2.
90 is the default because a 5 h window at 90 % still leaves time to act, while 80 % would flag routine heavy use. It is reversible (config).
Whether a near-limit degraded FILES a signal is SIG-010 (open, bucket 3). Until it is ruled, it files as every bad state does today.
Released. Decided — open to the Captain's veto.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T21:58:05+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
