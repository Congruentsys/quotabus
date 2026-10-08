---
id: SIG-007
title: "Q10: defer Anthropic/OpenAI admin keys (cost APIs) to E5?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q10: defer Anthropic/OpenAI admin keys (cost APIs) to E5?

Design §10 Q10. Admin keys: Anthropic's Admin API needs an organisation; OpenAI's cost API needs an admin key. **Recommendation:** defer — for these two the probe is the status; revisit with E5's spend view.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001 (no admin keys; the probe is the status); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "Defer to E5". This is the recommended default: no admin keys in v1.0, the probe result is the status for these two providers, and this is revisited with E5.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:52+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
