---
id: CHORE-006
title: "Record the Captain's 2026-10-08 rulings on §10 (SIG-001…008) in DESIGN.md"
type: chore
status: underway
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-08
tags: [v1.0, VOY-001, design]
depends_on: []
---

# Record the Captain's 2026-10-08 rulings on §10 (SIG-001…008) in DESIGN.md

Part of VOY-001. On 2026-10-08 the Captain answered every open §10 signal (SIG-001…SIG-008); each ruling is verbatim in a comment on its signal. `docs/DESIGN.md` still shows them as recommendations. Two rulings differ from the recommendation:
- **Q2 (SIG-001):** the central probe's host is a CONFIG choice (local, or a named host); the fleet sets Mini.
- **Q5 (SIG-004):** API probes and balance reads default to every 12 h; subscription reads stay at 5 min; every interval is config. §4's cost line ("≤ 96 calls … per day") changes to match.
- **Q9 (SIG-008):** statusLine + stream-json `rate_limit_event` + `~/.claude.json` fallback.

## Definition of Done
A PR to `docs/DESIGN.md` that, for each of Q2, Q3, Q4, Q5, Q7, Q8, Q9 and Q10 in §10 (and the Origin note):
1. Records the ruling verbatim as `Captain 2026-10-08: "…"`, citing its SIG, and says whether it was the recommended default.
2. Updates every other sentence it makes stale (e.g. §4's cadence and cost line, the Claude Max row's "pending SIG-008", the GLM row's "pending SIG-003", §3's host for the central probe).
3. `make check` is green. A distinct session reviews it (a DESIGN.md change).

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T18:53:23+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T19:08:02+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 18:56)

Captain 2026-10-08, adding to the Q5 ruling: "for how often to query, make sure there are separate times for the different types of queries." So each query kind has its OWN config interval, set independently: API (messages) probe, balance read, and subscription read (and any later kind, e.g. a quota-window read). There is no shared interval. Defaults per the SIG-004 ruling: API 12 h, balance 12 h, subscription 5 min.
