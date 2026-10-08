---
id: SIG-002
title: "Q3: publish balances and account labels on the bus (slugs, never emails)?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q3: publish balances and account labels on the bus (slugs, never emails)?

Design §10 Q3. Publish balances and account labels to the bus? **Recommendation:** numbers yes; labels as configured slugs, never emails; `publish_balance` stays a per-service switch for outsiders.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001 (balances and account labels published as slugs; publish_balance per service); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "Numbers + slugs". This is the recommended default: balances are published, accounts appear as configured slugs (never emails), and publish_balance is a per-service switch.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:41+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
