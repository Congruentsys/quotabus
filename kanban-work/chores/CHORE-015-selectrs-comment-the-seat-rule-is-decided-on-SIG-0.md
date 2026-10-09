---
id: CHORE-015
title: "select.rs comment: the seat rule is decided on SIG-009, no longer 'pending'"
type: chore
status: provisioning
priority: medium
assignee: null
created: 2026-10-09
depends_on: []
---

# select.rs comment: the seat rule is decided on SIG-009, no longer 'pending'

Found landing CHORE-014: `src/select.rs:75` has a code comment saying "a subscription seat is never a candidate, pending the Captain's SIG-009". SIG-009 is decided: an agent session's steer pass ruled it in bucket 2 on 2026-10-08. That was not a Captain ruling, and the Captain can still veto it. DESIGN §3 already says so (CHORE-012). The comment is the last stale copy of that wording.

## Done when
1. The comment states the rule as decided on SIG-009 (steer bucket 2, open to the Captain's veto) and keeps its reason (per-account row, no model to print, review r1 F1).
2. No behaviour change: only the comment changes. `make check` is green.
3. `grep -rn "pending the Captain's SIG-009" src/ docs/` prints nothing.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T01:54:00+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
