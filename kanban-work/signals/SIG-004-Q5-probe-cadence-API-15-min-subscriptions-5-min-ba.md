---
id: SIG-004
title: "Q5: probe cadence — API 15 min, subscriptions 5 min, balances 15 min?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Q5: probe cadence — API 15 min, subscriptions 5 min, balances 15 min?

Design §10 Q5. Cadence and spend. **Recommendation:** API probes every 15 min (at most 96 × ~30 tokens per model per day), subscription reads every 5 min (free), balance reads every 15 min — all configurable.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001/EXP-002 (15 min API and balances, 5 min subscriptions — all config); a different ruling is a config change or a later PR.
