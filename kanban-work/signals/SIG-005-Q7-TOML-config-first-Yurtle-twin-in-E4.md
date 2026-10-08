---
id: SIG-005
title: "Q7: TOML config first, Yurtle twin in E4?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q7: TOML config first, Yurtle twin in E4?

Design §10 Q7. The Captain's words led with a Yurtle config. **Recommendation:** TOML first (E1, fastest, what outsiders expect); the Yurtle twin follows in E4 as the same rows in one `yurtle-table` block.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001 (TOML first) and EXP-004 (the Yurtle twin); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "TOML first, Yurtle in E4". This is the recommended default.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:48+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
