---
id: CHORE-010
title: "Implement or drop §2's 'degraded at window ≥ warn %' — no warn % exists (EXP-002 r1 F2)"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, design]
depends_on: []
---

# Implement or drop §2's 'degraded at window ≥ warn %' — no warn % exists (EXP-002 r1 F2)

Found in the EXP-002 review (`reviews/EXP-002-r1.md` F2, PR #12). DESIGN.md §2's state table lists "a window ≥ warn %" under `degraded`, but no `warn %` config key or code exists, so a subscription window at 97 % reads `ok`. EXP-002 follows its own Plan (`allowed` → ok); the §2 clause predates it.

## What it lacks (why it stays in harbor)
A decision: either (a) add a per-kind or per-service `warn_pct` with a DEFAULT value, which no ruling names, or (b) drop the clause from §2 and leave the threshold to `alert` (E4). This is a Captain-level default, so it is not filed as a signal guess.

## Definition of Done (once the default is chosen)
(a) a `warn_pct` config with the chosen default; a window at or above it reads `degraded`, with a test and a control; DESIGN §2 cites it. Or (b) §2 no longer claims it. `make check` green.

## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 20:15)

Kept in harbor: it lacks a default warn % (or a ruling to drop the clause). No ruling names one; the question is for the Captain.
