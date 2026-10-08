---
id: EXP-004
title: "E4: alerts with crossing-dedup (yurtle-kanban, nusy-kanban, webhook) and the Yurtle config front-end"
type: expedition
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-003]
---

# E4: alerts with crossing-dedup (yurtle-kanban, nusy-kanban, webhook) and the Yurtle config front-end

Part of VOY-001. Design §3 (alert, config), §9 row E4. The dead balance scanner's good rule is kept: only a measured `ok` clears an alert; CANNOT-ASSESS never does.

## Plan
1. `quotabus alert`: one item per crossing (a service going bad), never auto-closed; sinks: yurtle-kanban (`create signal --push`), nusy-kanban, webhook, stdout.
2. The Yurtle front-end: the same rows as the TOML config in one `yurtle-table` block; `examples/` with both.

## Definition of Done
A model 404 files ONE signal across ten probe cycles; a measured `ok` re-arms it; a CANNOT-ASSESS does not; the Yurtle and TOML examples load to identical configs.