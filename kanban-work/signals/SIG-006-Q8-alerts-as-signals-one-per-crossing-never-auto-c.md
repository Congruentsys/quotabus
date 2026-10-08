---
id: SIG-006
title: "Q8: alerts as signals (one per crossing, never auto-closed)?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q8: alerts as signals (one per crossing, never auto-closed)?

Design §10 Q8. Alert item type. **Recommendation:** a `signal` tagged `provider-status`, one per crossing, never auto-closed; a `hazard` only if a crossing should block work.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-004 (signal, one per crossing, never auto-closed; item type is sink config); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "Signal per crossing". This is the recommended default: a signal tagged provider-status, one per crossing, never auto-closed; a hazard only when a crossing should block work.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:50+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
