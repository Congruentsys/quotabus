---
id: CHORE-011
title: "Round subscription windows' used_pct (0.14 × 100 is stored as 14.000000000000002)"
type: chore
status: underway
priority: low
assignee: Mac-mini/s-e7976c42
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
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T21:36:14+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ] .
```


## Comments

### Mac-mini/s-e7976c42 (2026-10-08 21:38)

tests red at 98335d9 (branch chore/CHORE-011-round-used-pct): tests/chore011_round_used_pct.rs via Runner::run_due — 6 red on assertions (unified 0.14 → 14.000000000000002, 0.123456 → 12.3456; stream-json same; Copilot 100−33.333 → 66.667; entitlement fallback 66.666…), 2 controls show the == and 2-dp checks can fail; window_pct also checked.
