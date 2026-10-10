---
id: CHORE-025
title: "alert summary: pin 'local skipped' for a discovering local service (CHORE-024 r1 F6)"
type: chore
status: arrived
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-10
depends_on: []
---

# alert summary: pin 'local skipped' for a discovering local service (CHORE-024 r1 F6)

From the CHORE-024 r1 review (reviews/CHORE-024-r1.md, PR #32), F6, non-blocking. The alert summary's M ('local skipped') is right for a local service that discovers its models (no `models`; CHORE-022 `slots_in`). The reviewer measured M=1 with an empty store and M=2 when the newest cycle served two models, using a throwaway test. No committed test pins it, so using `slots()` in alert would read M=0 for dgx1-qwen/dgx2-qwen and every test would stay green.

## Done when
1. A test runs `quotabus alert` over a discovering local service. It asserts M=1 when the store holds none of its rows, and M = the served-id count when the newest cycle wrote several. It includes a control: an older cycle's ids are not counted.
2. The test goes red if alert counts with `slots()` (shown once, then reverted). `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-10T14:16:06+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T14:16:21+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-10T14:27:01+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/33> ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-10 14:19)

tests at 5123e5f: tests/chore025_alert_local_skipped_discovering.rs, 4 tests, all green on today's code. This item is test-only: it pins behaviour that already exists. Red proven by mutation instead, with each change reverted. (1) alert_with counting with svc.slots(): 3 of 4 red (M read 0, expected 1/3/2). (2) slots_in without its newest-cycle filter: the older-cycle control is red (M=5, expected 2).
