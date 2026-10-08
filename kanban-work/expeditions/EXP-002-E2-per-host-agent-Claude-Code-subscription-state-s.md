---
id: EXP-002
title: "E2: per-host agent — Claude Code subscription state (statusLine capture + ~/.claude.json fallback) and GitHub Copilot quota, one row per host"
type: expedition
status: provisioning
priority: high
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-001, CHORE-002]
---

# E2: per-host agent — Claude Code subscription state (statusLine capture + ~/.claude.json fallback) and GitHub Copilot quota, one row per host

Part of VOY-001; after E1 and CHORE-002 (C2). Design §3 (agent), §4 (Claude Max, Copilot rows), §9 row E2.

## Plan
1. `quotabus agent` runs on each host and reads its OWN subscription state locally (the `claude` auth probe gives wrong answers over SSH on macOS, so nothing central can read it).
2. Claude Code: the official `statusLine` capture — **Captain 2026-10-08, §10 Q9: yes**, a `statusLine` entry in every host's `~/.claude/settings.json`, installed by this expedition's unit, only after CHORE-002 shows it fires under `claude -p`; fallback: `~/.claude.json` `cachedUsageUtilization`; a token-login host reads `unknown`, reason `cannot_assess:token_login_no_usage_panel`, never `ok`.
3. GitHub Copilot: `gh api copilot_internal/user` (undocumented — labelled `source = undocumented`).
4. Units for all five hosts (launchd on macOS, systemd user units on Linux), reset countdown and pace in the record.

## Definition of Done
Five `subscription.*` rows on the bus from five hosts, each `observed_by` its own host, with ages; the token-login case reads `cannot_assess`, never `ok`.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T17:22:10+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ] .
```
