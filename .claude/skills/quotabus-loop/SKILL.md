---
name: quotabus-loop
description: The simple loop for quotabus. IN-SESSION it runs quotabus-next, lands what it hands you with pairit (in the lane it names), cleans that item's worktrees, then picks again. It stops on nothing pickable, a halted board, a picker error or low disk. Never ScheduleWakeup; never end the turn between items.
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/*), Bash(python3 *), Bash(git *), Bash(gh *), Bash(claude *), Bash(env *), Bash(make *), Bash(ln *), Bash(cd *), Bash(scripts/*), Agent
---

# quotabus-loop: pick, land, pick again

```text
repeat:
  1. PICK   quotabus-next (it prints the item AND its lane: code, docs, measure or other-repo)
  2. STOP?  no candidate (nothing pickable: report the --explain reasons and what still sits in harbor), halted
            (rc 8), remote trouble (rc 4/5/6) or every candidate refused (rc 1) → stop and report one line naming
            what it waits on. rc 3 (lost the race) is NOT a stop: claim the next candidate
  3. LAND   by lane, through pairit: code → steps 0–5; docs → § The docs lane; measure → § The measure lane;
            other-repo → § The other-repo lane (it parks the item `stranded`, which is NOT a stop)
  4. CARRY  a finding that BLOCKS this item is filed and landed first; every other finding is FILED
            (`.venv/bin/yurtle-kanban create … --push`; ids come from origin), released per the rule below,
            unclaimed. A finding about another repo is an other-repo item, never an edit there
  5. CLEAN  remove this item's own worktrees and check the disk (below). Exit 14 = STOP and report
  6. goto 1 at once, in this same turn
```

- **One item = one context.** This session picks, spawns the partner, the implementer, the docs author and the
  reviewer, and lands. The partner, the implementer and the docs author are fresh `Agent` sub-agents; the reviewer is
  a distinct `claude -p` session.
- **Report through the board.** Each landed item is closed with its measurement (for a doc: what landed, at which
  sha, and the check that shows it), the command that produced it, and its PR (`--closed-by <PR URL>`). Give the
  Captain one line per landed item and keep going.
- **Release rule** (CLAUDE.md § Skills): only `provisioning` items are claimable, and well-defined work is released,
  with `depends_on` setting the order. A finding the loop files is released at once (`.venv/bin/yurtle-kanban move
  <ID> provisioning --agent "$ME"`) when it has a "Done when" and waits on no open Captain decision; otherwise it
  stays in `harbor` with a comment naming what it lacks. The loop never releases a voyage or a signal, and never
  decides a signal.
- **The board is `origin`** (CLAUDE.md § Syncing the board). Every PICK starts from a pull; every board write is on
  `origin` before the next step (3.4.0's verbs push by default; `create`/`update` take `--push`). No board write
  rides a code branch.
- **Never `ScheduleWakeup`, never end the turn to report between items.**

## Cleaning up between items

After the item is LANDED (its PR is merged and the merge is on `origin/main`) and before the next PICK, run the
clean step from the main checkout or any worktree of it:

```bash
scripts/qb_clean.sh <id>                   # a literal, as used in /tmp/<id>
```

Scope: only worktrees this repo registered whose directory name is this item's (`<id>`, `<id>-merge`, `rv-<id>`,
`rv-<id>-…`); it never touches the main checkout or another item's tree. A tree is kept and reported, never forced,
when it holds commits not on origin, when it holds uncommitted work, or when its gitignored `data/` or `runs/` holds
a file: `git worktree remove` deletes ignored files silently. The `[clean] KEPT` line says what to do: move the
artefacts to the durable per-item host path `~/.local/state/quotabus-work/<id>/`, record that path in the item, and
re-run; or, when they really are scratch, declare one tree's data disposable by name: `scripts/qb_clean.sh <id>
--disposable <id>-merge` (repeatable; it never overrides the commits or uncommitted rules). The measure lane writes
its raw artefacts under `~/.local/state/quotabus-work/<id>/` from the start, so its trees clean without either.

It then runs `git worktree prune` and prints the free space on `/tmp`. **Exit 14** means it is below
`${QB_MIN_FREE_GIB:-25}` GiB: the loop STOPS and reports the largest dirs (`du -sh`); what else is large belongs to
someone else. Exit 0 is done (kept trees are reported, not an error); exit 2 is a usage error. Tests:
`tests/test_qb_clean.py`.

Also delete the item's merged branch, local and remote, once the PR is merged (`gh pr merge --delete-branch` does
the remote one).
