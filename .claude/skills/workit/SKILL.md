---
name: workit
description: Claim → branch → implement every phase → red-first tests → the floor → self-review → sub-agent pair review → push → proposal. Re-cut at VY-11604 Commit B (CH-11826).
disable-model-invocation: false
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/workit/SKILL.md` on
> 2026-10-08 (Captain-directed). It was written for that monorepo's fleet. **In quotabus, until CHORE-003 adapts it,
> read every mechanism below through this mapping, and stop rather than guess when one has no equivalent:**
> - board: `nusy-kanban` / `nk` on NATS → **`yurtle-kanban`** over `kanban-work/` (`create … --push`, `claim`, `list`, `move`);
>   item ids are `EXP-`/`CHORE-`/`VOY-`/`SIG-`/`HAZ-`; there is no `nk pr` proposal store;
> - landing: monorepo proposals / `pairit` §5 scratch merge → **a branch and a GitHub PR** (`gh pr create`), reviewed by a
>   distinct `claude -p` session, merged after review;
> - Layer-B rows (`verdicts record`, `NUSY_VERDICT_CONF`), `scripts/lib/*`, `scripts/safety-paths.conf`, the pre-push hook,
>   arch-guard, `CA-*` campaign clauses and fleet canon (`CLAUDE.md` of the monorepo) **do not exist here** — the review
>   verdict goes in the PR, not a verdict row;
> - this repo's `CLAUDE.md` wins over anything below.

# Work It

> ## ⚠ STOP — is this item under `CA-12764`?
>
> **Then this skill does not run.** The Captain suspended the loop for that campaign on 2026-09-18: *"I won't use any skills for this
> work … those skills and automation are what has killed progress."* Project `CLAUDE.md`
> §`## ⚠ CA-12764 (PIVOT)` is the authority, and it outranks both this file and the machine-local
> canon's guardrail #3.
>
> **Check first — one hop, and it is cheap:**
>
> ```bash
> nusy-kanban relation query <ID> >/tmp/pivot-check.txt 2>&1; rc=$?; echo "rc=$rc"
> grep -qE 'CA-12764|VY-12771' /tmp/pivot-check.txt && echo "PIVOT SCOPE — DO NOT RUN THIS SKILL"
> ```
>
> ⚠ **`rc` IS HALF THE ANSWER — a silent grep is not a clean result.** The command runs BARE
> because canon requires it (a `$` expansion takes it out of a `Bash(nusy-kanban …)` permission
> rule), and bare means the binary resolves the bus from `NUSY_FLEET_KANBAN_SERVER` ITSELF. Where
> that knob is not exported the binary refuses with `no NATS server given`, rc 1, and writes nothing
> the `grep` can match — so the check FAILS OPEN and looks exactly like "not in scope". **Read the
> rc before the grep.** On that refusal, re-run the same one command with `--server` and the bus
> literal spelled in `CLAUDE.md`'s `nk` alias; on any other non-zero rc the probe did not answer.
> An unanswered probe is CANNOT-ASSESS, never absence (HZ-11813) — treat the item as IN scope and
> read `research/pivot/ACTION-PLAN-DISPATCH.md` until a probe ANSWERS.
>
> ⚠ `relation query` shows **direct** edges only, so this is a ONE-HOP check. An expedition under
> `VY-12771` matches on `VY-12771`; a measure or experiment under the campaign matches on `CA-12764`.
> **If in doubt, read `research/pivot/ACTION-PLAN-DISPATCH.md` and do the work directly.**
>
> **Instead:** do the work, land findings as markdown under `research/pivot/`
> (`<EX-ID>-<phase>-<what>.md`), and close with
> `nk move <ID> done --resolution completed --closed-by <the markdown path>`. **No proposal, no
> spawned reviewer, no verdict row, no item per finding.** ⚠ Still in force and not negotiable: the
> never-launder floor, the provable gate, `refuse_if_protected()`, genome/seal — and **no `PAPER`
> item, ever, on this campaign**.

Argument: an item id. `alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'`. Every rc UNPIPED.

## 1. Claim and read

```bash
nk claim <ID>                                   # typed refusal ⇒ stop; never --force
nk show <ID>                                    # phases, acceptance, tests, files
```

**A body naming a `scripts/` path that is absent from the tree ⇒ `nk bounce <ID> --reason "<path> is
not in the tree (deleted at VY-11604 Commit C); re-scope against the loop"`.** Do not reinterpret.

**The classifier:** *could a measurement change the answer?* Yes ⇒ the claim is a Hypothesis, the
metric a Measure, the run an Experiment — linked on the item and run BEFORE and AFTER the work.
A doubtful body ⇒ one fresh sub-agent definition pass (diff-free: the body against its cited canon);
a method/premise fix is a self-replan recorded on the item; a GOAL change ⇒ `nk bounce`.

## 2. Your OWN worktree, then branch from main's commit

**Work in your own worktree, never in the shared checkout** (`~/projects/nusy-product-team`). The
resident loop lives there and runs the merge transport in it (`campaign-watcher` §1), and a tree you
just created cannot hold another session — creating one IS most of the guard.

```bash
# EVERY step that can fail is checked, and the check runs on the ABSOLUTE path, never on `.`:
# an unchecked `worktree add` (it fails when the branch already exists — the revise/round-2
# path) leaves you standing in the SHARED checkout, where `.` answers about THAT tree and says
# CLEAR. `$WT` is safe HERE only because this is all one block.
git fetch origin || exit 1                                        # a stale fetch = a stale base
WT=~/fleet-wt/<id>                                                # <id> lowercased, e.g. hz12006
[ -d "$WT" ] || git worktree add -b <type>/<id>-<slug> "$WT" origin/main || exit 1
cd "$WT" || exit 1
# The REUSE path skips the `worktree add`, so nothing above has based this tree on anything.
# Verify rather than reset — a resumed item's worktree holds work that `switch -C` would eat.
git rev-parse --show-toplevel >/dev/null 2>&1 || { echo "$WT is not a working tree"; exit 1; }
[ "$(git symbolic-ref --short HEAD 2>/dev/null)" = "<type>/<id>-<slug>" ] || {
    echo "$WT is on $(git symbolic-ref --short HEAD 2>/dev/null || echo 'a detached HEAD'), not <type>/<id>-<slug>"; exit 1; }
git merge-base --is-ancestor origin/main HEAD ||
    echo "⚠ this branch is BEHIND origin/main — rebase before you finish, or your gates lie"
scripts/fleet/session-exclusivity-check.sh "$WT" >/tmp/sx-<id>.log 2>&1; rc=$?
cat /tmp/sx-<id>.log                                              # READ the path= field it prints
# The refusal branch is HERE, not in the prose below: `scope=path` ⇒ a different worktree;
# `scope=host` ⇒ that worktree LOOPS, apply the remedy the log prints.
[ "$rc" = 0 ] || { echo "not your workspace (rc $rc) — read scope= in the log above"; exit 1; }
grep -Fq 'predicate=weak' /tmp/sx-<id>.log && {              # a CLEAR the guard cannot stand behind:
    echo "CLEAR but predicate=weak — scope=host, like every rc 2 about the pattern or the probe."
    echo "The guard matched some candidate but never recognised a session of its own, so it cannot"
    echo "say this tree is empty. A DIFFERENT WORKTREE WILL NOT CLEAR IT — the pattern and the probe"
    echo "are properties of the HOST. remedy: run this from inside a real session (the strong control"
    echo "keys on your own session root), or point NUSY_SESSION_PROC_PATTERN at this host's session"
    echo "process name — walk ps -o comm= up your own ancestry to see what that name is."
    exit 1; }
pwd -P; git log --oneline -1                                      # say where you are and on what
```

`rc 0` CLEAR ⇒ proceed · `rc 1` OCCUPIED (a live session that is not yours or your descendant has
its cwd in that tree) ⇒ take a different worktree · `rc 2` UNDETERMINABLE (the probe could not
answer) ⇒ **also do not work there**. Neither refusal is aimed at the item — but only a `scope=path`
refusal costs one `git worktree add`. A `scope=host` rc 2 is about the probe or the pattern, and a
new worktree changes neither, so there the cost is whatever its printed remedy costs, up to a
genuinely stopped item (`candidate-cwd-unreadable` on a host carrying a session-named process whose
cwd you cannot read is the real case — usually another user's, but a same-user process that is
non-dumpable, or whose thread-group leader has exited, is unreadable to its own uid too). Refusing is still right in both: a guard that silently
passes where its probe is unavailable is worse than no guard. **`cat` the log rather than trusting `$rc` alone**: the `path=`, `pattern=` and `predicate=`
fields are how you see that it answered about the tree you meant, with a pattern that matched
something.

⚠ **On rc 2, read `scope=` before you take another worktree.** `scope=path` is about the target and
a different worktree clears it. **`scope=host` is about the probe or the pattern — no worktree
clears it, and re-running elsewhere LOOPS.** Those lines print their own remedy (which pattern to
set, which pids to deal with, or — only where that line says so — which other leg to try; the
`candidate-cwd-unreadable` line says explicitly NOT the other leg, and why); do that instead. HZ-12108 is why: `reason=name-predicate-inert`
means the guard cannot recognise this host's own session process, and before it that condition
returned CLEAR on M5, Mini and Air, so both skills' refusals were inert on three of five machines.
A `CLEAR` whose `predicate=weak` is a CLEAR the guard could not fully stand behind: it matched some
candidate but not a session of its own. A third value, `predicate=self-calibrated name=<x>`
(CH-12573), is a CLEAR the guard CAN stand behind and the block continues on it: the shipped name
predicate was inert, so the guard added the name this host's own enumerator gave for its session
root — a UNION with the shipped class, never a substitution — and re-ran its own control against the
re-filtered enumeration. ⚠ That re-ask is WEAK PROOF, not the proof the shipped default passes: it
shows the filter now accepts the one process the enumerator named, not that the predicate is fit. It is spelled
differently from `strong` because the predicate that earned it is one nobody reviewed — read the
`name=` field and check it looks like a session. **Treat a `predicate=weak` CLEAR as `scope=host` too**
— no worktree makes the guard recognise this host's session process, so the block above prints a
remedy rather than sending you to another tree. ⚠ That sentence is about `weak` and **NOT** about
`self-calibrated`, whose subject is stated rather than inherited for exactly this reason: a
calibrated CLEAR is rc **0**, carries no `scope=` field at all, and reached its value BECAUSE the
guard recognised this host's session process — so every clause of it would be false of that value,
and a reader applying it literally would stop on the one outcome CH-12573 exists to let them
continue past. §2's own block is the operative test, on its `grep -Fq 'predicate=weak'` line, and it
matches neither `self-calibrated` nor `strong` — it is a FIXED-string grep, so a line reading
`predicate=self-calibrated name=<x>` does not contain the string `predicate=weak` and the fence does
not fire. (Named by its section and its literal rather than by a direction: the sentence this
replaced said "the fence below" about a line that is above it, which is the cross-reference class
CH-12714 exists to stop repeating.)

If the worktree already exists (a revise, a resumed item), the `[ -d "$WT" ]` short-circuit reuses
it — reuse is the common case, not the exception — and **that is exactly why the block's post-`cd`
checks exist** — the ones asking whether this is a working tree, whether it is on the branch this
item names, and whether **`origin/main` is an ancestor of it**: the reuse path runs no
`worktree add`, so nothing has based the tree on `origin/main`.
⚠ **That third check is the one a reader inverts, so it is written out rather than glossed.**
`git merge-base --is-ancestor A B` asks whether A is an ancestor of B; the block spells it
`--is-ancestor origin/main HEAD`, and the `||` fires on any non-zero rc. Read the warning as
*nothing has based this tree on current main*. It says nothing either way about whether the branch
is CONTAINED in main — the two conditions are incomparable, and a fresh tree sitting exactly at
`origin/main` is contained and silent while a diverged one warns and is not.
`git merge-base --is-ancestor origin/main HEAD; echo $?` settles it for any tree in one command,
and an rc of **128** there is an unresolvable ref rather than a stale tree.
Without them a stale tree on the wrong branch (or a directory that is not a checkout at all) reads
CLEAR and the whole item is built on it. The guard itself answers rc 2 `not-a-working-tree` for the
last case, but the branch and the staleness are the caller's to check.

The check asks whether another live SESSION's cwd is inside that working tree — resolved pid plus
resolved cwd (`/proc` on Linux, `lsof` on Darwin), never a `pgrep -f` on a command line, and never a
path substring. It does **not** count worktrees: the expression it replaced
(`git worktree list --porcelain | grep -c '^worktree '` compared to 1) read **39** on DGX2 and
refused unconditionally, so it was routinely stepped over and protected nothing (HZ-12006).

## 3. Every phase in the body, tests red first

For each behaviour you add: write the test, see it FAIL, implement, see it pass. For each
assertion you rely on: mutate the property in a scratch copy, occurrence-count the mutation 1→0,
observe the RED, restore — and name the red test in the proposal (`red-evidence:`).

⚠ **NAME THE LAYER THE RED WAS OBSERVED AT, not just the test.** CH-12921: `PROP-6601` shipped a
guard that **never fired**. Its RED-first was real — on the **pure function**. The bug was in the
**call path**, where the store had already been written by a seeding step eleven lines earlier, so
the guard's early-return let every orphaned resume through. *"A test went red somewhere"* is what
review had to work against. Write which layer:

```text
red-evidence: revert the (d0) call site -> `cargo test -p nusy-being --test hz12916_e2e` REDs
              at the PUBLIC ENTRY POINT (not only at the pure fn)
```

⚠ **A guard whose WIRING is pinned only by a lint is not pinned.** Twice in one session a
`value_parser` / a call site was deleted and every test stayed green while clippy's dead-code lint
was the only thing that noticed. Either test the wiring, or make the unguarded state unwritable —
and say which you did.

⚠ **An honest decline is a declaration and it passes the gate**: `red-evidence: n/a — <why no
mutation applies>`. The pre-push refuses **silence**, never a wrong answer.

⚠ **If you are writing a paragraph justifying NOT verifying something, verify it instead.** Every
such paragraph in the CH-12921 session covered a real hole, and one of them was **itself false** —
it claimed a test would need a `pub` widening when the entry point was already `pub`. The declined
test was ~50 lines and found the bug on its first run. **The justification cost more than the check.**

## 4. The floor, in this order

```bash
cargo test -p <touched crates>                                >log 2>&1; rc=$?
cargo clippy -p <touched> --all-targets -- -D warnings        >log 2>&1; rc=$?
cargo fmt --all --check                                       >log 2>&1; rc=$?
cargo check --workspace                                       >log 2>&1; rc=$?
# invariant-change: yes ⇒ the whole workspace's tests, not just the touched crates:
cargo test --workspace --no-fail-fast                         >log 2>&1; rc=$?
cargo run -p nusy-arch-guard-runner                           >log 2>&1; rc=$?   # RED refuses; AMBER passes
cargo metadata --locked --format-version 1 >/dev/null         2>log;  rc=$?   # a new dep ⇒ commit Cargo.lock in the SAME commit
```

## 5. Protected-class check — the classifier first, then the clauses it does not carry yet

**Run the classifier. Do not hand-roll a matcher** — HZ-11894 records two agents doing that on one
day, and both improvisations were silently VACUOUS in the safe-sounding direction (a bare grep
against a stanza format matched nothing for any input; an exact matcher against a conf whose
`paths:` values are globs matched the literal rows and dropped the globbed ones):

```bash
scripts/lsc-class-check.sh origin/main HEAD --explain >log 2>&1; rc=$?   # lsc-class-check:invocation
# rc 1 PROTECTED-CLASS (the matching predicates are named) · rc 0 no predicate matched · rc 2 CANNOT ASSESS
```

It reads `scripts/lsc-class-predicates.conf` and hardcodes no predicate, so the scope lives in one
place. **rc 1 is settled — no judgement, no argument. rc 2 is never a clean tree**; fix the cause.

The trailing `# lsc-class-check:invocation` on the command line is a **machine sentinel, not a
comment for you** — `scripts/tests/test_lsc_class_check.sh` arm 9 credits this file with calling the
classifier only for a marked line inside a fenced `bash` block. Keep it if you re-spell the command;
drop it and that arm reds by name. (Prose about the classifier must NOT carry it.)

⚠ **rc 0 does NOT finish this step.** The conf is a measured SUBSET of floor row 2 (its header
carries the per-axis table). Row 2's clauses the classifier does NOT yet carry, which you still
apply by hand on rc 0:

- a **safety crate** (or any other safety path) — ANY path a row of `scripts/safety-paths.conf` matches; read the conf, never a
  copy of it. The copy that stood here named six paths and omitted `nusy-cortex`, `nusy-dream`,
  `nusy-genome`, `nusy-leib-burn`, `nusy-safety` and most Layer-B floor files (measured 2026-09-24,
  EX-8082). Plus `crates/nusy-si-resident`, which the conf does not carry yet. Nothing classifies these
- a **skill** — `.claude/skills/**`
- **CI** — `.github/workflows/**`
- **deleting a Rust test** — `git diff origin/main...HEAD -- 'crates/**/*.rs' | grep -c '^-#\[test\]'`
  (`> 0` ⇒ protected)
- **`Cargo.lock`** beyond a member-set change — the `cargo_lock_member_set_change` predicate is
  `set-diff`, so a pure version bump does not trip it

Converging the conf up to the whole of row 2 is HZ-12103; until it lands this list is load-bearing.

⚠ **`change-intent: n/a` is for a proposal protected by NEITHER route — never for this one.**
HZ-12227 made `n/a` / `none` a first-class DECLINE with its own rc, because four of the twelve
failures it measured were authors declining honestly and the reader having no token for it. It is
NOT a way to satisfy the citation a protected-class diff owes: the classifier above is what says
whether one is owed, and if it said rc 1, or any hand-applied clause matched, write the citation.
`approveit` §5's fence asks the same two questions after the merge: a decline on a protected diff
records `change_intent_reread --rc 1 --reason declined` and refuses the close, and a decline whose
owed-ness it cannot determine records `cannot_assess` and refuses too.

Protected by EITHER route ⇒ the pair review in step 7 runs on a **strong-tier** sub-agent, the proposal carries a
`change-intent:` citation — `manifest:<heading>` / `canon:<file>#<needle>` / `item:<ID>` /
`ruledBy:<ID>`, and **only those four resolve** (`skill:` does not). Since HZ-12227 the reader also
accepts citations separated by SPACES as well as `;`, and ends the list at the first non-citation
word so a prose tail on the line is read as prose. ⚠ Check what you wrote: `approveit` §5's fence
re-reads every token at transport, and a value that does not parse refuses the CLOSE. And the
ruling sub-classes (an `exception`-class manifest change, an asserted canon retirement, a FOSS
disposition row) carry `ruledBy:<SG>` **as an edge** (`nk update <ID> --relate ruledBy:<SG>`).

⚠ **CITATION TOKENS AND NOTHING ELSE ON THAT LINE.** The value is `;`-separated and EVERY element is
parsed as a citation — there is no slot for a rationale, and an unrecognised form is ITSELF a re-read
failure (CH-7020). A rationale appended after the heading makes the needle *heading + rationale*,
which is not in canon, so a TRUE citation re-reads as `canon_not_found`; moved after a `;` it becomes
`unknown_form` instead. Both were measured on real proposals. **Put the citations on the line and the
rationale in prose below the declaration block.**

⚠ **And the re-reader is a LIBRARY: SOURCE it, never run it** (HZ-12370). `bash
scripts/lib/change-intent.sh` used to define the functions and exit **0 with no arguments at all**,
having checked nothing — floor row 2's `change_intent_reread` was recorded from exactly that on three
landed proposals, all of which were really rc 1. It now refuses with rc 2 when executed. Re-read
**every** token, not the first: a value whose first token verifies and whose second does not reads as
clean if you stop at one.

## 6. Self-review

`git diff origin/main...HEAD` read against the body, phase by phase. Every phase addressed or its
omission explained in the proposal. `git status --porcelain research/shared beings knowledge`
must be empty of accidents.

⚠ **STAGE BY PATH, AND READ `git diff --cached` BEFORE YOU COMMIT.** `git add -A` is the wrong verb
in a tree another process may be writing into, and the fold-in round is the round most exposed to
one: `campaign-watcher` §7.2 sends a `--request-changes` round back through this skill, and the
reviewer's own trees can still be live on the host while it runs. `reviewit` §3 carries the measured
incident (PROP-6427 round 24) and the reviewer's half of the fix; **this is the author's half, and it
is the half that turns a foreign write into a commit.** One write-up, cited from both sides — the
duplication this fleet keeps paying for is a rationale copied per site.

```bash
git add <the paths this round touches>       # named paths — not -A, not .
git diff --cached                            # READ it: this IS the commit, not a summary of one
git commit -m "<message>"
```

⚠ **Why the rule is HERE and not in `reviewit`:** `git add` is the AUTHOR's verb and this is the
author's own procedure, the file a fold-in author re-enters. `reviewit` §3's fold-in clause REPORTS
the finding rather than staging it, and `reviewit` §4's revise loop names the fixing session but
hands it back to this file rather than carrying the verb.

## 7. Sub-agent pair review — diff and declarations only

Spawn ONE fresh sub-agent with: the three-dot diff, your declarations, the item's phases, the
invariants the diff touches. **Never your reasoning.** It returns BLOCKING / AMEND / NOTE with
`findings-disposition:`. Fix BLOCKING and AMEND here; NOTE goes into the proposal body.

## 8. Push and propose

```bash
git push -u origin HEAD                              # the pre-push is the floor's refusal half
```

**The next block is ONE Bash tool call — paste it whole, placeholders filled, and do not split it.**
Every Bash call starts a fresh shell, so a variable set in one call is empty in the next. The proposal
id does not exist until `nk pr create` returns, so the block READS it from the create's own output
rather than asking you to type it into a second call — where `$BRANCH` would already be gone and the
re-read would refuse a CORRECT proposal (PROP-6451 round 1, measured).

```bash
# ⚠ THE BODY FILE IS ITEM-SCOPED, AND IT IS VERIFIED BEFORE IT IS PASSED. See below.
BODY="<your session's scratchpad dir>/<id-lowercased>-prop-body.md"   # hz-12459-prop-body.md, NEVER prop-body.md
                                                     # ⚠ that is a PLACEHOLDER like every other <…> here:
                                                     # substitute the scratchpad path your own session prompt
                                                     # names. There is no $SCRATCH in an agent's environment.
CREATED="<your session's scratchpad dir>/<id-lowercased>-prop-create.out"
VIEW="<your session's scratchpad dir>/<id-lowercased>-prop-view.json"
BRANCH="<type>/<id>-<slug>"                          # the LITERAL branch §2 created and the push above sent
grep -q '<ID>' "$BODY" || { echo "$BODY does not name <ID> — it is not yours"; exit 1; }
nk pr create --title "<ID>: <title>" --base main --source-branch "$BRANCH" --body-file "$BODY" >"$CREATED" ||   # never --author; ALWAYS --source-branch
    { cat "$CREATED"; echo "pr create failed — a timed-out write may still have recorded: check nk pr list before creating again"; exit 1; }
cat "$CREATED"
PROP="$(python3 -c 'import re,sys; m=re.search(r"\"id\": *\"(PROP-[0-9]+)\"", open(sys.argv[1]).read()); print(m.group(1) if m else "")' "$CREATED")"
[ -n "$PROP" ] || { echo "no PROP id in $CREATED — read it; do NOT create again before checking nk pr list"; exit 1; }
nk pr view "$PROP" >"$VIEW" || exit 1               # RE-READ what the store recorded; see below
python3 -c 'import json,sys; sys.exit(json.load(open(sys.argv[1])).get("source_branch") != sys.argv[2])' "$VIEW" "$BRANCH" ||
    { echo "$PROP stored source_branch is not $BRANCH — see the repair below; do not move <ID>"; exit 1; }
nk move <ID> review
```

⚠ **`--source-branch` IS NOT OPTIONAL HERE, whatever the CLI's help says (CH-12583).** Without the
flag, `nk pr create` fills the field by running git in the CALLER'S CWD — and a cwd is shell state
that does not survive a tool call: every Bash call starts in a fresh shell, and an agent's is reset to
the project root. **What the client does with that cwd (HZ-12587, CH-12594), for a same-repo create —
no `--repo`, or a `--repo` naming this repo in any spelling (`nusy-product-team`, `owner/name`, a
remote URL, any case), which the client now treats as same-repo instead of detecting from the shared
checkout:** a cwd outside any checkout, or on a detached head, is REFUSED with
`[SOURCE_BRANCH_UNRESOLVED]`; a cwd on the `--base` branch (the shared checkout on `main`) is REFUSED
with `[SOURCE_BRANCH_IS_BASE]`; an explicit `unknown` or `HEAD`, in any case, is refused too. Nothing is
sent on a refusal. **Why the flag is still mandatory:** a cwd on a DIFFERENT real branch — the
resident's tree, a sibling's — cannot be told apart from the right one client-side, so the client
records THAT branch and only NAMES it on stderr (`recorded source_branch=<b> (DETECTED …)`), a note
nothing in §8's own `nk pr create` fence reads: **no call in that fence captures stderr** — the two
that redirect at all capture stdout only (`>"$CREATED"`, `>"$VIEW"`), and `/usr/bin/grep -cF '2>'`
over the fence answers **0** — so the note goes to the operator's terminal and into no variable. And the refusals live in the CLIENT only: a binary built before
HZ-12587, or anything that speaks the wire directly, still stores `unknown` silently, because the
server stores the value it is sent and defaults a missing one to `unknown`. **Measured on DGX2 on 2026-09-16, before HZ-12587:** the
CH-12581 work sub-agent ran `pr create` from `/tmp`, PROP-6449 was stored with `unknown`, and it had to
be closed and re-created as PROP-6450. Every consumer downstream keys on that field — `approveit` §1's
fetch, §3's compose, §6's branch delete, and `campaign-watcher` §5's stranded-review join — so a
proposal carrying `unknown` or the wrong branch cannot be transported correctly, and it reads normal in
`nk pr list`. So `BRANCH` is the LITERAL name from §2, typed, never `$(git symbolic-ref …)` (which
answers about whichever tree the call happens to stand in), and the re-read REFUSES unless the store
holds exactly that name. A refused create prints the refusal and stores nothing: fix the flag and
create again. The repair, when the RE-READ refuses (possible only from a stale client or a wrong
detected branch): if the stored value is `unknown` or equals the target branch, `nk pr edit <PROP>
--source-branch <branch>` — the server allows it only from the author's machine and only before the
proposal is approved, merged or closed, so run the check now, not at transport. For a stored DIFFERENT
real branch the server refuses that repair; close the proposal and create it again with the flag.

⚠ **THE SCRATCHPAD IS SHARED AND NOTHING GUARDS IT. This is the WORKTREE rule's missing twin.**
§2 makes you take your own worktree and `session-exclusivity-check.sh` refuses if another session is
in it. There is **no equivalent for the scratch directory**: every sub-agent on a host writes into
ONE directory, and on Mini on 2026-09-15 that was **nine concurrent sessions and 318 files**. A
generic filename is therefore a shared mutable global.

**Measured the same day: a sibling staged its proposal at the generic path `prop-body.md`, a
concurrent session wrote its own body to that path, and `nk pr create` recorded the WRONG BODY under
the first agent's title and branch.** It was caught only because that agent re-read what it had
created. That is HZ-12035's class — a clobbered file reads as a valid result, never as an error — and
it is what the inside-out loop's parallelism does when it outruns its isolation.

So, three rules, and the third is the one that actually saves you:

1. **Name every staged file after the ITEM** — body, `--reason-file`, logs, temp files. `<id>-prop-body.md`,
   not `prop-body.md`. This makes a collision unlikely.
2. **Verify the file is yours before you pass it** — it must name the item id. This makes a
   collision visible.
3. **RE-READ what the store recorded, after `nk pr create`.** This is the only step that catches a
   clobber that happened between writing the file and reading it. ⚠ **Field-name trap: `nk pr view`
   returns the body under `description`, NOT `body`** (`nk show <ID> --format json` uses `body`). A
   verifier that reads `body` off `pr view` gets `None` and reports "empty" for a perfectly good
   proposal — measured, on the same day, by the agent writing the verifier. Repair with `nk pr edit`
   submitting the WHOLE corrected body.

Body declarations, each on its own line: `class:` (in-plane|seam|admission|exception) ·
`invariant-change: yes|no` · `red-evidence:` · `figure-provenance:` (a headline number's committed
producer, or `n/a`) · `completes-item: yes|no` (no ⇒ the remainder item id) · `change-intent:`
(protected-class only) · `reviewed-by:` (left for the reviewer).

⚠ **And `merge-subject:` AS THE BODY'S FIRST LINE whenever the title will not still be true when this
lands (HZ-12537).** `nk pr` has no title-edit verb — the title you type at `nk pr create` is frozen —
and `approveit` §3 composes the merge subject from it unless the body's FIRST line declares a
replacement. A multi-round review that MOVES the conclusion therefore lands a permanently wrong
subject: `bcc1921f1c` says a measure settled and the artifact it landed says it did not. Write the
line when you create the proposal, and REWRITE it (`nk pr edit`, whole body) whenever a review round
changes what the work concludes. Only the first line is read, and only the plain `merge-subject:`
spelling — everything else falls back to the title, which is why this is a line you write, not a
convention approveit can infer.

## 9. The proposal is a waypoint, not the finish line

⚠ **BEFORE `nk pr create`, sweep for claims wider than what you measured — the WHOLE of every file
this branch touches, NOT the lines it adds.** An added-lines population (`git diff | grep '^+'`)
cannot contain the pre-existing sentence your new text contradicts, which is the case that matters.
PROP-6551's body records its added-lines sweep reporting 12 hits and no over-claim while six stood in
the two files that same diff was rewriting — and **four of the six rendered under
`cargo doc --features target-on-target`**, re-derived at `703dfc2bf3^1` rather than carried
(CH-12707). ⚠ Not under a bare `cargo doc`: all four sit in a module the crate gates behind that
feature, and its `default = []`, so the unflagged build renders NONE of the six. **Build the docs at
the flags the code is actually gated at, or the rendering evidence is a measurement of nothing.**

**The population is every line that ASSERTS something to a reader**, in every file this branch
touches: comment lines, prose, and the user-facing strings in `error`/`panic!`/`assert!` (this same
crate carries `#[error("counters are not bound to this gate instance: …")]` — the negated-possessive
form, in a string a CALLER sees, and a comment-only population never looks at it). **It is not
Rust-shaped, and a Rust-shaped population is how this step goes silently vacuous** — a markdown,
shell or python branch swept with the first line below reads 0 of 0 and reports clean (measured: that
line returns 0 on this very file, on `scripts/hooks/pre-push` and on `scripts/provenance-guard.py`):

```text
grep -nE '^[[:space:]]*(///|//!|//)' <file>    # Rust
grep -nE '^[[:space:]]*#'            <file>    # shell · toml
cat -n                               <file>    # markdown and other prose: the WHOLE file
python3 -c 'import sys,tokenize;p=sys.argv[1];L=open(p).readlines();T=tuple(x for x in (tokenize.COMMENT,tokenize.STRING,getattr(tokenize,"FSTRING_START",None),getattr(tokenize,"FSTRING_MIDDLE",None),getattr(tokenize,"FSTRING_END",None)) if x is not None);N={n for t in tokenize.tokenize(open(p,"rb").readline) if t.type in T for n in range(t.start[0],t.end[0]+1)};[print(f"{n}:{L[n-1]}",end="") for n in sorted(N)]' <file>   # python
```

⚠ **TWO of those four forms do not reach the string half of the population stated above, and that
paragraph must not be read as though they all do.** ⚠ **Count the FENCE, not the languages** — it has
four LINES, one of which (`grep -nE '^[[:space:]]*#'`) is a single expression covering shell AND toml;
CH-12742 miscounted this fence twice, once in each polarity, so the count is spelled out here rather
than left to a reader. Only the python form SELECTS the string half — every string literal in the
file, so `raise`/`error` text is in its output — and `cat -n` gets it only by taking the whole file
and selecting nothing. The two that deliver **none** of it are the **Rust form** and the
**shell/toml form**, both comment-anchored: measured below at 93 lines of `pre-push`, 363 lines of the
manifest, and an unbounded class in Rust, which is why no recall figure for the Rust population FORM
is quoted anywhere here.

⚠ **Python gets its own form because `^[[:space:]]*#` is not a population there — it is a MINORITY
SAMPLE, and reporting recall against it reads clean (CH-12742).** Python's comment idiom is the
DOCSTRING, which starts with a quote, and the `#` form cannot see one line of it. Measured today at
this branch's base (`522567e28e`) on `scripts/provenance-guard.py`, 2,153 lines. The denominator is
**1252** lines in two halves: **875** drawn by `ast` — a syntax TREE — as every line spanned by a
string-valued `Constant` or an f-string (`JoinedStr`), UNION **392** drawn by `tokenize` COMMENT.
⚠ **That second half is 31.3% of the denominator and it is NOT independent of the form below it** —
it is that form's own first token type. `ast` discards comments, so the comment half has nowhere else
to come from; the honest move is to say so, not to call the denominator a syntax tree and hope.

```text
grep -cE '^[[:space:]]*#' scripts/provenance-guard.py   #  363   the shell/toml form       29.0%
python3 -c '<a (COMMENT, STRING) tuple>'  … | wc -l     # 1201   the NARROW tuple, 3.12+   95.9%
python3 -c '<the guarded python line above>' … | wc -l  # 1252   the shipped form    — see below
```

⇒ the `#` form reads **363 of 1252 = 29.0%**: 414 docstring lines and 29 trailing-`#` lines are
invisible to it (443 missed of the 363 + 29 + 414 = **806** comment-or-docstring lines alone).
`conftest.py` has the same shape — **21 of 120 = 17.5%** — so the shortfall is not a property of one
file.

⚠ **The three `FSTRING_*` token types in that one-liner are load-bearing on python 3.12+, and the
`getattr` guard is what keeps the same line working below it.** PEP 701 made an f-string tokenize as
`FSTRING_START` / `FSTRING_MIDDLE` / `FSTRING_END`, and **none of those is `STRING`** — so a plain
`(COMMENT, STRING)` tuple drops f-string CONTENT there, with no signal. On this file that is **51
lines**, and they are verbatim the guard's user-facing refusal text (`provenance.git_sha not a valid
hex sha…`, `provenance sidecar … is not a JSON`, `[provenance-guard] CANNOT ASSESS — …`); **three of
the 51 are selected by the word list below**, i.e. lines this section's own reading ORDER puts first,
sitting in a population the form never produced. Measured on three interpreters, same file:

```text
                  FSTRING_* exist   (COMMENT, STRING)   + the three FSTRING_*
python 3.11.16          no                1252                  1252
python 3.12.3           yes               1201                  1252
python 3.13.15          yes               1201                  1252
```

Below 3.12 an f-string tokenizes as a `STRING`, so the narrow tuple was right there and wrong above —
its output was a property of the INTERPRETER, not of the file. The guarded tuple reads the same on all
three (the constants are simply absent below 3.12 and filter out). **A token-type allowlist carries a
standing blind spot of exactly this shape: one release added three token types that carry text, and
the next release can do it again.**

⚠ **The shipped form reads 1252 of 1252 on that denominator, and THAT IS NOT A RECALL MEASUREMENT OF
IT — nothing on this page measures THAT form's recall.** The denominator is a SUBSET of its own
output on **652 of 652** tracked `*.py` in this repo (IDENTICAL on 633 of them), and the cause is the
grammar rather than the corpus: a string-valued `Constant` is built from the very `STRING` tokens the
tuple names, a `JoinedStr` from the `FSTRING_*` tokens it names, and the denominator's third clause
literally IS `tokenize.COMMENT`, the tuple's own first element. **No denominator drawn from another
parse of the same file can hold a miss of this form**, so 100.0% is what it must report whatever the
form misses — the defect the ⚠ paragraph that adds a FOURTH thing you owe names. This item's first
round shipped it as a *comment ∪ docstring* denominator, and **widening the denominator did not cure
it**: it moved. Measuring this form's recall needs a denominator that is not a parse of the file at
all — a human-labelled sample of its assertion-bearing lines is the cheap one — and nobody has drawn
one. **So the shipped form ships with NO recall figure, and the two figures above that ARE
measurements are the other two:**

- **29.0%** — a `grep` numerator against a parser denominator. Two expressions that genuinely can
  disagree, and here disagree by 889 lines.
- **95.9%** — a measurement of the NARROW tuple, which is a different expression from the denominator
  and which the denominator caught out: it held **51** lines that tuple did not produce, and named
  them.

Both denominators still enumerate the same KIND of thing, literal text in this file, so neither
reaches a message the script assembles at runtime or reads out of a data file.

**The other three forms are stated here rather than assumed — and the two `#` figures are
WITHIN-FILTER, so they are not recall.** `scripts/hooks/pre-push` selects 980 of the 990 lines
bearing a `#`; `architecture-manifest.toml`, 299 of 303. Both denominators are the same expression
with its anchor removed, which is exactly what the paragraph below forbids. The class outside both:
**93** lines of `pre-push` carry `echo`/`printf`/`fail`/`warn` text and no `#` at all, and **363**
non-comment lines of the manifest carry a `= "…"` value — user-facing strings, which this section's
population includes. `cat -n` on markdown is 100% by construction and says nothing: its denominator
IS the file. **The python 1252-of-1252 above is the SAME nothing by a different route** — there the
denominator is a subset of the FORM's output rather than equal to the file — which is why it is not
quoted as a recall figure either. The measurement in that paragraph is the 95.9%.

⚠ **The Rust form is the one whose blind spot is NOT bounded here, and that is stated rather than
papered over.** On `crates/nusy-si-adoption-gate/src/compose.rs` it selects 446 and misses 2
trailing-`//` lines; `grep -vE '^[[:space:]]*(///|//!|//)' <file> | grep -c '"'` finds 120 more lines
carrying a string literal. **That 120 is a SAMPLE of the blind spot, not a bound on it.** The
CONTINUATION lines of a `\`-continued or multi-line string carry no quote of their own, so they fall
outside both counts: in `compose.rs` the `ComposeError` message whose first line ends `counters are \`
(one occurrence — `grep -n 'counters are \\$'` finds it) has three such continuation lines, and they
carry `bound to` once and `never` twice inside text a caller reads — the same sentence family as the
`#[error("counters are not bound to this gate instance: …")]` case this section names in its
population paragraph, and two word-list branches sitting where no form here looks. Nor does anything
here separate the assertions from the paths and identifiers. **So for a Rust file's POPULATION there
is no recall figure to quote: read the file.** (The 874 / 163 / 40 / 57 figures further down are a
different fraction — the WORD LIST's recall, over the comment lines the form did select.)

**The tokenize form's own blind spots come in a LOUD kind and a QUIET kind, and the quiet kind is the
dangerous half.** The loud kind is a file that does not parse: it exits non-zero with a traceback
rather than printing nothing, so it can never be read as a clean sweep the way an empty `grep` can —
and it is not fragile in practice, all **652** tracked `*.py` tokenizing at this base (652 tried, 0
failures). The quiet kind is the token-type allowlist measured above: text carried by a type the
tuple does not name is dropped with no signal at all, which is how the f-string class survived a
whole round of review. Those two do not exhaust it — nothing here separates an assertion from a path,
an identifier or a `# noqa`, and no form on this page sees a message assembled at runtime or read out
of a data file.

⚠ **So the FOURTH thing you owe, in the same sentence that reports recall, is what your form CANNOT
SEE.** A recall figure whose denominator was drawn by the same expression that drew the numerator is
100% by construction and measures nothing — and so is one whose denominator that expression must
CONTAIN. The denominator has to come from somewhere the filter cannot reach, and ⚠ **a different
PARSER is not automatically such a place: the worked example above is the counterexample.** `ast` is
a different parser from `tokenize`, and its denominator still cannot hold a miss of the widened
tuple, because both are parses of the same file and the tuple's token types are exactly what `ast`'s
string nodes are BUILT from. The asymmetry is the whole lesson — that SAME denominator IS outside the
`#` grep's reach, so it measures THAT form at 29.0% while measuring the tokenize form at nothing.
⚠ **So the test is not "did I use a different library" — `ast` and `tokenize` ARE different libraries
— it is: NAME THE INPUT on which your two expressions would give different answers, and then show one
EXISTS by showing the denominator DID hold a miss.** The 95.9% earns its keep that way: the `ast`
denominator held 51 lines the narrow tuple did not produce, and named them. Against the SHIPPED tuple
the same denominator has never once disagreed — 652 of 652 files, by grammar rather than by luck — so
for that form it reports nothing, and §9 quotes no recall figure for it. **A denominator that has
never disagreed with its numerator has not been tested; one that CANNOT disagree is not a
denominator.** If you cannot name the disagreeing input you do not have a measurement, and the
library question will hide that from you and from your reviewer — CH-12742 shipped a circular
denominator TWICE, the second time after widening the first one, because it asked the library
question. What qualifies instead: a hand-labelled sample, or an honest sentence naming the class you
did not enumerate.

Read that population, **starting with** the lines the alternation below selects. It is a reading
ORDER over the population, never the population itself — kept on ONE line so it can be copied and so
no branch of it is lost to a wrap:

```text
clos|cover|every|all |whole|ensures|guarantees|complete|never|always|cannot|refuses|is not|are not|belongs to|bound to|no other|nothing|used to|can no longer
```

⚠ **A word list's blind spot is invisible from inside it, so report the filter's RECALL and not only
its hits (CH-12728).** The eight branches from `is not` onward were added after this list missed
`counters that are not THIS GATE'S` — the form of all three over-claims left standing in the two
files PROP-6551 had just corrected. Measured over `crates/nusy-si-adoption-gate/src/{compose,decision}.rs`
at `703dfc2bf3`, with the commands written out because a figure no reader can re-derive is exactly
what this paragraph exists to stop:

```text
P='^[[:space:]]*(///|//!|//)'; T='counter|adopt|histor|instance|compan|composition'   # T = THIS change's subject
grep -cE "$P" <file>                                            # 874  the comment population
grep -E  "$P" <file> | grep -icE "$T"                           # 163  on the change's subject
grep -E  "$P" <file> | grep -iE '<the pre-CH-12728 list>' | grep -icE "$T"   # 40  selected before
grep -E  "$P" <file> | grep -iE '<the list above>'       | grep -icE "$T"   # 57  selected now
```

(summed over the two files; **the `-i` is load-bearing** — drop both and the same commands give 135
and 41.) All three claim-bearing lines were missed by the old list and are found by the new one. The
old list did select `compose.rs:316`, the literal continuation of one of those sentences, on
`cannot`, and `compose.rs:150`, a later line of the same doc comment, on `whole` — while selecting
nothing anywhere in the `decision.rs` comment. **That is the sharper lesson: it matched part of a
sentence and the part carrying the claim was still never read.**

⚠ **And this is the direction the next author must NOT strengthen this step in: do not add a
twenty-first branch to that list.** Ten same-class over-claims, written today against the list above
and the `$T` of the same change (`belonging to` defeats `belongs to`, `aren't` defeats `are not`,
`rejected` defeats `refuses`):

```text
# the ten, in a scratch file, one per line. They carry NO leading comment marker on purpose: prefix
# them with // and THIS file becomes a hit of the Rust form that this section measures at 0 on it —
# the PROP-6511 self-invalidation class, committed in the paragraphs above the one that names it.
the counters belonging to this gate are the only ones reported
ids that aren't adopted are dropped before the ledger sees them
a candidate the gate rejected leaves no trace in the adoption ledger
each adoption is recorded exactly once
the composition holds for any counter the gate sees
history is retained until the next rotation
no counter escapes the instance check
the companion id isn't derivable from the hash
adoption decisions are final once the ledger flushes
the instance binding survives a restart

grep -icE '<the list above>' <that file>   #  0 of 10   the word list
grep -icE "$T"               <that file>   # 10 of 10   the subject filter  ($T as defined above)
```

Natural language has no finite blocklist, so a miss here is evidence for a WIDER POPULATION and a
franker recall report — never for another branch. If the list ever grows again, the figure that
justifies it is a recall number against a subject-matter subset, not a count of new hits.

**State THREE numbers in your disposition, with the method**: the population, the subset naming the
change's own subject, and the subset the filter selected. ⚠ **The MIDDLE one — the subset naming the
change's own subject — is the one this lesson turns on.** The other two hide the recall gap between
them: a population and a hit count are consistent with any recall at all, and it is the subject-matter
subset that says how much the filter missed. So where the change's subject is a CLAIM about what the
code refuses or covers, the subject-matter subset is what you must READ, line by line; the filter's
output only ORDERS that reading. Check each line you read against what the code actually does.

⚠ **And a FOURTH thing, in the same sentence — what your POPULATION FORM could not see.** It is the
obligation the forms paragraph above states, and it is repeated here because this is where a
disposition is written and that paragraph is a long way back. Three numbers with no blind spot reads
complete and is not: on a Rust file the form silently drops a multi-line error string's continuation
lines; on python a `#`-anchored form drops every docstring, and a token-type allowlist drops whatever
token type the interpreter adds next — CH-12742 measured both, and closing the second one did not
retire the class. Name the class you could not enumerate. `n/a` is not available — every form above
has one.

This is the AUTHOR's job because it is cheap here and expensive later: a claim narrowed at this step
costs a sentence, and the same claim found in review costs a fresh tip, a fresh reviewer session and a
full gate re-run. Measured on PROP-6526, the author's own sweep ran over 121 hits and
caught **seven** over-claims before submission, three of them: "every candidate a gate EVALUATES" →
"ADOPTS"; "such an id can never be reported"; "the held-out hash changes at every rotation". An eighth
was caught before the commit.

⚠ **A claim true in a narrower scope, written generally, is this codebase's most-repeated defect.** Its
sharpest measured form: a comment asserting `git grep X` had no match, which BECAME one of X's own hits
the moment it was written (PROP-6511). **If a sentence's truth depends on the state of the file
carrying it, rewrite it.**

**`nk pr create` is where THIS skill ends, not where the work ends.** `workit` is a SUB-STEP: the
resident loop (`campaign-watcher` §7) spawns it, awaits it, and then carries the same item the rest
of the way — take the review slot (`nk pr claim-review`), spawn the reviewer as a DISTINCT session
(`NUSY_SESSION_ID=sub-<PROP> claude --dangerously-skip-permissions -p …`, so `reviewer ≠ author`
holds by its own id; the flag is what lets the child actually run the gates it owes, and
`campaign-watcher` §7.2 says why it is load-bearing — HZ-12032), fix what
comes back, and transport the approved proposal inline via `approveit`. The store applies no identity
bar at merge: the author may transport its own approved proposal. Nothing is routed to another agent
or another machine, and nothing waits for a dispatcher — Layer A is deleted.

So: if you were spawned BY the loop to run this skill, returning at step 8 is correct — report the
proposal id to your caller. **If you ARE the resident session and you invoked this skill yourself,
step 8 is one third of one iteration.** Do not end the turn to announce the proposal; go to
`campaign-watcher` §7.2 and keep going. A session that creates a proposal and stops has moved a
kanban row, not landed a change — and it is the single most common way this loop dies.
