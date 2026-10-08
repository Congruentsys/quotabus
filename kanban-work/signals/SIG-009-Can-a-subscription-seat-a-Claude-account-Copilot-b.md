---
id: SIG-009
title: "Can a subscription seat (a Claude account, Copilot) be a 'select' candidate — and as which model?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Can a subscription seat (a Claude account, Copilot) be a 'select' candidate — and as which model?

Raised by the EXP-003 review (`reviews/EXP-003-r1.md` F1, PR #14). Since EXP-002, every Claude account and Copilot is a `kind = "subscription"` row with a 5 h / 7 d (or monthly) window, but **no model**: the model slot is the service id. `quotabus select` prints `provider model` on line 1 for `$(…)`. A healthy seat with `roles = ["review"]` would print e.g. `github copilot`, which is not a model a reviewer can run.

The design pulls both ways: §1 treats Copilot as a reviewer source, while §2 gives a subscription row no model.

## The question
1. **Never.** Selection is over API models only, and subscription seats are only read and alerted on. This is EXP-003's interim fix.
2. **Yes, with a configured model per seat** (e.g. `models = ["claude-opus-5-5"]` on a Claude account, `["gpt-5.6"]` on Copilot). The seat is chosen while its windows are under a threshold, and line 1 then prints that model.
3. **As a gate only.** `select` refuses an API model whose family's subscription seat is exhausted, but it never prints the seat itself.

## Recommendation (an agent's, not a decision)
1 for v1.0, which is what EXP-003 ships. Then 2 as a later item, if the fleet wants `select` to route reviews to Copilot or a Claude account.