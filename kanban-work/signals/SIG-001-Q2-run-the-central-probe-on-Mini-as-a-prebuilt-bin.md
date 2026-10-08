---
id: SIG-001
title: "Q2: run the central probe on Mini as a prebuilt binary?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Q2: run the central probe on Mini as a prebuilt binary?

Design §10 Q2. Where does the central probe run? **Recommendation:** Mini, as a prebuilt binary under launchd (no build on the bus host); M5 runs the per-host agent plus the hub's status/serve. Blocks EXP-001's deployment step, not its code.