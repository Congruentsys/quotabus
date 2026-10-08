---
id: EXP-003
title: "E3: the selector — 'a healthy model for this role, excluding this family' as a library call and a CLI"
type: expedition
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-002]
---

# E3: the selector — 'a healthy model for this role, excluding this family' as a library call and a CLI

Part of VOY-001; after E2 (the selector is only as honest as the subscription rows it refuses on). Design §3 (select), §9 row E3.

## Plan
A pure function over status rows plus a static model table (role, family, context): `quotabus select --role review --exclude-family anthropic` and the same as a library call; change subjects on the bus; wiring notes for nusy-product-team's review skills and its external-review doc (that wiring is a separate item there — the Captain froze the loop skills until this app exists).

## Definition of Done
The selector refuses a family the author uses; rc 3 when every candidate is `unknown`; unit tests run without a bus.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T17:22:12+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T20:34:03+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Released with depends_on EXP-002 (design §9: the selector is only as honest as the subscription rows it refuses on). No open signal blocks it.

### M5-MBP-2/s-72a67d16 (2026-10-08 20:41)

Tests red at 0bc5620 (partner): tests/exp003_select.rs (19, pure function, no bus), exp003_cli_select.rs (12), exp003_change_subject.rs (1, throwaway nats-server) and exp003_docs.rs (1). 33 of 36 are red, each on a todo!() stub or an assertion; the 3 green are controls or a parser. Calls made where the design is silent: degraded is never chosen; cheapest puts local before metered; it is left open whether a healthy subscription seat can be a review candidate.
