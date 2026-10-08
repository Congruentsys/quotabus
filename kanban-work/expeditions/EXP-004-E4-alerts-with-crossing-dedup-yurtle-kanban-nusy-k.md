---
id: EXP-004
title: "E4: alerts with crossing-dedup (yurtle-kanban, nusy-kanban, webhook) and the Yurtle config front-end"
type: expedition
status: arrived
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-003]
---

# E4: alerts with crossing-dedup (yurtle-kanban, nusy-kanban, webhook) and the Yurtle config front-end

Part of VOY-001. Design §3 (alert, config), §9 row E4. The dead balance scanner's good rule is kept: only a measured `ok` clears an alert; CANNOT-ASSESS never does.

## Plan
1. `quotabus alert`: one item per crossing (a service going bad), never auto-closed; sinks: yurtle-kanban (`create signal --push`), nusy-kanban, webhook, stdout.
2. The Yurtle front-end: the same rows as the TOML config in one `yurtle-table` block; `examples/` with both.

## Definition of Done
A model 404 files ONE signal across ten probe cycles; a measured `ok` re-arms it; a CANNOT-ASSESS does not; the Yurtle and TOML examples load to identical configs.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T17:22:14+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T21:03:58+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-08T21:40:35+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/15> ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Released with depends_on EXP-003. SIG-006 is built to the recommendation (alert = signal, one per crossing, never auto-closed; the item type is sink config), SIG-005's Yurtle twin lands here per the recommendation.

### M5-MBP-2/s-72a67d16 (2026-10-08 21:13)

Tests red at 0ea5162 (partner): tests/exp004_alert.rs (17) and exp004_config.rs (11). 27 of 28 red, each on a todo!() stub or an assertion; the one green is the fake-sink control. Noted: DESIGN §3's sample says yurtle-kanban item_type 'issue', but the Plan and SIG-006 say signal, and the tests follow signal. No Rust Yurtle reader exists (PRIOR-ART names only Python yurtle-rdflib), so the yurtle-table reader is written here.

### Mac-mini/s-e7976c42 (2026-10-08 21:48)

Heads-up from CHORE-011 (merged, PR #16): subscription windows' used_pct is now rounded to 2 dp at Window::measured, so a threshold compare sees 99.995 as 100.0, and unified-header / stream-json values are NOT clamped to 0–100 (Copilot's are). If E4's alert compares used_pct against a threshold, decide whether that matters (reviews/CHORE-011-r1.md F1).
