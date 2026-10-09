# quotabus — instructions for agents

- **The board is `kanban-work/`, run with yurtle-kanban** (`pip install yurtle-kanban`; 3.4.0 at creation). Create
  items with `yurtle-kanban create <type> "<title>" --body-file - --push` (atomic: it allocates the id, commits and
  pushes). Never hand-number an item.
- **Design first:** `docs/DESIGN.md` is the agreed design; a change to it is a PR, not a silent edit.
- **Never commit, log or publish a secret.** Keys arrive by environment only; every output passes the redactor.
  The repo is PUBLIC: a key in a commit, a PR, a review or a board comment is published.
- **Code lands by a branch and a GitHub PR** reviewed by a different session than the one that wrote it. Tests are
  written before the code.
- **No human time estimates.** Size work by agent context: a chore is a fraction of a context, an expedition is what
  one session can land.
- This repo is tracked from nusy-product-team by IDEA-13333 (dual-tracking for FOSS repos).

## Precedence
This file governs this repo. The machine-global `~/.claude/CLAUDE.md` and nusy-product-team's canon (`nk`, `nk pr`
proposals, verdict rows, the pre-push floor) govern nusy-product-team only. Here: work is tracked with yurtle-kanban
on this repo's board (never `nk`), and code lands through `pairit` as a GitHub PR.

## Session start
```bash
git pull -q --rebase=merges
python3 -m venv .venv && .venv/bin/pip install -q -r requirements-dev.txt   # first time
make check                                       # the gate: script tests + `yurtle-kanban validate` (+ Rust, from EXP-001)
.venv/bin/yurtle-kanban board
/quotabus-loop                                   # picks (quotabus-next), lands (pairit), picks again
```
- **Use this repo's `.venv/bin/yurtle-kanban` (3.4.0)**, never a global or another repo's: versions differ in which
  verbs push (below). Check with `.venv/bin/yurtle-kanban --version`.
- **Rust:** `cargo` may not be on PATH in a sandboxed shell; use the absolute path to the rustup shim when it is not,
  and a `CARGO_TARGET_DIR` of your own outside the tree (one per session; never share another session's).
- **Never edit in the shared main checkout.** Branch work, docs and merges happen in the session's own worktree:
  `git fetch -q origin && git worktree add -q --no-track -b <branch> /tmp/<ID> origin/main && ln -s "$PWD/.venv"
  /tmp/<ID>/.venv`. Every Edit/Write path then starts `/tmp/<ID>/`. The check: `git status --short` in the main
  checkout prints nothing.

## The board
One board in `.kanban/config.yaml`: **development** (nautical theme — expedition `EXP-`, voyage `VOY-`, chore,
hazard `HAZ-`, signal `SIG-`; `kanban-work/`). Nautical status names (what `move` writes): `harbor` (backlog), `provisioning`
(ready), `underway` (in progress), `approaching` (review), `arrived` (done), `stranded` (blocked); `move` also takes
the canonical names (`ready`, `review`, `done`, `blocked`). Item files are the database; git is the transport.
- **Claims are per SESSION:** `ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"`, passed as
  `--agent "$ME"` on `claim`, `move`, `comment`, `bounce` and `control`. Without it 3.x falls back to `$YURTLE_AGENT`,
  then git `user.name` (the GitHub account, not the session), and `claim` refuses. Shell variables do not survive
  between tool calls: set `ME` in the same command. `claim` is the atomic gate; `next` is advisory. Never `move … underway` by
  hand.
- **No assignees.** Work is never assigned to a machine, agent or person: no `create --assign`, no `move --assign`.
  `assignee:` is written only by `claim`; a finished item keeps its claimant as the record of who did it.
- **No host tags.** An item whose inputs exist only on some hosts (a key in one Doppler project, Mini's bus, a
  particular login) says so in its body; a session that cannot meet it `bounce`s it with the reason.
- `control halt` stops all new claims repo-wide; `control status` is read at the top of every pick.
- **An ID from another board is written with its repo:** "nusy-product-team IDEA-13333". Bare IDs mean this board.

### Syncing the board: origin is the board
A board write that is not on `origin/main` does not exist for any other session. yurtle-kanban **3.4.0 pushes every
board write to origin by default** (#1279; measured 2026-10-08 against a throwaway bare origin: `move`, `comment`,
`voyage add` and `claim` each printed "pushed to origin/main" and the commit was on the origin):

| group | verbs | what to do |
|---|---|---|
| **reads the local tree** | `next`, `list` (incl. `--pickable`), `show`, `board`, `blocked`, `validate`; `control status` reads origin as last fetched | `git pull -q --rebase=merges` (or `git fetch` and read `origin/main`) right before a read you act on |
| **writes origin** (edits the item as origin's `main` has it, then pushes one kanban-only commit) | `claim`, `bounce`, `control`, `move`, `comment`, `rank`, `voyage add`; `create` and `update` **with `--push`** | already shared. Run from a feature branch, the write still lands on `main` and the branch is untouched (measured, same run): pull `main` before reading the result |

Rules:
1. **`create` and `update` always take `--push`.** IDs are allocated from origin; two local creates take one id.
2. **Never pass `--no-push` / `--no-commit`.** A local-only board write is invisible to every other session.
3. **A board write never rides a code branch.** No item file is edited by hand on a branch; the verbs above write
   `main` directly.
4. **Never remove a worktree that holds commits not on origin** (`git -C <tree> log --oneline origin/main..HEAD`
   is empty first). Removing a detached tree loses them silently.
5. **`update --related` and `--depends-on` REPLACE the list:** read the item's `related:` / `depends_on:` first and
   pass the whole set.

## Skills
Ported 2026-10-08 from the yurtle-kanban skill set of `Congruentsys/nusy-replicant-24@2de49870` (CHORE-003; the PR
lists what was kept, adapted and cut).
- **`quotabus-next`** (the picker), **`quotabus-loop`** (the loop), **`pairit`** (land one item). Prefer these.
  `pairit` has four lanes and the picker names the one an item takes: **code** (partner tests, an implementer, a PR,
  a distinct `claude -p` reviewer, merge), **docs** (an author, a check against "Done when", a PR), **measure** (a
  findings file with the exact command and its output, a PR reviewed like code) and **other-repo** (a change that
  lands in another repo: the change is written here as a packet, ONE chore asking for it is filed on that repo's
  board, and the item waits `stranded` (blocked) on that repo, naming the chore's id).
- **Cross-repo rule:** filing that one chore on the receiving repo's board is the ONE sanctioned write in another
  repo. Captain 2026-10-09 (a direction, not a default an agent offered): "Can you create a chore in
  nusy-product-team for this - this is how we move work between repos" (CHORE-001 → nusy-product-team CH-13371).
  A quotabus session still never edits another repo's files: the change itself is made there, under its own rules.
- **`steer`** (the outer loop above `quotabus-loop`; ported 2026-10-08 from yurtle-kanban@ecf76fa, MIT): sweeps the
  open signals, the harbor items waiting on a decision and the `stranded` items; decides buckets 1–2 on the board
  against quotabus's goals (G1 honesty and safety, G2 the fleet's use, G3 simplicity and FOSS) and puts only bucket 3
  to the Captain, batched, each with a recommended default.
- **Release rule:** only `provisioning` items are claimable, and well-defined work is released, with `depends_on`
  setting the order. A well-defined work item (an expedition, chore or hazard whose body has a "Definition of Done" /
  "Done when" and that waits on no OPEN Captain decision; a decision the deliverable only records is not a wait) goes
  to `provisioning` when it is filed (`.venv/bin/yurtle-kanban move <ID> provisioning --agent "$ME"`), whatever its
  dependencies: `claim` enforces them. An item that is not yet well defined stays in `harbor` with a comment naming
  what it lacks. Voyages and signals stay in `harbor`; they are not work items.
- **Signals are the Captain's open questions.** No session decides one, EXCEPT through **`steer`**'s bucket 1
  (a measurement settles it) or bucket 2 (the goals or an existing Captain ruling settle it), recorded on the signal
  as a `[steer] bucket-N` comment with its basis and open to the Captain's veto. Bucket 3 (a feature or goal change,
  or human authority) stays the Captain's. Where a SIG blocks nothing, build to the design's recommendation
  (`docs/DESIGN.md` §10) and say so in a comment on the SIG and on the item that relied on it.
- Review and research method: **`uxr-review`** and **`editor-review`** (role-played expert reviews of a surface or a
  draft, before the Captain reads). **`hypothesize`** and **`refine-idea`** need yurtle-kanban's research (HDD) board,
  which this repo has not configured: until a question needs the full Hypothesis → Measure → Experiment trio, a
  measure-first question is a chore in the measure lane (CHORE-002's shape).
- The stock yurtle-kanban 3.x skills (`/work`, `/done`, `/review`, `/expedition`, `/blocked`, `/handoff`, `/status`,
  `/sync`, `/release`, `/release-foss`) also exist; `/work` and `/done` do not follow pairit, so use the three above
  for code.

## Models
Claude Opus throughout. pairit's test partner and implementer are fresh Opus sub-agents (`Agent`, `model: opus`);
every code or measurement review is an independent `claude -p` session, in a context that wrote none of the work.
The driver writes no tests and no code itself.

## Rules
1. **Report the measurement and the command that produced it.** A finding records the exact command, its output
   (redacted), the host, the date and this repo's sha.
2. **Red first.** Tests come from the item body, written by a partner who did not write the code, and are shown red
   for the right reason before the code exists. Every check that guards a property carries a control that shows it
   can fail.
3. **Secrets:** a key is read only from the environment, by the NAME config gives; it never appears on argv, in a
   row, a log, an error, a test fixture or a finding. Test with fake keys; run real probes only under `doppler run`
   or `secretspec run`.
4. **The bus is shared.** Write only quotabus's own bucket(s) and subjects; never write, purge or delete another
   bucket or stream. Create a bucket only when an item says to.
5. **FOSS is pinned to a version**, and every lifted file names its source URL, commit and licence in its header.
   Never build a component from scratch when working FOSS exists.
6. Commit as the machine's GitHub noreply identity. Build output stays out of git and out of the tree.
