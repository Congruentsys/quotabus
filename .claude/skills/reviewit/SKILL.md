---
name: reviewit
description: Independent review of a proposal — trial merge in a scratch worktree, the floor there, then a fresh sub-agent's judgment recorded under its own session id. Re-cut at VY-11604 Commit B (CH-11826). Closed to the Skill tool by design (CH-11917); the spawned reviewer REACHES it by opening this file and following it — campaign-watcher section 7.2.
disable-model-invocation: true
argument-hint: "PROP-XXXX"
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/reviewit/SKILL.md` on
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

# Review It — the judgment is the sub-agent's

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

`alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'`. Every rc UNPIPED. **Reviewer ≠ author
is not waived:** the session that produced the work never approves it; the approving act is a
fresh sub-agent's, under its own session id.

## How this skill is REACHED — the Skill tool is closed on purpose (CH-11917)

`disable-model-invocation: true` is deliberate and **stays**. The harness refusal it produces ends
*"Do not replicate this skill's workflow by other means — it is reserved for explicit user
invocation."* (Captured on DGX1, 2026-09-12. ⚠ The wording is the harness's and may differ — the
skill may simply be ABSENT from the model's surface instead; no gate matches on this string.)
⚠ **That clause is scoped to the Skill tool and does NOT bar the loop.** The sanctioned
reach is `campaign-watcher` section 7.2: the resident takes the review slot, then spawns a distinct
session — `NUSY_SESSION_ID=sub-<PROP> claude --dangerously-skip-permissions -p "<NL prompt>"` — whose
prompt **instructs the child to open this file and perform the procedure**, never a bare `/reviewit`
(a bare one carries no PROP id, no tip sha and no refute mandate — CH-11942). A `-p` prompt is a USER
turn, so the flag never reaches that child in any case (measured under CH-11917).

SG-11818 (*"just use a sub agent to review anything"*) is about WHO may review, not about which tool
invokes the procedure; it does not bear on this flag. **Do not flip it.**

⚠ **ASSERT THE PROMPT BEFORE THE SPAWN, AND CONFIRM A CHILD AFTER IT — a misfired spawn produces a
plausible agent, not an error.** When the prompt is assembled from a file, `claude -p "$(cat
<promptfile>)"` with a path that is missing, misspelled or empty passes an EMPTY prompt, and the
child is not guaranteed to fail on it. In the one measured case it inferred a role instead: on Air
2026-09-16 (CH-12579) a reviewer launched that way, with a bare directory path as its only other
argument, received `/Users/hans1995/fleet-wt/ch12470` as its entire first user message, inferred it
was the machine's resident, ran the `campaign-watcher` loop, claimed another item's work, and wrote
into the author's worktree. The spawning session got no error and saw a healthy tmux window
throughout.

```bash
PROMPTFILE="$TMPDIR/rv-PROP-0000-prompt.txt"         # item-scoped, like every staged file
[ -s "$PROMPTFILE" ] || exit 3                       # absent or empty ⇒ do NOT spawn
NUSY_SESSION_ID=sub-<PROP> claude --dangerously-skip-permissions -p "$(cat "$PROMPTFILE")" < /dev/null
```

⚠ **All three lines in ONE Bash call.** Every call starts a fresh shell, so an `exit 3` in a previous
call refuses nothing — `workit` §8 makes the same demand of its create block for the same reason. And
if the child needs a working directory, say so IN the prompt text or `cd` into it on the launch line;
never hand it a bare positional, which is exactly how `/Users/hans1995/fleet-wt/ch12470` became that
child's first user message.

Then confirm a child actually exists — and know what that answers. **A pid confirmation says only
that a child EXISTS, not that it got your prompt:** in the incident above a `claude` WAS under that
pane the whole time, so this check would have passed. The `[ -s … ]` fence is what catches a wrong
prompt; this check catches the other half, a launch that produced no child at all, which posts no
verdict and does not say so. A harness-tracked background task (`campaign-watcher` §7.2's preferred
shape) hands back a task id, which confirms the harness ACCEPTED the launch — §7.2's own 9-proposal
measurement is what speaks to survival. A detached launch hands back nothing, so ask the host:
`pgrep -lP <the pid you launched into>` (bare `pgrep -P` prints pids with no names), or for tmux
`tmux list-panes -s -t <session> -F '#{pane_pid}'` — the `-s` is load-bearing, since without it a
session name resolves to that session's CURRENT WINDOW only — and then `pgrep -lP` on the pane pid.
⚠ `-P` matches DIRECT children only, so a launch wrapped in `bash -c`, `env` or `nohup` puts `claude`
one level down and the check answers empty.

## 1. Take the review slot FIRST — the CAS, then read and pin the tip

```bash
nk pr claim-review <PROP>                          >log 2>&1; rc=$?   # rc != 0 ⇒ READ the refusal
nk pr view <PROP>                                   # author, source_branch, declarations, comments
git fetch origin main <branch>
REVIEWED_AT="$(git rev-parse origin/<branch>)"      # the approve binds to THIS sha, not the branch
```

`claim-review` is the store's compare-and-set on the reviewer slot (EX-7624). **A typed refusal
naming another holder means that machine is already reviewing this proposal — go review a different
one.** It is not a retry, and there is no override flag to reach for — the verb takes only
`--reviewer`. A refusal naming a non-claimable STATE
(`open/reviewing` only) means the proposal moved on without you — also: take the next one.

Take it BEFORE section 2, not after. The trial merge and `cargo check --workspace` ARE the cost, so
a race lost after them is an entire review thrown away: measured live on PROP-6227 (CH-11934), two
residents each ran a full strong-tier review of the same proposal and the store only refused at the
very end — `nk pr review <PROP> --approve` rc 1, "Invalid transition from merged to approved". The
store's state machine was the backstop; nothing in the loop was.

**Claim with the DEFAULT reviewer — never `--reviewer sub-<PROP>`.** The slot is MACHINE-grain and
this verb is the one exception to the session-identity rule in section 4: `bin/nk.rs`
`PrCommand::ClaimReview` sends `reviewer.clone().unwrap_or_else(resolve_agent_name)` — a bare
`DGX2` / `Air` — and `identity::actor_for` prefers that explicit key over the session-composed
envelope actor, so an `NUSY_SESSION_ID=…` prefix does NOT reach it. (Verified on the live board:
held slots read `reviewer='Air'` and `reviewer='DGX1'`.) Machine grain is exactly what this CAS
needs, because `sub-<PROP>` is derived from the PROP id alone and is byte-IDENTICAL on every host —
passing it would make the second host's claim compare equal to the first's and be admitted as an
idempotent self-re-claim, letting both reviewers through and silently defeating the CAS. Conversely,
re-claiming a slot this machine already holds IS idempotent, so a watcher that claimed at its drain
step and this call do not collide.

⚠ **The slot has no release verb and no TTL** (`nk pr --help` lists none; `graph-review-core`'s
`claim_review` has no lease). Every ORDINARY exit frees it: `approve` and `reject` both overwrite
the slot with the verdict's own actor, and `revise` clears it — so finishing your review, in either
direction, releases the hold. It stays HELD when no verdict is ever RECORDED — the reviewer dies, or
its verdict is REFUSED. Machine grain bounds the first case — the holding machine re-claims idempotently and its
next iteration resumes — but ONLY for a machine-grain holder. **Read the holder string the refusal
names.** A bare name (`DGX1`, `Air`) is a peer's live claim: take the next proposal. A holder
containing `/` (`DGX2/sub-PROP-9999`) is a SESSION-grain stamp that no default claim can ever match:
`pr review` writes the slot through `add_reviewer` and DISCARDS its result (`organs/pr.rs`), so a
`--approve` that then fails its unresolved-comment or ambiguity gate leaves `reviewing` + a
session-grain holder — and `revise` cannot clear it, because `revise` is only reachable from
`rejected`. Left alone under "take the next one", such a row is invisible to every resident forever.
If its machine half is YOUR machine, `nk pr claim-review <PROP> --reviewer '<the exact holder
string>'` lets you proceed — but know exactly what that does, because rc 0 here is NOT proof of
ownership: with `claimant == current` the CAS skips its holder-refusal and returns `Ok(())`
**writing nothing** (proposals.rs, the `Reviewing` arm with a non-empty slot). The row still names
the stale string, and any OTHER session on your machine running the default claim is still refused
by it. The slot is only actually rewritten when your verdict records — `approve`/`reject` overwrite
it — so the row stays wedged for everyone else until you finish. If the holder names another
machine, it is not yours to take at all: say so on the item and on HZ-11951 (the release path), and
never force it.

⚠ `--reviewer` accepts ANY string and, on an `open` proposal, `add_reviewer` writes it with only the
author-refusal in front of it. It is a mis-stamp vector, not a general escape hatch: use it ONLY for
the stale-slot case above, with the holder string copied verbatim from the refusal — never to invent
a reviewer, and never to take a claim a peer currently holds.

## 2. Trial merge in a scratch worktree, the floor there

```bash
git worktree add --detach /tmp/rv-<PROP> origin/main
git -C /tmp/rv-<PROP> merge --no-commit origin/<branch>      >log 2>&1; rc=$?   # conflict ⇒ request-changes: "rebase onto main"
( cd /tmp/rv-<PROP> && cargo check --workspace )             >log 2>&1; rc=$?
( cd /tmp/rv-<PROP> && cargo test -p <touched crates> )      >log 2>&1; rc=$?   # workspace-wide if invariant-change: yes
( cd /tmp/rv-<PROP> && cargo clippy --workspace -- -D warnings ) >log 2>&1; rc=$?   # CI's exact form
( cd /tmp/rv-<PROP> && cargo fmt --all --check )             >log 2>&1; rc=$?
MERGED_TREE="$(git -C /tmp/rv-<PROP> write-tree)"
```

⚠ **`cargo clippy -- -D warnings` and `cargo fmt --all --check` belong HERE, and this section is the
enforcement point** (HZ-12459 round-4 AMEND 5). CI runs `clippy --workspace -- -D warnings`, so a
diff that is correct, tested and unformatted still reds after it lands; `check` and `test` both pass
a tree `fmt` refuses. Two things make the reviewer the reader rather than the author: this is the
TRIAL-MERGED tree, where a lint the branch never saw can be introduced by main moving underneath it,
and `-W` is not `-D` — verify on the toolchain CI uses, not on a local default. ⚠ Skip them ONLY when
the diff has no rust delta at all (`git diff --name-only origin/main...origin/<branch> | grep -E
'^crates/|^Cargo\.'` empty), and say so in the verdict rather than leaving the row silent. The
retired `review` skill's duty table routes this pair to `workit` §4 and to THIS section; until
round 4 this file contained the word `clippy` zero times, so the routing resolved to a live file that
did not carry the duty — the orphan-with-one-extra-hop that `scripts/tests/test_hz12459_loop_binding.sh`
arm 13 exists to catch, which arm 13 could not see because it resolved destinations at FILE level.
It now pins this pair by content too.

⚠ **Run every gate with `cd` INSIDE the scratch tree, exactly as the fence spells it** —
`( cd /tmp/rv-<PROP> && … )`, never `bash /tmp/rv-<PROP>/scripts/…` from where you are standing.
`scripts/mini-disk-guard.sh`'s liveness probe is `lsof -d cwd`, so a scratch tree with no process whose CWD is inside it reads IDLE and is reapable at the CRIT band. Measured on Mini,
2026-09-15: EVERY `/tmp/rv-*` and `/tmp/redfirst-*` tree on the host was deleted inside one guard
window while gates were running against them (29 GiB → 77 GiB free); three batteries returned rc 127
and one rc 2 CANNOT ASSESS. **The guard is behaving as designed** — it splits detached trees by band
and at CRIT *"a dead Mini is worse"* — and the invocation form is the whole of what makes it see you.
The fence above already has it right; the risk is someone "simplifying" it to an absolute path.
`approveit` §3 carries the same rule for the transport side. (`workit` is NOT exposed: its tree is
`~/fleet-wt/<id>`, outside the reaped namespace — verified, every `/Users/hankh19/fleet-wt/*` survived
the same window.)

⚠ **Read `git worktree list` FIRST, and key on TREE LIFETIME — not on whether anything is building.**
If another scratch tree is live (a concurrent `/tmp/rv-*` review, a `/tmp/ap-*` transport, another
`~/fleet-wt/sub-*`), either give THIS review its own `CARGO_TARGET_DIR` and reap it on every exit, or
`cargo clean -p <the crates the branch touches>` in the shared dir first. Otherwise the host-shared
warm dir is correct and much cheaper — `approveit` §3 carries the same rule for the transport side.

⚠ **This line used to read *"only when nothing else on the host is building"*, and that is FALSE
(HZ-12365). Serialising builds does NOT make a shared dir sound.** Cargo's fingerprint for a path
package compares the source files' mtimes against the dep-info recorded in the target dir, so a tree
whose files are OLDER than another tree's last build is judged **`Fresh`** — no overlap in time
required. Measured on DGX2, two trees of one crate, one shared dir, builds strictly SEQUENTIAL: the
later-built tree reported `Fresh` and then failed **`error[E0063]: missing field newfield`** on a
field that is not in its own sources (control with its own dir: rc 0, 1 passed). It happened again
UNPLANNED on Mini on 2026-09-15 — a pre-push build refused with `extern location for memchr does not
exist` in the shared dir while `ps` showed **zero** live `cargo`/`rustc`: the other session's tree had
finished, and its LIFETIME still overlapped.

⚠ **A review QUEUED behind a one-build-at-a-time rule is created early and built late — exactly the
failing order** — so a concurrency-phrased condition tells you the dir is sound in the arrangement
serialisation makes most likely. `E0063` (a field MISSING from the tree that is compiling, because a
sibling tree's artifact supplied it) and `E0560` (a field the tree itself defines being reported as
unknown) are both the loud
direction; **the same mechanism yields a false GREEN**, a gate row for a state nothing compiled.

## 3. The sub-agent

Spawn ONE fresh sub-agent and give it exactly: the three-dot diff (`git diff
origin/main...origin/<branch>`), the proposal's declarations, the item's phases (`nk show <ID>`),
the invariants the diff touches, `REVIEWED_AT`, and the path `/tmp/rv-<PROP>`. **Never the
author's reasoning.** It must: re-run the declared `red-evidence:` mutation (occurrence-count
1→0, observe RED, restore); re-run one `figure-provenance:` command and compare the figure; read
any `CLAUDE.md`/`docs/` diff against the supersession table; apply the **empirical-claim
classifier** — does any change assert a measurable claim (coverage, accuracy, latency, "reproduces",
"scales to N")? A claim a measurement could settle must trace to a filed Hypothesis→Measure→Experiment
with on-disk eval JSON under `research/shared/eval-data/`, never a number in the PROP body; an unfiled
one is a finding, and `/hypothesize` is the remedy (CH-12247: this check was in `review`, `workit` and
`docs/research-integration.md` and absent from the one file a REVIEWER reads, so `review`'s citation of
it had nothing to resolve to); return BLOCKING / AMEND / NOTE with
`findings-disposition:` (`fixed=<n>` · `filed=<ID>` · `declined=<n — one clause>` ·
`none-required`). Do not let it end its turn without the verdict.

⚠ **Re-run the `red-evidence:` mutation AT THE LAYER THE DECLARATION NAMES, and if it names none,
ask for one.** CH-12921 records a guard whose author's RED was real on a pure function while the
guard never fired on the call path — a mutation re-run at the wrong layer confirms nothing.

⚠ **If a guard's WIRING is pinned only by a lint, say so.** Deleting a call site and watching every
test stay green while clippy alone notices is not a pinned guard.

⚠ **THE MUTATION RUNS IN YOUR OWN SCRATCH TREE, AT THE SAME RELATIVE PATH.** The `red-evidence:`
mutation above is applied to `/tmp/rv-<PROP>/<the file's path in the repo>` — the trial-merged tree
§2 created — and to nothing else. Both halves of that are load-bearing, and they fail differently.

**Your own tree**, because a mutation left in the tree the AUTHOR is committing from can be swept
into the author's next commit. Measured: PROP-6427's round-24 fold-in commit `122fc63005` carried a
reviewer's live defang of the build-serialisation gate (`&& $2~/rustup/`) to `origin`, re-creating
the failure HZ-12522 exists to fix. Neither session got an error, and both were behaving correctly
in isolation. ⚠ `scripts/fleet/session-exclusivity-check.sh` would not have caught it: it answers
about a live session's CWD, and this reviewer's cwd was elsewhere — it only WROTE into that tree.
(HZ-12571 carries the separate Darwin-probe half.) `workit` §6 carries the AUTHOR's half of the same
incident, the staging verb.

**The same relative path**, because a path-dependent arm — one that resolves a sibling script, a
conf, or its own location in the repo — reds for the PATH rather than for the mutation once the file
has been copied somewhere loose such as `/tmp/x.sh`. The RED you then record as red-evidence is a
real RED and is evidence of nothing: it fires for the mutated copy and for an UNMUTATED copy at the
same loose path alike, so the arm was never shown to discriminate. Running the unmutated copy at
that same loose path is what exposes it; keeping the file where its own path resolves is what removes
the PATH confound this paragraph is about. It does not remove the need for the 1→0-and-restore
control — run that too. Mutating somewhere else is not a cheaper version of this rule.

⚠ **RE-READ THE `change-intent:` CITATIONS YOURSELF, AT THE REVIEWED TIP — before the landing,
nothing else does.** This paragraph is HZ-12399's and it is RE-HOMED here, from the `review` skill
HZ-12459 retired: that stub's duty table sends *"re-reading a `change-intent:` value at the reviewed
tip"* to this section, and on `origin/main` the text was a single-file singleton, so the retirement
would otherwise have deleted the fleet's only copy. It lives HERE now and nowhere else — do not paste
it back into the stub. (The declarations' PRESENCE is a different duty and is `workit` §8's; nothing
has checked presence since CH-11826. What you re-read here is the VALUE.)

`change-intent:`'s value is re-read by `approveit` §5's inline fence (HZ-12370), **but only AFTER the
merge is pushed**: §3 pushes and §5 records, so that re-read is a post-hoc record which can refuse the
CLOSE, never the LANDING. ⚠ Since HZ-12399 a PRE-MERGE half exists too — `scripts/hooks/pre-push`
stage 2c REFUSES the push when the diff is protected class by `scripts/lsc-class-predicates.conf` and
the citation obligation is unmet. **Read its POPULATION before relying on it**: only `Merge PROP-<id>`
MERGE commits pushed to `refs/heads/main`, and only the classes that conf covers. It is a measured
SUBSET of floor row 2 (HZ-12103), so a diff protected only by a safety crate, a skill, CI or a deleted
Rust test reaches main with nothing having re-read its citation — and a DIRECT commit to `main` is
outside the population entirely, including a doc-only one touching `CLAUDE.md`, which that same conf
calls protected. **Your review is the only reader of all of those.** So read them here: source
`scripts/lib/change-intent.sh`, split the value with `change_intent_tokens`, and run
`change_intent_reread_token <token> <sha>` on every token, against the tip you pinned in §1.

⚠ **Since HZ-12227 that reader has FOUR outcomes, not two** — `0` verified · `1` failed · `2` CANNOT
ASSESS (the store could not be asked; an outage is never an absence) · `3` DECLINED (`n/a` / `none` /
`na`). So a refusal that says `NOT FOUND (superseded/absent)` means a RETRACTED citation and nothing
else does, and a DECLINE is only acceptable on a diff protected by neither route (`workit` §5).

⚠ **`filed=<ID>` IS NOT A DISPOSITION ON ITS OWN — filing is where the handoff moved to** (HZ-12459,
Captain 2026-09-15: *"there should be no remainders, no handoffs — the agent is supposed to bring the
work all the way to merge."*). A finding's DEFAULT disposition is `fixed=` — applied in this round,
re-reviewed, landed. Measured 2026-09-15: **97 open hazards and chores named a `PROP-` in their
title and 83 carried no assignee**, which is what `filed=` accumulates to when nothing binds the
filed item to anybody. ⚠ That is a SCOREBOARD and it moved to 90 / 75 within hours — run
`scripts/remainder-carrier-check.sh --sweep`, never quote this sentence (CH-11968). So every `filed=<ID>` owes the same two-question test `approveit` §6
carries, and the answer is never "the pool":

1. **Same mechanism as this diff?** ⇒ **FOLD IT IN.** Report it BLOCKING or AMEND, let the author fix
   it on the branch, and review the delta. One more round costs one spawned sub-agent; a board item
   costs an unbounded wait, and Layer A is deleted so nothing ends the wait.
2. **A genuinely different mechanism**, such that folding it would make one change into two? ⇒ it
   gets its own proposal, **driven by the session that files it**, which `nk claim`s it before it
   stops. `approveit` §6's gate refuses the close of THIS proposal while an item referencing it sits
   unassigned, so a `filed=` with no carrier does not get past the transport either.

⚠ **Before you file, ask what the ITEM BUYS — an item OBLIGES somebody, a comment merely RECORDS.**
File when the work needs a different HOST, a different AUTHORITY, or genuinely different EXPERTISE, or
when it is big enough that folding it would make one change into two. Do NOT file for something you
could fix in the same pass, and do NOT file to record an observation — a comment on the PARENT item
records it and costs nobody a claim. Measured on M5 2026-09-17: **nine items filed across four
landings**, of which about half recorded a finding rather than commissioning work, and each one then
needed a carrier, an assignee and a `remainder-carrier-check` pass before its proposal could close.

⚠ **"Out of scope" is not a third answer.** Scope decides which PROPOSAL a fix lands in; it never
decides who does it. The one exception is physical incapacity — a rust delta on the bus host, GPU
work that needs a Spark, or an act only the Captain performs (⚠ that third is a flagged widening of
`CMT-HZ-12459-25823` point 4, not a ruled case; `handoff` carries the argument) — and that item is
ASSIGNED to the named host with `handoff-cause:` in its body, never parked unassigned. ⚠ **And
incapacity is a fact about the two HOSTS, not about the finding:** `scripts/remainder-carrier-check.sh`
refuses `rust-delta` from any closer but Mini, `gpu-required` from a Spark, and `captain-authority`
to anyone but the Captain. You cannot declare your way out of work your own machine can do.

## 4. Record the verdict — under the sub-agent's OWN id

⚠ **DO NOT END YOUR TURN UNTIL `nk pr review` HAS RETURNED rc 0.** This one is addressed to YOU, the
reviewer. It is the reviewer-side twin of `campaign-watcher` §7.2's *"post the verdict the MOMENT it
is established"*; the grounds and the incident behind it are stated there and are deliberately NOT
repeated here, because a rationale copied per site is the drift surface this fleet keeps paying for.
§3 above tells the SPAWNER not to let its sub-agent end without a verdict — until CH-12579 no skill
in this loop said it to the party that actually ends the turn.

A slow gate is not an exit. Poll it in-session — `cmd >log 2>&1; rc=$?` in the foreground, or a
tracked background task you wait on — and post the verdict when it returns. An interim
`nk pr comment --kind progress` is optional; stopping after posting one is not a substitute for a
verdict. Measured on PROP-6425: two of five rounds (r1 and r4) posted an interim and stopped with no
verdict recorded, both while waiting on a gate run. §1 above is why that costs more than the round —
the slot has no release verb and no TTL, so a review that stops before recording leaves it held.

An Agent-tool sub-agent INHERITS this session's `CLAUDE_CODE_SESSION_ID`, so it resolves to the
author's own id and its approve is refused (measured live, CH-11826 phase 5: rc 1 on both the bare
call and on `NUSY_SESSION_ID=s-<own>`; a `claude -p` child gets its own id and is a different
session). **The reviewer is therefore a `claude -p` child**, taking
`NUSY_SESSION_ID=sub-<PROP>` ONCE in its launch environment, so every `nk` call it makes carries that
id with no prefixing (HZ-11839). `CLAUDE.md`'s "## Identity" is the authority and it rules the
alternative out flat — *"An Agent-tool sub-agent cannot satisfy this"* — so an Agent-tool sub-agent is
not a second way to do this: it inherits this session's `CLAUDE_CODE_SESSION_ID`, and an `export` does
not survive a tool call, which leaves per-call prefixing as its only carrier and that is the retired
form, not a co-equal one. The value is the SESSION COMPONENT only, never a full `<agent>/…` value (a
full value is re-prefixed to
`M5/M5/…` and slips the guard — HZ-11839), and never a bare `--author` anywhere. A bare-author proposal refuses every sub id (`author: M5,
caller: M5/sub-<PROP>`); the author's own refusal on an `open` proposal reads `Invalid transition`
rather than naming the author — an rc 1 is a refusal whatever the sentence says.

⚠ **If you are a `claude -p` child, check you can actually RUN a gate before you report anything.** A
`claude -p` session inherits none of its parent's permission mode and has no approver, so without an
explicit grant every `cargo`, `git worktree add`, `bash scripts/…` and `verdicts` call is refused —
floor row 7 goes undischarged while the review still reads as a review (HZ-12032). `campaign-watcher`
§7.2 is why the spawn carries `--dangerously-skip-permissions` and says never to drop it. One
ungranted child was measured **fabricating a `cargo` version string** rather than reporting that it
could not run: so confirm with a real `cargo --version` first, and if you cannot, say so at the TOP
of your review, not in a closing caveat — that is where the last one said it and two readers missed
it.

⚠ **AMEND ALONE DOES NOT REQUIRE `--request-changes`, and treating it as though it does is how a
one-round review becomes four.** A finding is OWED ON THE BRANCH only when it changes BEHAVIOUR, or
changes what a reader would DO. A finding that only narrows a comment's wording, with no behavioural
and no actionable consequence, is a **NOTE**: record it with its measurement and **APPROVE**.

The asymmetry is the point. A round costs a fresh tip, a fresh reviewer session and a full gate re-run;
a wording nit costs a sentence. And each round's fix ADDS prose, which is fresh surface for the next
sweep — measured on PROP-6490, where round 6's own clarifying note was followed by **round 6's
REVIEW** (`CMT-29017`) finding three OLDER sentences that note did not cover, both of them sitting
ABOVE it. ⚠ Round 7 approved clean, so cite the comment and not the round number.

⚠ **This does not soften the floor.** A guard that does not fire, a pin that pins nothing, a claim the
code does not support that a reader would ACT on, an orphaned remainder — all stay BLOCKING. The bar is
CONSEQUENCE, not politeness: "the comment is imprecise" is not a consequence unless somebody would do
something different because of it.

⚠ **Measured, and stated with its limit.** Over four landings on M5 2026-09-17 (13 reviews): six
BLOCKING findings, ALL on one proposal; the other three took six reviews and produced ZERO. The
single-round one (PROP-6520) had the same reviewer population and the same adversarial instructions,
and differed in its closing instruction — *"judge it on whether the pins discriminate and the text is
true, not on how much it changes."* ⚠ That proposal was also the EASIEST of the four, so difficulty is
a confound and this paragraph does not claim the rule alone explains the spread.

**ONE review round (Captain 2026-09-28, CH-13158):** *"the sub-agent finds the issues and they are
then fixed immediately, as long as the fixes pass tests, we should not send out for another review."*
So a BLOCKING finding a tested fix can close does NOT request changes. Every finding goes out in ONE
pass, numbered `F1`, `F2`, …, and:

- **Fixable ⇒ APPROVE WITH FIXES.** Record the row FIRST, exactly as the clean case below, then
  `--approve` with, directly below `reviewed-at-sha:`, `approve-with-fixes: F1 F2 …` (every BLOCKING
  finding) and `fix-scope: <the exact repo paths the fixes may touch>` — exact paths, no directory, no
  glob. The driver fixes each in a commit whose subject names it (`fix r1 F<n>: …`), and `approveit`
  §1 merges that fixed tip only when `scripts/lib/fix-scope.sh`'s `fix_scope_check` answers
  `in_scope`: a descendant of your sha, every changed path in your `fix-scope:`, every F<n> named, not
  protected-class, floor green on the tip. There is no re-review round for fixable findings. ⚠ v1
  limit: a PROTECTED-CLASS diff cannot take that path (`fix_scope_check` refuses it), so for one,
  request changes as below.
- **Needs a ruling, changes the item's scope, or you expect a dispute ⇒ request changes:**

```bash
NUSY_SESSION_ID=sub-<PROP> nk pr review <PROP> --request-changes --body-file <findings>
```

then the author — or **any session that claims the item**, which is the live model since HZ-8185
retired author-return (`docs/AUTONOMOUS-FLEET.md` §7; a rejected proposal returned to a dead session
stalled 5 measured times) — fixes, pushes, `nk pr revise <PROP>`, `nk pr resolve <PROP> --comment-id <CMT>`,
and a NEW sub-agent reviews the NEW tip (a `REVIEWED_AT` never carries across a push). **`revise`
CLEARS the review slot**, so round 2 starts at section 1 with a fresh `nk pr claim-review` — first
claim wins again, and this machine has no standing hold on the new round.

Clean ⇒ the sub-agent records its row FIRST, then approves, carrying the three lines a later reader
needs:

```bash
NUSY_SESSION_ID=sub-<PROP> verdicts record --decision reviewer_not_author --rc 0 --reason independent \
                --subject <REVIEWED_AT> --kind branch_sha --lineage <PROP>
NUSY_SESSION_ID=sub-<PROP> nk pr review <PROP> --approve --body-file <body>
#   reviewed-at-sha: <REVIEWED_AT>
#   approve-with-fixes: F1 F2 …        (ONLY when approving with fixes — both lines, or neither)
#   fix-scope: <exact repo paths>
#   merged-state: CLEAN @ <MERGED_TREE>
#   findings-disposition: …            (its verbatim findings above these lines)
```

⚠ **THE ROW COMES FIRST, and the order is the fix, not a tidy-up (HZ-12194).** These two lines used
to run the other way round, and an approve that lands with the row still unwritten is one session
ending away from a landing whose floor row 1 has no record: `PROP-6326` merged at `1a3d5d9d7b` with a
store-side approve, a correct derived attestation line and all five of the transport's floor rows,
and **zero** `reviewer_not_author` rows by its reviewer. Canon: *"A gate that fired and left no row
did not fire."* Every review the resident briefed by hand carried *"record your verdict row FIRST"*
and all six of those landings have the row; PROP-6326's reviewer was spawned by a work sub-agent and
inherited no such brief — so the instruction belongs HERE, in the procedure every reviewer reads,
rather than in whatever prompt happened to spawn it. `approveit` §1b is the other half: the transport
now REFUSES a merge whose approver left no row, so an approve written before the row is a wedged
proposal rather than a silent gap.

⚠ **A RE-REVIEW TAKES A NEW ID: `sub-<PROP>-r<N>`.** Round 1 is `sub-<PROP>`; every later round
appends its round number (`sub-<PROP>-r2`, `-r3`, …). `approveit` §1b's classifier is EXISTENTIAL —
one passing row under an approver's session outranks every non-passing row under it — so a reviewer
that reuses its round-1 id carries its round-1 `independent` row into round 2, and a round-2 approve
is then satisfied by a row recorded against a tip that no longer exists. That is HZ-12220's most
common route. ⚠ **This file is NOT where the id is chosen, and the round-4 sentence that said it was
("closes it at the only place the id is chosen") was simply untrue.** The SPAWNER chooses it, in
`campaign-watcher` §7.2; a child that reads this skill already carries the id it was launched with,
and all it could do about it is re-spell the value on every `nk` call, which is the retired HZ-11839
form. So the rule is stated in BOTH files and §7.2 is the one that ACTS on it — its snippet is round
1, and a re-spawn after a `revise` takes `sub-<PROP>-r<N>`. Stating it only here is what left this
fleet's residents appending `-r<N>` by hand on every round, or not at all. It does not change the
value's shape: still the SESSION COMPONENT only, never a full `<agent>/…` value. Renaming the
template fleet-wide — six files, `CLAUDE.md` included — is HZ-12220's own scope, not this one's.

⚠ **BOTH lines carry `NUSY_SESSION_ID=sub-<PROP>`, and the row's is not redundant.** A `claude -p`
reviewer has it in its process env already, so the prefix is idempotent there — but the snippet is
copied verbatim by readers who are not that child, and `verdicts record` composes its ACTOR from the
same carrier `nk` does. Unprefixed in a shell where only `CLAUDE_CODE_SESSION_ID` is set, the row
lands under `<agent>/s-<8>` while the approve lands under `<agent>/sub-<PROP>` — and `approveit` §1b
keys on the APPROVER's session, so the two never compare equal and a genuinely independent review is
refused as `absent`, with the refusal pointing the reviewer back at this very snippet. That is
HZ-12177's bare-vs-composed split in a third costume. The value is still the SESSION COMPONENT only
— never a full `<agent>/…` value, which re-prefixes to `M5/M5/…` (HZ-11839).

Recording first costs nothing when the approve is then refused for an ORDINARY reason (unresolved
comments, a state transition): the row says independence was assessed at that tip, which is true, and
nothing merges. ⚠ **The one exception is an approve refused on IDENTITY** — the byte-equal
inheritance failure "## Identity" in `CLAUDE.md` describes. There the row you just wrote attests
`independent` under the AUTHOR's own actor, which is a false attestation of exactly the class
HZ-12022 exists to prevent. Write the correcting row in the same breath:
`verdicts record --decision reviewer_not_author --rc 1 --reason same_session --subject <REVIEWED_AT>
--kind branch_sha --lineage <PROP>`, then fix the identity and review again. ⚠ **Call it a
CORRECTION, not a supersession, because it does not supersede for `approveit` §1b:** that
classifier is EXISTENTIAL — one passing row outranks any number of non-passing ones — so the rc-0
row you wrote a moment ago still answers `present` for that session no matter what follows it. It is
harmless in the case described (a session refused on identity never becomes an approver, so §1b
never asks about it), but do not reach for this line believing it retracts the first.

**The SUBJECT is the tip you read; the PROPOSAL is the `--lineage`** (HZ-12133). Keying this row on
the proposal alone was measured hiding half the record in both directions: `verdicts about <merged
sha>` returned five gate rows and ZERO independence rows, while `verdicts about PROP-6291` returned
six independence rows and ZERO gate rows — the same event, and neither output said the other half
existed. It also lost which tip each round read, which is floor row 6's own question: six rounds all
keyed `PROP-6291` are indistinguishable. `about` now answers on either key, so recording both is what
makes the two queries agree.

## 5. Clean up

```bash
git worktree remove --force /tmp/rv-<PROP>
```

A finding that SURVIVES the proposal (out of scope, "worth a follow-on") is an item
(`nk create …`), never only a comment ON THE PROPOSAL — `merged` is the one transition after which a comment
cannot be reached.
