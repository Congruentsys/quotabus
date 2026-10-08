---
name: quotabus-next
description: ONE pass — what should THIS session do next in quotabus? Checks the board's halt, resumes this session's own in-progress item, else atomically claims the first pickable expedition/chore/hazard with yurtle-kanban 3.3's `claim`, prints it with its LANE (code, docs or packet), and STOPS. Same contract as the labs' rachael-next, built on yurtle-kanban's own next/claim instead of a repo picker script.
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(.venv/bin/python *), Bash(git *), Bash(env *)
---

# quotabus-next: the picker

Ported from rachael-neural-lab `rachael-next` (2026-10-02). The lab needed its own picker script
(`scripts/nlab_next.py`); yurtle-kanban 3.3.0 ships `next` (resume-first suggestion), `list --pickable`,
`claim` (a race-free compare-and-swap commit on origin) and `control` (the board's halt), so this skill is
those verbs in order.

**Identity.** A claim is made by the SESSION, `<agent>/s-<8>`, so two sessions on one host never
resume each other's work. Set it once per command (an `export` does not survive a Bash tool call):

```bash
ME="${NUSY_AGENT_NAME:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
```

**The pass** (run from the repo root; `PYTHONPATH=` keeps a Spark's NVIDIA path out of the venv):

```bash
ME="${NUSY_AGENT_NAME:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
git pull -q --rebase=merges || echo "STOP: pull failed — the board read below would be stale"   # every read is local
git status -sb | head -1                                           # "[ahead N]" => unpublished board writes: STOP, report
PYTHONPATH= .venv/bin/yurtle-kanban control status                 # mode halt => STOP, report the reason
PYTHONPATH= .venv/bin/yurtle-kanban next --agent "$ME" --json      # {"kind":"resume",…} => that item is the answer
# else: the candidates, in pick order, WORK ITEMS ONLY (see rule 2), gpu-required dropped off a Spark (rule 3)
case "${NUSY_AGENT_NAME:-$(hostname -s)}" in DGX1|DGX2|spark-*) GPU=1 ;; *) GPU=0 ;; esac
PYTHONPATH= .venv/bin/yurtle-kanban list --pickable --agent "$ME" --json | GPU=$GPU .venv/bin/python -c '
import json, os, sys
for i in json.load(sys.stdin):
    if i["item_type"] in ("expedition", "chore", "hazard") and (os.environ["GPU"] == "1" or "gpu-required" not in i["tags"]):
        print(i["id"])'
# claim the first candidate; read the rc ALONE (not through a pipe)
PYTHONPATH= .venv/bin/yurtle-kanban claim <ID> --agent "$ME"
```

| `claim` rc | meaning | do |
|---|---|---|
| 0 | claimed (or already yours) | print the item (`show <ID>`) and its lane (below), and stop |
| 3 | lost the race to another agent | `git pull -q --rebase=merges` (the list is stale now), then claim the NEXT candidate |
| 1 | refused (e.g. no longer pickable on origin) | claim the next candidate; all refused → report |
| 8 | the board is halted | stop: report `control status` |
| 4, 5, 6 | remote unreachable / busy / push refused | nothing was claimed: report |

`claim` writes `origin/main` directly (CLAUDE.md § Syncing the board) and fast-forwards only a clean checkout on
`main`; pairit's step 0 fetches before it cuts the tree, so the claim is in it.

**No candidates** (the empty list is the loop's "nothing pickable", its rc 7) → stop and report what waits, in
one line each:
- ready items that are not pickable, and why: `list --pickable --agent "$ME" --explain`;
- items still in `harbor` (not released): `list -s harbor -t expedition`, `-t chore`, `-t hazard`. Under the release
  rule (CLAUDE.md § Skills) each should carry a comment naming what it lacks; one that is well defined is a missed
  release: report it. The picker itself never moves an item.

**The lane** — print it with the claimed item, so the loop lands it the right way (`pairit` § Scope):

| the item's deliverable | lane |
|---|---|
| code: packages, `scripts/`, runners, a scorer, tests | **code** — `pairit` |
| a document: a review, an arch doc, a design doc, a verification pass, a LIT review | **docs** — `pairit` § The docs lane |
| a change in ANOTHER repo (its body says "lands in <repo>") | **packet** — `pairit` § The packet lane |
| an experiment run | **code** for what it needs, then `pairit` § The run protocol |

Read it from the item's "Done when" / "Exit criterion"; an item with both a doc and code is code (the doc rides
the branch). When unsure, it is code.

**Rules**
1. **Resume before you pick.** An `in_progress` item held by `<agent>/s-…` of ANOTHER session is that
   session's work, not yours. Take it over only when it is really orphaned (`list --stale` names it):
   `claim <ID> --take-over --agent "$ME"` (recorded as `kb:takenOverFrom`).
2. **Work items only:** expeditions, chores, hazards. Never `claim --next`: it takes the first pickable item of
   ANY type, a voyage or a signal included (measured 2026-10-02, VOY-001 and SIG-001 in `provisioning`).
   Hypotheses, measures and experiments are records an expedition runs. A literature item cannot be claimed
   (the research board has no ready state); its review is the chore that names it (CHORE-002 … CHORE-013).
3. **Host:** an item tagged `gpu-required` runs only on a Spark (DGX1/DGX2). It is the only host tag (CLAUDE.md
   § The board). The candidate list above drops
   them off a Spark; one claimed anyway is `bounce`d straight back with the reason.
4. Dependencies (`depends_on`, across both boards: a research item counts when `complete`) are enforced by
   `claim` itself; never claim around them.

The item file is the brief. Land it with `pairit`, in its lane.
