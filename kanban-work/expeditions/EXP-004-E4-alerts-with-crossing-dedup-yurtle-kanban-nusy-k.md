---
id: EXP-004
title: "E4: alerts with crossing-dedup (yurtle-kanban, nusy-kanban, webhook) and the Yurtle config front-end"
type: expedition
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-003]
---

# E4: alerts with crossing-dedup (yurtle-kanban, nusy-kanban, webhook) and the Yurtle config front-end

Part of VOY-001. Design §3 (alert, config), §9 row E4. The dead balance scanner's good rule is kept: only a measured `ok` clears an alert; CANNOT-ASSESS never does.

## Plan
1. `quotabus alert`: one item per crossing (a service going bad), never auto-closed; sinks: yurtle-kanban (`create signal --push`), nusy-kanban, webhook, stdout.
2. The Yurtle front-end: the same rows as the TOML config in one `yurtle-table` block; `examples/` with both.

## Definition of Done
A model 404 files ONE signal across ten probe cycles; a measured `ok` re-arms it; a CANNOT-ASSESS does not; the Yurtle and TOML examples load to identical configs.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T17:22:14+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T21:03:58+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Released with depends_on EXP-003. SIG-006 is built to the recommendation (alert = signal, one per crossing, never auto-closed; the item type is sink config), SIG-005's Yurtle twin lands here per the recommendation.
