---
id: CHORE-011
title: "Round subscription windows' used_pct (0.14 × 100 is stored as 14.000000000000002)"
type: chore
status: provisioning
priority: low
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: []
---

# Round subscription windows' used_pct (0.14 × 100 is stored as 14.000000000000002)

Seen in HAZ-003's real path (PR #13): `subscription.anthropic.hankh95.claude-hankh95` stored `seven_day used_pct = 14.000000000000002`, because the unified header's `0.14` is multiplied by 100 as an f64. Readers that compare thresholds are fine, but the row and the `status` table show the noise.

## Definition of Done
`used_pct` for every window (unified headers, stream-json `utilization`, Copilot `100 − percent_remaining`) is rounded to at most 2 decimals when the row is built. A known-answer test (0.14 → 14.0, 0.005 → 0.5 and similar) and a control that fails on the unrounded value. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T20:33:56+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
