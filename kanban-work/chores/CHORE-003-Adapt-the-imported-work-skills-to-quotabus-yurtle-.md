---
id: CHORE-003
title: "Adapt the imported work skills to quotabus — yurtle-kanban board, GitHub PRs, no fleet Layer-B tooling"
type: chore
status: backlog
priority: high
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, skills]
depends_on: []
---

# CHORE-003: Finish porting the work skills — from nusy-replicant-24's yurtle-kanban set

Part of VOY-001; FIRST, before EXP-001 is landed through the loop. Captain 2026-10-08: copy our skills over (a loop skill and the other work skills), and take the SIMPLER versions from the sibling repos (nusy-replicant-24 / the rachael labs), which already run on yurtle-kanban.

## What was copied (2026-10-08, from `Congruentsys/nusy-replicant-24@2de49870`)
- `quotabus-next` (was `replicant-next`) and `quotabus-loop` (was `replicant-loop`); `pairit`, `refine-idea`, `hypothesize`, `uxr-review`, `editor-review`. yurtle-kanban's own ten skills from `init` stay.
- `scripts/yk_push.sh` (board writes go to origin) and `scripts/qb_clean.sh` (was `r24_clean.sh`); a repo `.venv` with yurtle-kanban 3.4.0 (gitignored — `python3 -m venv .venv && .venv/bin/pip install yurtle-kanban==3.4.0`).
- Names and paths were renamed mechanically; the CONTENT still carries LUM/replicant specifics.

## Plan
1. Port the CLAUDE.md sections the skills cite from nusy-replicant-24's CLAUDE.md: § The board, § Syncing the board (origin is the board; board writes via `scripts/yk_push.sh`), § Skills (the release rule: only `provisioning` items are claimable), § Rules — adapted to quotabus (no LUM, no G-gates, no packet lane unless quotabus needs one; CHORE-001 IS a change in another repo, so keep a packet/other-repo lane).
2. Strip LUM-only content from `pairit` (the gate/prereg run protocol, research/LUM paths, CHORE-001…013 examples), `quotabus-loop` and `quotabus-next`; keep the code and docs lanes; landing = branch + GitHub PR reviewed by a distinct `claude -p` session.
3. Make the scripts' tests (if any came with them) pass here, or bring them.
4. Release the v1.0 items that are well defined (`move <ID> provisioning`), with `depends_on` setting the order: CHORE-003 → EXP-001 → …; the SIG-* questions stay with the Captain.

## Definition of Done
`quotabus-next` run as written prints a pick from this board; no skill or script names nusy-replicant-24, LUM, `research/LUM`, or `r24`; the PR lists what was kept, adapted or cut.
