---
id: SIG-009
title: "Can a subscription seat (a Claude account, Copilot) be a 'select' candidate — and as which model?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
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

## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 21:57)

[steer] bucket-2: option 1. A subscription seat is NEVER a select candidate (what EXP-003 ships, PR #14).
Basis:
- G1, never a false answer: `select`'s line 1 must be a runnable `provider model`, and a seat has no model. DESIGN §2: 'the model slot is the service id'. A healthy Copilot seat printed 'github copilot' (reviews/EXP-003-r1.md F1).
- G3: the smallest change, and already shipped and reviewed.
- Measured (M5, 2026-10-08, quotabus b5c20f5): no subscription service in DESIGN §3 or examples/quotabus.toml has `roles`, and no text in DESIGN makes a seat a select candidate (grep 'copilot' with review/select/role → none). No reader relies on seats being picked.
Option 2 (route reviews to a seat by a configured model) would be a NEW feature, not a decision; the Captain can file it.
Closed with move --force, because the board refuses harbor→arrived (see the steer skill). Decided — open to the Captain's veto.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T21:57:57+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```
