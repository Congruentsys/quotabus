---
id: VOY-001
title: "quotabus v1.0 — every LLM key and AI subscription's status on NATS KV, with freshness, for humans and agents"
type: voyage
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0]
depends_on: []
resolution: completed
---

# quotabus v1.0 — every LLM key and AI subscription's status on NATS KV, with freshness, for humans and agents

## Context
The fleet never knew which external models had usage left: the GitHub Copilot subscription ran out on 2026-09-23 and was found only when a review failed, and the one earlier balance scanner died with no TTL on its rows (its DeepSeek row still read EXHAUSTED weeks later). Design: `docs/DESIGN.md`; prior art: `docs/PRIOR-ART.md` (no FOSS tool does the whole job). Tracked from nusy-product-team by IDEA-13333.

## Captain's rulings (2026-10-08)
- Name `quotabus`, under Congruentsys (§10 Q1).
- v1.0 reports and alerts only; pausing a provider is a later expedition, not a v1.0 feature (Q6).
- Yes to a `statusLine` entry in every host's `~/.claude/settings.json` (Q9), installed by the per-host agent expedition after the measurement chore confirms it fires under `claude -p`.
- All work is filed on this repo's board (Q11); further questions are filed here as signals.

## Members (v1.0)
The probe runner + KV + CLI (E1), the doc-pointer chore (C1), the measurement chore (C2), the per-host agent (E2), the selector (E3), alerts + the Yurtle front-end (E4).

## Definition of Done
`quotabus status` on the hub shows every configured API key and every host's subscription state with ages; a stale or missing row reads unknown; `quotabus select --role review --exclude-family anthropic` returns a healthy model or refuses with rc 3; an outage files exactly one alert.

## Comments

### M5-MBP-2/s-72a67d16 (2026-10-09 03:26)

VOY-001's Definition of Done is measured MET on the live bus (CHORE-016, PR #25, docs/findings/CHORE-016-mini-deploy.md, 2026-10-09). Alerts go to stdout until SIG-010 is ruled and a board sink is chosen. Closing the voyage and a v1.0 release are the Captain's call.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-10T22:02:18+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/releases/tag/v1.0.0> ;
    kb:resolution "completed" ;
  ] .
```
