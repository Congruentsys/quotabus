---
id: SIG-007
title: "Q10: defer Anthropic/OpenAI admin keys (cost APIs) to E5?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Q10: defer Anthropic/OpenAI admin keys (cost APIs) to E5?

Design §10 Q10. Admin keys: Anthropic's Admin API needs an organisation; OpenAI's cost API needs an admin key. **Recommendation:** defer — for these two the probe is the status; revisit with E5's spend view.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001 (no admin keys; the probe is the status); a different ruling is a config change or a later PR.
