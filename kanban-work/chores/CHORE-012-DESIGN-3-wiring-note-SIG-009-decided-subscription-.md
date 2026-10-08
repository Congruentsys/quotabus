---
id: CHORE-012
title: "DESIGN §3 + wiring note: SIG-009 decided — subscription seats are never select candidates (no longer 'interim')"
type: chore
status: backlog
priority: low
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, design]
depends_on: []
---

# DESIGN §3 + wiring note: SIG-009 decided — subscription seats are never select candidates (no longer 'interim')

steer decided SIG-009 by bucket 2 (2026-10-08, open to the Captain's veto; basis on SIG-009): a `kind = "subscription"` row is never a `select` candidate. DESIGN §3 and `docs/wiring/nusy-product-team.md` still call this the "interim rule pending SIG-009".

## Definition of Done
DESIGN §3 (the select section) and the wiring note state the rule as decided, citing SIG-009's steer comment and noting that a seat-routing feature would be a new item. No other text changes. `make check` is green.