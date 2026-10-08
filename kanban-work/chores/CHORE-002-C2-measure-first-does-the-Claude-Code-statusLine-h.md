---
id: CHORE-002
title: "C2: measure first — does the Claude Code statusLine hook fire under 'claude -p', and which z.ai endpoint answers an API key"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, measure]
depends_on: [CHORE-003]
---

# C2: measure first — does the Claude Code statusLine hook fire under 'claude -p', and which z.ai endpoint answers an API key

Part of VOY-001; before E2. Design §4 and §10 Q9. The Captain said yes (2026-10-08) to a `statusLine` entry in every host's `~/.claude/settings.json`, so E2 installs it — but only once this chore confirms it actually fires for `claude -p` sessions (the fleet's reviewers run that way). Also find which z.ai endpoint answers a plain API key (the docs list several).

## Definition of Done
Two findings files under `docs/findings/`, each with the exact command and its output: (1) statusLine under `claude -p` — fires or not, with the JSON it receives; (2) the z.ai endpoint that answers, and what it reports.