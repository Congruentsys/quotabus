---
id: SIG-002
title: "Q3: publish balances and account labels on the bus (slugs, never emails)?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Q3: publish balances and account labels on the bus (slugs, never emails)?

Design §10 Q3. Publish balances and account labels to the bus? **Recommendation:** numbers yes; labels as configured slugs, never emails; `publish_balance` stays a per-service switch for outsiders.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001 (balances and account labels published as slugs; publish_balance per service); a different ruling is a config change or a later PR.
