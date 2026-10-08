---
id: EXP-005
title: "E5 (optional, post-v1.0): local web view, spend via ccusage, key expiry, model-drift sweep"
type: expedition
status: backlog
priority: low
assignee: null
created: 2026-10-08
tags: [post-v1.0]
depends_on: []
---

# E5 (optional, post-v1.0): local web view, spend via ccusage, key expiry, model-drift sweep

Optional, after v1.0. Design §7 and §9 row E5: `quotabus serve` (one page on 127.0.0.1), spend from `ccusage --json` against a configured budget, key expiry dates (`expires` within 14 days reads `degraded`), a sweep that probes every configured model id for drift; polish of the file backend for outsiders without NATS.

## Definition of Done
One page on 127.0.0.1 shows the table; an `expires` 14 days out reads `degraded`.