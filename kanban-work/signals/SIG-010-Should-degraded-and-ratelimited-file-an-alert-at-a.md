---
id: SIG-010
title: "Should 'degraded' (and 'rate_limited') file an alert at all — or only hard failures?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Should 'degraded' (and 'rate_limited') file an alert at all — or only hard failures?

Raised by the EXP-004 review (`reviews/EXP-004-r1.md` F2, PR #15). Under the agreed rule (SIG-006, "Signal per crossing"; only a measured `ok` re-arms), every crossing from `ok` into ANY bad state files one signal. The reviewer measured `ok`↔`degraded` flapping (a slow 200, or a window near its limit) at **5 signals in 10 cycles**. That is correct under the rule, but noisy for a state that is not a failure. (Bad→bad alternation is being fixed in EXP-004 itself: after an alert, only a measured `ok` re-arms it.)

## The question
1. **Every bad state files** (`degraded`, `rate_limited`, `quota_exhausted`, `auth_failed`, `model_missing`…). This is what EXP-004 ships now.
2. **Only hard failures file** (`quota_exhausted`, `auth_failed`, `model_missing`, unreachable). `degraded` and `rate_limited` are published on the bus, and `select` refuses them, but they file no signal.
3. **A configurable `[alert] states = [...]`** with 2 as its default.

## Recommendation (an agent's, not a decision)
3 with 2 as the default: a signal is for a person to act on, and `degraded`/`rate_limited` usually clear on their own.

## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 21:57)

[steer] bucket-3: the trigger is REVERSING OR NARROWING A §10 RULING.
The Captain's SIG-006 ruling, 'Signal per crossing' (DESIGN §10 Q8), is built as one signal per state crossing into ANY bad state. Filing only hard failures would narrow what that ruling files, and that is the Captain's call, not a session's.
Recommended default: option 3 with option 2 as the default: `[alert] states`, defaulting to the hard failures (quota_exhausted, auth_failed, model_missing, unreachable) PLUS a near-limit window (see CHORE-010, decided alongside). Latency-degraded and rate_limited stay on the bus and are refused by select, but file no signal.
Left open.
