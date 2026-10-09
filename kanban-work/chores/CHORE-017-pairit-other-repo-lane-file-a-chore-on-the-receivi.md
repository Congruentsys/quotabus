---
id: CHORE-017
title: "pairit other-repo lane: file a chore on the receiving repo's board (the Captain's cross-repo rule, 2026-10-09)"
type: chore
status: underway
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-09
depends_on: []
---

# pairit other-repo lane: file a chore on the receiving repo's board (the Captain's cross-repo rule, 2026-10-09)

On 2026-10-09 the Captain directed, verbatim: "Can you create a chore in nusy-product-team for this - this is how we move work between repos". CHORE-001 had sat `stranded` with no item on nusy-product-team's board, so nobody there had been asked to land the packet. It was then filed as nusy-product-team CH-13371.

pairit's other-repo lane (`.claude/skills/pairit/SKILL.md` § The other-repo lane) and `CLAUDE.md` (§ Skills, the lane list) describe only the packet and the `stranded` wait. They don't describe filing the ask.

## Done when
1. pairit's other-repo lane has a step after the packet lands: file a chore on the receiving repo's board.
   - nusy-product-team: `nusy-kanban --server … create chore … --body-file … --relate related:<their tracking item>`.
   - a yurtle-kanban repo: `create chore … --push` run in that repo.
   - The chore's body links the packet, quotes the Captain's direction and asks for the landing sha back.
   - The quotabus item records the other repo's id when it moves to `stranded`.
2. CLAUDE.md says plainly that filing that one chore is the sanctioned cross-repo write, citing the Captain's words above. A quotabus session still never edits another repo's files.
3. `tests/test_skills_port.py`, or a new test, pins the step if the skill tests pin lane steps. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T13:37:40+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-09T13:38:47+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ] .
```
