---
id: EXP-007
title: "Pause a provider when its key is dead or out of quota — an actuator, NOT a v1.0 feature"
type: expedition
status: backlog
priority: low
assignee: null
created: 2026-10-08
tags: [post-v1.0, actuation]
depends_on: []
---

# Pause a provider when its key is dead or out of quota — an actuator, NOT a v1.0 feature

**Captain 2026-10-08 (§10 Q6):** add an expedition for pausing a provider, but it is not a needed feature for v1.0. v1.0 publishes and alerts only.

## Context
The fleet's earlier balance scanner paused a provider at a measured zero balance (`fleet-provider-pause.sh`, deleted in nusy-product-team's Commit C). Design §6–§7 explain why v1.0 does not act: keeping the tool report-only is what makes it safe to share.

## Plan (to refine before work starts)
1. What "pause" means for each consumer: a flag on the bus that selectors and loop skills honour, rather than the tool editing anyone's config.
2. Who may set and clear it: a measured `ok` clears; a human can override; CANNOT-ASSESS never pauses.
3. Opt-in per service, off by default for outsiders.

## Definition of Done
A paused service is skipped by `quotabus select`, the pause and its reason are visible in `quotabus status`, and only a measured `ok` (or a human) clears it.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Not released: tagged post-v1.0 (optional / not a v1.0 feature per VOY-001 and the Captain's 2026-10-08 rulings). Stays in harbor until the Captain asks for it after v1.0.
