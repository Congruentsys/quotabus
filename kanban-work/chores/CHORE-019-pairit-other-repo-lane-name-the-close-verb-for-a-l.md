---
id: CHORE-019
title: "pairit other-repo lane: name the close verb for a landed stranded item (3.4.0 refuses stranded → done)"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-09
depends_on: []
---

# pairit other-repo lane: name the close verb for a landed stranded item (3.4.0 refuses stranded → done)

Found closing CHORE-001 (2026-10-09). pairit § The other-repo lane, step 4, says "Whoever records the receiving repo's
landing commit moves it back and closes it, citing that commit", but names no verb. In yurtle-kanban 3.4.0 a plain
`move <ID> done` from `stranded` is refused, rc 1: "Illegal move CHORE-001: stranded → arrived. Legal from stranded:
provisioning, underway, harbor." The only legal route back goes through `underway`, which CLAUDE.md forbids by hand
("Never `move … underway` by hand"), and the item is still held by the session that parked it. CHORE-001 was closed
with `move CHORE-001 done --force -m "<landing commit and check>"`, the comment saying `--force` skips the transition
table only (steer's precedent for signals).

## Done when
pairit step 4 names the exact close command for a landed other-repo item (`move <ID> done --force -m "<the other
repo's landing commit, and the read that shows it>"`, or another route that 3.4.0 accepts and CLAUDE.md allows) and
why `--force` is needed there; the stranded-close rule in CLAUDE.md, steer and quotabus-loop agrees with it.
`make check` green.