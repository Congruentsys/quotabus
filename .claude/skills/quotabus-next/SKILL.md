---
name: quotabus-next
description: ONE pass — what should THIS session do next in quotabus? Checks the board's halt, resumes this session's own in-progress item, else atomically claims the first pickable expedition/chore/hazard with yurtle-kanban 3.4's `claim`, prints it with its LANE (code, docs, measure or other-repo), and STOPS. Built on yurtle-kanban's own next/claim, no picker script.
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(.venv/bin/python *), Bash(git *), Bash(env *)
---

# quotabus-next: the picker

yurtle-kanban 3.4.0 ships `next` (resume-first suggestion), `list --pickable`, `claim` (a race-free
compare-and-swap commit on origin) and `control` (the board's halt), so this skill is those verbs in order.

**Identity.** A claim is made by the SESSION, `<agent>/s-<8>`, so two sessions on one host never resume each
other's work. Set it in every command (an `export` does not survive a Bash tool call):

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
```

**The pass** (run from the main checkout; it only reads it and fast-forwards it):

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
git pull -q --rebase=merges || echo "STOP: pull failed — the board read below would be stale"   # every read is local
git status -sb | head -1                                    # "[ahead N]" => commits in the shared checkout: STOP, report
.venv/bin/yurtle-kanban control status                      # mode halt => STOP, report the reason
.venv/bin/yurtle-kanban next --agent "$ME" --json           # {"kind":"resume",…} => that item is the answer
# else: the candidates, in pick order, WORK ITEMS ONLY (rule 2)
.venv/bin/yurtle-kanban list --pickable --agent "$ME" --json | .venv/bin/python -c '
import json, sys
for i in json.load(sys.stdin):
    if i["item_type"] in ("expedition", "chore", "hazard"):
        print(i["id"])'
# claim the first candidate; read the rc ALONE (not through a pipe)
.venv/bin/yurtle-kanban claim <ID> --agent "$ME"
```

| `claim` rc | meaning | do |
|---|---|---|
| 0 | claimed (or already yours) | print the item (`show <ID>`) and its lane (below), and stop |
| 3 | lost the race to another agent | `git pull -q --rebase=merges` (the list is stale now), then claim the NEXT candidate |
| 1 | refused (e.g. no longer pickable on origin) | claim the next candidate; all refused → report |
| 8 | the board is halted | stop: report `control status` |
| 4, 5, 6 | remote unreachable / busy / push refused | nothing was claimed: report |

`claim` writes `origin/main` directly (CLAUDE.md § Syncing the board); pairit's step 0 fetches before it cuts the
tree, so the claim is in it.

**No candidates** (the loop's "nothing pickable") → stop and report what waits, one line each:
- ready items that are not pickable, and why: `list --pickable --agent "$ME" --explain`;
- items still in `harbor` (not released): `list -s harbor -t expedition`, `-t chore`, `-t hazard`. Under the release
  rule (CLAUDE.md § Skills) each should carry a comment naming what it lacks; one that is well defined is a missed
  release: report it. The picker itself never moves an item.

**The lane** — print it with the claimed item, so the loop lands it the right way (`pairit` § Scope):

| the item's deliverable | lane |
|---|---|
| code: the crate, `scripts/`, packaging, CI, tests | **code** — `pairit` steps 0–5 |
| a document: the design, a README, a how-to, a review | **docs** — `pairit` § The docs lane |
| a measurement: "measure first", a findings file with the command and its output | **measure** — `pairit` § The measure lane |
| a change in ANOTHER repo (its body says "lands in <repo>") | **other-repo** — `pairit` § The other-repo lane |

Read it from the item's "Definition of Done" / "Done when". An item with both code and a measurement is code, with
the measurement recorded as part of its PR (EXP-001's per-key TTL step); an item with both a doc and code is code (the
doc rides the branch). When unsure, it is code.

**Rules**
1. **Resume before you pick.** An `underway` item held by `<agent>/s-…` of ANOTHER session is that session's work,
   not yours. Take it over only when it is really orphaned (`list --stale` names it):
   `claim <ID> --take-over --agent "$ME"` (recorded as `kb:takenOverFrom`).
2. **Work items only:** expeditions, chores, hazards. Never `claim --next`: it takes the first pickable item of ANY
   type, a voyage or a signal included.
3. **Inputs a host lacks** (a key, the bus, a login) are written in the item's body, not as a tag. A session that
   cannot meet them `bounce`s the item straight back with the reason.
4. Dependencies (`depends_on`) are enforced by `claim` itself; never claim around them.

The item file is the brief. Land it with `pairit`, in its lane.
