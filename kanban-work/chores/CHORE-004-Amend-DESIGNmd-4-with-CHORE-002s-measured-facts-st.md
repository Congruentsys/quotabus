---
id: CHORE-004
title: "Amend DESIGN.md §4 with CHORE-002's measured facts (statusLine under claude -p; z.ai quota endpoint)"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, design]
depends_on: []
---

# Amend DESIGN.md §4 with CHORE-002's measured facts (statusLine under claude -p; z.ai quota endpoint)

Part of VOY-001. CHORE-002 measured two §4 questions (`docs/findings/CHORE-002-statusline-under-claude-p.md`, `docs/findings/CHORE-002-zai-endpoint.md`, PR #3). §4 still says "measure first" and "no documented endpoint". This chore RECORDS the measured facts. It decides nothing: the Q9 consequence is SIG-008 (open) and the undocumented-source policy is SIG-003 (open). Both are cited as open.

## Definition of Done
A PR to `docs/DESIGN.md` §4 (and the §10 Q9 line) that:
1. Claude Max row: replaces "whether the hook fires under `claude -p` is **measure first**" with the result (it does not; interactive only; `rate_limits` is absent on the first render, so absent means unknown, not 0 %). Names the stream-json `rate_limit_event` as a measured alternative, pending SIG-008.
2. GLM / z.ai row: adds `GET /api/monitor/usage/quota/limit` (5 h and weekly quota windows; undocumented; field meanings [inferred]; the counter did not move after a 35-token call) and `/api/biz/subscription/list` (plan status; billing fields never published), pending SIG-003. Keeps "no wallet balance". Records that **a bad key gets HTTP 200 with body `code: 401`** on three of the five endpoints, so an adapter reads the body's `code`/`success`. Records that there are no rate-limit headers.
3. Each change cites the findings file. `make check` is green.