---
id: CHORE-025
title: "alert summary: pin 'local skipped' for a discovering local service (CHORE-024 r1 F6)"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-10
depends_on: []
---

# alert summary: pin 'local skipped' for a discovering local service (CHORE-024 r1 F6)

From the CHORE-024 r1 review (reviews/CHORE-024-r1.md, PR #32), F6, non-blocking. The alert summary's M ('local skipped') is right for a local service that discovers its models (no `models`; CHORE-022 `slots_in`). The reviewer measured M=1 with an empty store and M=2 when the newest cycle served two models, using a throwaway test. No committed test pins it, so using `slots()` in alert would read M=0 for dgx1-qwen/dgx2-qwen and every test would stay green.

## Done when
1. A test runs `quotabus alert` over a discovering local service. It asserts M=1 when the store holds none of its rows, and M = the served-id count when the newest cycle wrote several. It includes a control: an older cycle's ids are not counted.
2. The test goes red if alert counts with `slots()` (shown once, then reverted). `make check` green.