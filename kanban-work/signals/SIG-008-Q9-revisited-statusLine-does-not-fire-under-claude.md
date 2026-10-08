---
id: SIG-008
title: "Q9 revisited: statusLine does not fire under claude -p — install it anyway, and/or capture stream-json rate_limit_event?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Q9 revisited: statusLine does not fire under claude -p — install it anyway, and/or capture stream-json rate_limit_event?

The Captain's §10 Q9 "yes" (2026-10-08) was conditional: EXP-002 step 2 installs the `statusLine` entry "only after CHORE-002 shows it fires under `claude -p`". CHORE-002 measured the opposite: **the statusLine hook does not fire under `claude -p`** (text or stream-json). It does fire in an interactive session, with `rate_limits.{five_hour,seven_day}`, from the first render after an API reply (`docs/findings/CHORE-002-statusline-under-claude-p.md`, PR #3; the reviewer re-ran it and got the same result).

A second official route was found: `claude -p --output-format stream-json` emits a `rate_limit_event` carrying the same two windows (`unifiedWindows.five_hour/seven_day.{utilization,resetsAt}`), which agreed with the statusLine to the unit in the same minute.

## The question
1. Install the `statusLine` entry anyway, for hosts where an interactive session runs (it is necessary but not sufficient)?
2. Add the stream-json `rate_limit_event` as a second source (e.g. a reviewer launcher tees that event to the same cache file)?
3. Or rely on the `~/.claude.json` `cachedUsageUtilization` fallback alone?

## Recommendation (an agent's, not a decision)
Yes to 1 and 2, with the fallback kept: the statusLine covers interactive hosts, `rate_limit_event` covers `claude -p`-only hosts, and both are official. EXP-002 waits on this answer for its Claude Code step.

## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08 (asked by M5-MBP-2/s-72a67d16): "Both + fallback". This is the recommended default. E2 installs the statusLine for interactive hosts, ALSO captures the stream-json rate_limit_event from `claude -p` runs into the same cache, and keeps the ~/.claude.json cachedUsageUtilization fallback.
