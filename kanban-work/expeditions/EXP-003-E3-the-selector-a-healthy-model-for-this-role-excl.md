---
id: EXP-003
title: "E3: the selector — 'a healthy model for this role, excluding this family' as a library call and a CLI"
type: expedition
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: []
---

# E3: the selector — 'a healthy model for this role, excluding this family' as a library call and a CLI

Part of VOY-001; after E2 (the selector is only as honest as the subscription rows it refuses on). Design §3 (select), §9 row E3.

## Plan
A pure function over status rows plus a static model table (role, family, context): `quotabus select --role review --exclude-family anthropic` and the same as a library call; change subjects on the bus; wiring notes for nusy-product-team's review skills and its external-review doc (that wiring is a separate item there — the Captain froze the loop skills until this app exists).

## Definition of Done
The selector refuses a family the author uses; rc 3 when every candidate is `unknown`; unit tests run without a bus.