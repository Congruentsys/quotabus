---
id: SIG-001
title: "Q2: run the central probe on Mini as a prebuilt binary?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q2: run the central probe on Mini as a prebuilt binary?

Design §10 Q2. Where does the central probe run? **Recommendation:** Mini, as a prebuilt binary under launchd (no build on the bus host); M5 runs the per-host agent plus the hub's status/serve. Blocks EXP-001's deployment step, not its code.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001 (its launchd plist targets Mini, a prebuilt binary); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "If this is FOSS, the config will have to ask to run locally or on another host. In this case, run on mini". This is NOT the plain default: where the central probe runs must be a CONFIG choice (local, or a named other host) for FOSS users. Our fleet sets it to Mini (prebuilt binary under launchd, as recommended).

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:39+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
