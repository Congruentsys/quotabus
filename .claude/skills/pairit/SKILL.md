---
name: pairit
description: XP pair work for ONE item in quotabus. A test partner in a fresh context writes the tests from the item body and proves them red; an implementer sub-agent writes code to green without touching those tests; a DISTINCT reviewer session checks measuring code (scorers, runners, evals; tooling skips it); the driver merges to main. No proposals, no verdict rows, no monorepo gates.
argument-hint: "<EXP-ID>"
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/*), Bash(python3 *), Bash(git *), Bash(claude *), Bash(env *), Bash(make *), Bash(ln *), Bash(cd *), Agent
---

# pairit (quotabus): tests from a partner, code from an implementer, one reviewer, merge

Ported from rachael-neural-lab `pairit` (2026-10-02), itself ported from nusy-product-team's.

**Why a partner writes the tests:** tests written in the same context as the code share its blind spots.
Tests written first, from the spec, by another context, do not (measured in nusy-product-team, 2026-09-24).

## Scope

| the change | path |
|---|---|
| a quick doc, prereg, note or finding edit outside the loop | straight to `main`, no pair, no item |
| code (packages, `scripts/`, runners) | **this skill**, steps 0–4 |
| a DOCS item the loop claimed (a review, an arch doc, a design doc, a verification pass, a LIT review) | § The docs lane |
| an item whose change lands in ANOTHER repo | § The packet lane (plus steps 0–4 for any code half here) |
| a RUN (an expedition that runs an EXPR) | this skill for any code it needs, THEN the run protocol below |

`quotabus-next` prints the lane with the item. Most of LUM's first work is docs (CHORE-001 … CHORE-013,
EXP-001, EXP-003, EXP-006 … EXP-017); EXP-004 is code; EXP-005 is code plus a packet.

## Models (Claude Opus throughout; reviews in an independent session)

The session running this skill is the **driver**. It cuts the worktree, writes the two briefs, reads their
results, runs `make check`, writes and COMMITS the reviewer's brief, spawns the reviewer and merges. **It writes
no tests and no code itself.**
- Steps 1 and 2 run as fresh **Opus sub-agents** (the `Agent` tool, `model: opus`), one per step.
- Step 3 (MEASURING code only, below), and every result review, runs as an **independent Opus session**: a separate
  `claude -p` process, in a context that wrote none of the work.
- The one exception is the **gate audit** (run protocol step 6). Before a gate conclusion is quoted outside this
  repo, a model of a DIFFERENT family audits it: not Claude, and not the Library's family (Qwen). The auditor is
  Laguna-S on a Spark, driven by the Claude Code harness. See `GATE-AUDIT-BRIEF.md`.

**Which code gets a reviewer (Captain 2026-10-02; CLAUDE.md rule 6, "review the RESULT, not every landing").**
The driver classes the item at step 0 and names the class in the merge message:
- **measuring**: code that scores, gates, runs or reads a bar. That covers scorers, battery runners, eval
  harnesses, controls/stubs for a bar, anything a prereg names, and anything that writes `results/`.
  It gets step 3.
- **tooling**: everything else, such as the board, pins, scripts, plumbing and refactors. It SKIPS step 3:
  the partner's red-first tests and `make check` are its check.
When in doubt, it is measuring.

Give each sub-agent a hard definition of done: its tests or checks pass; where the item touches real data or
a real external call, a real path is proven (a direct dry run), because test fakes have hidden real-API crashes
before; the work is committed; no background process is left running.

## The pair

```text
0. CLAIM   quotabus-next already claimed it; cut a worktree + branch from origin/main
1. TESTS   the test partner (fresh Opus sub-agent) writes tests from the item body → commit T, proven RED
2. CODE    the implementer (a SECOND fresh Opus sub-agent) writes code to GREEN without editing T's test files;
           the driver runs `make check` (`make check-full` if the change touches `crates/` or `Cargo.*`)
3. REVIEW  MEASURING code only: an INDEPENDENT Opus session (`claude -p`) reviews the branch tip: approve or
           request changes. TOOLING skips to 4
4. MERGE   the driver merges --no-ff into main in a merge worktree, `make check` passes, push, close the item
```

**Which check (CHORE-030).** A change that touches `crates/` or `Cargo.*` runs `make check-full` (`make check`, then
`make check-rust`: `cargo fetch`, then `cargo test --workspace --no-fail-fast`) at steps 2 and 4 and on the merge;
a docs-only or Python-only change keeps `make check`. `check-rust` exit 77 is a SKIP (no cargo, or the git
dependency cannot be fetched): such a change is not merged from that host; land it from a host with cargo (a Spark).

**0. Branch:** `git fetch origin main && git worktree add --no-track -b exp/<id>-<slug> /tmp/<id> origin/main && ln -s "$PWD/.venv" /tmp/<id>/.venv`.
Every later step uses the literal path `/tmp/<id>`, and every python command in the worktree runs as
`PYTHONPATH=/tmp/<id> .venv/bin/python …`, so it runs THIS worktree's code.

**1. The partner's brief:** spawn a fresh Opus sub-agent whose prompt names the worktree `/tmp/<id>` and
carries this brief. Its final report gives the sha, the paths and the failing tests. The brief, verbatim:
- *"Read the item file for <ID>. Write tests for what the body SAYS (Objective, Exit criterion, Phases,
  Refusal), not for how you would build it. Work only in /tmp/<id>."*
- *"Tests go in their OWN files under `tests/`. For a measurement or runner, the tests ARE its controls: a
  known-answer fixture, a negative control that must fail, and a mutation that must turn a test red. Each control
  is shown able to fail; a control that scores its value BY CONSTRUCTION (0, chance or ceiling whatever the input)
  tests nothing and is never a conjunct of a bar. Add minimal stubs (`raise NotImplementedError` or the language's
  equivalent) outside the test files if needed."*
- *"Commit the tests ALONE as `<ID>: tests (red)`. Run them; confirm they are RED for the RIGHT reason (an
  assertion or the stub), never an import or compile error. Return the sha, the paths and the failing test names."*

Record the red run on the board NOW, through `scripts/yk_push.sh` (CLAUDE.md § Syncing the board), never as a commit on the
branch (it would reach `main` only at the merge): `scripts/yk_push.sh comment <ID> --agent "$ME" --body "tests red at <T>: …"`.
**Every board write in this skill goes through `scripts/yk_push.sh`**, with `ME` set in the same Bash call.

**2. Code to green:** spawn a SECOND fresh Opus sub-agent. Its brief gives the item file path and T's sha and
test paths, and says *"make these tests green without editing any file T added or changed; prove the real path
where the item touches real data or external calls; commit as `<ID>: <what>`; return the sha"*.
The driver then checks `git diff <T>..HEAD -- <T's test files>` is EMPTY. Never edit T's test files: a wrong
test goes back to the partner. Then the driver runs `make check` in `/tmp/<id>` (`make check-full` for `crates/` or `Cargo.*`).

**3. The review, by a DISTINCT session** (measuring code; tooling goes straight to step 4):
```bash
# the brief is COMMITTED before the reviewer is spawned (CHORE-062): written to reviews/<ID>-r<N>.brief.md, then
git -C /tmp/<id> add reviews/<ID>-r<N>.brief.md && git -C /tmp/<id> commit -q -m "<ID>: review brief r<N>"
git -C /tmp/<id> push -u origin HEAD           # <SHA> below is this tip: the brief commit is the reviewed tip
env -u ANTHROPIC_BASE_URL -u ANTHROPIC_AUTH_TOKEN \
  claude --model opus --dangerously-skip-permissions -p "$(cat /tmp/<id>/reviews/<ID>-r<N>.brief.md)" < /dev/null > /tmp/<id>.review.out 2> /tmp/<id>.review.err
# Its own INDEPENDENT process, run in the FOREGROUND (a backgrounded waiter kills a -p child). Tool permission is
# REQUIRED — without it a -p reviewer cannot read the code and invents findings (measured 2026-09-24).
# -p prints only the final message, so the brief tells the reviewer to WRITE its review to a file.
```
**The brief is committed, not only in the driver's context** (PAPER-13257 § 5.3 rec. 4, pinned at
`refs/nusy-product-team/research/Paper13257-Agent-Run-Research-Adversarial-Review/PAPER-13257-DRAFT.md`, cited in
this skill as `P57`; :704-706). The driver writes it to `reviews/<ID>-r<N>.brief.md` (a result review: `reviews/<ID>-result-r<N>.brief.md`; a pre-lock
review: `reviews/<ID>-prereg-r<N>.brief.md`), commits it as `<ID>: review brief r<N>` and pushes it BEFORE the spawn,
and passes the committed bytes (`$(cat …)`), never a re-typed copy. The driver is the author: committing the brief
makes what the reviewer was pointed at checkable; it does not make the brief independent (`P57`:297).
A result review or a pre-lock review uses the template **`.claude/skills/pairit/RESULT-REVIEW-BRIEF.md`**, filled.
A code review's brief tells the reviewer to:
- review branch `exp/<id>-…` at tip `<SHA>` in its OWN scratch worktree (`/tmp/rv-<id>`), removed when done;
- check that the tests in `<T>` match the item body, not the code;
- confirm `git diff <T>..<SHA> -- tests/` is empty;
- confirm the code does not special-case the tests;
- mutate the code at two points and see a test go red each time;
- run the gate ITSELF, in its own worktree (`make check`, or `make check-full` for `crates/` / `Cargo.*`), and record
  its exit code on the review's third line, `gate: make check rc=<N> at <SHA> on <host>`; a gate it could not run is
  written `gate: NOT RUN — <reason>` and named as a limitation. The driver's green run is the author's evidence, not
  the reviewer's (`P57`:644-646, 704-706);
- write the authorship line (below);
- quote, for every finding, the command it ran and its output (a finding it did not run is not a finding).
- run EVERY command in the FOREGROUND (no `run_in_background`, `&` or `nohup`): a `-p` session ends when the model
  stops, killing its background work. On 2026-10-05 an EXP-024 reviewer backgrounded `make check-full` and a mutant
  sweep, exited, and left no review and a mutated tree. Say this in the brief, and in every result-review brief.

The reviewer writes `reviews/<ID>-r<N>.md`, first line `reviewed-at-sha: <SHA>`, second line
`verdict: approve|changes`, third line `gate: …` (above), fourth line `brief: reviews/<ID>-r<N>.brief.md @ <sha>`,
commits it to the branch and pushes. (A result review adds a fifth, `escalations:`; RESULT-REVIEW-BRIEF.md § C.)

**The authorship line, in every review and every finding** (`P57`:721). Under the header, a paragraph headed
`Authorship:` says: all the work and the review were done by LLM agent sessions (name the model); no human reviewed it
(or name the human and what they read); "distinct" means a separate `claude -p` process with a fresh context that
wrote none of the work, of the SAME model family, briefed by the driver (the author) with the committed brief; and the
commissioning session (`<agent>/s-<8>`) and its stake (what it wrote, and which verdict it gains from).

ONE review round. The findings are fixed AT ONCE, and a fix that passes the tests is NOT sent for another review:
- `verdict: approve` → merge.
- `verdict: changes` → hand EVERY finding to the implementer in one message; a finding about a test goes to the
  test partner as a ruled test edit, in its own named commit. Each fix commit names the finding it closes
  (`<ID>: fix r1 F<n> — …`); a behaviour fix carries a test that goes red without it.
- `make check` green on the fixed tip → merge, with `reviewed-at-sha <SHA>; r1 findings fixed at <FIX-SHA>` in the
  merge message and a board comment listing each finding and its fix commit.
- Escalate instead of merging only when a finding cannot be closed by a tested fix: it needs a Captain ruling,
  it changes the item's scope, or the driver disputes it. Then ask the Captain (AskUserQuestion), never a second review.

**4. Merge** — in a MERGE WORKTREE of your own, never in the shared main checkout (two sessions on one host
share it; a concurrent `pull --rebase` there flattened a `--no-ff` merge mid-check in rachael-neural-lab):
```bash
ME="${NUSY_AGENT_NAME:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
git fetch -q origin && git worktree add -q --detach /tmp/<id>-merge origin/main && ln -s "$PWD/.venv" /tmp/<id>-merge/.venv
cd /tmp/<id>-merge
git merge --no-ff origin/exp/<id>-<slug> -m "<ID>: <what landed> (reviewed-at-sha <SHA> | review: skipped (tooling))"
make check && push_main      # `make check-full && push_main` when the change touches crates/ or Cargo.*
scripts/yk_push.sh move <ID> done --agent "$ME" -m "<ID> done: <the measurement> — <the command>"   # only AFTER the merge is on origin
cd - && git worktree remove --force /tmp/<id>-merge /tmp/<id> && git branch -D exp/<id>-<slug>
```

**`push_main`** — a rejected push is a peer landing first, not a stop (ported from rachael-lab's CHORE-015). Run it
in the worktree you are landing from; it keeps a merge a merge and re-checks what it is about to push. A shell
function does not survive a Bash tool call: define it in the same command that calls it.
```bash
push_main() {
  for i in 1 2 3; do
    git push -q origin HEAD:main && return 0
    git fetch -q origin && git rebase -q --rebase-merges origin/main || { echo "rebase failed: resolve by hand (git rebase --abort?)"; return 1; }
    make check || { echo "make check red after rebasing onto the peer's work: stop, do not push"; return 1; }
  done
  echo "push_main: 3 rejected pushes — stop and report"; return 1
}
```
Any pull or rebase over a checkout that may hold a local merge keeps it (`--rebase=merges`, `--rebase-merges`); plain `--rebase` linearises it.
Done means the merge is on `origin/main`.

## The docs lane (a DOCS item the loop claimed)

CLAUDE.md sends docs straight to `main`: no branch, no test partner, no reviewer session, no merge. What the loop
still needs is an author in a context of its own (a LIT review fetches dozens of pages; the loop's driver must stay
thin for the next item), a check of the item's "Done when", and a landing from the session's OWN worktree.

```text
0. TREE    a detached worktree at origin/main (never the shared main checkout)
1. AUTHOR  a fresh Opus sub-agent writes the deliverable from the item body and commits it → sha D
2. CHECK   the driver checks D against the item's "Done when", line by line, and runs `make check`
3. LAND    push_main; then the board moves through scripts/yk_push.sh; clean the tree
```

**0.** `git fetch -q origin && git worktree add -q --detach /tmp/<id> origin/main && ln -s "$PWD/.venv" /tmp/<id>/.venv`

**1. The author's brief** (`Agent`, `model: opus`), verbatim plus the item path:
- *"Read the item file for <ID> and every file its body names as the brief. Write what its 'Done when' asks for,
  in /tmp/<id> only. Cite a pinned source by its pinned path (`refs/…`, `fixtures/…`, CLAUDE.md § Working across
  repos). Never edit `refs/` or `fixtures/` by hand: a missing source is pinned with `scripts/pins.py add`
  (`refs/README.md`). Never commit licence-restricted text (NICE/WHO): cite it by manifest and sha256. Do not
  soften a claim to fit a source: change the claim."*
- *"Never trust a summary over its source. When you quote a verdict, a count or a claim, cite the source's path
  and line range, copy its tags verbatim ('flips on one seed', 'knife-edge', 'by construction', NULL / NOT READ),
  and recount every number from the primary line, not from a document that summarises it. Record a human decision
  verbatim with the human named (`Captain <date>: "…"`, path:line), say whether the human chose the default an
  agent offered, and mark a question an agent decided by `/steer` as **(steer)**, never as the human's. A finding
  carries the authorship line (step 3)."*
  (This bullet is PAPER-13257 § 5.3 recs 10 and 12, `P57`:719-720, 722-723; `P57` is its pinned path, step 3.)
- *"Commit as `<ID>: <what>` (several commits are fine). Make no board writes. Return the sha, the paths, and the
  item's 'Done when' as a checklist with, for each line, met / not met and the evidence (a path:line, or the command
  and its output)."*

**2. Check.** In `/tmp/<id>`: every checklist line is met, or the gap is a finding (back to the SAME author, once,
in one message). Spot-check two quoted counts or verdicts against their primary lines (the source's tags copied,
the number recounted); a mismatch is a finding. `git diff --name-only origin/main..HEAD -- refs fixtures` is empty
unless the author pinned a source, and then `make check` (`pins.py verify`) covers it. `make check` is green. A doc item that names a
distinct-session review in its body (EXP-017; any item that says "adversarial review") runs that review exactly as
step 3 above (brief committed as `reviews/<ID>-r1.brief.md` first, `claude -p`, `reviews/<ID>-r1.md`, ONE round,
findings fixed at once); no other docs item gets one (CLAUDE.md rule 6 reviews the RESULT, and EXP-017 is the result review for Phase 3).

**3. Land**, from `/tmp/<id>`:
```bash
ME="${NUSY_AGENT_NAME:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
cd /tmp/<id> && push_main
scripts/yk_push.sh move <ID> done --agent "$ME" -m "<ID> done: <what landed, at <sha>> — <the check that shows it>"
# a LIT chore (CHORE-002 … CHORE-013) also closes its LIT and reports to the IDEA:
#   scripts/yk_push.sh move LIT-00N active --agent "$ME" && scripts/yk_push.sh move LIT-00N complete --agent "$ME" -m "<review path>"
#   scripts/yk_push.sh comment IDEA-R-001 --agent "$ME" --body "LIT-00N: <one-line verdict>"
cd - && git worktree remove --force /tmp/<id>
```
The document lands first and the board second, each on `origin` before the next step: a peer never sees an item
`done` whose deliverable is not on `main`.

## The packet lane (an item whose change lands in another repo)

This repo never edits another repo (CLAUDE.md § Working across repos); the result flows back ONE hop as a packet,
filed on the receiving board by whoever works there. So this lane lands the packet here and parks the item:

1. Any half that is measured or built HERE (EXP-005's "measurement half") goes through steps 0–4 (code) first.
2. The packet is a doc, landed by the docs lane: `docs/flowback/<ID>-to-<repo>.md` with the finding, its
   measurement and the command, the pinned commits it depends on (this repo's sha, every `refs/` / `fixtures/` pin),
   and the exact change asked of the receiving repo. It copies each quoted result's tags ("flips on one seed",
   "knife-edge", "by construction") and carries the authorship line (step 3). A packet that quotes a **gate
   conclusion** (G0, G1a, G1b, …) cites the gate's approving audit, `reviews/<GATE>-audit-r<N>.md @ <sha>`, and lands
   only after it (run protocol step 6). The packet's item `depends_on` that gate's audit item.
3. The item is NOT done: it waits on the other repo. `scripts/yk_push.sh move <ID> stranded --agent "$ME" -m "packet at
   docs/flowback/<ID>-to-<repo>.md; waits on <repo> filing and landing it"`, and the loop picks
   again. Whoever records the receiving repo's item id and landing commit moves it back and closes it.

## The run protocol (every expedition that runs an EXPR)

1. `prereg/EXPR-00N.md` is committed and pushed BEFORE the first checkpoint, with bars, arms, seeds, nulls,
   the unit, and the **device class of every arm** (arms compared inside one bar train on one device class).
   `git log` must show it earlier than any `results/` commit. A bar is never moved after a read. Every control a
   bar names is shown able to fail; a control that is 0, chance or ceiling **by construction** is never a conjunct
   of a bar (`P57`:701-702). A pre-lock review, when the item names one, uses RESULT-REVIEW-BRIEF.md (§ B.3, B.4).
2. Run exactly what the prereg says. `results/<H>/eval.json` records the script sha, this repo's sha, any pinned
   dependency's commit, fixture sha, seed, device and command.
   **A run's gitignored artefacts live outside any worktree from the start** (HAZ-009): transcripts, logs, packs,
   private gold and converter outputs are written to the durable per-item host path `~/.nusy/qb-data/<ID>/`, never
   to a worktree's `data/`, `runs/` or `checkpoints/`, which the clean step keeps a tree for and `git worktree remove`
   would delete silently. The result records that path (and the host) beside the sha256 of each artefact it cites.
3. `make check` is green on the run's commit BEFORE the result review; a known host-local failure is named,
   never skipped silently. This is the author's gate; the reviewer runs its own and records the exit code (step 4).
4. ONE adversarial reviewer context — launched as in step 3 above, its brief filled from
   **`.claude/skills/pairit/RESULT-REVIEW-BRIEF.md`** and committed as `reviews/<ID>-result-r<N>.brief.md` on `main`
   before the spawn — reviews the RESULT against the prereg before it is quoted. Its findings are fixed at once; no
   second result review. The reviewer runs the gate itself (`gate: … rc=<N>`) and writes the authorship line. It
   answers the **construction check** for every arm and read, "what does each arm actually receive?" (tokens, ids,
   rows, fixtures: path:line → does it match the arm's name?; `P57`:709-712). **Disclosure escalates**
   (`P57`:713-715): a review that writes "by construction", "cannot be excluded" or "knife-edge" about what a
   verdict-bearing bar or a quoted claim rests on either (a) sets `verdict: changes` with a finding a tested fix
   closes, or (b) names a signal on its `escalations:` line, which the driver files (`.venv/bin/yurtle-kanban create
   signal "<ID>: <phrase> on <bar>" … --push`) before the result is quoted; the phrase is a tag that travels with every
   quote. A phrase with neither is a defect in the review: the driver files the signal itself. The result review
   REQUIRES a **leave-one-seed-out** table for every verdict-bearing bar: per-seed values (with the eval-JSON key path),
   the threshold, the verdict as read, and the verdict recomputed with each seed dropped in turn. A verdict that
   changes is tagged **"flips on one seed"**, and the tag travels with every quote.
5. Move the EXPR to complete with the figure and the command, carrying every tag the review attached ("flips on
   one seed", "by construction", "knife-edge") and any SIG id from step 4. A finding written from it carries the
   authorship line (step 3). A null is a result.
6. **A gate conclusion leaves this repo only after a different-family audit** (CHORE-064; `P57`:716-717, made
   mandatory by the (steer) ruling in `research/LUM/16-RACHAEL-PHASE-1-IMPACT.md` § 5 row 8, open to the Captain's veto).
   A gate is G0, G1a or G1b (`research/LUM/15-LUM-SCHOOLING-PLAN.md`:266), any later `prereg/G<…>.md`, or any read
   that decides whether a voyage or the programme proceeds; when in doubt, it is a gate. Before a gate's conclusion is
   quoted **outside this repo** (a packet, another repo's board, a paper, a shared artifact), the following must hold:
   - one **out-of-context audit** by a model of a **different family** has read the prereg, the result, the result
     review and the code. The family must be neither Claude (the authors) nor the Library's family (Qwen). A Qwen
     auditor of a Qwen-Library result is not independent of the model being measured, so it is not admissible;
   - the audit ran agentically (Laguna-S through `nusy-laguna-proxy`, with the Claude Code harness), so that it runs
     the gate and re-reads the primary files itself;
   - its brief and its verdict are committed as `reviews/<GATE>-audit-r<N>.brief.md` and `reviews/<GATE>-audit-r<N>.md`,
     and the verdict is `approve`;
   - `git diff <audited sha>..origin/main` over the rules (CLAUDE.md, this skill, `scripts/verify_pins.py`) and over the
     audit's scope is empty. Any change there means a re-audit, `r<N+1>`.
   The audit does not gate this repo's own use of the result (step 4 does). How the auditor is chosen, invoked and
   checked (the vLLM request counter must rise), the brief, the header and the fallback are all in
   **`.claude/skills/pairit/GATE-AUDIT-BRIEF.md`**. If no admissible model can run, the conclusion stays in the repo
   and the driver signals the Captain.
7. **Items that read under ONE prereg whose amendments pin a shared tooling tree are SERIALISED** (HAZ-012; the
   default in force, open to the Captain's veto). One item at a time holds the tree, from its `prepare` (its first
   session) until its finished read (the result and its banks) is on `main`.
   **Why, from the case that made the rule (EXP-045, groups G1–G4 = EXP-072…075):** every group's amendment pins a
   tooling commit (`prereg/EXP-045-gold-without-a-panel.md` § 11), and `read` refuses with `PinMismatch` unless
   `g1.tooling_pin_holds` is true for it (`scripts/exp045/g1.py` L293; `g3.py` L604 and `g4.py` L763 delegate to it).
   That rule covers two WHOLE trees, `TOOLING_TREES = ("scripts/exp045", "scripts/exp049")` (`g1.py` L267): the pinned
   commit must be an ancestor of HEAD and both trees unchanged and clean since, with one exemption (`_only_relock`,
   L271: `drift.py`'s `EXPECTED_LOCK` line replaced, one out and one in). Each group's runner, its `run.py`
   subcommand and its shared-helper changes land in that same tree, so one group's landing breaks another's pin
   (EXP-073's merge was held twice on 2026-10-08; `results/EXP-045-REPLAY.md` has the measurements).
   1. **Declare the hold on the board before `prepare`, and release it the same way.** A comment on the shared parent
      item (for EXP-045's groups: EXP-045), through `scripts/yk_push.sh`, from any tree:
      `scripts/yk_push.sh comment <PARENT> --agent "$ME" --body "TOOLING HOLD: <ID> holds <the pinned trees> from
      prepare until its read is on main; expected end: <what will be on main>"`. First `git fetch -q origin` and read
      the parent's comments on `origin/main`: every earlier `TOOLING HOLD` must be followed by its `TOOLING RELEASED`.
      If two holds land, the earlier one on `origin/main` holds and the later is withdrawn by a comment. When the read
      is on `main`, or the item gives up: `… comment <PARENT> --agent "$ME" --body "TOOLING RELEASED: <ID>; result at
      <sha>; the tree is free"`.
   2. **While the tree is held, nobody else lands a change under it on `main`**: not a runner, not a shared-helper
      change, not a chore. A second item still does everything that stays on its BRANCH (partner tests,
      implementation, code review and its fixes, briefs, merging `main` into the branch and re-running the suites)
      and waits to merge. A chore that touches the tree `depends_on` the items in flight.
   3. **A tooling fix the HOLDER needs mid-run is the holder's to land**, with the re-pin its prereg prescribes (for
      EXP-045, the group's amendment names the commit `prepare` LAST ran under; G4 did this at `26e50b3`, Amendment 4
      § G4-9). Nobody else's fix lands until the release.
   4. **Amendments are committed one at a time, each on a `main` that holds the previous one, never from two
      bases.** Every amendment commit of EXP-045 rewrites the same things: the prereg's final `sha-lock:` line,
      `scripts/exp045/drift.py` `EXPECTED_LOCK` and `results/EXP-045-runs-manifest.json` (`git show --stat 8a2f9de3
      256e39de c10b63e5`: those files, the prereg, and the test's then hard-coded `A1_LOCK`, which since CHORE-088
      reads the prereg's lock live; a landed amendment's commit may be added to its `PAST_LOCKS`). Two amendments from different bases conflict there, and the merged text needs a third lock
      that neither read ran under.
   5. **An amendment's heading names its own group token and no other group's.** Each group's parser finds its
      section by one regex on the `## ` heading line: `^## Amendment\b.*\bG1\b` (`g1.py` L322), `…\bG3\b` (`g3.py`
      L612), `…\bG4\b` (`g4.py` L772). A heading holding two tokens is read by both parsers.
   6. **After ANY later landing under the pinned tree, a finished item's pin no longer holds at HEAD, by design.**
      Its finished read is replayed only as `results/EXP-045-REPLAY.md` says: on the host that holds the run
      directory, in a checkout from the item's own window (at or after its amendment commit, before the next
      landing under the tree). The pinned tooling commit itself is NOT such a checkout: the amendment is not
      committed there.
   **What it costs:** waiting. A reviewed, green runner stays unmerged while another item's sessions run, and it
   re-merges `main` each time the holder lands something. **What it does not cover:** nothing enforces it. No
   `claim`, `control` verb, hook or test knows about a hold; it is a convention the board carries, and a session
   that does not read the parent's comments can break an in-flight pin (the holder's `read` then refuses with
   `PinMismatch`, before any session, as soon as its checkout takes the landing in: a stopped read, not a wrong
   one). Narrowing the pin to the files a read
   actually executes (HAZ-012's option (b)) would remove most of the waiting; it changes what a prereg pins, so it
   is an amendment and the Captain's decision (on SIG-001), not this rule's.
