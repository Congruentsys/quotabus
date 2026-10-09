---
id: CHORE-014
title: "Flowback packet CHORE-001: re-pin its source-table cites to current main (after CHORE-010)"
type: chore
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: [CHORE-010]
---

# Flowback packet CHORE-001: re-pin its source-table cites to current main (after CHORE-010)

Found landing CHORE-013 (PR #20): `docs/flowback/CHORE-001-to-nusy-product-team.md` says (line ~66) that every claim in its replacement text holds on quotabus `6f5a5fa`. Its source table cites `docs/DESIGN.md` lines (e.g. `:318-320`, `:333-337`, `:427`) taken at that sha. DESIGN has moved since then (CHORE-012, and CHORE-010 adds warn_pct text to §2/§3), so those cites can point at the wrong lines. A receiving session checks the cites against main.

## Done when
1. Every `path:line` cite in the packet's source table is re-counted against quotabus main at one named sha, and the "holds on quotabus <sha>" sentence names that sha.
2. Each claim still holds at that sha. A claim that no longer holds is changed and named in the PR body.
3. No other text changes. A comment on CHORE-001 gives the new sha. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T01:40:52+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-09T01:50:48+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
