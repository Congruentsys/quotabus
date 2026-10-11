---
id: CHORE-030
title: "status: 'resets in' counts from checked_at, not now"
type: chore
status: backlog
priority: low
assignee: null
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