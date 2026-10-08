---
id: SIG-003
title: "Q4: undocumented usage endpoints — off by default, on for the fleet, always labelled?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q4: undocumented usage endpoints — off by default, on for the fleet, always labelled?

Design §10 Q4. Undocumented sources (Anthropic oauth/usage, Copilot copilot_internal/user, z.ai biz endpoints). **Recommendation:** off by default in the shipped config; on in the fleet's config; every row from one labelled `source = undocumented`, so it degrades to unknown, never to a false all-clear.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-002 (undocumented sources off by default in the FOSS config, on in the fleet config, labelled source=undocumented); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:29)

CHORE-002 (PR #3) measured z.ai's undocumented GET /api/monitor/usage/quota/limit and /api/biz/subscription/list. Both answer an NUSY_GLM API key. The findings file builds to §10 Q4's RECOMMENDATION (off in the FOSS config, on and labelled in the fleet's) and cites this signal as open; it decides nothing.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "Off FOSS, on fleet". This is the recommended default: undocumented sources are off in the shipped config and on in the fleet config, and every row is labelled source=undocumented.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:43+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
