---
id: CHORE-026
title: "quotabus-cycle wrapper passes --force to alert, which refuses it (found in HAZ-005 case 1)"
type: chore
status: arrived
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-10
depends_on: []
---

# quotabus-cycle wrapper passes --force to alert, which refuses it (found in HAZ-005 case 1)

Found in HAZ-005's case 1 (`docs/findings/HAZ-005-local-network.md`, M5's raw log under Mini
`~/.local/state/quotabus-work/HAZ-005/`): a one-shot run of the production chain with `probe --force` made the
`quotabus-cycle` wrapper's alert step fail with "error: unexpected argument '--force' found". The wrapper (the one in
`packaging/README.md`, installed on Mini as `~/.local/bin/quotabus-cycle`) drops its first argument (`probe`) and
passes every remaining argument to BOTH `quotabus probe` and `quotabus alert`; `alert` has no `--force`. The scheduled
unit is unaffected (it passes only `--config <file>`); a forced one-shot run alerts on nothing and exits non-zero.

## Done when
The wrapper in `packaging/README.md` passes only `--config <file>` to `alert` (or `alert` accepts and ignores
`--force`, if that is the smaller change), with a test that runs the wrapper with `probe --force --config <file>`
against stub `quotabus` binaries on PATH and shows both steps get valid argv; Mini's installed wrapper is updated to
match (recorded on this item). `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-10T15:12:17+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T15:12:24+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-10T15:27:20+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/35> ;
  ] .
```


## Comments

### Mac-mini/s-e7976c42 (2026-10-10 15:13)

tests red at 266f9ca (branch chore/CHORE-026-cycle-force): tests/test_chore026_cycle_wrapper.py runs the README's wrapper under /bin/sh against a clap-like stub quotabus — 2 red (probe --force --config X, in either order: alert gets '--force', stub exits 2 'unexpected argument'), 8 green (scheduled argv; failure still reported; stub and checker controls incl. mutants that skip alert or drop --force from probe).
