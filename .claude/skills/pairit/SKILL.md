---
name: pairit
description: XP pair work for ONE item in quotabus. A test partner in a fresh context writes the tests from the item body and proves them red; an implementer sub-agent writes code to green without touching those tests; the driver opens a GitHub PR; a DISTINCT `claude -p` reviewer session reviews it; the driver merges the PR. Lanes for code, docs, measurements and changes that land in another repo.
argument-hint: "<ID>"
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/*), Bash(python3 *), Bash(git *), Bash(gh *), Bash(claude *), Bash(env *), Bash(make *), Bash(ln *), Bash(cd *), Bash(scripts/*), Agent
---

# pairit (quotabus): tests from a partner, code from an implementer, a PR, one reviewer, merge

**Why a partner writes the tests:** tests written in the same context as the code share its blind spots. Tests
written first, from the spec, by another context, do not.

## Scope

| the change | path |
|---|---|
| a board write (create, move, comment) | the yurtle-kanban verb itself; it lands on `origin/main` (CLAUDE.md § Syncing the board) |
| code (the crate, `scripts/`, packaging, CI) | **this skill**, steps 0–5 |
| a DOCS item the loop claimed (the design, a README, a how-to) | § The docs lane |
| a MEASUREMENT ("measure first"; a findings file) | § The measure lane |
| an item whose change lands in ANOTHER repo | § The other-repo lane (plus steps 0–5 for any code half here) |

`quotabus-next` prints the lane with the item.

## Models (Claude Opus throughout; reviews in an independent session)

The session running this skill is the **driver**. It cuts the worktree, writes the two briefs, reads their results,
runs `make check`, opens the PR, writes and COMMITS the reviewer's brief, spawns the reviewer and merges. **It writes
no tests and no code itself.**
- Steps 1 and 2 run as fresh **Opus sub-agents** (the `Agent` tool, `model: opus`), one per step.
- Step 4, and every measurement review, runs as an **independent Opus session**: a separate `claude -p` process, in a
  context that wrote none of the work. **Every code PR gets one** (CLAUDE.md: code is reviewed by a different session
  than the one that wrote it); there is no tooling exemption.

Give each sub-agent a hard definition of done: its tests or checks pass; where the item touches a real external call
(a provider API, the bus), a real path is proven by a direct dry run, because test fakes have hidden real-API crashes
before; the work is committed; no background process is left running; **no secret is printed, logged or committed**
(real probes run only under `doppler run` / `secretspec run`, and their output is redacted before it is pasted
anywhere).

## The pair

```text
0. CLAIM   quotabus-next already claimed it; cut a worktree + branch from origin/main
1. TESTS   the test partner (fresh Opus sub-agent) writes tests from the item body → commit T, proven RED
2. CODE    the implementer (a SECOND fresh Opus sub-agent) writes code to GREEN without editing T's test files;
           the driver runs `make check`
3. PR      the driver pushes the branch and opens a GitHub PR
4. REVIEW  an INDEPENDENT Opus session (`claude -p`) reviews the PR's head: approve or request changes
5. MERGE   the driver merges the PR (`gh pr merge --merge`), then closes the item
```

**0. Branch:** `git fetch -q origin && git worktree add -q --no-track -b <type>/<id>-<slug> /tmp/<id> origin/main &&
ln -s "$PWD/.venv" /tmp/<id>/.venv` (`<type>` is `exp`, `chore` or `haz`). Every later step uses the literal path
`/tmp/<id>`. Cargo builds use a `CARGO_TARGET_DIR` of this session's own outside the tree (CLAUDE.md § Session start).

**1. The partner's brief:** spawn a fresh Opus sub-agent whose prompt names the worktree `/tmp/<id>` and carries this
brief. Its final report gives the sha, the paths and the failing tests. The brief, verbatim:
- *"Read the item file for <ID>, and the sections of docs/DESIGN.md it cites. Write tests for what the body SAYS (Plan,
  Definition of Done), not for how you would build it. Work only in /tmp/<id>."*
- *"Tests go in their OWN files (Rust: `tests/*.rs` integration tests, or a `#[cfg(test)]` module in a file of its
  own; scripts: `tests/test_*.py`). A check that guards a property carries its controls: a known-answer case, a
  negative control that must fail, and a mutation that must turn a test red. Each control is shown able to fail; a
  control that passes BY CONSTRUCTION (whatever the input) tests nothing. Tests never need a real key or the real
  bus: use fakes (a local HTTP stub, a fake key such as `sk-test-not-a-key`, an in-process or throwaway NATS where a
  test needs one). Add minimal stubs (`todo!()` / `unimplemented!()`, `raise NotImplementedError`) outside the test
  files if needed so the tests compile."*
- *"Commit the tests ALONE as `<ID>: tests (red)`. Run them; confirm they are RED for the RIGHT reason (an assertion
  or the stub), never a compile or import error. Return the sha, the paths and the failing test names."*

Record the red run on the board NOW: `.venv/bin/yurtle-kanban comment <ID> --agent "$ME" --body "tests red at <T>: …"`
(it lands on `main`, never on the branch), with `ME` set in the same Bash call.

**2. Code to green:** spawn a SECOND fresh Opus sub-agent. Its brief gives the item file path and T's sha and test
paths, and says *"make these tests green without editing any file T added or changed; prove the real path where the
item touches real external calls; never print or commit a secret; commit as `<ID>: <what>`; return the sha"*. The
driver then checks `git diff <T>..HEAD -- <T's test files>` is EMPTY. Never edit T's test files: a wrong test goes
back to the partner. Then the driver runs `make check` in `/tmp/<id>`.

**3. The PR:**
```bash
cd /tmp/<id> && git push -q -u origin HEAD
gh pr create --base main --title "<ID>: <what>" --body-file <a file>     # never --body-file -: write the file first
```
The body names the item (`Item: <ID>` — the board, not a GitHub issue), what landed, T's sha, the `make check` run,
and ends with the attribution line the session's instructions give. **Secrets:** the repo is public; nothing in the
body, a commit or a review may carry a key, a raw provider error body or an email.

**4. The review, by a DISTINCT session:**
```bash
# the brief is COMMITTED to the branch before the reviewer is spawned: written to reviews/<ID>-r<N>.brief.md, then
git -C /tmp/<id> add reviews/<ID>-r<N>.brief.md && git -C /tmp/<id> commit -q -m "<ID>: review brief r<N>"
git -C /tmp/<id> push -q                       # <SHA> below is this tip: the brief commit is the reviewed tip
env -u ANTHROPIC_BASE_URL -u ANTHROPIC_AUTH_TOKEN \
  claude --model opus --dangerously-skip-permissions -p "$(cat /tmp/<id>/reviews/<ID>-r<N>.brief.md)" \
  < /dev/null > /tmp/<id>.review.out 2> /tmp/<id>.review.err
# Its own INDEPENDENT process, run in the FOREGROUND (a backgrounded waiter kills a -p child). Tool permission is
# REQUIRED — without it a -p reviewer cannot read the code and invents findings.
# -p prints only the final message, so the brief tells the reviewer to WRITE its review to a file.
```
The driver passes the committed bytes (`$(cat …)`), never a re-typed copy. A code review's brief tells the reviewer
to:
- review PR #<n>, branch `<branch>` at tip `<SHA>`, in its OWN scratch worktree (`/tmp/rv-<id>`), removed when done;
- check that the tests in `<T>` match the item body (and the design sections it cites), not the code;
- confirm `git diff <T>..<SHA> -- <T's test files>` is empty;
- confirm the code does not special-case the tests;
- mutate the code at two points and see a test go red each time;
- check the secret rules: no key on argv, in a row, a log, an error or a fixture; every output path goes through the
  redactor; nothing in the diff or the PR looks like a real key;
- run the gate ITSELF, in its own worktree (`make check`), and record its exit code on the review's third line,
  `gate: make check rc=<N> at <SHA> on <host>`; a gate it could not run is written `gate: NOT RUN — <reason>` and
  named as a limitation. The driver's green run is the author's evidence, not the reviewer's;
- write the authorship line (below);
- quote, for every finding, the command it ran and its output (a finding it did not run is not a finding);
- run EVERY command in the FOREGROUND (no `run_in_background`, `&` or `nohup`): a `-p` session ends when the model
  stops, killing its background work and leaving a mutated tree;
- make NO board writes, NO GitHub writes, and no commits other than the review file.

The reviewer writes `reviews/<ID>-r<N>.md`, first line `reviewed-at-sha: <SHA>`, second line `verdict:
approve|changes`, third line `gate: …` (above), fourth line `brief: reviews/<ID>-r<N>.brief.md @ <sha>`, commits it to
the branch and pushes. The driver then posts it on the PR: `gh pr comment <n> --body-file
/tmp/<id>/reviews/<ID>-r<N>.md`. (GitHub does not let an account approve its own PR, and every session here pushes as
the same account, so the committed review file is the approval of record, not GitHub's review state.)

**The authorship line, in every review and every finding.** Under the header, a paragraph headed `Authorship:` says:
all the work and the review were done by LLM agent sessions (name the model); no human reviewed it (or name the human
and what they read); "distinct" means a separate `claude -p` process with a fresh context that wrote none of the work,
of the SAME model family, briefed by the driver (the author) with the committed brief; and the commissioning session
(`<agent>/s-<8>`) and its stake (what it wrote).

ONE review round. The findings are fixed AT ONCE, and a fix that passes the tests is NOT sent for another review:
- `verdict: approve` → merge.
- `verdict: changes` → hand EVERY finding to the implementer in one message; a finding about a test goes to the test
  partner as a ruled test edit, in its own named commit. Each fix commit names the finding it closes (`<ID>: fix r1
  F<n> — …`); a behaviour fix carries a test that goes red without it.
- `make check` green on the fixed tip → merge, with `reviewed-at-sha <SHA>; r1 findings fixed at <FIX-SHA>` in the
  merge subject and a PR comment listing each finding and its fix commit.
- Escalate instead of merging only when a finding cannot be closed by a tested fix: it needs a Captain ruling, it
  changes the item's scope, or the driver disputes it. Then ask the Captain (AskUserQuestion), never a second review.

**5. Merge** — through GitHub, never by a push to `main` from the shared main checkout:
```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
gh pr view <n> --json headRefOid,mergeable,statusCheckRollup    # head = the reviewed (or fixed) tip; mergeable; checks green
gh pr checks <n> --watch                                         # once CI exists; a red check is not merged
gh pr merge <n> --merge --delete-branch \
  --subject "<ID>: <what landed> (#<n>; reviewed-at-sha <SHA>)"  # a merge commit, so the PR's history stays readable
git -C ~/Projects/quotabus pull -q --rebase=merges               # or wherever the main checkout is
.venv/bin/yurtle-kanban move <ID> done --agent "$ME" --closed-by "<PR URL>" \
  -m "<ID> done: <the measurement> — <the command>"              # only AFTER the merge is on origin
scripts/qb_clean.sh <ID> && git branch -D <branch>
```
If `main` moved and the PR no longer merges cleanly, merge `origin/main` into the branch in `/tmp/<id>` (never a
rebase of a reviewed branch), re-run `make check`, push, and merge; a conflict in code the reviewer read is a finding,
not a silent resolution. Done means the merge is on `origin/main`.

## The docs lane (a DOCS item the loop claimed)

An author in a context of its own (the loop's driver must stay thin for the next item), a check of the item's "Done
when", and a PR from the session's own worktree.

```text
0. TREE    a worktree + branch `docs/<id>-<slug>` at origin/main (never the shared main checkout)
1. AUTHOR  a fresh Opus sub-agent writes the deliverable from the item body and commits it → sha D
2. CHECK   the driver checks D against the item's "Done when", line by line, and runs `make check`
3. LAND    a PR; merged by the driver after the check. A change to docs/DESIGN.md, or an item that asks for a
           review, gets a distinct reviewer exactly as step 4 above
```

**1. The author's brief** (`Agent`, `model: opus`), verbatim plus the item path:
- *"Read the item file for <ID> and every file its body names as the brief. Write what its 'Done when' asks for, in
  /tmp/<id> only. Do not soften a claim to fit a source: change the claim. Never trust a summary over its source:
  when you quote a number, a verdict or a claim, cite the source's path and line range and recount it from the
  primary line. Record a human decision verbatim with the human named (`Captain <date>: "…"`), and say whether the
  human chose the default an agent offered. A Captain's open question (a SIG) is quoted as open, never as decided."*
- *"Commit as `<ID>: <what>` (several commits are fine). Make no board writes. Return the sha, the paths, and the
  item's 'Done when' as a checklist with, for each line, met / not met and the evidence (a path:line, or the command
  and its output)."*

**2. Check.** In `/tmp/<id>`: every checklist line is met, or the gap is a finding (back to the SAME author, once, in
one message). Spot-check two quoted numbers or claims against their primary lines; a mismatch is a finding. `make
check` is green.

**3. Land:** `gh pr create`, then step 5 above. The document lands first and the board second: a peer never sees an
item `done` whose deliverable is not on `main`.

## The measure lane (a measurement the loop claimed, or a measure-first step inside an item)

A measurement answers a question the design marks **[unverified]** or "measure first" BEFORE anything is built on the
answer (CHORE-002; EXP-001's per-key TTL step). The deliverable is a findings file, and the finding is reviewed
because other work is built on it.

1. **Tree and branch** as the docs lane, branch `measure/<id>-<slug>` (or the item's own code branch, when the
   measurement is a step of a code item).
2. **Run it in the foreground**, in this session or a fresh sub-agent, exactly as the finding will record it. Raw
   artefacts (full outputs, logs, captured JSON) are written to the durable per-item host path
   `~/.local/state/quotabus-work/<id>/`, never to a worktree's `data/` or `runs/`, which the clean step keeps a tree
   for and `git worktree remove` would delete silently.
3. **Write `docs/findings/<ID>-<slug>.md`**: the question; the answer in one line; the exact command(s), the host,
   the date, the versions involved (`nats-server`, natscli, the crate, the provider endpoint) and this repo's sha;
   the output, **redacted** (no key, no auth header, no email, no account id the operator did not choose as a label);
   what the answer changes in the design or the plan; and the authorship line. A null is a result: "it does not fire"
   is a finding.
4. **Shared systems** (CLAUDE.md rule 4): a measurement on Mini's bus creates and writes ONLY a bucket of quotabus's
   own (name it `qb_measure_<id>` or the design's bucket), and deletes only a bucket it created itself, stating so in
   the finding. Never write, purge or delete any other bucket or stream.
5. **Review:** the finding lands by a PR reviewed by a distinct session exactly as step 4 above; the brief tells the
   reviewer to re-run the command where it can (it is read-only or touches only quotabus's own bucket) and to compare
   its output with the finding's.
6. If the answer contradicts the design, file a chore to amend `docs/DESIGN.md` (a PR) and say so on the item.

## The other-repo lane (an item whose change lands in another repo)

This repo's sessions never edit another repo's files; the change flows ONE hop, as a packet, and is made there by
whoever works there under that repo's own rules. The ONE sanctioned write in the other repo is the chore of step 3, on
its board (Captain 2026-10-09: "Can you create a chore in nusy-product-team for this - this is how we move work
between repos", quoted in CHORE-017's body).

1. Any half that is built or measured HERE goes through its own lane first.
2. The packet is a doc, landed by the docs lane: `docs/flowback/<ID>-to-<repo>.md` with the exact change asked of the
   receiving repo (the file, the section, the new text), why, and the quotabus sha and finding it depends on.
3. Once the packet is on quotabus `main`, file ONE chore on the receiving repo's board, so someone there is asked to
   land it. Its body links the packet (its URL on quotabus `main`), quotes the Captain's direction above, and asks for
   the landing sha back (a comment on the quotabus item, or a reply on their chore). Write the body to a file in your
   scratchpad first; it names no key, email or private detail (both boards are read by others).
   - **nusy-product-team** (nusy-kanban; `--relate` takes a typed edge `predicate:TARGET-ID`, repeatable):
     `nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}" create chore "<title>"
     --body-file <file> --relate related:<their tracking item>` (quotabus's is nusy-product-team IDEA-13333).
   - **a yurtle-kanban repo:** in THAT repo's own checkout, `.venv/bin/yurtle-kanban create chore "<title>"
     --body-file <file> --push` (its own `.venv`, its own id allocation; never a file edited by hand there).
   - Worked example: CHORE-001's packet `docs/flowback/CHORE-001-to-nusy-product-team.md` was filed as
     nusy-product-team CH-13371, related to nusy-product-team IDEA-13333 (CHORE-001's comment of 2026-10-09).
4. The item is NOT done: it waits on the other repo. `.venv/bin/yurtle-kanban move <ID> blocked --agent "$ME" -m
   "packet at docs/flowback/<ID>-to-<repo>.md; filed as <repo> <THEIR-ID>; waits on <repo> landing it"` (the other
   repo's id is on the item from here on), and the loop picks again. Whoever records the receiving repo's landing
   commit closes it, citing that commit and the read that shows the change is there:
   `.venv/bin/yurtle-kanban move <ID> done --force --resolution completed --agent "$ME" -m "<ID> done: landed in
   <repo> as <their sha> (<date>, '<their subject>'). Check: <the command> → <its output>. Closed with --force: 3.4.0
   refuses stranded → arrived; --force skips the transition table, WIP limits and gates (none configured here), not
   the holder"`. `-m` is the commit message, so the citation is the close's record (CHORE-001's close, 5f570b8,
   which had no `--resolution`). **Why `--force`:** 3.4.0's table allows from `stranded` only `provisioning`,
   `underway` and `harbor` (`workflow.py` 64–68, `BLOCKED: [READY, IN_PROGRESS, BACKLOG]`), and the one route to
   `arrived` runs through `underway`, which CLAUDE.md forbids by hand. Measured 2026-10-09 on Mac-mini in a throwaway
   clone pushing to a throwaway bare origin (quotabus b21fcd8), the item held by the session that parked it and closed
   by another `--agent`: a plain `move <ID> done` → "Illegal move CHORE-019: stranded → arrived. Legal from stranded:
   provisioning, underway, harbor.", rc 1; the forced move above → "…: pushed to origin/main", rc 0, the item
   `arrived`, `resolution: completed`, `kb:forcedMove true`, its `assignee:` unchanged. The holder guard covers
   in-progress items only (`move --help`), so any session may close a LANDED OTHER-REPO item whose `-m` carries the
   landing commit AND the read that shows it; the claimant stays the record. Any other `stranded` item is never
   closed with `--force`: it goes back through the table (`provisioning` or `harbor`).
