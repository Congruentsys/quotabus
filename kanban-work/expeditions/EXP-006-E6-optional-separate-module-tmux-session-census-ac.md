---
id: EXP-006
title: "E6 (optional, separate module): tmux session census across the hosts, read from the hub"
type: expedition
status: backlog
priority: low
assignee: null
created: 2026-10-08
tags: [post-v1.0]
depends_on: []
---

# E6 (optional, separate module): tmux session census across the hosts, read from the hub

Optional, after v1.0, behind a feature flag. The Captain (2026-10-08): M5 will be the hub from which every host's tmux sessions are reached. The per-host agent could publish its host's sessions (`census/1.0` rows: session, windows, last activity, attached) so the hub sees them all. Design §7, §9 row E6.

## Definition of Done
`census/1.0` rows for five hosts on the bus, behind a feature flag, off by default.