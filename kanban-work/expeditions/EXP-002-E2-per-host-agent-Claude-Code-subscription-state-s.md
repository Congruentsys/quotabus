---
id: EXP-002
title: "E2: per-host agent — Claude Code subscription state (statusLine capture + ~/.claude.json fallback) and GitHub Copilot quota, one row per host"
type: expedition
status: underway
priority: high
assignee: M5-MBP-2/s-72a67d16
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
  ],
  [
    kb:status kb:backlog ;
    kb:at "2026-10-08T18:29:05+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:ready ;
    kb:at "2026-10-08T18:53:15+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T18:57:09+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Released with depends_on EXP-001, CHORE-002 (statusLine install waits on CHORE-002's measurement, Captain Q9). SIG-003 is built to the recommendation (undocumented sources off by default in the FOSS config, on in the fleet config, every row labelled source=undocumented); SIG-004's 5-min subscription cadence is config.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:29)

CHORE-002 (PR #3): the statusLine hook does NOT fire under claude -p. Step 2's precondition ('only after CHORE-002 shows it fires under claude -p') is therefore false, and the Captain's conditional Q9 yes goes back as SIG-008 (open). That signal also names a measured alternative: stream-json rate_limit_event. Back to harbor until SIG-008 is answered; the Copilot and ~/.claude.json fallback steps do not depend on it.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:53)

SIG-008 ruled (Captain 2026-10-08, 'Both + fallback'). Step 2 becomes: install the statusLine for interactive hosts, ALSO tee the stream-json rate_limit_event from claude -p runs into the same cache, and keep the ~/.claude.json fallback. A missing rate_limits reads as unknown, never 0 % (docs/findings/CHORE-002-statusline-under-claude-p.md). SIG-004/Q5: subscription reads stay at 5 min. SIG-003/Q4: Copilot copilot_internal is on in the fleet, off in FOSS, labelled. Released again to provisioning.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:56)

Captain 2026-10-08, adding to the Q5 ruling: "for how often to query, make sure there are separate times for the different types of queries." So each query kind has its OWN config interval, set independently: API (messages) probe, balance read, and subscription read (and any later kind, e.g. a quota-window read). There is no shared interval. Defaults per the SIG-004 ruling: API 12 h, balance 12 h, subscription 5 min.
