---
id: CHORE-013
title: "Flowback packet CHORE-001: restate the seat rule as decided (SIG-009), fix the DESIGN §3 line cite"
type: chore
status: provisioning
priority: medium
assignee: null
created: 2026-10-09
depends_on: []
---

# Flowback packet CHORE-001: restate the seat rule as decided (SIG-009), fix the DESIGN §3 line cite

Found landing CHORE-012 (PR #19): `docs/flowback/CHORE-001-to-nusy-product-team.md` lines 55 and 80 still call the seat rule "interim" / "pending SIG-009 (open)", and line 80 cites `docs/DESIGN.md:321-325`, which is now 321-327. SIG-009 was decided by steer bucket 2 (2026-10-08, open to the Captain's veto). CHORE-001 waits stranded on nusy-product-team landing this packet, so the packet should say what is true before they do.

## Done when
1. The packet's two places state the rule as decided (steer bucket 2 on SIG-009, not a Captain ruling, open to veto), and its DESIGN citation points at the current lines.
2. No other text in the packet changes; a comment on CHORE-001 says the packet was amended and at which sha.
3. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T01:38:05+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
