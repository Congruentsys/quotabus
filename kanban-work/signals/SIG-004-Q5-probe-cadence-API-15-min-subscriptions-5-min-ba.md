---
id: SIG-004
title: "Q5: probe cadence — API 15 min, subscriptions 5 min, balances 15 min?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q5: probe cadence — API 15 min, subscriptions 5 min, balances 15 min?

Design §10 Q5. Cadence and spend. **Recommendation:** API probes every 15 min (at most 96 × ~30 tokens per model per day), subscription reads every 5 min (free), balance reads every 15 min — all configurable.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001/EXP-002 (15 min API and balances, 5 min subscriptions — all config); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "Slower - twice a day, but make it a config setting"; on scope: "API + balance only". This is NOT the default. API probes and balance reads default to every 12 h, subscription reads stay at every 5 min, and every interval is a config setting.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:46+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
