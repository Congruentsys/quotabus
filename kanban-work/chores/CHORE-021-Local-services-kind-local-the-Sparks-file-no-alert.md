---
id: CHORE-021
title: "Local services (kind = local, the Sparks) file no alert — Captain 2026-10-09"
type: chore
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: []
---

# Local services (kind = local, the Sparks) file no alert — Captain 2026-10-09

Part of VOY-001. The Captain ruled on 2026-10-09, in this repo's Mini session (Mac-mini/s-e7976c42), after saying:
"Local qwen should be on http on DGX1 or DGX2 or both - should not need an API key - but does go up and down as they
do different work". Asked "When a local Spark (DGX1/DGX2) goes down or switches models, should quotabus file an
alert?", the Captain chose the option offered as recommended: Captain 2026-10-09: "No alert for local (Recommended)",
whose text was: "kind = "local" rows are still published and select still refuses a down box, but
unreachable/model_missing on a local service files no signal. API and subscription alerts keep your SIG-010 default."

Today (CHORE-018, `src/alert.rs` `decide_listed`) the `[alert] states` default includes `unreachable`, and a local
service's `model_missing` (a Spark switched models) also files: both are routine for the Sparks.

## Definition of Done
1. `quotabus alert` files no signal for a row whose service has `kind = "local"`, whatever its state; such a row does
   not arm or clear an alert. API and subscription rows are unchanged (SIG-010's default and `[alert] states`).
2. Local rows are still published, and `select` still refuses a local row that is not `ok` (a test pins both).
3. Tests: a local service crossing ok → unreachable, ok → model_missing and back, over several cycles, files 0; an API
   service with the same crossings still files per SIG-010; controls show each check can fail.
4. DESIGN.md §3 alert row (and §10 Q8 where it lists what files) records the ruling verbatim, named and dated, and
   says it was the recommended option. `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T22:31:49+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T13:30:24+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-10 13:33)

tests red at 5637ecc: tests/chore021_local_no_alert.rs — 10 red (local flapping ok/unreachable, ok/model_missing, mixed: filed 4 not 0; any state under default and all-listed; arms alert.local.* entry; local ok clears stale entry; unreachable Spark published but files; DESIGN §3 and §10 Q8 lack the ruling), 7 green (fixtures control, API arms/clears/SIG-010 default, select picks ok local, select refuses non-ok local, ruling-check control)
