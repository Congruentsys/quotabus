---
id: CHORE-001
title: "C1: nusy-product-team docs/external-review.md §1a — replace the hand probe with 'run quotabus status'"
type: chore
status: stranded
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-001]
---

# C1: nusy-product-team docs/external-review.md §1a — replace the hand probe with 'run quotabus status'

Part of VOY-001; after E1. Lands in **nusy-product-team** (doc-only, straight to its main), not in this repo: `docs/external-review.md` §1a's hand-probed table (2026-09-24) is retired in favour of `quotabus status`, keeping one line on how to probe by hand if the bus is down.

## Definition of Done
The §1a section points at `quotabus status` and the bucket name; the commit is cited on this item and on IDEA-13333.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Held in harbor on purpose (resident session brief, 2026-10-08): this lands in nusy-product-team as a doc-only change, so it is left for LAST, after EXP-001…EXP-004 land here. It goes through pairit's other-repo lane (a packet at docs/flowback/CHORE-001-to-nusy-product-team.md); no session here edits nusy-product-team.

### M5-MBP-2/s-72a67d16 (2026-10-08 21:41)

Hold met: EXP-001, EXP-002, EXP-003 and EXP-004 have all landed (PRs #6?/12/14/15 on origin/main). Released, and taken through pairit's other-repo lane: a packet at docs/flowback/CHORE-001-to-nusy-product-team.md.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T21:41:05+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T21:41:07+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:blocked ;
    kb:at "2026-10-08T21:48:41+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```

### M5-MBP-2/s-72a67d16 (2026-10-08 21:41)

Correction to my last comment: EXP-001 landed in PR #2, not '#6?'. EXP-002 is #12, EXP-003 #14, EXP-004 #15.

### M5-MBP-2/s-72a67d16 (2026-10-08 21:58)

[steer] bucket-1 read (M5, 2026-10-08): nusy-product-team origin/main 34e7be27ff still has the 2026-09-24 hand-probe §1a ('quotabus' does not appear in docs/external-review.md). Not landed there, so this stays stranded.

### M5-MBP-2/s-72a67d16 (2026-10-09 01:40)

Packet amended by CHORE-013 (PR #20, e8a7eda, merged at 6f6ce98): the seat rule is now stated as decided on SIG-009 (steer bucket 2, open to the Captain's veto), no longer 'interim'. nusy-product-team should take the packet as it is on quotabus main.

### M5-MBP-2/s-72a67d16 (2026-10-09 01:57)

Packet re-pinned by CHORE-014 (PR #22, merged at 84204b3): every quotabus cite now holds on quotabus 8213424. nusy-product-team should take the packet as it is on quotabus main.
