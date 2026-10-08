---
name: quotabus-loop
description: The simple loop for quotabus. IN-SESSION it runs quotabus-next, lands what it hands you with pairit (and the run protocol for experiment runs), cleans that item's worktrees, then picks again. It stops on nothing pickable, a halted board, a picker error or low disk. Never ScheduleWakeup; never end the turn between items. Same contract as the labs' rachael-loop.
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/*), Bash(python3 *), Bash(git *), Bash(claude *), Bash(env *), Bash(make *), Bash(ln *), Bash(cd *), Agent
---

# quotabus-loop: pick, land, pick again

Ported from rachael-neural-lab `rachael-loop` (2026-10-02); lanes added 2026-10-02 for LUM, whose first work is
mostly documents.

```text
repeat:
  1. PICK   quotabus-next (it prints the item AND its lane: code, docs or packet)
  2. STOP?  no candidate (nothing pickable: report the --explain reasons and what still sits in harbor), halted
            (rc 8), remote trouble (rc 4/5/6) or every candidate refused (rc 1) → stop and report one line naming
            what it waits on. rc 3 (lost the race) is NOT a stop: claim the next candidate
  3. LAND   by lane, through pairit: code → steps 0–4; docs → § The docs lane; packet → § The packet lane
            (it parks the item `stranded`, which is NOT a stop); a run also follows § The run protocol
  4. CARRY  a finding that BLOCKS this item is filed and landed first; every other finding is FILED
            (`.venv/bin/yurtle-kanban create … --push`; `--push` always, ids come from origin), released per the rule below, unclaimed. A finding about another repo is a
            packet (pairit § The packet lane), never an edit there
  5. CLEAN  remove this item's own worktrees and check the disk (below). Exit 14 = STOP and report
  6. goto 1 at once, in this same turn
```

- **One item = one context.** This session picks, spawns the partner, the implementer, the docs author and the reviewer,
  and lands. The partner, the implementer and the docs author are fresh `Agent` sub-agents; the reviewer is a distinct
  `claude -p` session.
- **Report through the board.** Each landed item is closed with its measurement (for a doc: what landed, at
  which sha, and the check that shows it) and the command that produced it. Give the Captain one line per
  landed item and keep going.
- **Release rule** (CLAUDE.md § Skills, Captain 2026-10-03): only `provisioning` items are claimable, and
  well-defined work is released, with `depends_on` setting the order. A finding the loop files is released at once
  (`scripts/yk_push.sh move <ID> provisioning`) when it has a "Done when" and waits on no open Captain decision;
  otherwise it stays in `harbor` with a comment naming what it lacks. The loop never releases a voyage or a signal.
- **The board is `origin`** (CLAUDE.md § Syncing the board). Every PICK starts from a fetch; every board write is
  on `origin` before the next step (`claim`/`create --push` do it themselves, `move`/`comment` go through `scripts/yk_push.sh`).
  No board write rides a code branch.
- **Never `ScheduleWakeup`, never end the turn to report between items.**

## Cleaning up between items

After the item is LANDED (its merge is on `origin/main`) and before the next PICK, run the clean step from the
main checkout or any worktree of it:

```bash
scripts/qb_clean.sh <id>                   # a literal, as used in /tmp/<id>
```

Scope: only worktrees this repo registered whose directory name is this item's (`<id>`, `<id>-merge`, `rv-<id>`,
`rv-<id>-…`); it never touches the main checkout or another item's tree. A tree is kept and reported, never forced,
when it holds commits not on origin, when it holds uncommitted work, or (HAZ-009) when its gitignored `data/`,
`runs/` or `checkpoints/` holds a file: `git worktree remove` deletes ignored files silently, and EXP-067 lost its
session transcripts that way. The `[clean] KEPT` line says what to do: move the artefacts to the durable per-item
host path `~/.nusy/qb-data/<id>/`, record that path in the item, and re-run; or, when they really are scratch,
declare one tree's data disposable by name: `scripts/qb_clean.sh <id> --disposable <id>-merge` (repeatable; it
never overrides the commits or uncommitted rules). A run that followed pairit's run protocol wrote its artefacts
under `~/.nusy/qb-data/<id>/` from the start, so its trees clean without either.

It then runs `git worktree prune` and prints the free space on `/tmp`. **Exit 14** means it is still below
`${RL_MIN_FREE_GIB:-25}` GiB: the loop STOPS and reports the largest dirs (`du -sh`); what else is large belongs to
someone else. Exit 0 is done (kept trees are reported, not an error); exit 2 is a usage error.

**When the loop stops**, remove the session's board worktree too (it is always pushed or reset; CLAUDE.md
§ Syncing the board): `git worktree remove --force "/tmp/qb-board-${CLAUDE_CODE_SESSION_ID:0:8}" 2>/dev/null; git worktree prune`.
