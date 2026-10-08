---
id: CHORE-009
title: "DESIGN.md: CHORE-006 review nits — §8 org row, §3 diagram local kind, cite CHORE-007 for TTL rules"
type: chore
status: harbor
priority: medium
assignee: null
created: 2026-10-08
depends_on: [CHORE-007]
bounce_sha: "5a1cbaad6b4c836010d50a64b4fd3e97455aceabb8de3c36ea141a6bf5dfaac8"
bounced_by: M5/s-b1fd4c67
bounced_at: 2026-10-08T19:21:10+00:00
bounces: 1
---

# DESIGN.md: CHORE-006 review nits — §8 org row, §3 diagram local kind, cite CHORE-007 for TTL rules

Found by the CHORE-006 review (`reviews/CHORE-006-r1.md`, PR #8; all nits, merged as 76a07a9). Three small DESIGN.md
corrections:

1. **F1:** §8's org row still offers `hankh95` as an option, though §10 Q1 records the name/org as chosen
   (`quotabus` under `Congruentsys`).
2. **F2:** the §3 diagram no longer shows the central probe's `local` kind, though E1 still probes local Qwen from
   there.
3. **F3:** two rules in §3/§4 (each kind's TTL is 3 × its interval; an API probe never triggers a balance read) come
   from CHORE-007's Definition of Done, not from a Captain ruling; cite CHORE-007 for them rather than §10.

Coordinate with CHORE-007, which also edits §3/§4: if CHORE-007 has landed, edit on top of it.

## Done when
A PR amends DESIGN.md on F1, F2 and F3, each citing `reviews/CHORE-006-r1.md`; `make check` green; reviewed by a
distinct session (a DESIGN.md change) and merged.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T19:17:48+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T19:20:44+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:backlog ;
    kb:at "2026-10-08T19:21:10+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
    kb:bounced "true"^^xsd:boolean ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 19:21)

[bounce by M5/s-b1fd4c67, body-sha:5a1cbaad6b4c] Edits DESIGN.md §3/§4 text that CHORE-007 (underway, M5-MBP-2 session) is rewriting; the item says to edit on top of CHORE-007, so it now depends_on CHORE-007 and goes back unclaimed.
