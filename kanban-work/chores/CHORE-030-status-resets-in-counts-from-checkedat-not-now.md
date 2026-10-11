---
id: CHORE-030
title: "status: 'resets in' counts from checked_at, not now"
type: chore
status: arrived
priority: low
assignee: Mac-mini/s-e7976c42
created: 2026-10-11
depends_on: []
related: [CHORE-029]
---

# status: 'resets in' counts from checked_at, not now

## Why
`quotabus status` prints each subscription window as `<window> N% (resets in <age>)`, computed from the row's stored `reset_in_s` (src/main.rs:888-889). `docs/DESIGN.md` §2 (`headroom` row) defines `reset_in_s = reset_at − checked_at`, so the printed time is relative to when the row was CHECKED, not to now: it overstates the time left by the row's age (up to 1 h at the subscription interval, more on an older row). A reader sees "resets in 87m" when it resets in 83m.

Found by the CHORE-029 r1 reviewer (reviews/CHORE-029-r1.md, side note): on Mini 2026-10-11 the "resets in …" text was the same at AGE 4m and 8m. The source line confirms it. The row's fields are right; only the reader's rendering is off.

## Plan
In `status`'s DETAIL, render "resets in" from `reset_at − now` (the reader's clock, as AGE already is), falling back to `reset_in_s − age` when `reset_at` is absent. A reset already in the past renders as `(reset due)` or similar, never a negative duration. The record format and `reset_in_s` are unchanged (DESIGN §2 stays as is). `status --json` is unchanged.

## Done when
- A test with a fixed "now" and a row checked 10 min earlier, `reset_in_s` = 3600, shows "resets in" ≈ 50m, not 60m. A control row checked at "now" still shows 60m.
- A row whose `reset_at` is before "now" shows no negative duration.
- `--json` output and the stored row are byte-for-byte unchanged (test).
- `make check` green; PR reviewed by a distinct session (pairit code lane).

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-11T15:07:33+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-11T15:07:44+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-11T15:20:40+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/39> ;
  ] .
```


## Comments

### Mac-mini/s-e7976c42 (2026-10-11 15:10)

tests red at 13d5e2e: tests/chore030_resets_in_now.rs — 5 red by assertion (checked_ten_min_ago_shows_about_fifty_minutes: 'rendered (resets in 60m) = 3600s, want 47–50 min from NOW'; reset_at_is_read_against_now; fallback_without_reset_at_subtracts_age; past_reset_at_shows_no_negative_duration; past_reset_via_fallback_shows_no_negative_duration), 3 green controls (checker_controls, control_checked_now_shows_sixty_minutes — shown able to fail by a human_age(s-120) mutation, json_and_stored_row_unchanged).
