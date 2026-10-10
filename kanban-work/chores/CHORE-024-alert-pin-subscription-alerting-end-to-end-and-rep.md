---
id: CHORE-024
title: "alert: pin subscription alerting end to end and report skipped local slots in the summary (CHORE-021 r1 nits)"
type: chore
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-10
depends_on: []
---

# alert: pin subscription alerting end to end and report skipped local slots in the summary (CHORE-021 r1 nits)

From the CHORE-021 r1 review (reviews/CHORE-021-r1.md, PR #30), both non-blocking:
- F1: no integration test drives a SUBSCRIPTION service through `quotabus alert` after CHORE-021. Only a unit test (`alerts_for`) pins that subscription still alerts.
- F2: `alert: N keys checked` counts local slots that CHORE-021 now skips, without saying so.

## Done when
1. An integration test (fake nusy-kanban, file backend) drives a subscription service through the same crossings as tests/chore021_local_no_alert.rs. It files per SIG-010's default, with a control that goes red if subscription is skipped like local.
2. The alert summary line reports skipped local slots separately (e.g. `N keys checked, M local skipped`), or excludes them and says so. A test pins the counts.
3. `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-10T13:42:12+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T14:04:18+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
