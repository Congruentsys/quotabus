---
name: approveit
description: After a clean review — merge the store record, transport the branch to main under the floor, verify ancestry, record every verdict, close the item or CARRY the remainder yourself, delete the branch. Re-cut at VY-11604 Commit B (CH-11826). Closed to the Skill tool by design (CH-11917); the resident REACHES it by opening this file and following it inline — campaign-watcher section 7.3.
disable-model-invocation: true
argument-hint: "PROP-XXXX"
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/approveit/SKILL.md` on
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

# Approve It — land it, prove it landed, record what fired

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

`alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'`. Every rc UNPIPED. The merger may be
any session — including the author's — because the store applies no identity bar at merge; what
it may never be is the approver.

## How this skill is REACHED — the Skill tool is closed on purpose (CH-11917)

`disable-model-invocation: true` is deliberate and **stays**. It keeps a bare model from starting a
**push to `main`** as an autonomous tool call. The harness enforces it with a refusal of the form
(captured on DGX1, 2026-09-12, attempting the Skill tool on `reviewit`): *"Skill reviewit cannot be
used with Skill tool due to disable-model-invocation. Ask the user to run /reviewit themselves … Do
not replicate this skill's workflow by other means — it is reserved for explicit user invocation."*

⚠ **The exact wording is the harness's and may differ from this quote** — a round-2 reviewer probing
from a different working directory got `Unknown skill: reviewit` instead, because a skill closed to
the model is not merely refused, it is absent from the model's surface. Do not match on this string;
no gate does. What is stable is the EFFECT, and it is what the rest of this section is about.

⚠ **That last clause is scoped to the Skill tool and does NOT bar the resident loop.** Two reaches
are sanctioned by canon, and a resident that stops at the refusal has misread it:

- The resident **opens this file and follows it inline.** `campaign-watcher` section 7.3 says it in
  its own words — *"`approveit` (`.claude/skills/approveit/SKILL.md`) is the authority — open that
  file and follow every step; do NOT merge from this summary"* — and `CLAUDE.md`'s loop section says
  *"`reviewit` and `approveit` are the procedures the resident follows inline"*. This is the normal path.
- A `claude -p "/approveit <PROP>"` child. A `-p` prompt is a USER turn, so the flag never reaches it.
  Measured under CH-11917: a `disable-model-invocation: true` skill executed in full through `-p`.

So SG-8070's *"an unattended spawned session may run `/approveit` and land a proposal to `main`"* is
satisfied **today, with the flag set**. The flag governs the model's TOOL SURFACE; SG-8070 and
SG-11818 govern AUTHORITY (who may merge, who may review). Different axes — the rulings do not bear
on the flag, and flipping it would widen nothing except a bare model's ability to start a merge
transport unprompted. **Do not flip it.**

## 1. Preconditions

```bash
nk fleet status >/tmp/cord.txt 2>&1; rc=$?     # rc 2 or a ^\[fleet\] HALTED: line ⇒ stop
nk pr view <PROP>                              # status approved, zero unresolved comments (the binding sha is DERIVED below, never eyeballed here)
git fetch origin main <branch>
scripts/hooks/install-pre-push.sh origin/main  # the hook you are about to be gated by is a COPY
# ── THE BINDING SHA. A multi-round proposal declares ONE PER ROUND; only the newest binds.
. scripts/lib/prop-declaration.sh              # the ONE declaration grammar (CH-8403) — never a private regex
rm -f /tmp/ap-<PROP>.fixnote                   # §3's merge-message note: written below on the FIX path ONLY, never stale
D="$(mktemp -d)" || { echo "REFUSE — CANNOT ASSESS: no work dir"; exit 1; }
nk pr view <PROP> >"$D/view.json"; rc=$?       # UNPIPED: an unreadable store is CANNOT ASSESS, never "no approver"
[ "$rc" = 0 ] || { rm -rf "$D"; echo "REFUSE — CANNOT ASSESS: nk pr view rc $rc"; exit 1; }
# Per APPROVER, that approver's LAST comment declaring a hex reviewed-at-sha (HZ-10276/HZ-11991), and
# the body file holding it. The library is bash and is EXECUTED under bash, never sourced into this
# shell (zsh on M5/Air/Mini — §1b's "executed, not sourced" paragraph is the same reasoning).
FIX_SCOPE_WORKDIR="$D" bash -c '. scripts/lib/fix-scope.sh && fix_scope_approver_decls "$@"' _ "$D/view.json" >"$D/declared"; rc=$?
awk -F'\t' '{ print "declared: " $3 "  " $1 "  " $2 }' "$D/declared"
[ "$rc" = 0 ] || { rm -rf "$D"; echo "REFUSE — CANNOT ASSESS: no approver of <PROP> declares a parseable reviewed-at-sha: (fix_scope_approver_decls rc $rc)"; exit 1; }
TIP="$(git rev-parse origin/<branch>)"; MATCH=0; MODE=""; FIXSHA=""; FIXBIND=""; FIXANS=""
while IFS=$'\t' read -r appr BIND BIND_AT BODY; do   # the tip need satisfy only ONE approver
    [ -n "$appr" ] || continue
    FULL="$(git rev-parse --verify "$BIND^{commit}" 2>/dev/null)" || { echo "CANNOT ASSESS: $appr declared $BIND, which resolves to nothing here"; continue; }
    case "$FULL" in "$BIND"*) ;; *) echo "CANNOT ASSESS: $appr declared $BIND, which resolved as a REF, not an object id"; continue ;; esac
    echo "binding: $appr  $BIND ($BIND_AT)"
    [ "$MATCH" = 1 ] && continue                     # already bound — never run a second floor
    # exact: no approve-with-fixes and TIP == FULL (today's comparison). in_scope: a findings-scoped
    # fix of FULL, floor GREEN on TIP — the floor runs HERE, before `nk pr merge` (§3).
    FS="$(bash -c '. scripts/lib/fix-scope.sh && fix_scope_check "$@"' _ <PROP> "$FULL" "$TIP" "$BODY" </dev/null)"; fs_rc=$?   # </dev/null: the floor's cargo/git/bash must never eat this loop's stdin (the declared TSV)
    echo "fix-scope: $appr  $FS (rc $fs_rc)"
    case "$FS" in
      "rc=0 reason=exact")    MATCH=1; MODE=exact ;;
      "rc=0 reason=in_scope") MATCH=1; MODE=fix; FIXSHA="$TIP"; FIXBIND="$FULL" ;;
      *) [ -n "$(prop_decl_token 'approve-with-fixes' <"$BODY" 2>/dev/null || true)" ] && FIXANS="$FS" ;;   # a FIX path refused
    esac
done <"$D/declared"
rm -rf "$D"                                    # ONE reap point, above every exit below — nothing strands it
if [ "$MODE" = fix ]; then
    echo "FIX PATH: reviewed-at-sha $FIXBIND; r1 findings fixed at $FIXSHA"
    printf 'reviewed-at-sha %s; r1 findings fixed at %s\n' "$FIXBIND" "$FIXSHA" >/tmp/ap-<PROP>.fixnote
fi
if [ "$MATCH" != 1 ] && [ -n "$FIXANS" ]; then  # RECORD BEFORE YOU REFUSE (§1b's rule): the fix path fired
    fx_rc="${FIXANS#rc=}"; fx_rc="${fx_rc%% *}"; fx_tok="${FIXANS##*reason=}"
    verdicts record --decision approval_fix_scope --rc "$fx_rc" --reason "$fx_tok" --subject "$TIP" --lineage <PROP> \
        || echo "⚠ the approval_fix_scope row did NOT record — say so in the refusal comment"
fi
[ "$MATCH" = 1 ] || { echo "REFUSE — no approver's binding sha is the tip $TIP, and no approve-with-fixes admits it: DRIFT (withdraw the approval, then back to reviewit), a fix-scope refusal named above (fixes_owed / out_of_scope / floor_red — fix it, or a fresh review round), or CANNOT ASSESS if every line above says so"; exit 1; }
```

**WHICH `reviewed-at-sha:` — the LATEST one, and it is DERIVED, never picked by eye (HZ-11991).** The
comparison above is unchanged in intent — the branch tip against the sha a reviewer measured, refusing
on drift unless the tip is a findings-scoped fix of it (approve-with-fixes, below) — but the value it reads used to be written `"<reviewed-at-sha>"`, singular, and a multi-round
proposal accumulates one per round. Measured on PROP-6284: **6 distinct values across 16 declarations
in 12 comments, with ONE approver** (`47be74379e · 4babc83b0e · 4d8efb4454 · a233e96d82 · 071e1205b2 ·
775c34d27d`). Five are stale by construction. A merger who takes the FIRST gets `47be74379e`, five
rounds stale, and §1 refuses a perfectly correct transport. Two mergers on two hosts have now made this
call by eye — Mini on PROP-6238 (two approvers, two shas), DGX1 on PROP-6284 — and neither was told to.

- **The population is the comments whose `reviewer` the store lists in `approvers`.** Not every
  comment: on PROP-6284 the AUTHOR's own newest comment carries a mid-sentence
  ``…the LATEST `reviewed-at-sha:` on the proposal…``, which the shared grammar reads as the value
  `on` — so a read pooled over ALL comments answers NOT-A-SHA and refuses a correct transport.
- **Per approver, that approver's OWN comments in store order (`created_at_ms`), the LAST one that
  DECLARES** — never simply their latest comment (HZ-10276, observed live on PROP-5613): pooling
  across approvers let approver B's unrelated later note discard approver A's declaration, and a
  later non-declaring note by the SAME approver must not erase their earlier one. `fix_scope_approver_decls` keeps the
  LAST declaring comment per approver, and the tip need match only ONE of them — measured on PROP-6238, the only
  two-approver proposal in the corpus: `Mini/sub-PROP-6238` binds `c7f6967164`,
  `Mini/sub-PROP-6238-r2` binds `e48168afc8`, and either as the tip is accepted. A pooled
  latest-wins read would refuse the first of those, at rc 1, on a proposal that reviewer approved.
- **No `kind` filter.** The retired `approval-sha-drift-check.sh` excluded `kind: progress` to skip
  Layer A's `[VW] reviewing as X` markers. Those are gone with Layer A, and PROP-6284's rounds 5 and 6
  were both posted `--kind progress`: that filter now selects `071e1205b2` — one round stale.
- **Through the shared reader, never a private regex** (CH-8403). EX-11605 measured a private
  `reviewed-at-sha:` parser in the merge transport misreading 3 of 45 declarations the shared reader
  gets right — backticked and bolded values — and pooling across comments as above.
- ⚠ **Resolve the declaration before comparing it.** Reviewers paste 10-char shas (`775c34d27d`,
  `09568029a6`, `cde9829b34` — measured on three recent proposals) while `git rev-parse` prints 40, so
  the old line's string `=` refused EVERY abbreviated declaration. Two conditions on the resolve,
  because either alone is defeatable: hex-only, so `main` and `origin/hz-x` are out, AND the resolved
  id must START WITH the declaration, so a branch literally named `abc1234` is refused. A REF would
  re-resolve to the CURRENT tip and compare equal BY CONSTRUCTION — a guaranteed pass, forever.
- **ZERO declarations is a REFUSAL, not a pass.** §1 used to list "`reviewed-at-sha:` present" as a
  precondition and never said what an absence meant; absence is CANNOT-ASSESS, which CH-7020 says is
  not a pass, so §1 stops and §5 records `--rc 2 --reason cannot_assess`. The emptiness test is
  load-bearing: measured, a bare `reviewed-at-sha:` line makes `prop_decl_token` exit **0** with an
  EMPTY value, so rc 0 alone does not mean a sha was read.
- **Residual, declared.** The grammar prefers an unfenced, unquoted declaration when the body has
  one and otherwise falls back to a fenced or backticked one rather than answering absent (measured
  both directions), so a review body that quotes the canonical form in a fence ABOVE its real
  declaration binds the example — the hex-and-resolve control catches a fabricated sha, not a
  real-but-stale one. And the declaration is SELF-ASSERTED: a reviewer who re-derives the tip at
  write-up time rather than capturing it at fetch time declares an already-drifted value and gets a
  true-looking clean. Nothing merger-side can tell those two apart.

**APPROVE-WITH-FIXES — the ONE moved tip §1 accepts (CH-13158; Captain 2026-09-28, rulings
2026-09-29).** The Captain's rule is one review round: *"the sub-agent finds the issues and they are
then fixed immediately, as long as the fixes pass tests, we should not send out for another review."*
A reviewer with FIXABLE findings therefore APPROVES in the store (the store's reviewer ≠ author
predicate is untouched) and writes, below its `reviewed-at-sha:`, `approve-with-fixes: F1 F2 …` and
`fix-scope: <exact repo paths>`. The driver fixes every finding in commits whose subject names it
(`fix r1 F<n>: …`), which moves the tip past the approved sha — the move row 6 exists to refuse.
`fix_scope_check` (`scripts/lib/fix-scope.sh`) is the only thing that admits it, and it answers
`in_scope` only when ALL hold: the tip DESCENDS from the approved sha (a force-push is refused), with
no merge commit and no rename in between; every changed path is BYTE-EQUAL to a `fix-scope:` path
(no prefix, no glob); every declared `F<n>` is named by a commit subject; the proposal is NOT
protected-class; and **the floor is green on the TIP** — a whole-workspace `cargo check`, `cargo test -p` for
each touched crate and every shell battery that COVERS a touched path (a touched
`scripts/tests/*.sh`, one whose text names the path, or one whose `scripts/battery-wiring.conf`
inputs list the path or a directory above it), in a scratch `git worktree add --detach`
that is always reaped. **The floor runs HERE, in §1, BEFORE §3's `nk pr merge`**: once the store
records the merge there is no fix path left (see the §3 paragraph on a red composed tree). It runs
only after every other condition already holds, so a refusal costs no build; it uses its own
`CARGO_TARGET_DIR` unless you export one (one per concurrent tree, HZ-12365), and no variable skips it.
- `exact` is today's behaviour: no `approve-with-fixes:` and TIP = the approved sha. §5 records
  `approval_sha_drift`. `in_scope` is the FIX path: §3's merge message carries
  `reviewed-at-sha <SHA>; r1 findings fixed at <FIX-SHA>` (read from `/tmp/ap-<PROP>.fixnote`, which
  this fence deletes first and writes on the fix path ONLY), and §5 records `approval_fix_scope`.
- `fixes_owed` (a finding no commit names, or no fix commit at all), `out_of_scope` (a path outside
  the scope, a merge, a rename, a rewritten history, a protected-class proposal) and `floor_red`
  REFUSE, and so does `cannot_assess` (no `fix-scope:`, a glob or `..` in it, a prose tail on
  `approve-with-fixes:`) — never a pass (CH-7020). A refused FIX path is recorded HERE, before the
  refusal, as an `approval_fix_scope` row with the token it answered: §5 is unreachable on that path.
  A tip that moved with NO `approve-with-fixes:` is still DRIFT (`out_of_scope` from the library,
  `approval_sha_drift drift` in the record) — unchanged.
- ⚠ **v1 limit: a PROTECTED-CLASS proposal never takes the fix path.** `fix_scope_check` classifies
  `origin/main..TIP` with `scripts/lsc-class-check.sh`, `scripts/safety-paths.conf` and floor row 2's
  hand clauses (a skill, a hook, CI, a safety crate, the manifest, `Cargo.lock`, a deleted Rust test)
  and answers `out_of_scope` ("protected"): its fixes go back for a FRESH review round, as before.
- Only an APPROVER's declaration counts: a `fix-scope:` in the author's own comment is never read
  (the population rule above), so an author cannot widen its own licence.

**The hook is installed by hand and nothing refreshes it**, so a host can be running one older than
main with whole stages (the attestation refusal, HZ-12022) simply absent — and an absent gate is not
a passing one, absent exactly where it would have fired. `install-pre-push.sh` takes its source from
**`origin/main`, after the fetch — never from your working tree.** That is not a detail: the install
lands in the SHARED `--git-common-dir` and outlives your branch, so copying from the tree you happen
to be standing on installs *your branch's* hook host-wide — and if your branch predates a hook
change, it DELETES a live gate. Run it after the fetch, and read its line.

## 1b. Floor row 1 left a ROW — the transport ASKS, and refuses when it did not (HZ-12194)

```bash
# The store enforces the PROPERTY at approve (it refuses a self-approve) and the pre-push
# attestation stage verifies the merge NAMES the approver. Neither asks whether floor row 1 left
# its RECORD, which is the half the floor measures it by. This does.
# ⚠ READ rev-parse's rc. For a ref that does NOT resolve it exits 128 and PRINTS THE REF NAME on
# stdout, and `verdicts record` accepts that string as the row's subject (both measured) — so an
# unread rc here keys floor row 6's subject to `origin/<branch>` instead of a sha. This is the one
# refusal ABOVE the record, and deliberately so: nothing has been asked yet, so there is no answer
# to record, and recording a subject we know to be wrong is worse than recording nothing.
RR_SHA="$(git rev-parse origin/<branch>)"; sha_rc=$?
[ "$sha_rc" = 0 ] || { echo "REFUSE — origin/<branch> does not resolve (git rev-parse rc $sha_rc, stdout '$RR_SHA'): a row keyed to a ref NAME is not a row about a sha"; exit 1; }
# ⚠ THE READER IS EXECUTED, NOT SOURCED — see the shell paragraph under this fence. `bash` is not
# decoration: this block runs INLINE in the agent's own shell, which is zsh on M5, Air and Mini, and
# a sourced reader answered `2:cannot_assess` (a DECLARED PROCEED pair) for EVERY proposal there.
RR="$(bash scripts/lib/reviewer-row.sh check <PROP> 2>/tmp/rr-<PROP>.err)"; rc=$?
cat /tmp/rr-<PROP>.err                                 # the reason is on stderr; READ it
# ⚠ RECORD BEFORE YOU REFUSE. This is the one gate in this file whose row is written HERE and not
# in §5: §5 runs only on the path that merges, so a §1 refusal that recorded nothing would be a
# gate that fired and left no row — the exact defect this section exists to close, reproduced in
# its own fix (`approval_sha_drift` still has that hole on the EXACT path; HZ-12194 does not widen
# into it, and CH-13158's FIX path records `approval_fix_scope` in §1 BEFORE its refusal).
verdicts record --decision reviewer_row_present --rc "$rc" --reason "$RR" \
                --subject "$RR_SHA" --kind branch_sha --lineage <PROP>; rec_rc=$?
# ⚠ EVERY refusal below is spelled "PROCEED ONLY ON THE VALUE WE DECLARED", never "refuse on
# the bad value" — see the polarity paragraph under this fence. A `verdicts record` that failed is
# precisely "fired and left no row", so its OWN rc is read FIRST, on every path, and the ONLY
# thing it admits is rc 0.
[ "$rec_rc" = 0 ] || { echo "REFUSE — the row was NOT written (verdicts record rc $rec_rc, reason '$RR'): a gate that fired and left no row did not fire"; exit 1; }
# HZ-12023's LANDED arm lives in §3, BELOW this one, and it is what reconciles a proposal a PEER
# already landed. Refusing such a proposal HERE would send an already-merged branch "back to
# reviewit" and that reconcile would never run. So the rc-1 refusal — and only it — yields to
# ancestry. The row above is written either way.
# ⚠ `--is-ancestor` HAS THREE VALUES, not two: 0 an ancestor, 1 not, and **128 could not tell** —
# what you get when `origin/main` does not resolve (§3 declares the same three on its own ancestry
# line — `# 0 landed · 1 not · 128 no such ref` — and refuses the third by name; cited by its TEXT
# and not by a line number, which every edit above it moves). `&& LANDED=yes` folds 128 in with 1
# deliberately, so an ancestry read that could not be MADE reads as "not landed" and the rc-1 arm
# refuses. That is the fail-CLOSED direction.
# ⚠ Do NOT respell it `; [ $? != 1 ] && LANDED=yes`. That reads 128 as LANDED, a proposal with no
# passing row exits 0 here, and §3 then re-fetches, gets "not an ancestor" and COMPOSES — the
# landing this whole section exists to prevent, through the one value nothing drove until round 10
# (`test_hz12194_reviewer_row.sh` X16-X20b and M52; the zsh battery drives the same cells).
LANDED=no
git merge-base --is-ancestor "$RR_SHA" origin/main >/dev/null 2>&1 && LANDED=yes
# ⚠ THE DISPATCH IS ON THE (rc, reason) PAIR, not on rc alone, and that is the gate. With rc alone,
# ONE line anywhere below the record — `[ "$RR" = unattested ] && rc=2` — turns a REFUSE row into a
# proceeding transport while every arm of the battery stays green (measured, round 3). A pair the
# lib cannot emit is by construction an undeclared pair, so any reassignment of rc alone lands in
# `*)` and refuses. The five pairs below are the WHOLE of what the lib emits, and the conf declares
# the same five tokens (`scripts/layerb-decisions.conf`, checked by the battery's section F).
case "$rc:$RR" in
  0:present|2:off_host|2:cannot_assess) ;;   # the DECLARED proceed pairs, and the only ones
  1:absent|1:unattested)
      [ "$LANDED" = yes ] || { echo "REFUSE — floor row 1 left NO passing row for <PROP> ($RR): back to reviewit for a review that records one"; exit 1; }
      echo "NOT REFUSED ($RR) — <PROP> is ALREADY an ancestor of main: there is no landing left to prevent here, only a RECORD to reconcile. Do NOT compose; §3's LANDED arm (HZ-12023) takes it from here. The row above stands." ;;
  *)  echo "REFUSE — the UNDECLARED (rc,reason) pair '$rc:$RR': the check did not answer in its own vocabulary, and a check that did not run is not a check that passed"; exit 1 ;;
esac
```

⚠ **THE INTERPRETER IS PART OF THE GATE, and it was the eighth defang.** `approveit` is
user-invoked and runs **inline, in the agent's own interactive shell** — and that shell is **zsh on
M5, Air and Mini** (HZ-11990's body; PROP-6240's commit message; `agent-identity.sh`'s HZ-11270
block). Until round 6 this fence read `. scripts/lib/reviewer-row.sh`, and under zsh the reader
could not locate its own siblings (`${BASH_SOURCE[0]}` is EMPTY there) and answered
`2:cannot_assess` — a DECLARED PROCEED pair — for **every** proposal. Measured against the real
board and store, zsh 5.9 vs bash 5.2:

```text
                bash                       zsh
PROP-6326       1:absent        REFUSE     2:cannot_assess   PROCEED   <- this item's own defect
PROP-6345       0:present                  2:cannot_assess
```

So the gate this section exists to add would never have fired on **three of the five machines**, and
would have written a row saying the question had been asked. The fence therefore **EXECUTES** the
reader under `bash` rather than sourcing it: the reader, its two siblings and every expansion in all
three then run under bash whatever the agent's shell is, so **the READER's answer** — the token on
stdout and the rc — is shell-independent by CONSTRUCTION rather than by test coverage. Do not
"simplify" it back to a `.`.

⚠ **THAT CLAIM IS ABOUT THE READER AND NOT ABOUT THIS FENCE, and round 6 shipped it overstated.**
Every line in the block above — `sha_rc`, `rec_rc`, `LANDED`, and the `case` itself — still runs in
**the agent's own shell**, zsh on M5, Air and Mini. Executing the reader moves the reader out of that
shell; it does not move the fence. So the fence is shell-independent **by TEST COVERAGE ONLY**, and
the coverage is `scripts/tests/test_hz12194_reviewer_row_zsh.sh`, which drives section X's whole
declared population under bash AND under zsh (arms Z6/Z7). ⚠ **"The whole population" means the
whole of the battery's LIST, which is not the fence's input space** — round 7's wording called 16
cases "the whole population" and the ninth defang lived in a cell the list did not contain. The list
is now a product over (rc, token) × landed × record-outcome, `XC1`/`XC2` assert that product, and
the **case set exists ONCE**: the zsh battery DERIVES its cases from the main battery's single
declaration rather than keeping a copy, and Z8 pins that the derivation is LIVE (change one tuple
there and the population here moves). Round 7 measured why that matters — the old Z8 compared one
copy against another while two further hand lists drifted beside it unwatched, green at 15/0.
Round 6's reviewer measured the gap the overstatement hid: two one-line insertions,
`rec_rc=${rec_rc[0]:-0}` and `sha_rc=${sha_rc[0]:-0}`, are no-ops under bash and read `0` under zsh
(zsh subscripts a scalar; bash does not), and they restored round 3's S2 and round 3's A4
respectively **with both batteries green** — because the failure and already-landed halves of this
fence had no zsh arm at all. They are pinned as controls MZ4 and MZ5. **The lesson is the general
one: coverage that is complete on one axis can be empty on another.** Section X's case axis was
complete and its interpreter axis had one value.

⚠ **AND FIXING ONLY THE FILE LOOKUP WOULD HAVE FLIPPED THE GATE THE OTHER WAY.** zsh does not
word-split an unquoted expansion, so with the siblings hand-loaded the same reader refused a REAL
passing review (`PROP-6345` → `1:absent`) and read a two-approver set as `off_host`. Both layers are
closed in `scripts/lib/reviewer-row.sh`, and both DIRECTIONS are pinned — a refusal token still
refuses and a passing review is still not refused — by `scripts/tests/test_hz12194_reviewer_row_zsh.sh`,
which runs this fence and that reader under a real `zsh` and is CANNOT-ASSESS (rc 2, never a pass) on
a host with none. ⚠ Two limits are stated in that battery's header rather than left to be
rediscovered: **nothing has been run on macOS by anyone** (all zsh figures are zsh 5.9 on Linux
aarch64, and M5/Air/Mini — the three hosts the gate was inert on — are macOS), and **profile options
are untested**, because every leg runs `zsh <file>` and so sources no startup file, while the agent's
real shell is interactive and has. This is the class's fourth landing (HZ-11270,
HZ-11558/CH-11568, HZ-11990) and
every remedy in the tree is a POINT remedy: the two durable pieces those items produced — HZ-11270's
validated self-dir resolver and HZ-11990's per-file zsh battery — are both adopted rather than
re-spelled, but nothing would have CAUGHT this one. `portability-lint.sh` carries exactly one zsh
rule (HZ-11990's `RX_ZSH_SPECIAL_LOCAL`) and is a static scan that never invokes zsh, and each zsh
battery covers the single file it was written for. That absence is recorded on HZ-11990.

⚠ **THE POLARITY IS THE GATE, and getting it backwards is invisible.** Round 1 of this proposal
spelled the refusal `[ "$rc" != 1 ] || … exit 1`, which refuses on the ONE bad value and lets every
other rc through — including **127**, what the shell returns when `scripts/lib/reviewer-row.sh` is
not there to run (a stale tree, the wrong cwd, a skill loaded from a copy newer than the tree it
runs in). The reviewer ran the fence verbatim with
the lib unreachable: the gate never ran, **zero rows were written, and the transport proceeded**.
That is PROP-6326's own shape reproduced inside the detector built to catch it. Every other refusal
in this file is already the other polarity — `[ "$rc" = 0 ] ||` in §3, `[ "$MATCH" = 1 ] ||` in §1
— and §1b was the single exception. The `case` above proceeds on the declared PAIRS and refuses on
everything else, and the unreachable-lib path lands as rc 127 with an EMPTY `$RR`, which the recorder
refuses (`--reason` with no value records nothing, measured), so `rec_rc` catches it even before the
`case` does. ⚠ **That value is an INPUT of this fence with a domain of its own, and until round 11
it was driven at two of its three ancestry values.** `127:` × an `origin/main` that does not resolve
— a check that did not RUN beside an ancestry read that could not be MADE — was the one corner
nothing presented; the tip refuses it, and `test_hz12194_reviewer_row.sh` X4h-X4k now drive it
(M56 is the one-line `exit 0` that lived there, MZ14 the same under zsh). ⚠ **An unloadable DEPENDENCY lands in exactly the same place, and that is a round-6 fix
rather than an accident:** until then, a reader that could not source `attestation.sh` or
`agent-identity.sh` printed `cannot_assess` and returned 2 — the proceed pair — under bash as much as
under zsh, so "the check's own implementation would not assemble" read as "the check ran and could
not decide". It now returns rc 1 with an EMPTY token, which is the rc-127 class one level down. Both are pinned BEHAVIOURALLY, by extracting this fence and EXECUTING it
(`scripts/tests/test_hz12194_reviewer_row.sh` section X) — not by a regex over this prose, which is
how five one-line defangs of the round-1 fence kept the whole structural section green.

⚠ **AND PROCEEDING ON A DECLARED *rc* WAS NOT ENOUGH — the pair is.** Round 3's spelling was
`case "$rc" in 0|2)`, which is the right polarity and still had two one-line defangs that kept all
86 arms green, both built and RUN by the strong-tier reviewer against the real recorder:

```text
S1  `[ "$RR" = unattested ] && rc=2`  after the record   ->  exit 0, row says rc 1 unattested
S2  `[ "$rec_rc" = 0 ] || [ "$rc" = 2 ] || { …REFUSE… }`  ->  exit 0 with NO row, read-only store
```

S1 is closed STRUCTURALLY by dispatching on `$rc:$RR`: the reader's rc and its token move together
or not at all, so `rc=2` beside `unattested` is a pair the lib cannot emit and `*)` refuses it. The
same construction closes the residual N2 found — a non-numeric `return` is mapped by bash to rc 2,
which under an rc-only dispatch PROCEEDED, and under the pair dispatch is `2:present` and refuses.
S2 is closed BEHAVIOURALLY — `rec_rc` is read before anything else on every path — and **round 7
proved that the behavioural closure was only as wide as the population driving it.** ⚠ The sentence
that stood here until round 8 said *"section X now drives a failed record on each of rc 0, 1 and 2"*,
and that was true and NOT ENOUGH, which is the finer-grained form of the same lesson: **the
failed-record axis was enumerated per RC while this dispatch keys on the (rc, token) PAIR, and rc 2
has TWO tokens.** Only `off_host` was ever crossed with a failing record, so one inserted line —

```text
N9a  `[ "$RR" = cannot_assess ] && rec_rc=0`  after the record  ->  exit 0 with NO row, read-only store
```

— restored S2 on the other rc-2 token with **both batteries green** (main 119/0/0, zsh 15/0) and no
zsh needed. And the uncovered cell is the LIKELY one, not a contrived one: `cannot_assess` is what
the reader answers when *the store could not be queried*, which is the same outage in which
`verdicts record` cannot WRITE. The combination nobody tested is the combination reality produces.
⚠ Round 8's reviewer measured that the case driving that cell reached `cannot_assess` by the reader's
OTHER producer — an EMPTY approver set — and not by the outage this paragraph cites; round 9 drives
BOTH producers and `XE1` measures that the fence cannot tell them apart (see below).
Two more of the same shape sat on the LANDED axis, whose every case had a writable store and a
declared rc-1 pair: `… && rec_rc=0` when ancestry holds (N9b) breaks *"the row above is written
either way"* below, and `[ "$LANDED" = yes ] && exit 0` above the `case` (N9c) lets an UNDECLARED
pair proceed on a landed branch — D9 sees neither, because it greps the proceed and `*)` arm BODIES
for `LANDED` and a read above the `case` is in neither.

⚠ **AND ROUND 8's OWN SENTENCE HERE WAS TOO GENEROUS, which is this lesson for the third time at a
finer grain.** It said the population was *"a PRODUCT over the pair and over the landed and
record-outcome axes"*. It was a product over **pair × record-outcome**; the landed axis was crossed
with a failing record on the two rc-1 pairs only, with `0:present` and `2:cannot_assess` in the
record-ok outcome only, and with **`2:off_host` not at all** — the very token this fence argues is
the likely one on a landed branch, since the approvers of a peer's landing are routinely bare or
foreign. Round 8's reviewer built the two one-line insertions that live in those empty cells,

```text
T3a  `{ …is-ancestor "$RR_SHA" origin/main && [ "$rc" != 1 ] && rec_rc=0; } 2>/dev/null`
T3b  the same line keyed on `[ "$RR" = off_host ]`
```

and each left **both** batteries green (main 138/0/0/0, zsh 18/0) while a landed `off_host` /
`present` / `cannot_assess` proposal went exit 1 → exit 0 **with no row** — N9b's consequence
restored on the cells the yield does not own. **So the population is a product over the five
declared pairs × record-outcome AND, separately, over those same five pairs × record-outcome ON AN
ALREADY-LANDED BRANCH**, plus the inputs the fence never declared; `XC1` and `XC2` assert the
two products and `M45`/`M46`/`M46b` negate them against the derived round-7 and round-8 populations,
while `M40`-`M43`, `M47`-`M50` and `M52`-`M55` pin the eight defangs with the counterfactual that
each is invisible to the population it was found against and visible to this one. ⚠ **Round 10
subsumes both of those products into a DERIVED one** (`XD2`), for the reason above: the sentence
"the population is a product over the five declared pairs × record-outcome, and again on an
already-landed branch" was the third statement of this shape in three rounds, and each was true and
short — the ancestry axis is not a boolean (`landed`/`not`) but an rc with three values, and the
reader axis is the ENTRY's six pairs, not the classifier's five. `cannot_assess` has **two producers**
in the reader — an empty approver set, and a store query that failed — and they are ONE cell, not
two: this fence reads the token and the rc, `cat`s the stderr without branching on it, and both
producers present `2:cannot_assess`. `XE1` measures that equality rather than asserting it, and both
producers are driven, which is what makes a cause-keyed defang (`grep -q 'could not be queried' … &&
rec_rc=0`) visible (`M49`). **The recorder cannot supply this binding** — the
conf does not tie a token to an rc, and `verdicts record --rc 0 --reason absent` is accepted (round
1 N4, re-measured in round 3). It has to be spelled here.

⚠ **WHERE THE BATTERY'S OBLIGATION ENDS, stated so the next round does not rediscover it as a
finding — and AMENDED after round 9.** The obligation is an exhaustive pin over a matrix **DERIVED
FROM WHAT THIS FENCE READS**: every input, each with its FAILURE value included — `sha_rc`, the
reader's `rc:RR` **as the `check` ENTRY emits it**, `rec_rc`, the rc of `git merge-base
--is-ancestor`, and the inherited `LANDED`. A hole in that space is a defect. ⚠ The rule it
replaces — "the declared product: rc × token × landed × record-outcome" — enumerated the cells its
author had thought of, and it was surprised twice running: round 8 by `landed × off_host`, round 9
by the ancestry rc's **third** value (128, which `&& LANDED=yes` correctly folds into "not landed"
and which nothing drove) and by the ENTRY's **sixth** output (`1:` with an empty token, which the
classifier cannot emit, so a coverage predicate built on the classifier could not see it). **A
population you enumerate by hand is a population you can be surprised by; a population you derive
from the artifact is not.** `test_hz12194_reviewer_row.sh`'s `XD0`/`XD1`/`XD2` extract the reads
from this fence, derive each domain from its producer, and refuse when a read has no declared
domain — so a NEW input added here cannot sit outside the matrix silently **in a spelling that
extractor recognises**. ⚠ **That qualifier is round 11's A1 and it is load-bearing: `xd_reads` is a
RECOGNISER OVER SYNTAX, not a parser.** Round 10 shipped the sentence unqualified while the
extractor knew exactly two spellings — `<cmd>; VAR=$?` at end of line and `<cmd> && VAR=<word>` —
and **this file's dominant idiom is neither**: 11 of its 19 rc captures are `<cmd>; VAR=$?   # …`
with a trailing comment, including §3's own ancestry line, the very line §1b cites as the
declaration of the ancestry domain. It now also recognises the trailing-comment form, `VAR="$?"`,
`VAR=${?}` and `|| VAR=<word>`; it does **not** recognise a `$?` captured on a following line,
`if <cmd>; then`, `&& { VAR=…; }`, `$(<cmd>; echo $?)` or `PIPESTATUS`, and a read spelled one of
those ways sits outside the matrix with `XD0` green. Widening a recogniser buys the forms named,
never closure — so if you add a read here, spell it the way the rest of this file spells one.
⚠ **And the reader axis's FAILURE half has now been DERIVED THREE WAYS, which is itself the
finding (round 12's B1).** Round 10 took it from ONE hand-picked fixture, which yields `1:` (an
unloadable sibling) and cannot yield `127:` (no reader to run at all — the value round 1's defect
actually produced, and by `reviewer-row.sh`'s own words the ROOT of the class `1:` sits in one level
down); deleting every rc-127 case then left the battery passing 176/0 **while still reporting all 42
cells covered**. Round 11 replaced the fixture with a RULE — what the ENTRY emits with each of its
load-time dependencies REMOVED, one at a time, the dependency list read out of the reader — and a
rule is still a **generator whose range you have to trust**. Removal expresses exactly two failures.
A reader that is PRESENT and cannot RUN emits three more, measured against the shipped entry:

```text
conflict markers mid-file (a failed merge left in the tree)  ->  `2:`      rc 2 is a PROCEED rc half
truncated before its CLI block                               ->  `0:`      rc 0 is the PRESENT rc half
the path is a DIRECTORY, or mode 000                         ->  `126:`    POSIX "found, cannot execute"
```

The fence REFUSES all three (the recorder refuses an empty `--reason` for every rc), so this was a
hole in the PIN and not in the gate — but a one-line `[ "$rc:$RR" = "<value>" ] && exit 0` at the
seat below the record exits 0 with NO row for each of them, under bash and zsh alike, with **both
batteries green** (main 185/0/0/0 on `2:` and on `0:`; zsh 24/0/0 on `2:`). **A derived matrix that
announces full coverage while blind to an input value is worse than a hand-written one** — and a
fourth generator would have been found out by round 13. So the domain is taken from the one place a
fixture cannot surprise: **what THIS FENCE DISTINGUISHES.** The entry emits either a classifier
token or an EMPTY token with some rc, and the rc values this fence tells apart are exactly the rc
halves the `case` patterns above name. The axis is therefore the classifier's pairs ∪ `<rc>:` for rc
in { those halves (today 0, 1, 2) ∪ the POSIX could-not-run pair (126, 127) ∪ one value outside all
of them (137, a reader killed by SIGKILL — not hypothetical on a fleet whose harness kills sessions
under a memory monitor) }. It grows when a `case` pattern names a new rc half, with no edit in the
battery (`M59` measures that); 126 and 127 are in by construction because they are what the SHELL
answers and so can never appear as literals here; and the outside value is what makes the scope
argument honest — a defang keyed on `3:` is genuinely the unbounded-edit population, because this
fence cannot tell `3:` from `137:`. Round 11's dependency sweep is kept as a FALSIFIER (every value
it produces must be IN the domain), which is the only direction a generator can be used honestly.
All six empty-token values are driven across the record and ancestry axes, `M57` loops the
delete-every-case measurement over each of them, and `M56`/`MZ14`, `M60a`-`M60d`/`MZ15`/`MZ16` are
the bypasses that lived in the cells each earlier derivation could not ask for. It does **not** defend
against an author EDITING this fence: that space is unbounded, and every arm built to catch one
hand-written line invites the next one a token further out. What defends against that is floor row 2 — this section
lives in a SKILL, a skill is protected class, and every change to one takes a branch, a proposal and
a strong-tier independent review. That process is what found every defang this battery carries as a
control — and the three it deliberately does NOT (round 8's evasions of `XS1`, which is a text
predicate over the battery file and evadable the same way). Those limits are written down in the
battery's header rather than chased with a further arm (scope ruling on HZ-12194, flagged for
Captain veto).

**Measured, and it is why this section exists.** `PROP-6326` landed at `1a3d5d9d7b` with a correct
DERIVED attestation line, a store-side approve, and all five of §5's rows — and **zero**
`reviewer_not_author` rows by its reviewer (`grep -c 'sub-PROP-6326' ~/.nusy/layerb-verdicts.jsonl`
→ 0). Every gate was green and nothing anywhere refused or even remarked. Canon: *"A gate that
fired and left no row did not fire."* The reviewer that left no row was spawned by a WORK
sub-agent rather than by the resident, so it inherited none of the resident's brief —
`reviewit` §4 now puts the row BEFORE the approve so the instruction travels with the procedure
instead of with whoever happened to spawn the reviewer.

**THE SEVERITY IS A HARD REFUSAL on rc 1, and a debt row on rc 2 — chosen, not defaulted.** Floor
row 1 is the table's one row marked *not waived*, and its home is `refusal`. The two rc-1 tokens are
the cases where the evidence is DECIDABLE here and says no: `absent` (an approver whose row could be
in this store left none) and `unattested` (rows exist and none passes). A `cannot_assess` row there
would be the weaker choice and it is rejected — CH-7020 says CANNOT-ASSESS is not a pass, and a
"visible but landable" debt row is how row 1's record became optional in the first place. The rc-2
tokens are the cases where the evidence is NOT decidable here: `off_host` (the approver names another
machine, so its row is in THAT host's store — the verdict store is host-local, and booking an
unreadable store as absence is HZ-11813's false-absence shape) and `cannot_assess` (the approver set,
the agent name or the store could not be read). Those record their row and proceed LOUDLY, exactly
as floor row 5's `undeterminable_permissive` does. What this must never become is a refusal whose
cheapest escape is writing the row yourself: a transport-written `reviewer_not_author` row would
attest an independence the merger did not perform, which is the false attestation HZ-12022 exists to
prevent. **The remedy for rc 1 is a review that records its row, never a row.**

⚠ **WHERE THIS BLOCK SITS IS PART OF THE GATE.** It runs BEFORE §2, before `nk pr merge`, before the
scratch worktree, before the push. The item records its author's own process failure in one line —
*"I checked the row set AFTER the push, not before"* — and a §1b relocated below §3 would be that
failure with a gate bolted to it: it would refuse a landing that has already happened. Do not move
it, and do not assign to `rc` or `RR` between the capture and the record. Ordering against the
transport is pinned STRUCTURALLY (`scripts/tests/test_hz12194_reviewer_row.sh` D7, with its own
mutation control) because it is the one property execution cannot see; everything else about this
fence is pinned by RUNNING it (section X). The structural arms alone are not enough and that is
measured: five separate one-line defangs of the round-1 fence — `rc=0` after the record, a
`; true)` inside the capture, a commented-out `exit 1` the regex still matched, a local
`reviewer_row_check()` override, and `: $((rc=0))`, which D6's "an assignment anywhere" claim did
not actually cover — left every structural arm GREEN. Round 3 added two more (S1 and S2 above),
which is why the dispatch is on the pair and why section X now drives every declared token.

⚠ **AND THE ONE PRICE OF SITTING HERE: §3's LANDED arm is BELOW this one.** HZ-12023 put an
`--is-ancestor` re-check inside §3 that discovers a proposal a PEER has already landed and
reconciles the store record instead of composing. Because that arm runs later, a §1b that refused
unconditionally would send an ALREADY-MERGED branch "back to reviewit" and the reconcile would
never run — the approvers of a peer's landing are routinely bare or foreign, so `absent` is the
likely token. The fence yields on exactly that fact and on nothing else: the row is still written,
`LANDED` is read from ancestry alone, only the rc-1 arm consults it, and a stale `origin/main`
answers "not landed", which refuses. Moving §1b below the LANDED arm was the other option and is
rejected: that arm's own `nk pr merge <PROP>` reconcile is a transport anchor, so the record would
then follow a store merge — D7's refusal, and the same "checked it AFTER" shape one level in.

⚠ **TWO CASES THE TOKENS DO NOT NAME OUT LOUD, so read them here.**

- **A BARE approver — one the store returns with no `<machine>/` half — is treated as LOCAL, not as
  unanswerable.** Under the inside-out loop the machine holding the review claim is the machine that
  transports, so its reviewer ran here. **Bare approvers are a LIVE, RECURRING, three-host shape**
  — HZ-12177 is the standing item — not this proposal's own one-off: replay the survey rather than
  quoting a figure from here (CH-11968), because the window moves on every landing:

```bash
nk pr list --status merged --limit 14 | sed -nE 's/^  (PROP-[0-9]+) .*/\1/p' | while read -r p; do
    printf '%s ' "$p"; nk pr view "$p" | python3 -c 'import json,sys; print(*json.load(sys.stdin)["approvers"])'
done
```

  The round-1 text of this section froze one such replay into canon ("13 of 14 … the ONE bare
  approver"). Three re-derivations across this proposal's rounds read 13, then 12, then 13 again —
  on the same command, over a window that had moved twice underneath it — which is the whole reason
  the command and not the number is what is written down. The other reading (bare ⇒
  `cannot_assess`) answers "unknown" on exactly the landings this gate exists to catch.
  **The declared cost:** a review that really did
  happen on another machine and was recorded bare is refused here as `absent`, and the refusal text
  will say `absent`, not `off_host`. If you are looking at such a case, the remedy is still a local
  review that records its row — that is what floor row 1 asks for — but know that this is the one
  input on which the gate can refuse a correct landing.

- **An EMPTY approver set is `cannot_assess` and PROCEEDS here.** It is not a hole: §1 above has
  already REFUSED when no approver declares a parseable `reviewed-at-sha:`, and §3 refuses again when
  `attestation_approvers` answers rc 1. "Nobody approved" never reaches a merge through this section.

⚠ **THE LOOKUP KEYS ON THE ACTOR, NEVER THE SUBJECT — this is measured and it is not obvious.** A
`reviewer_not_author` row's `.subject` is the reviewed SHA, so `verdicts about <PROP>` answers *"no
verdict recorded"* at **rc 0** while five rows exist (PROP-6290, HZ-12133). The key is `.actor`, the
reviewer's session id. ⚠ **And BOTH of its spellings.** One real store holds `sub-PROP-6229` and
`DGX2/sub-PROP-6220` side by side, and

```text
verdicts by sub-PROP-6229        ->  1 verdict
verdicts by DGX2/sub-PROP-6229   ->  NO verdict   (the bare row is only a "near miss")
```

so the query is keyed on the **bare session component** of the approver, which reaches both, and
never on the approver string the store hands back — that reaches one. HZ-12177 is the precedent for
what a bare-vs-composed split does to a string comparison. The lib does this; do not re-spell it.

## 2. Protected-class diff ⇒ read the off-host verifier's row, and record what you read

```bash
SHA="$(git rev-parse origin/<branch>)"
unset NUSY_VERIFIER_KV_DIR                          # the lib's TEST-ONLY filesystem seam; a battery run in this shell leaves it exported, and it would divert this production read to a local dir
. scripts/lib/verifier-row.sh                       # the ONE reader; a raw `nats kv get` cannot tell "no row" from "bus down" (both exit 1)
                                                    # SOURCED, unlike §1b's reader — safe because the lib
                                                    # locates ITSELF shell-independently (HZ-12535)
verifier_row_get "$SHA" >/tmp/vr 2>/dev/null; rc=$?   # 0 row · 1 ABSENT · 2 no nats binary · 3 READ FAILED (HZ-11813)
```

- rc 0, `verifier_row_field "$(cat /tmp/vr)" verdict` = `PASS` ⇒ `verdicts record --decision exogenous_verifier_row --rc 0 --reason pass --subject $SHA --lineage <PROP>`
- rc 0, `FAIL` ⇒ `verdicts record … --rc 1 --reason fail --subject $SHA --lineage <PROP>` and **REFUSE** — stop here
- rc 1 (absent — the store answered `key not found`: nothing was ever published for this sha) ⇒
  `verdicts record … --rc 0 --reason absent_permissive --subject $SHA --lineage <PROP>` (the debt row; the floor's stage
  is `probe` in `scripts/verifier-floor.conf` — a missing row proceeds loudly, never silently)
- rc 3 (the READ failed — bus unreachable, read error, unreadable/empty row; say why: `echo "$VERIFIER_ROW_LAST_ERROR"`)
  ⇒ `verdicts record … --rc 0 --reason undeterminable_permissive --subject $SHA --lineage <PROP>` — **never `absent_*`**:
  absent is a queryable claim that no row exists, and an outage is not evidence of that (HZ-11813)
- rc 2 (no `nats` binary on this host) ⇒ cannot assess — install it (`brew install nats-io/nats-tools/nats`) and re-read; do not record
- **ANY OTHER rc ⇒ CANNOT ASSESS. Fix the source and re-read; record NOTHING.** The common value is
  **127**: the `.` on the line above FAILED, so `verifier_row_get` was never defined. Two reachable
  causes, both shell-independent — the sourcing path is RELATIVE and nothing in §1/§1b establishes a
  cwd, so running §2 from anywhere but the repo root gives 127; and the lib REFUSES TO LOAD when it
  cannot find `fleet-endpoint.sh` beside it (HZ-12356), which also leaves the function undefined.
  ⚠ Without this bullet an rc outside 0-3 matches no branch and the caller falls through, so floor
  row 5's debt row — the whole point of this step — is silently never written, and "a gate that fired
  and left no row did not fire".

⚠ **Why this reader is SOURCED when §1b's is EXECUTED, since the two look inconsistent.** §1b's
warning is about `reviewer-row.sh`, which could not locate its siblings under zsh and answered a
DECLARED PROCEED pair for every proposal. `verifier-row.sh` had the same class of exposure at one
line — `VERIFIER_LIB_DIR` was resolved from an unguarded `${BASH_SOURCE[0]}`, EMPTY under zsh, so it
became the CALLER'S CWD — and HZ-12535 fixed it in the lib with the same three-way + git fallback the
file's own endpoint block already used. Measured on Air 2026-09-16, sourcing from `/tmp`: before,
`floor_value stage` answered rc 1 / empty under zsh against rc 0 / `probe` under bash; after, both
answer `probe`. Pinned by `scripts/tests/test_verifier_row_get.sh` with a non-vacuity control that
forces the retired resolution and asserts it still misresolves.

⚠ **Its blast radius was ZERO, and saying otherwise here would be writing a false severity into a
gated file.** What the defect broke is the lib's whole FLOOR READER — `floor_value` AND
`floor_version`, the latter embedded by `verifier_row_build` in every published row — not "the floor
stage". But **§2 never calls either**: the only `floor_value stage` caller in the tree is
`scripts/layerb-reference.sh`, which is `#!/usr/bin/env bash`, never zsh-exposed, and which already
degrades with `|| EV_STAGE="probe"` to the same value. The `probe` named in §2's rc-1 bullet is
hardcoded prose, not a runtime read. So this was LATENT with no live consumer, and it is fixed before
it acquires one — which is the reason to fix a shared lib at the root, not a reason to overstate what
it cost.

⚠ **Do NOT "fix" this by executing the lib instead.** Its CLI form accepts **`floor <key>` and
`version` only** — there is no `get` subcommand, so `bash scripts/lib/verifier-row.sh get "$SHA"`
would exit 2 on the usage line and record nothing. ⚠ And note what the HZ-12535 defect did NOT do:
`verifier_row_get`'s own rc was correct in both shells throughout, so the row read and the token this
step books were never wrong. What was lost was the lib's whole FLOOR READER — and, as the paragraph above records, it had no live consumer.

## 3. Store merge, then the git transport — in that order, in a SCRATCH worktree, and the git half is not optional

**The credit line is DERIVED, in the SAME block as the merge.** Nothing is carried in a `$VAR` across
a block boundary (`campaign-watcher` §1: *resolve fresh, carry NOTHING across blocks*) — a `$CREDIT`
that arrived empty would write a merge with no `Reviewer:` line at all.

```bash
. scripts/lib/attestation.sh                                          # the ONE credit-line grammar
. scripts/agent-identity.sh                                           # SOURCE it — a leading dot, because
                                                                      # EXECUTING it would define the fns in
                                                                      # a subshell this block cannot see
AGENT="$(resolve_agent_name)"                                         # Merged-by: the AGENT, never the host
[ -n "$AGENT" ] || { echo "REFUSE — cannot resolve this agent's name"; exit 1; }
APPROVERS="$(attestation_approvers <PROP> 2>&1)"; rc=$?               # 0 derived · 1 the store names NOBODY · 2 CANNOT ASSESS
[ "$rc" = 0 ] || { echo "REFUSE — cannot attest <PROP> (rc $rc): $APPROVERS"; exit 1; }
# ⚠ LATE RE-CHECK (HZ-12023) — the drain's read of `nk pr list` is minutes stale by now and the
# composed-tree gate below is the most expensive in the floor (58m 23s measured). ANCESTRY FIRST,
# THEN the store: only ancestry ANSWERS "is it on main"; `merged_by` records who claimed it first.
git fetch origin main >log 2>&1; rc=$?
[ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: fetch failed, origin/main is stale: $(cat log)"; exit 1; }
git merge-base --is-ancestor origin/<branch> origin/main >log 2>&1; rc=$?   # 0 landed · 1 not · 128 no such ref
case "$rc" in
  0) echo "LANDED — already an ancestor of main, whoever pushed it. Do NOT compose; RECONCILE the record."
     nk pr view <PROP> >log 2>&1; rc=$?                               # direction A: git-landed, store maybe not
     [ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: store unreadable on the LANDED arm (rc $rc); it IS on main, so do NOT compose and do NOT run \`nk pr recover\`: $(cat log)"; exit 1; }
     ST="$(python3 -c 'import json;print(json.load(open("log"))["status"])' 2>log.err)"; rc=$?
     [ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: unparseable store answer on the LANDED arm: $ST $(cat log.err)"; exit 1; }
     case "$ST" in                                                    # `-> merged` is admitted from `approved` ALONE
       merged) : ;;                                                   #  (ProposalStatus::valid_transitions) — so the
       approved)                                                      #  write is ATTEMPTED only where it can succeed,
         nk pr merge <PROP> >log 2>&1; rc=$?                          #  the GRAPH record only, never a second compose,
         [ "$rc" = 0 ] || { echo "REFUSE — the reconcile was REFUSED (rc $rc); it IS on main, so do NOT compose and do NOT run \`nk pr recover\`. Repair the RECORD by hand: $(cat log)"
                            nk pr comment <PROP> --resolved --body "late-recheck (HZ-12023): LANDED; not composed; reconcile REFUSED rc $rc; store was $ST."; exit 1; } ;;
       *) echo "REFUSE — it IS on main, but the record says '$ST' and the store admits '-> merged' from 'approved' ALONE, so \`nk pr merge\` is NOT attempted. Do NOT compose and do NOT run \`nk pr recover\`. By hand: carry '$ST' back to 'approved' (\`nk pr revise\` → re-review → approve), then \`nk pr merge <PROP>\`."
          nk pr comment <PROP> --resolved --body "late-recheck (HZ-12023): LANDED; not composed; record NOT reconciled; store was $ST."; exit 1 ;;
     esac
     nk pr comment <PROP> --resolved --body "late-recheck (HZ-12023): LANDED before compose; not composed; store was $ST."
     exit 0 ;;
  1) : ;;                                                             # not on main — only NOW ask the store
  *) echo "REFUSE — CANNOT ASSESS: is-ancestor rc $rc: $(cat log)"; exit 1 ;;
esac
nk pr view <PROP> >log 2>&1; rc=$?                                    # UNPIPED. An unreadable store is
[ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: store unreadable (rc $rc): $(cat log)"; exit 1; }
ST="$(python3 -c 'import json;print(json.load(open("log"))["status"])' 2>log.err)"; rc=$?   # stderr is NOT the value (CH-12581)
[ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: unparseable store answer: $ST $(cat log.err)"; exit 1; }
# ⚠ THE SUBJECT (HZ-12537) — DERIVED HERE, from the view `log` still holds, because the `else` arm
# below overwrites `log` with `nk pr merge`'s output. `nk pr` has NO title-edit verb, so the store
# `title` is frozen at `nk pr create`; a conclusion that MOVED in review lands a permanently wrong
# subject. The body's `merge-subject:` FIRST LINE is the correction. The VALUE is read with the ONE
# declaration grammar (CH-8403) — `prop_decl_value`, never a private regex, and never
# `prop_decl_token` (the FIRST WORD, LOWERCASED: it would compose `…: m-11072:`). The POPULATION is
# narrowed to that first line, and the paragraph below this fence says why. ABSENT ⇒ the title.
. scripts/lib/prop-declaration.sh                                     # SOURCE it, and CHECK it loaded, as §5 does
command -v prop_decl_value >/dev/null || { echo "REFUSE — CANNOT ASSESS: prop_decl_value is not defined; the shared reader did not load (cwd not the repo root?)"; exit 1; }
TITLE="$(python3 -c 'import json;print(json.load(open("log")).get("title") or "")' 2>log.err)"; rc=$?   # `or ""`: a null title prints `None`
[ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: unparseable store answer (title): $TITLE $(cat log.err)"; exit 1; }
DESC="$(python3 -c 'import json;print(json.load(open("log")).get("description") or "")' 2>log.err)"; rc=$?   # `description`, NOT `body`
[ "$rc" = 0 ] || { echo "REFUSE — CANNOT ASSESS: unparseable store answer (description): $DESC $(cat log.err)"; exit 1; }   # an empty DESC would DROP a declared subject
SUBJ=""
case "$(printf '%s\n' "$DESC" | awk 'NF{print;exit}')" in            # the body's FIRST non-blank line, and only it
  merge-subject:*) SUBJ="$(printf '%s\n' "$DESC" | awk 'NF{print;exit}' | prop_decl_value 'merge-subject' || true)" ;;
esac                                                                 # ⚠ BOTH `awk 'NF{print;exit}'` stages are
                                                                       # load-bearing. Removing the SECOND one looks
                                                                       # like a duplicate and is not: the grammar's
                                                                       # empty-value shadow rescan then reaches a
                                                                       # DECORATED MENTION further down the body and
                                                                       # composes it, reopening the defect this fence
                                                                       # exists to close. Arm S10 pins it.
[ -n "$SUBJ" ] || SUBJ="$TITLE"                                       # the fallback IS the old behaviour
[ -n "$SUBJ" ] || { echo "REFUSE — CANNOT ASSESS: neither a merge-subject: declaration nor a non-empty title"; exit 1; }
echo "subject: $SUBJ"                                                 # SEE it before it is permanent — it is never edited again
FIXNOTE="$([ -s /tmp/ap-<PROP>.fixnote ] && printf '\n%s' "$(cat /tmp/ap-<PROP>.fixnote)")"   # CH-13158: §1 wrote it on the FIX path ONLY — EMPTY on the exact path
if [ "$ST" = merged ]; then   # merged in the store but NOT on main — THREE states; `merged_by` splits them
    MB="$(python3 -c 'import json;print(json.load(open("log")).get("merged_by") or "?")')"
    AGE="$(python3 -c 'import json,time;t=json.load(open("log")).get("merged_at_ms");print(-1 if not t else int(time.time()-t/1000))')"
    if [ "$MB" = "$(resolve_session_name)" ]; then                    # (a) THIS session's own half-run transport
        echo "RESUME — this session recorded the store merge; the git half never finished. Compose below."
    elif [ "$AGE" -ge 0 ] && [ "$AGE" -lt 21600 ]; then               # (b) a PEER, inside a plausible window
        echo "STOP — $MB's transport is IN FLIGHT (${AGE}s ago). Do not compose. Do NOT run \`nk pr recover\`."
        nk pr comment <PROP> --resolved --body "late-recheck: stopped, merged_by=$MB age=${AGE}s"; exit 0
    else                                                              # (c) stale, or NO timestamp to judge by
        echo "REFUSE — store-merged by $MB (age ${AGE}s; -1 = no merged_at_ms, NEGATIVE = clock skew, not absence), still not on main: the CH-4720 shape. Investigate by hand."
        nk pr comment <PROP> --resolved --body "late-recheck: split-state suspected, merged_by=$MB age=${AGE}s"; exit 1
    fi
else
    nk pr merge <PROP> >log 2>&1; rc=$?                               # the GRAPH record only — AFTER the derivation
    [ "$rc" = 0 ] || { echo "REFUSE — \`nk pr merge\` was REFUSED (rc $rc); the branch is NOT on main, so do NOT compose: $(cat log)"; exit 1; }
fi
git worktree add --detach /tmp/ap-<PROP> origin/main   >log 2>&1; rc=$?   # A stranded tree from a
[ "$rc" = 0 ] || { echo "REFUSE — no scratch tree: $(cat log)"; exit 1; }   # prior run fails HERE; without
                                                                             # the rc every later -C uses it
git -C /tmp/ap-<PROP> merge --no-ff origin/<branch> -m "Merge <PROP> from <branch>: ${SUBJ:?REFUSE — the merge subject was not derived IN THIS SHELL; §3 derives it above and a shell var does not survive a tool-call boundary}
${FIXNOTE}
$(attestation_line "$APPROVERS" "$AGENT")" >log 2>&1; rc=$?           # e.g. Reviewer: Air/sub-PROP-6255-r1  Merged-by: DGX1
[ "$rc" = 0 ] || { echo "REFUSE — the compose CONFLICTED (rc $rc). The STORE merge is already recorded, so this is a RECONCILE, not a retry: rebase or merge origin/main into the branch, get it re-reviewed, and do NOT push. $(cat log)"
                   git -C /tmp/ap-<PROP> merge --abort 2>/dev/null; git worktree remove --force /tmp/ap-<PROP>; exit 1; }
# NO RUST DELTA ⇒ NO cargo check (Captain 2026-10-08, the default): the composed delta HEAD^1..HEAD is
# judged by the SAME classifier the pre-push's stage 3e uses. FAIL CLOSED — only rc 0 AND an exact
# `rust-delta: no` line skips; a crash, a missing script or any other answer runs the check.
RD="$( cd /tmp/ap-<PROP> && python3 scripts/lib/rust-delta.py HEAD^1 HEAD 2>&1 )"; rd_rc=$?
RD_NO="$(printf '%s\n' "$RD" | grep -xE 'rust-delta: no')" || true   # capture form, never a pipe into grep -q
if [ "$rd_rc" = 0 ] && [ -n "$RD_NO" ]; then
    GATE4A=nothing_to_sweep; echo "composed tree: no rust delta — cargo check SKIPPED; §5 records gate4a --rc 0 --reason nothing_to_sweep"
else
    ( cd /tmp/ap-<PROP> && cargo check --workspace )       >log 2>&1; rc=$?   # the composed tree, not the branch
    [ "$rc" = 0 ] || { echo "REFUSE — the composed tree is RED (rc $rc): do NOT push. The STORE merge is already recorded, so this is the CH-4720 split state: run §4 against /tmp/ap-<PROP> BEFORE reaping (it records ancestry --rc 1 --reason unlanded), then record gate4a (§5) by WHICH red this is. FIRST: --rc 2 --reason cannot_assess when the check reached no verdict — rc >= 128, OR a SIGKILL anywhere in the log FILE, not just the tail below (grep -F 'signal: 9, SIGKILL' log: earlyoom kills rustc/ld, and cargo then exits 101); any other signal is re-run before it is booked. Otherwise this fence measured the COMPOSED tree only, so check both parents before choosing, each in its OWN scratch tree with its OWN CARGO_TARGET_DIR (HZ-12365): /tmp/ap-<PROP>-main at origin/main and /tmp/ap-<PROP>-branch at origin/<branch> alone, never the tree you stand in and never a shared target dir (a parent-check tree is NOT the transport tree: only /tmp/ap-<PROP> takes the ONE host-shared dir of the HZ-12061 F5 paragraph, and these two live beside it), reaped with their target dirs once read: --rc 1 --reason red ONLY when both are green; --rc 2 --reason pre_existing_debt when a parent is ALREADY red; --rc 2 --reason unattributed while a parent is unmeasured or mixed. Then reap the tree and stop. There is NO branch-side repair: a store-merged <PROP> cannot be revised, and a fix pushed to <branch> moves its tip past the approver's reviewed-at-sha (§1 refuses that as DRIFT — CH-13158's approve-with-fixes path is no way round it: its floor runs in §1 BEFORE the store merge, for the findings the approver named, and this red is neither — and a §3 entered directly would land it UNREVIEWED). Land the fix through its OWN reviewed proposal, routed by WHERE THE FIX LIVES, not by whose red it is: on main ONLY when the fix touches main's code alone, after which RESUME re-composes against the repaired main; cut from origin/<branch> with main merged in whenever the fix touches the branch's code (the branch's own red, AND usually an interaction red: main changing an API the branch's new code calls), whose landing carries <PROP>'s commits so the next pass takes the LANDED arm and reconciles. RESUME is a same-session retry: cannot_assess needs no fix, only a RESUME once memory is free; a fix that outlives this session or the 6-hour window is repaired by hand from the stale arm: $(tail -20 log)"; exit 1; }
    GATE4A=clean
fi
git tag -f checkpoint/pre-<PROP> origin/main
git -C /tmp/ap-<PROP> push origin HEAD:main            >log 2>&1; rc=$?
```

**On the FIX path the message carries one line more** (CH-13158): between the subject and the credit
line, `reviewed-at-sha <SHA>; r1 findings fixed at <FIX-SHA>` — the approved sha and the fixed tip
§1's `fix_scope_check` admitted, read back from `/tmp/ap-<PROP>.fixnote`. On the exact path the file
does not exist (§1 deletes it first) and the message is byte-for-byte what it was.

⚠ **THE MERGE rc IS TESTED, AND IT WAS NOT UNTIL HZ-12646.** Every other step in this fence carried
an rc check — the worktree add, `cargo check`, the push — and the compose did not. On a conflict the
consequence is not a loud failure but a SILENT SUCCESS, reproduced end to end in a throwaway repo:

```text
git merge --no-ff other        rc 1        <- nothing tested this
git status --porcelain         1 unmerged path
git rev-parse HEAD             UNCHANGED   <- the merge did not commit
the push at the end of the fence
                               rc 0        "Everything up-to-date"
```

**The push is a NO-OP that exits 0.** The transport reports success, `main` is untouched, and
`nk pr merge` has ALREADY recorded the graph — the store says merged, git says it is not on main.
**That is the CH-4720 split-state this same skill gives a three-way recovery for**, created by its
own fence.

⚠ **The workspace check does not save it, and cannot.** On a diff with an EMPTY rust delta — which
is every proposal Mini transports, by the rust-delta bar — the check runs green on an unmerged tree,
because nothing in the conflict is Rust. The one gate between the merge and the push is blind
to the failure by construction for a whole class of proposals.

⚠ **It needs only a stale base plus a conflicting peer landing**, which is ordinary: PROP-6482 landed
a `battery-wiring.conf` row immediately above the row PROP-6483 rewrote, and `git merge-tree` went
rc 1. That proposal was caught by a REVIEWER, not by this fence.

⚠ **The prose above deliberately avoids writing the literal push and workspace-check commands.**
`scripts/tests/test_approveit_compose_isolation.sh` COUNTS every occurrence of those strings in this
file and requires each to be scoped to the scratch worktree — a grep-based guard cannot tell an
example from an instruction, and it is right to err that way, because an unscoped one is the HZ-12000
defect. Writing them out here took the battery from 20/0 to 18/2. Describe them; do not spell them.

The refusal names the state rather than just the rc, because the remedy differs from the `cargo
check` one: a red composed tree is a code problem, a conflicted compose is a BASE problem, and in
both cases the store merge is already recorded so neither is a retry. It aborts the merge and reaps
the scratch tree before stopping — §6's reap is only reachable after the push.

**The SUBJECT is the PROPOSAL's, not the store's frozen title (HZ-12537).** `nk pr` has no
title-edit verb, so `title` is fixed at `nk pr create`, and a multi-round review whose CONCLUSION
moves cannot correct it. `bcc1921f1c` (PROP-6421) is the measured instance: the landed subject says
the measure identified the binding constraint and falsified its own pre-registered reading; the
artifact it landed says `settles: null` and `R1 NOT SETTLED BY THIS MEASURE`; and the corrected
subject was sitting in the body as `merge-subject:`. Before this item `grep -c 'merge-subject'` on
this file answered **0** — the convention had no reader, so it held only where the transporting
session happened to have read a six-round review thread. That is HZ-12022's shape one field over: a
merger writing a string nothing derives. ⚠ **So write your own `merge-subject:` as the body's FIRST
LINE whenever the title no longer says what the work concludes** — it is the only place the
transport looks, and `nk pr edit`'s superseded half is cut before the read, so the CURRENT
declaration is the one that lands.

⚠ **THE POPULATION IS THE FIRST NON-BLANK LINE, AND THE NARROWING IS THE POINT — pair review
BLOCKING-1, reproduced end to end.** Read over the WHOLE body, the shared grammar composes a MENTION
as the subject, and the mention outranks the real declaration. `PROP_DECL_DECOR` admits `-`, `*`,
backtick, `#` and whitespace, and `_prop_decl_is_quoted_mention` demotes only a line that OPENS with
a backtick, so a bulleted one does not qualify. Measured, driven through this fence:

```text
body line 1:  - `merge-subject:` is read through the shared grammar (CH-8403), never a private regex
body line 3:  merge-subject: HZ-12537: THE REAL DECLARED SUBJECT
composed:     Merge PROP-9001 from x1: is read through the shared grammar (CH-8403), never a private regex
```

That is this item's own defect through a different door, and **strictly worse than the status quo**,
which would have composed the title. The body most likely to carry such a line is a proposal that has
to EXPLAIN the convention — this one.

**So the POPULATION is narrowed and the GRAMMAR is not.** The candidate is the body's first non-blank
line, admitted only when it BEGINS with `merge-subject:`; `prop_decl_value` still produces the value
(the trim, the emphasis stripping, the superseded cut). The narrowing can only ever REFUSE — it never
invents a value, and everything it refuses falls back to the title, which is what shipped before this
item. ⚠ **The cost, stated rather than discovered later: the decorated spellings CH-8403 measured
(`**merge-subject:**`, `- merge-subject:`, `## merge-subject:`) are NOT honoured for this field**, and
a declaration on any line but the first is not honoured either. Both cost today's behaviour, never a
wrong subject. This is the one consumer of the grammar whose MISREAD IS UNRECOVERABLE — every other
one writes a row or a refusal that can be corrected, while this one writes a string onto `main` that
`nk pr` has no verb to edit, which is the whole subject of this item. Tolerance is right for them and
wrong here.

⚠ **`echo "subject: $SUBJ"` is not decoration.** Before it, the fence composed and pushed the subject
blind: nothing printed it, and the pre-push hook checks the `Reviewer:` line and not the subject, so
the first time anyone saw it was in `git log` after it was permanent. One line, no new mechanism, and
it is what makes the residue above — a first line that is itself a mention — visible before the
58-minute composed-tree gate rather than after the push.

⚠ **The reader's own absence is asserted, not absorbed.** `. scripts/lib/prop-declaration.sh`
followed by `command -v prop_decl_value` is one line of belt for the reasons §5 already gives about
the identical pair of sources: both are RELATIVE paths resolved against the cwd, which is shell state
that does not persist across the separate tool calls a skill is executed in, so driven from anywhere
but the repo root the source fails; and under macOS `/bin/sh` the file's `done < <(…)` is a syntax
error, so the function never defines even from the repo root. Without the check `prop_decl_value` is
simply undefined, the command substitution yields nothing, and an empty `$SUBJ` is
INDISTINGUISHABLE from "this body declares no subject" — the transport would compose the stale title
while reporting nothing wrong. An instrument that cannot run answers CANNOT ASSESS. (The same holds
for a `title` the store returns as `null` or `""`: `.get("title") or ""` keeps it out of the string
`None`, and the second `[ -n "$SUBJ" ]` refuses rather than composing a subject that is nothing.)

**An absent declaration falls back to the title WITHOUT a refusal, and a declaration that DIFFERS
from the title is used without being flagged as different.** The alternative considered and rejected
was to REFUSE on a `merge-subject:` that differs from the title. It buys nothing in either direction.
A difference is the declaration's WHOLE PURPOSE, so the only sound response to one is to use it —
refusing on it would refuse the very case the reader exists to serve, i.e. it would refuse PROP-6421.
And the residual hazard — a title that is stale beside a body that declares nothing — has nothing to
compare against, so no comparison can see it: the differs-check adds zero coverage of the thing that
actually goes wrong, a control that tests the MECHANISM ("a declaration was read") rather than the
PREDICATE ("the subject is true of the work"). What a refusal WOULD add is a new way to stop a
CORRECT transport, and a false refusal there is worse than the hole, because the remedy the operator
reaches for is to stop running the gate (CH-9839; `test_approveit_compose_isolation.sh`'s round 9
re-measured the same direction). ⚠ **That argument is about REFUSING on a difference and does NOT
extend to showing the operator what is about to land** — an earlier draft of this paragraph made that
slide, pair review caught it, and the `echo` above is the correction. The reader is
`scripts/tests/test_hz12022_attestation.sh` section S — S1 the differing body, S2 the absent one (the
WHOLE message compared, so the credit half cannot be disturbed to fix the subject), S3 a prose
mention, S6 a BULLETED mention above the real declaration (BLOCKING-1's own fixture), S7 a
declaration below the first line, S8 a null and an empty title (refused BEFORE the store write), S9
the announcement, S5/S5b the reader's own absence, M8/M8b the mutation control — and S10-S14 and
M9/M9b, below.

⚠ **Every line of this fence is pinned against DELETION, or says here why it is not (CH-12552 — the stopping test).**
This fence reached four review rounds of one shape: each fix added a defensive line, and the line
was unpinned — the `${SUBJ:?}` guard (S11), then the `SUBJ=""` that guard's premise reads (mutant Z
deleted it and survived 137/0 while composing ANOTHER proposal's subject from an inherited `SUBJ`;
S12 now, with M9 proving the arm depends on the line). **An arm that pins a guard does not pin the
state the guard reads.** So the population was enumerated rather than sampled: every non-blank,
non-comment line of the fence was deleted, ONE at a time, in a copy passed as the approveit argument
of `test_hz12022_attestation.sh`, `test_approveit_compose_isolation.sh` and
`test_hz12023_late_recheck_arms.sh` (ignoring S14 below, which reds on EVERY deletion by
construction). The lines whose deletion left all three green were the guards
no arm ever drives, because every arm takes the GOOD path where a guard never fires — the agent-name
refusal, the attestation rc refusal, the status-parse refusal, the empty-subject refusal (whose
deletion was masked by the `${SUBJ:?}` guard further down, which refuses the same answer only AFTER
`nk pr merge` has written the store: the CH-4720 split state), the scratch-tree refusal, and the push
itself. S13 injects each one's failure and requires the refusal BY THAT GUARD (its own `REFUSE` text)
BEFORE the write it protects; S8 and S2p carry the other two. ⚠ **The title-parse refusal was first
dispositioned here as UNREACHABLE, and pair review disproved it:** under an interpreter whose stdout
cannot encode the title (`PYTHONIOENCODING=ascii`, a Latin-1 locale — python3 falls back to UTF-8
only under C/POSIX), printing a title containing `—` fails while the ASCII status parses, and with the
refusal deleted the block composed `Merge PROP-x from <branch>: Traceback (most recent call last):` and
pushed it. S13/titleenc pins it. The same fault reached the description read, which had NO refusal: an
empty `DESC` silently dropped a declared `merge-subject:` and landed the title. It now refuses, and
S13/descenc pins that. One line needs no arm, and this is why:

- `git tag -f checkpoint/pre-<PROP> origin/main` is a LOCAL rollback marker. Nothing reads it (0
  hits outside this line), it is never pushed (the push is `HEAD:main`), and deleting it changes no
  record, no ref on origin, and no refusal.

⚠ **Deletion is the population swept, not every mutant.** The mutants that started this family (the
`:?` spelling, the second `awk` swapped for `head -n1`, a python read's `2>log.err` put back to
`2>&1`) were SUBSTITUTIONS, and each has its own arm (S11/S11d, S10/S11c, S15a/S15b and the late
re-check's `stderr warning` cases); a substitution nobody has named is not covered by this list.

The push's own `rc` is deliberately not read here: §4's ancestry check, on the scratch tree's `HEAD`,
is the refusal for a push that did not land, and a second one here would be a drift surface.
**The composed-tree `rc` IS read** — the fence used to capture it and push anyway, leaving the
refusal to the pre-push hook's second `cargo check`; it now refuses in place (S13/redtree). ⚠ By then
`nk pr merge` has recorded the store merge, so its message sends you through §4 BEFORE the reap: §4
reads the scratch tree and records the `unlanded` row, and a tree reaped first leaves no row.

⚠ **That refusal names WHICH red, and the ROUTE back — both were missing (CH-12581, PROP-6445's
NOTE-1 and NOTE-3).** It used to say `record gate4a --rc 1 --reason red` for every red. §5's gate4a
vocabulary (`scripts/layerb-decisions.conf`) books `red` only for a regression clean on BOTH parents;
a parent ALREADY red is `pre_existing_debt`, a composed red whose parents are unmeasured or mixed is
`unattributed`, and a check that reached no verdict is `cannot_assess` — the last three RECORDED at
rc 2. This fence measures the composed tree ONLY, so a `red` or `pre_existing_debt` row needs both
parents checked first — and checked SOMEWHERE SOUND, which the first cut did not say (CH-12586 note
3): each parent in its OWN scratch tree (`/tmp/ap-<PROP>-main` detached at `origin/main`,
`/tmp/ap-<PROP>-branch` at `origin/<branch>`) with its OWN `CARGO_TARGET_DIR`. Checking
`origin/<branch>` in the tree you stand in is HZ-12000's false row again, and two parents sharing one
target dir is HZ-12365's; either gives a parent verdict about a tree nobody built. Reap both trees and
both target dirs once the verdicts are read, on the same terms as `/tmp/ap-<PROP>` below. On a
Spark `cannot_assess` is live, and its tell is NOT only the rc: earlyoom kills the largest process, which is `rustc` or `ld` rather than `cargo`, and cargo then exits **101**
with `(signal: 9, SIGKILL: kill)` in its log (pair-review-measured with a self-SIGKILLing
`RUSTC_WRAPPER`); only a kill of cargo itself gives rc ≥ 128. The tell is read from the whole `log`
file, because parallel jobs can push the signal line out of the printed tail, and it is SIGKILL
specifically: a reproducible SIGSEGV in rustc or a build script is a real red, so any other signal is
re-run before it is booked. The message names all four tokens and
both tells — S13/redtree-book; and S13/redtree-logfile has a stub cargo write the SIGKILL line to
STDERR 25 lines above the end at rc 101, then requires that the printed tail does NOT carry it and
that the grep the message prints, on the file it names, DOES find it (CH-12586 note 2: an rc-101 pass
over the rc-137 needles re-checked only the `(rc N)` header). ⚠ Here the conf's rc-2 disposition
("proceed, not a clean pass") and the transport DIFFER, deliberately: the row records what the gate could say, and the push still stops
because a push would carry a tree nobody saw green.

"RESUME only once the tree is green" named no way there, and the obvious way is UNSOUND: a
store-merged proposal cannot be `nk pr revise`d, and a commit pushed to its branch moves the tip past
the approver's `reviewed-at-sha:` — §1 refuses that as DRIFT with no revise route left. The
approve-with-fixes path (§1, CH-13158) does not reopen it: that path admits only a fix of the findings
the approver NAMED, with its floor run on the fixed tip BEFORE `nk pr merge`; after the STORE merge
there is no fix path, and a red composed tree is not a finding anyone approved a fix for. And a §3
entered directly (it fetches `main` only, so this needs a hand re-fetch of the branch) lands it
UNREVIEWED. **The reviewed route is a fix through its OWN proposal, routed by WHERE THE FIX
LIVES, not by whose red it is** (CH-12586 note 1): on main ONLY when the fix touches main's code alone, after which RESUME
re-composes against the repaired main; cut from `origin/<branch>` whenever the fix touches the
branch's code — that landing carries `<PROP>`'s commits, so the next pass finds the branch an
ancestor and takes the LANDED arm, never RESUME. ⚠ The first cut routed every INTERACTION red to
main, and the usual interaction red is main changing an API that the branch's NEW code calls: that
code does not exist on main, so a fix cut from main usually cannot reach it and the attempt is
wasted (nothing lands unreviewed either way). A fix that restores or shims the API on main touches
only main's code and is still routed to main. Such a fix is cut from `origin/<branch>` with main merged
in. And RESUME is the same-session retry the table below describes: `cannot_assess` needs no fix, only a RESUME once
memory is free, while a fix that outlives this session or the 6-hour window is repaired by hand from
the stale arm's refusal — S13/redtree-route.

⚠ **Every value this fence reads out of python captures STDOUT ONLY (CH-12581, PROP-6445's NOTE-2).**
The two status reads, the title read and the description read each captured `2>&1`, so anything the
interpreter prints on stderr at rc 0 — a `DeprecationWarning`, a site hook — became part of the
value. In `DESC` it becomes the body's first non-blank line, the `merge-subject:` case stops matching,
and the declared subject is SILENTLY swapped for the title; in `TITLE` it is composed onto main; in
`ST`, `merged` stops equalling `merged`. Stderr now goes to `log.err`, which each refusal prints, so
a real parse failure still shows its traceback. S15a/S15b (the subject) and the two `stderr warning`
cases of `test_hz12023_late_recheck_arms.sh` (both status reads) inject the warning through a real
`sitecustomize.py` and each reds on its read's `2>&1` spelling; S13/nostatus-trace requires the
refusal to still carry python's traceback, so a `2>/dev/null` spelling of THAT read (the status read
below ancestry) reds too; the other three reads' diagnostics are not pinned.

⚠ **The fence has 81 executable lines.** S14 counts them (non-blank, first field not a comment) and
REDS when that number and this sentence disagree. That is a tripwire, not a proof: when it fires,
re-run the one-line deletion sweep above over the new line(s), pin or disposition each one HERE, then
change the number — a fifth round costs more than a list.

⚠ **HZ-12646 added the two lines that took this from 70 to 72, and they are dispositioned here as the
tripwire demands** — the count was changed only after the sweep, not instead of it:

⚠ **These two lines are ONE BRACE GROUP, so the one-line deletion protocol does not apply to them
and the column below says REMOVE THE GUARD, not delete the line.** Measured: dropping either line on
its own is a PARSE ERROR under `/bin/bash` 3.2 — `syntax error near unexpected token '}'` for the
first, `unexpected end of file` for the second — so a literal one-line sweep would produce a bash
failure and learn nothing about the guard. Each row below was verified by SEMANTIC deletion: remove
both lines together, or neuter the action inside the group, and observe. S14's remedy says "re-run
the one-line deletion sweep over the new line(s)"; for a multi-line construct, sweep the CONSTRUCT.

| new guard | remove it and what happens |
|---|---|
| `[ "$rc" = 0 ] \|\| { echo "REFUSE — the compose CONFLICTED (rc $rc)..."` | the conflict goes untested again: an unmerged tree reaches the workspace check, which passes on an empty rust delta, and the push then exits 0 having done nothing — with the store merge already recorded. **This is the line the item is about.** |
| `git -C /tmp/ap-<PROP> merge --abort ...; git worktree remove --force ...; exit 1; }` | the refusal still fires, but leaves a CONFLICTED scratch tree registered. The next transport of the same PROP dies on *"already exists"* and the registration counts against the cap (HZ-9661). §6's reap is only reachable after the push, which is why the refusal must reap for itself. |

The merge line itself gained `>log 2>&1; rc=$?` and stayed one line, so the one-line protocol DOES
apply to it: deleting that suffix restores the original defect exactly, which is the first row above.

⚠ **The no-rust-delta skip (Captain 2026-10-08) added the seven lines that took this from 74 to 81**,
swept with the three batteries above (each mutant passed as the approveit argument;
`test_hz12022_attestation.sh` as its `$3`). M4 is already red on `origin/main` and S14 reds on every
edit by construction, so both are excluded. `if`/`else`/`fi` form ONE construct, so it was swept
SEMANTICALLY, as the HZ-12646 brace group was:

| mutant | result | disposition |
|---|---|---|
| condition forced to `if true` (skip ALWAYS) | **RED**: S13/redtree, redtree-book, redtree-logfile, redtree-route | pinned. A red composed tree can never be skipped past |
| delete `RD="$( … rust-delta.py HEAD^1 HEAD 2>&1 )"; rd_rc=$?` | green | fail-closed by construction: `RD_NO` comes out empty, so the check RUNS. That is the pre-2026-10-08 behaviour, never a false skip |
| delete `RD_NO="$(… grep -xE 'rust-delta: no')" \|\| true` | green | the same: an empty `RD_NO` runs the check |
| condition reduced to `[ "$rd_rc" = 0 ]` alone, or to `[ -n "$RD_NO" ]` alone | green | the two conjuncts are redundant ON PURPOSE: `rust-delta.py` exits 0 only after printing `rust-delta: no` (`out('no')`). Either alone holds today; both together survive a classifier that one day prints `no` on a non-zero exit |
| `GATE4A=nothing_to_sweep` or `GATE4A=clean` neutered | green | read only by §5's row. An unset `GATE4A` makes `verdicts record --reason ""` refuse (rc 2, writes nothing), which §5's "gate every line" turns into a REFUSE. Unpinned here, and caught at the writer |

⚠ **Not pinned, and said so:** no arm drives the SKIP path itself. The harness's stub tree cannot run
`rust-delta.py`, so every S-arm takes the check-runs branch. A future arm that puts a real classifier
in the stub tree (no rust delta ⇒ no `cargo` call, gate4a `nothing_to_sweep`) is the remainder.

⚠ **CH-13158 added the two lines that took this from 72 to 74** (the fix path's merge-message line),
swept one line at a time:

| new line | delete it and what happens |
|---|---|
| `FIXNOTE="$([ -s /tmp/ap-<PROP>.fixnote ] && printf '\n%s' …)"` | the FIX path's merge loses its `reviewed-at-sha <SHA>; r1 findings fixed at <FIX-SHA>` line — an audit line, not a guard: nothing refuses on it, and no arm here has a fix-path fixture, so it is UNPINNED by this battery (the fix path's gate is §1's `fix_scope_check`, pinned by `scripts/tests/test_ch13158_fix_scope.sh`). The exact path is unaffected: the value is empty there either way. |
| `${FIXNOTE}` (the line between the subject and the credit line, where a blank line was) | the credit line joins the SUBJECT paragraph (`subject\nReviewer: …`), so `git log --oneline` and every `^Merge PROP-` subject reader see a two-line subject. S2's whole-message comparison REDS on it. On the exact path this line expands to nothing, so the message stays byte-for-byte `subject`, blank line, credit line. |

⚠ **Compose in a scratch worktree, never in the tree you are standing in.** Floor row 7 says the
composed tree is trial-merged *"in a scratch worktree"*, and `reviewit` §2 already does. HZ-12000
measured the cost on M5: a second session checked the tree out from under an in-flight gate4a that had
cleared ~324 crates. **A green rc there would have been a FALSE gate4a row on a tree that was never
checked** — a silently unsound Layer-B row, not a crash.

⚠ **And run every gate with `cd` INSIDE that scratch tree, exactly as the fence spells it** —
`( cd /tmp/ap-<PROP> && … )`, never `bash /tmp/ap-<PROP>/scripts/…` from where you are standing.
`scripts/mini-disk-guard.sh`'s liveness probe is `lsof -d cwd`, so a scratch tree with no process whose CWD is inside it reads IDLE and is reapable at the CRIT band. That is a
SECOND way to get HZ-12000's false row, from the other direction: HZ-12000 is a tree pulled out from
under you by another SESSION, this is the same tree pulled out from under you by the DISK GUARD, and
a gate whose tree evaporates between a green `cargo check` and the row it writes is unsound in
exactly the same way. Measured on Mini, 2026-09-15: every `/tmp/rv-*` and `/tmp/redfirst-*` tree on
the host was deleted inside one guard window while gates ran against them. `reviewit` §2 carries the
same rule for the review side.

⚠ **The path is a LITERAL on every line — never `WT=…` carried across blocks.** `git -C ""` is a
documented no-op: the man page says *"the current working directory is left unchanged"*, so it runs in
whatever tree you are standing in. Measured at rc 0 from two:

```text
from the shared checkout   git -C "" rev-parse --show-toplevel  ->  the shared checkout
from ~/fleet-wt/<agent>    git -C "" rev-parse --show-toplevel  ->  that worktree, HEAD = a FEATURE BRANCH
```

Since HZ-12006 the resident works in `~/fleet-wt/<agent>`, so the realistic failure is the second row:
an unset variable turns the push into a bare push of **a feature-branch tip onto `main`**. Do not
expect the `cd` half to catch it — that safety net is bash-only:

```text
bash -c 'cd ""'   rc 1   "cd: null directory"            fails CLOSED
zsh  -c 'cd ""'   rc 0   stays in the current directory   fails OPEN
sh   -c 'cd ""'   rc 0   stays in the current directory   fails OPEN
```

The fleet's shell is zsh, so under it BOTH halves fail open together and the transport is silently
unsound end to end. The remedy is the literal, not a guard.

**Reap the tree on EVERY exit, not just the happy one.** §6's `git worktree remove` is reachable only
after the push succeeds, ancestry is rc 0 and the rows are written. On a red gate4a, a second push
rejection, or the §4 ancestry stop, run `git worktree remove --force /tmp/ap-<PROP>` before you stop —
otherwise the next transport of the same PROP dies on *"already exists"* and the registration counts
against the cap (HZ-9661). ⚠ If you set a PER-TRANSPORT `CARGO_TARGET_DIR`, delete it on those SAME
exits: `git worktree remove` does not touch it, and the measured cost is ~472 MB of worktree plus
~2.1 GB of target per transport — on the bus host a full disk takes the kanban store down with it.
(`--force` on §6's line is not decoration, but the usual reason given for it is wrong: `target/` is
gitignored, so a worktree holding only that removes at rc 0 — measured both directions here. What
actually refuses at rc 128 is a genuinely UNTRACKED, un-ignored file in the tree. ⚠ **And §3's own
`>log` lines are NOT how one gets there** — the example this paragraph shipped was wrong about its
own mechanism (HZ-12099 F4): the redirection is opened by the OUTER shell, in the OPERATOR's cwd.
Measured — `( cd /tmp/ap-<PROP> && cargo check --workspace ) >log` writes `log` beside YOU and
leaves the scratch tree clean (0 untracked paths, `git worktree remove` rc 0). The stray file
arrives when the redirect sits INSIDE the scope, one token left of where §3 puts it:
`( cd /tmp/ap-<PROP> && cargo check --workspace >log 2>&1 )`, or a bare `>/tmp/ap-<PROP>/log`. (The python reads'
`2>log.err` is the same shape as `>log` — opened in the operator's cwd, never inside the scratch
tree. ⚠ So BOTH are left untracked in THAT cwd, and neither is in an ignore list (CH-12586 note 4).
They are not moved to `/tmp` here: every refusal in §3's fence (about 30 lines, the red-tree
stop's SIGKILL tell among them), S14's line count and the S harness read them by that relative name,
so a move is a fence rewrite, not a fix to one note (§1b, §2 and §6 already log under `/tmp`). The cleanup is yours, from the same cwd, AFTER you
have read them: `rm -f log log.err` on the TWO exits that read them last — after §4 and the gate4a row
on the red-tree stop, and after §6's reap on the happy path (S16 pins that line). Every OTHER exit —
every other exit of §3's fence (its refusals, the LANDED arm's exit 0 and the in-flight STOP's exit 0), a push rejection, the §4 ancestry stop, a §5 refusal, §6's
carrier refusal — owes no cleanup: the next §3 run truncates both files before any refusal reads
them, so a stale pair is litter in your cwd, never a wrong answer; `rm -f` there is optional.)
And `log` is in nobody's ignore list (`check-ignore` rc 1), so THAT spelling measures `?? log` in the
scratch tree and `git worktree remove` at rc 128 — *"contains modified or untracked files, use
--force to delete it"*. Any process writing a RELATIVE path while its cwd is that tree lands in the
same state by the same measurement — no stage of the hook REDIRECTS there (they all redirect to an
absolute `/tmp/pp-*` or an `$(mktemp)`), though each stage `cd`s into the tree first, so a child that
writes a relative path of its own still would. `--force` covers it.)

⚠ **WHICH WAY to point `CARGO_TARGET_DIR` — naming it a hazard is not advice (HZ-12061 F5, which
also records that a round-6 disposition claimed this sentence already said which way, and it did
not).** Point it at ONE dir that is HOST-SHARED and ABSOLUTE, never at one per transport (this governs the
TRANSPORT tree `/tmp/ap-<PROP>` only — the red-tree stop's per-parent check trees are DIFFERENT trees
living beside it, so each takes its OWN dir, the first of the HZ-12365 remedies below, which the fence mandates; the two rules do not conflict):
`$HOME/.cache/nusy/arch-guard-target` (the arch-guard cache —
`scripts/hooks/lib/arch-target-dir.sh` precedence 2, and on the bus host the one target dir
`mini-disk-guard.sh` names and CAPS, CH-10500), or `$NUSY_MPB_ARCH_TARGET_DIR` where the host sets
one. `/tmp/ap-<PROP>` is FRESH on every merge, so its own `target/` is cold — and the hook you are
about to be gated by runs the workspace `cargo` check and `arch-guard` *in that tree*
(`scripts/hooks/pre-push:310`, `:312`) while sourcing `arch-target-dir.sh` nowhere, so nothing points
them anywhere warm on your behalf. CH-10457 measured that gate cycle at **~13 min cold against 62 s
warm, and cold LOST THE PUSH RACE TWICE**: the cost is not the minutes, it is `origin/main` advancing
underneath you. A per-transport value is worse than no value, because `$CARGO_TARGET_DIR` outranks
that lib for the consumer that does read it (`scripts/layerb-reference.sh:395`) — so setting one per
transport switches the host-shared cache OFF for the `arch_guard` decision too. The shared dir is the
exception to the deletion rule above: leave it, since deleting it only buys the next transport a cold
build.

⚠ **And the one condition on that, or you trade a slow gate for an UNSOUND one.** A shared dir is
safe only while no OTHER TREE'S LIFETIME overlaps this one's — **not merely while nothing else is
building** (HZ-12365). Serialising builds does not satisfy it: cargo's fingerprint compares source
mtimes against the dep-info in the target dir, so a tree whose files are OLDER than another tree's
last build is judged `Fresh`, with no overlap in time required. ⚠ **A run QUEUED behind a
one-build-at-a-time rule is created early and built late — exactly the failing order** — so a
concurrency-keyed reading asserts soundness in the arrangement queueing makes most likely. Measured
on DGX2 with strictly SEQUENTIAL builds, and again unplanned on Mini 2026-09-15 with ZERO live
`cargo`/`rustc`. That is why the test below is `git worktree list` and not a process check. HZ-11832 measured two trees at
DIFFERENT main tips on one shared dir: cargo judged the older tree's rmetas fresh, and clippy
reported `E0560: no field foss_sync_load_failed` on a field that tree's own sources define. That
direction is a false RED and loud; **the same mechanism yields a false GREEN** — a gate4a row for a
merged state nothing ever compiled, which is HZ-12000's failure arriving through a second door. So
read `git worktree list` first: if another scratch tree is live (a concurrent `/tmp/rv-*` review,
another `/tmp/ap-*`), either give THIS transport its own `CARGO_TARGET_DIR` and pay the cold build,
or `cargo clean -p <the crates the branch touches>` in the shared dir before §3's check. `reviewit`
§2 carries the same rule for the review side.

**Re-check ancestry BEFORE spending the gate.** Measured (HZ-12023): M5 and DGX1 both transported
PROP-6101 on 2026-09-09. M5 composed and ran the composed-tree gate to completion — **58m 23s**,
rc 0 — and only then discovered DGX1 had already landed it. The store had the answer the whole time
and nothing looked. This does not PREVENT the duplicate work: two sessions can still pass this check
seconds apart, and the real fix is a `claim-merge` CAS mirroring `claim-review` (HZ-12023 remainder,
which must be designed WITH the release/TTL story `claim-review` still lacks — HZ-11951). It bounds
the loss from ~1 hour to ~1 fetch, which is the argument CH-11934 already won for the review step:
move the refusal to the START of the expensive thing.

⚠ **ANCESTRY IS CONSULTED FIRST, and that ordering IS the guard.** `merged_by` records who
claimed the STORE merge; only `--is-ancestor` answers whether the branch is on main. The two come
apart, and the first one asked decides the outcome. Measured 2026-09-12 on PROP-6071:

```text
03:07:23.040Z  M5 ran `nk pr merge PROP-6071`  ->  store merged_by = M5/s-429224c9  (THIS session)
03:08:26Z      Air pushed c9013effb0 of the SAME branch, credit line `Merged-by: Air`
afterwards     store STILL says M5/s-429224c9   BUT the branch IS an ancestor of origin/main
```

Ask `merged_by` first and that reads "my own transport half-ran — RESUME it", so the session composes
and pushes a **duplicate merge of a branch that is already landed**. Ask ancestry first and it is the
LANDED arm: stop, reconcile the record, take the next proposal. **The arm PERFORMS that reconcile —
it reads `nk pr view <PROP>` and, when the store says `approved`, records the merge with `nk pr merge
<PROP>`, the graph record only, never a second compose — and it READS THAT WRITE'S rc.** That is
direction A of the old merge-sync sweep (git-landed, store not), the one direction that was never
blind. It is performed rather than prescribed because the arm exits 0 and a loop reading that as
"done" re-drains the proposal on the next pass: leave the record `approved` and every pass re-enters
this same arm and posts another comment. Every read AND the write fail CLOSED, and each refusal says
the branch IS on main — an unreconciled RECORD is never a reason to compose or to `nk pr recover`. So
`merged_by` is asked only after
ancestry has said "not on main" — where it genuinely separates three states that are not one:

| store `merged`, NOT an ancestor | what it is | what to do |
|---|---|---|
| `merged_by` = **this session** (`resolve_session_name`) | your own transport recorded the store merge and stopped before the push, IN THIS SESSION — a red gate4a, a rejected push, §4's ancestry stop | **RESUME** — skip `nk pr merge`, compose and push. This is the CH-4720 REPAIR path, and blocking it strands the very state it fears |
| `merged_by` = a **peer**, inside the window | the ORDINARY in-flight state of a peer's transport | **stop**, exit 0. Not this session's error |
| `merged_by` = anyone, **stale** or no timestamp | the real CH-4720 split state | **refuse**, investigate by hand |

⚠ **RESUME is a SAME-SESSION retry, and a KILLED session is the one cause of that first row it
cannot cover.** `resolve_session_name` composes `<agent>/s-<8>` from `CLAUDE_CODE_SESSION_ID`
(`scripts/agent-identity.sh`), which the harness re-rolls for every session, so a session that was
killed does not come back as itself: its successor reads a `merged_by` it can never equal and falls
to the peer arms — "IN FLIGHT" until the window expires, then the stale arm's refusal. That is
fail-SAFE (it never composes wrongly and it never points at `nk pr recover`) but it is not RESUME, so
a killed transport is repaired BY HAND, from the stale arm's refusal. Matching on the agent half
would cover it and is deliberately not done: `merged_by` values with no session component already
exist, and admitting the agent half is exactly what would let a bare `M5` resume a transport this
session never started — the direction the last paragraph of this section routes away from.

⚠ **`merged`-but-not-an-ancestor is NOT by itself the CH-4720 split state — for a peer inside the
window it is the ORDINARY in-flight state, and calling that CH-4720 is dangerous.** §3 records the
STORE merge BEFORE the push with the composed-tree gate in between, so for the entire duration of any
transport the store says `merged` while the branch is not yet an ancestor. Measured on the incident
this guard was first written for:

```text
PROP-6101  store merged_at_ms 1788912534187  ->  2026-09-09 00:08:54Z   (DGX1 ran `nk pr merge`)
           git merge 25df20b320 committed    ->  2026-09-09 00:57:15Z   (DGX1 pushed)
           window = 48m 21s
```

An earlier revision of this block called that condition "the CH-4720 split state. Investigate." The
name points at `nk pr recover`, whose help text is this condition verbatim — so following the label
would **un-merge a proposal whose transport is still running and manufacture the very corruption the
label warns about**. That is why the peer arm exits 0 and says nothing more: do not compose, do not
diagnose, do not recover.

**The 6h window is a HEURISTIC with a named basis and a deliberately ASYMMETRIC error.** The widest
store→push window measured on this fleet is PROP-6101's 48m 21s above, and M5's own composed-tree run
on that same branch took 58m 23s; 21600s is roughly seven times that. Set it too SHORT and a live
peer's transport is labelled a split state — round 1's measured danger, because the label sends an
agent to `nk pr recover` and manufactures the corruption. Set it too LONG and a genuine split state
waits longer to be NOTICED, which costs nothing automatic: the stale arm only refuses and says
"investigate by hand". The error is not symmetric, so the threshold is long on purpose. A
`merged_by` carrying no session component (a bare `M5` from an older record) never equals
`resolve_session_name`'s `<agent>/s-<8>`, so it falls to the peer arms — stop or investigate, never
resume; that direction is fail-safe by construction.

⚠ **THE RECONCILE IS GATED ON `approved` BECAUSE THE STORE ADMITS `-> merged` FROM NOTHING ELSE, and
its rc is READ.** Source, not inference — `ProposalStatus::valid_transitions`
(`crates/graph-review-core/src/proposals.rs:69`, the `Approved` row at `:88`):

```text
Approved  => [Merged, Rejected, Closed]      <- the ONLY in-edge to Merged
Rejected  => [Revised, Closed]      Revised => [Reviewing]      Reviewing => [Approved, Rejected, Closed]
Merged    => []                     Closed  => []               <- no exits at all
```

`mark_merged` (`proposals.rs:849`) reaches that table through `set_field_and_transition` (`:1709`), so
any other status is `Err(InvalidTransition)`; it refuses BEFORE transitioning when the proposal has
unresolved comments; and `do_merge` (`crates/nusy-fleet-kanban/src/organs/pr.rs:692`) maps every one of
those to `Err` deliberately (CH-7526/CH-7534 — *"on the IRREVERSIBLE act an Err reading as 0 would
silently remove the last catch"*). `Approved -> Rejected` and `Approved -> Closed` are both live edges
a peer can take while this session is mid-transport, so a LANDED arm that fires `nk pr merge` at
whatever status it happens to find is asking for a refusal it then ignores — and an ignored refusal
leaves the record exactly as unreconciled as doing nothing, while the arm posts `--resolved` and
exits 0. Every later drain then re-enters this arm and adds another comment: the very loop the
reconcile exists to close. **So the arm branches on `$ST` — `merged` no-ops, `approved` writes and
checks rc, anything else REFUSES WITHOUT ATTEMPTING and names the by-hand repair.** A refusal here is
CANNOT-ASSESS about the RECORD and never about the landing: all three paths still say the branch IS
on main, still forbid a compose, and still forbid `nk pr recover`.

⚠ **The ordinary path's `nk pr merge` reads its rc too, and is deliberately NOT status-gated.** There
the branch is *not* on main, so a refused store merge must stop the transport outright rather than be
classified — and the store's own refusal text (`Invalid transition from <st> to merged`, or the
unresolved-comment message naming each `CMT`) is more precise than any status the block could
pre-judge. On the LANDED arm the classification is load-bearing because the arm never composes in
either direction and the operator needs to know WHICH of "already reconciled", "reconciled now" and
"cannot be reconciled by this tool" happened; on the ordinary path there is exactly one right answer
to a refusal, which is to stop.

**The fetch takes `main` ONLY, and that is deliberate.** §1 already fetched `origin/<branch>` and bound
its tip to the approver's `reviewed-at-sha:`; re-fetching the branch here would silently swap in a
push made after the approval, so the ancestry question would be asked about a tip nobody reviewed.
Only `origin/main` can have moved in a way this check must see.

⚠ **Every read here fails CLOSED, because an unreadable store is CANNOT-ASSESS and CANNOT-ASSESS is
never a pass (CH-7020).** The store read is UNPIPED — `nk pr view <PROP> >log 2>&1; rc=$?` — and a
non-zero rc REFUSES instead of falling through to `nk pr merge`. Measured: a dead bus gives rc 1 with
53 bytes of `nk-fleet: IO error: Connection refused` and no JSON, and an unknown id gives rc 1 with a
NOT_FOUND line; piping into a matcher books both as "not merged" and composes. The same rule governs
the fetch (a failed fetch leaves `origin/main` stale, so the ancestry answer is not an answer) and
`--is-ancestor` itself, whose rc 128 for an unresolvable ref is a THIRD outcome, refused explicitly
rather than suppressed with `2>/dev/null` into the rc-1 arm.

⚠ **There is no periodic sweep for the real split state today, and the stale arm does not pretend
otherwise.** `scripts/store-git-merge-sync-check.sh` — direction B of the git↔store merge-sync sweep —
was deleted by this voyage's own Commit C (`f8b7f2f531`) and is not on `scripts/KEEP.txt`; nothing
replaced it, and `merge_transport_in_flight` in `scripts/agent-identity.sh` prints UNEVALUATED on
every host because Commit C also took `transport-liveness.sh`. So the stale arm above is a
PER-TRANSPORT detector and the only one running. The enumeration a rebuilt sweep needs does now work:

```text
nk pr list --status merged   ->  rc 0, 0.28 / 0.24 / 0.25 s over 3 runs   (M5, 2026-09-12T05:48Z, live bus)
```

⚠ The **row count is not part of that measurement and is deliberately not quoted here** — it is the
board's merged population and it moves on every landing (4161 and 4162 an hour apart on this same
date, which is the drift, not a disagreement). What the measurement says is that the call RETURNS,
at rc 0, in well under a second. Re-run it rather than reading a number off this page.

**HZ-9335's premise is therefore FALSE** — that hazard says this call times out at a ~10s server
deadline and blinds direction B fleet-wide; the timeout is gone (HZ-12131 / PROP-6307, merged). It is
still `backlog`/unassigned and should be re-measured and re-scoped to "the sweep itself was deleted",
which is the live half. Until one exists, an agent that hits the stale arm investigates by hand and
does NOT run `nk pr recover` on a proposal whose transport may still be live.

**Derive BEFORE `nk pr merge`, and REFUSE if you cannot.** rc 1 means the store answered and its
approver set is EMPTY — the lost-approval shape, where a revise or an override erased the record; go
back to reviewit rather than merge something with nothing to attest. rc 2 means the read did not
happen; unknown is not clear, and it is never booked as rc 1 (HZ-11813). Refusing here, before the
store merge, is why neither answer can leave a store-merged/git-unlanded proposal behind (CH-4720).
The `2>&1` is not cosmetic: the reason is announced on stderr because a `$( )` capture cannot see the
variable the lib also sets.

The store is read TWICE — here, and again by the hook at push. A revise or a re-approve landing in
that window (seconds) turns a correct line into a refusal; re-derive and re-merge rather than
suspecting the gate.

**Do not type the line by hand.** It used to be a LITERAL — `Reviewer: sub-<PROP>` — so a merger
wrote that string whether or not such a reviewer existed, and the HZ-11821 consumer measured the
template. `25df20b320` (Merge PROP-6101) says `Reviewer: sub-PROP-6101` while the store says
`Air/s-prop6101`; a comment on that very proposal had said in advance that writing `sub-` would be a
false attestation, and the template won anyway (HZ-12022). The pre-push hook now REFUSES a push to
main whose merge message does not name the store's approver, so a hand-typed line costs you a
`git -C /tmp/ap-<PROP> commit --amend`, not a wrong measurement. ⚠ **The `-C` is the whole remedy
here (HZ-12099 F3).** The refused merge commit is in the scratch tree while you are standing in
`~/fleet-wt/<agent>` with a FEATURE BRANCH checked out, so an unscoped amend rewrites YOUR BRANCH
TIP, leaves the refused merge message byte-identical, and the next push is refused for the same
reason with one extra rewritten commit on the branch. (Arm 15 of
`scripts/tests/test_approveit_compose_isolation.sh` counts every `git … commit --amend` in this file
and accepts either prescribed idiom; prose that DESCRIBES the unscoped form without spelling the
invocation is outside that population, exactly as arm 1's prose is.)

⚠ **`Merged-by:` is `resolve_agent_name`, NOT `hostname -s` (HZ-12039).** They differ on exactly the
hosts that matter: DGX1 is `spark-2f10`, Mini is `Mac-mini`. This file used to ship
`$(hostname -s)` as an executable line while `CLAUDE.md` and `campaign-watcher` only ever showed a
`<agent>` placeholder — so this was the spelling agents copied, and it wrote a name no consumer looks
for. Measured on `origin/main` over the **last 100 first-parent merges** (the window matters — over ALL of
main it is 381 `DGX1` / 147 `Mini`, i.e. ~2% mis-attribution, not ~50%): 9 say `spark-2f10` and 2 say
`Mac-mini`, against 9 `DGX1` and 3 `Mini`. The recent window is the one that hurts, because the
consumer's own window is 7 days and all 9 `spark-2f10` merges fall inside it. The concrete cost is `concurrency-metrics.sh:121`, which counts
`Merged-by:[[:space:]]*${HOST}` where `HOST` is `resolve_agent_name` — so DGX1's own per-host figure
silently under-reports, at rc 0, with a plausible smaller number. Resolve the name ONCE at the top of
this block (`$AGENT`); do not inline `$(hostname -s)`, and do not fall back to it — a fallback is the
defect wearing a guard.

Rejected ⇒ `git fetch origin main`, re-base the SAME worktree
(`git -C /tmp/ap-<PROP> checkout --detach origin/main`), re-merge, re-check, retry ONCE; then
`nk pr comment <PROP> --resolved --body "push-retries: <n>"`. A second rejection ⇒ stop and say so.

⚠ **The `-C` is load-bearing here too, for the same reason §4's is.** Under the retired transport this
line needed no scoping — the tree you stood in WAS the composed tree, so "re-base" was correct by
construction. With the composition moved, an UNSCOPED re-detach of `origin/main` re-bases the tree
you are STANDING in and leaves the scratch tree holding the stale merge, so the retry re-pushes the
same rejected commit. That is the round-2 finding's class, in the one paragraph it did not reach.

## 4. Ancestry before ANY close — the store saying merged is not landing

```bash
git fetch origin main
git merge-base --is-ancestor "$(git -C /tmp/ap-<PROP> rev-parse HEAD)" origin/main; rc=$?
```

⚠ **`git -C /tmp/ap-<PROP>` is the whole check — a bare `git rev-parse HEAD` here always passes.** The
merge commit lives in the scratch tree; your own tree never moved (that is the point of §3). A bare
`HEAD` reads whatever YOU are standing on — on the drain path the `origin/main` commit, an ancestor of
`origin/main` by definition — so `is-ancestor` returns rc 0 whether or not the push landed, recording a
green `ancestry` row without testing anything. §3 records the STORE merge before the push, so this is
the only thing between a rejected push and the CH-4720 state. Read the scratch tree, and read it
BEFORE §6 reaps it.

- rc 0 ⇒ `verdicts record --decision ancestry --rc 0 --reason landed --subject "$SHA" --lineage <PROP>`
- else  ⇒ `verdicts record --decision ancestry --rc 1 --reason unlanded --subject "$SHA" --lineage <PROP>` and **stop
  before any close** (CH-4720: a store-merged, git-unlanded proposal is the worst available failure)

## 5. Record every gate that fired

```bash
verdicts record --decision gate4a             --rc <0|1|2> --reason <clean|nothing_to_sweep|red|pre_existing_debt|unattributed|cannot_assess> --subject "$SHA" --lineage <PROP>
# ↑ §3 set GATE4A: `nothing_to_sweep` (rc 0) when the composed delta carried NO rust delta and cargo check was skipped, `clean` (rc 0) when it ran green.
verdicts record --decision change_intent_reread --rc <0|1|2> --reason <ok|…>    --subject "$SHA" --lineage <PROP>   # protected-class only
# §1's MODE picks ONE of the next two rows (CH-13158) — `fix` iff /tmp/ap-<PROP>.fixnote exists:
verdicts record --decision approval_fix_scope  --rc 0 --reason in_scope --subject "$SHA" --lineage <PROP>   # FIX path ONLY: the token §1's fix_scope_check answered (a refused fix path was recorded in §1 and never reaches here)
verdicts record --decision approval_sha_drift  --rc 0 --reason clean --subject "$SHA" --lineage <PROP>   # EXACT path ONLY: §1 held; DRIFT ⇒ --rc 1 --reason drift; NO parseable declaration by any approver, or one that will not resolve to an object id ⇒ --rc 2 --reason cannot_assess (§1 has already REFUSED — all three directions, no fourth)
# reviewer_row_present — NOT re-recorded here. §1b wrote it at the point of decision, which is the
# only place a REFUSAL can leave a row (this block is unreachable on that path). Two record sites
# for one decision is a drift surface; do not add one.
#
# ⚠ ASSERT $SHA AND READ EACH rc — the same two holes the change-intent fence below closed, still
# open three lines up. `verdicts record … --subject ""` exits 2 and records NOTHING, so an unset
# $SHA silently produces no row while this block reads as though three gates were recorded. And an
# undeclared reason token is refused the same way. Both are the shape "a gate that fired and left
# no row did not fire", from the WRITER's side:
: "${SHA:?REFUSE — \$SHA is unset; §3 sets it to the merge commit. An empty --subject records NOTHING.}"
# …and gate every line, e.g.:
#   verdicts record --decision gate4a --rc 0 --reason clean --subject "$SHA" --lineage <PROP> \
#     || { echo "REFUSE — the gate4a row did not record."; exit 1; }
```

**Whether the `change_intent_reread` row is owed is a MEASUREMENT, not a reading of the diff.**
Ask the classifier, from the repo it can resolve both refs in:

```bash
scripts/lsc-class-check.sh origin/main origin/<branch> --explain >log 2>&1; rc=$?   # lsc-class-check:invocation
# rc 1 ⇒ protected-class, the matching predicates named: the row is OWED
# rc 0 ⇒ no predicate in the conf matched · rc 2 ⇒ CANNOT ASSESS, record --rc 2, never a green row
```

The trailing `# lsc-class-check:invocation` is a **machine sentinel, not a comment for you** —
`scripts/tests/test_lsc_class_check.sh` arm 9 credits this file with calling the classifier only for
a marked line inside a fenced `bash` block. Keep it if you re-spell the command; drop it and that arm
reds by name. (Prose about the classifier must NOT carry it.)

`scripts/lsc-class-check.sh` reads `scripts/lsc-class-predicates.conf` and hardcodes no predicate.
**Never hand-roll the matcher.** HZ-11894 records two transports on one day where an improvised
grep answered "not a safety path" for every input — once including the three files CLAUDE.md says
nobody deletes. The conf's rows are stanzas and its `paths:` values are globs, so a hand-rolled
matcher is wrong in the direction that reads as clean.

Two things rc 0 does **not** mean. It is not "no row is owed": the conf is a measured SUBSET of
floor row 2 (per-axis table in its header — a safety crate, a skill, CI, a deleted Rust test and a
bare `Cargo.lock` version bump have no predicate — convergence tracked at HZ-12103), so apply
those clauses by hand exactly as `workit` §5 does. And it is not a licence to skip the re-read when the proposal declares
`change-intent:` anyway — a declared citation is re-read at the reviewed tip regardless
(`scripts/lib/change-intent.sh`). ⚠ **SOURCE it; do NOT run it** — it is a library, and executing it
used to define the functions and exit **0 having checked nothing** (HZ-12370: floor row 2's row was
recorded from exactly that on three landed proposals, and all three were really rc 1). It now REFUSES
with rc 2 when executed, but the call to make is:

```bash
# $TIP is §1's VERIFIED tip — the same value §1 matched against the approver's reviewed-at-sha (or, on
# the CH-13158 fix path, admitted as a findings-scoped fix of it).
# ⚠ NEVER leave the ref unset: `change_intent_reread_token tok ""` runs `git show ":CLAUDE.md"`,
# which reads the standing tree's INDEX, so a citation can verify against your uncommitted work
# instead of the reviewed tip. Measured: rc 0 with the ref unset, rc 1 at the real ref. That is the
# unset-$VAR hazard the transport section names, arriving through the re-read.
. scripts/lib/prop-declaration.sh
. scripts/lib/change-intent.sh
# ⚠ AND CHECK THAT THEY LOADED. Both sources are RELATIVE PATHS, so they resolve against the cwd —
# and cwd is shell state exactly like $TIP below, across procedure text executed in separate tool calls.
# Driven from anywhere but the repo root, in BOTH production shells, the sources fail,
# `prop_decl_value` is undefined, INTENT_VALUE comes back empty, and the zero-token branch records
# `change_intent_tokens reason=0` for a declaration that carries a perfectly good citation — a
# RECORDED FALSEHOOD that also blames the author. Reachable from the repo root too: under macOS
# `/bin/sh`, `prop-declaration.sh` uses `done < <(…)`, a syntax error in sh mode, so the function
# never defines and every case takes that same false path. A benign false NEGATIVE (the store
# refusing an undeclared token, writing nothing) is better than a false POSITIVE — HZ-12022's own
# argument, applied to this fence.
command -v prop_decl_value >/dev/null 2>&1 && command -v change_intent_tokens >/dev/null 2>&1 \
  || { echo "REFUSE — the change-intent libraries did not load (cwd is not the repo root?)."; exit 1; }
# ⚠ §1 SETS $TIP AND §5 READS IT, WITH §1b/§2/§3/§4 IN BETWEEN. A skill is executed by an agent
# across SEPARATE tool calls and shell state does not persist between them, so "$TIP is still set"
# is an assumption, not a fact. An EMPTY ref makes `change_intent_reread_token tok ""` run
# `git show ":CLAUDE.md"`, which reads the standing tree's INDEX — so a citation verifies against
# your uncommitted work instead of the reviewed tip. Measured with a needle staged but absent at
# the tip: TIP set -> rc 1 `canon_not_found`; TIP unset -> rc 0 `ok`. The fence CONCLUDES ok; the
# only thing that stopped a false row was `verdicts record` refusing a blank `--subject`, which is
# downstream and accidental. Assert it instead of relying on that:
: "${TIP:?REFUSE — \$TIP is unset or empty; §1 sets it. An empty ref makes git show read the INDEX.}"
INTENT_BODY="$(nk pr view <PROP> | python3 -c 'import json,sys; print(json.load(sys.stdin)["description"])')" \
  || { echo "REFUSE — CANNOT ASSESS: could not READ <PROP> from the store."; exit 1; }
INTENT_VALUE="$(printf '%s' "$INTENT_BODY" | prop_decl_value change-intent)"
# ⚠ THE READ FAILURE ABOVE REFUSES WITHOUT WRITING A `change_intent_reread` ROW (HZ-11813: an
# outage is never booked as absence). This comment used to give a VOCABULARY reason for that — that
# the decision had no token for "could not assess", so the store would refuse such a row. HZ-12227
# declared `cannot_assess` for `change_intent_reread` (scripts/layerb-decisions.conf, the pairs
# baseline, the differential's pin), so that reason is gone: the store accepts the row now, and two
# branches of the fence below write it. This read failure still refuses without one. That is the
# behaviour as it stood, kept unchanged; it is no longer forced by the vocabulary.
#
# ⚠ AND BE HONEST ABOUT WHAT THE `exit 1` BELOW CAN AND CANNOT DO. An earlier cut of this comment
# said "refusing the TRANSPORT needs no row: nothing merges". **That is false in this file.** §3
# PUSHES THE MERGE (`git -C /tmp/ap-<PROP> push origin HEAD:main`) and §5 — which is where this fence
# lives — runs several hundred lines LATER. By the time it runs the merge is ALREADY ON `main`, and
# `exit 1` does not un-push. ⚠ Stated by SECTION and by the pushing command, not by line number: an
# earlier cut of this very comment gave `:596 / :901 / :943`, and the `:943` was invalidated by the
# next edit to this same block. A line number in a comment about its own file goes stale on the next
# edit — the same finding HZ-12389 landed one file over, reproduced here while writing the fix for it.
# Measured: there is NO change-intent mention anywhere before §3's push, `reviewit` has none at all,
# `workit` records no change_intent row,
# and `scripts/merge-prop-branch.sh` — which `layerb-corpus-harvest.sh` still names as the home of
# the downstream zero-token refusal — was deleted by Commit C.
#
# So floor row 2's re-read is a POST-HOC RECORD, not a pre-merge gate, which is exactly the word
# canon uses for it ("convention, recorded as a `change_intent_reread` row"). What the `exit 1`
# genuinely buys is real but smaller: it stops §6 from CLOSING the item on a proposal whose
# declaration yielded nothing, and it makes the operator read the refusal. The structural gap —
# that nothing re-reads the citation BEFORE the push — was filed as HZ-12399 and is now CLOSED FROM
# THE OTHER SIDE, and the correction is appended here rather than spliced into the paragraph above,
# which stays true of THIS fence: `scripts/hooks/pre-push` stage 2c re-reads the same citation with
# the same library, at the same reviewed tip, BEFORE the merge reaches main, and REFUSES the push
# when the diff is protected class and the obligation is unmet. Two things follow, and neither is
# "this fence is now redundant".
#   · The stage's `owed?` question is `scripts/lsc-class-check.sh` ALONE — a hook has no agent to
#     apply the hand clauses, i.e. the four uncovered axes enumerated once in the prose above this
#     block. A diff protected only by one of those passes the stage and is caught HERE, by the
#     `ROW2_HAND` branch below. The stage is a backstop over the conf-covered classes, never a
#     replacement for this fence. (⚠ The four are deliberately NOT re-listed here:
#     `scripts/tests/test_lsc_class_check.sh` arm 10 requires each to occur EXACTLY ONCE in this
#     section, so a second enumeration reds it — measured while writing this paragraph.)
#   · The VERDICT ROW is still written here and ONLY here. The stage records nothing: two record
#     sites for one decision is a drift surface (the rule three blocks up), and a refused push has no
#     landed sha to be a `--subject`.
# The re-read LOGIC is not duplicated either: the stage sources `scripts/lib/change-intent.sh` and
# calls the same `change_intent_tokens` / `change_intent_reread_token` this fence calls, because
# HZ-12227 measured twice that a copy stops observing changes to the original (M15).
ci_n=0 ci_bad=0 ci_badtok="" ci_dec=0 ci_unk=0 ci_unktok=""   # ⚠ INITIALISE: an unset counter reads as 0
while IFS= read -r tok; do
    [ -n "$tok" ] || continue
    ci_n=$((ci_n + 1))
    change_intent_reread_token "$tok" "$TIP"; rc=$?       # rc captured IMMEDIATELY
    # ⚠ NOT `!= 0` (HZ-12227). The reader has FOUR outcomes, not two: 0 verified · 1 failed ·
    # 2 CANNOT ASSESS (the store could not be ASKED — an outage is never absence, HZ-11813) ·
    # 3 DECLINED (`n/a` / `none` / `na`). Under `!= 0` both 2 and 3 land in the failure count, so a
    # decline books a FALSE FAILURE row and an outage books a finding. Neither REFUSES anything:
    # that branch records and falls through to §6, and nothing in §5 can refuse the transport,
    # which §3 has already pushed. A decline is not automatically a pass either — see its branch.
    case "$rc" in
        0) : ;;
        3) ci_dec=$((ci_dec + 1)) ;;
        2) ci_unk=$((ci_unk + 1)); [ -n "$ci_unktok" ] || ci_unktok="$tok" ;;
        *) ci_bad=$((ci_bad + 1)); [ -n "$ci_badtok" ] || ci_badtok="$tok" ;;   # ⚠ the FIRST failing token, kept
    esac
done <<EOF
$(change_intent_tokens "$INTENT_VALUE")
EOF
# ⚠ THE COUNT IS ITS OWN ROW, ON ITS OWN DECISION, AND IT IS WRITTEN ON EVERY PATH.
# `change_intent_tokens` is declared PARAMETRIC — field 4 is `<count>` and the reference emits the
# ACTUAL DIGIT (CH-11649) — so `reason=0` is a legal, declared row meaning "the declaration yielded
# no tokens". That is the row the zero case needs, and it exists today: measured against an
# isolated store, `--decision change_intent_tokens --reason 0` and `--reason 3` both record at
# rc 0. It also costs the pair baseline nothing — `layerb-decisions-pairs.expected` carries ONE
# parametric row, `change_intent_tokens:<count>`, so new counts add no pairs and arm 26's coverage
# is unchanged.
verdicts record --decision change_intent_tokens --rc 0 --reason "$ci_n" \
                --subject "$TIP" --kind branch_sha --lineage <PROP> || \
  { echo "REFUSE — the token-count row did not record; floor row 2 would have no count."; exit 1; }
# ⚠ ZERO TOKENS IS NOT A PASS, AND IT IS NOT A `change_intent_reread` ROW EITHER. A declaration
# that yields nothing means the re-read loop NEVER RAN, so there is no re-read to report — the
# count row above is the honest record, and the transport stops. A gate that could not run is not
# a gate that found nothing (CH-12363).
# ⚠ Do NOT "fix" this by recording `change_intent_reread --rc 2 --reason cannot_assess`. The token
# IS declared for this decision now (HZ-12227 declared it, and the outage branch BELOW writes it), so
# the store would accept the row — but it would be false here: that token means the loop RAN and
# could not reach the store, and on this path the loop never ran at all. Zero tokens gets the count
# row and no re-read row. (This paragraph used to give a vocabulary reason instead; the token's
# declaration made that reason false, and the rule survives on this one.)
# ⚠ A VALUE THAT WAS `rc 1 unknown_form` BEFORE HZ-12227 MAY ARRIVE HERE NOW: an all-prose value, or
# `skill:<name>`, yields ZERO tokens under the new splitter, so it stops at this refusal before §6
# instead of recording rc 1 and continuing. That is the fail-closed direction, and it is deliberate.
if [ "$ci_n" -eq 0 ]; then
    echo "REFUSE — <PROP> is protected-class and its change-intent: declaration yields ZERO citation tokens."
    echo "  (the merge is already pushed — this stops the CLOSE in §6, it does not un-push §3.)"
    exit 1
elif [ "$ci_bad" -eq 0 ] && [ "$ci_unk" -gt 0 ]; then
    # HZ-12227 declared `cannot_assess` for this decision, so this row now RECORDS (see below).
    verdicts record --decision change_intent_reread --rc 2 --reason cannot_assess \
                    --subject "$TIP" --kind branch_sha --lineage <PROP> || \
      { echo "REFUSE — the change_intent_reread row did not record."; exit 1; }
    echo "REFUSE — <PROP>'s citation $ci_unktok could not be re-read at all (the store could not be asked)."
    echo "  Unknown is not clear: this is NOT an absence and NOT a supersession (HZ-11813)."
    exit 1
elif [ "$ci_bad" -eq 0 ] && [ "$ci_dec" -gt 0 ]; then
    # ⚠ A DECLINE IS A PASS ONLY WHERE NO CITATION WAS OWED (HZ-12227, round-8 review BLOCKING-1).
    # Without this question, `change-intent: n/a` on a PROTECTED-CLASS diff recorded `--rc 0 --reason
    # declined`: a green floor-row-2 row on exactly the proposal that owed a citation, where the
    # two-outcome reader had at least recorded rc 1. So the branch asks BOTH halves of "was one owed?",
    # and anything short of two clear answers is not clear:
    #   · the CLASSIFIER, over the diff the merge actually brought to main, `<merge>^1 <merge>`. NOT the
    #     row-owed call at the top of this section: it reads `origin/main origin/<branch>`, and after
    #     §3's push the branch is already inside origin/main, so that diff is EMPTY and answers rc 0
    #     for every proposal (HZ-12395, still open for that call). A landed merge is fixed history,
    #     and it must be the merge that carried $TIP, or the classifier is asked about the wrong diff.
    #   · the floor-row-2 clauses the conf does NOT cover — the per-axis list in the prose above this
    #     block — which you applied BY HAND. Replace the placeholder with `none` or `matched`. Left
    #     as written, it reads as UNDETERMINABLE — never as `none`.
    # A value that declines AND cites (`n/a; item:X`) is a contradiction and is read as its stricter
    # half: it lands here only when every citation re-read clean, and it is still a decline.
    ROW2_HAND='<none|matched>'
    ci_m="$(git log --first-parent --merges --format=%H --grep='^Merge <PROP> ' origin/main -1 2>/dev/null)"
    ci_lsc=2                                             # no landed merge carrying $TIP ⇒ CANNOT ASSESS
    if [ -n "$ci_m" ] && [ "$(git rev-parse "$ci_m^2" 2>/dev/null)" = "$(git rev-parse "$TIP" 2>/dev/null)" ]; then
        scripts/lsc-class-check.sh "$ci_m^1" "$ci_m" >log 2>&1; ci_lsc=$?
    fi
    case "$ci_lsc:$ROW2_HAND" in
        1:*|*:matched) ci_owed=yes ;;                    # protected by EITHER route
        0:none)        ci_owed=no ;;                     # both routes answered, and neither owes one
        *)             ci_owed=unknown ;;                # classifier rc 2 (or any other rc), or no hand answer
    esac
    case "$ci_owed" in
        no)  verdicts record --decision change_intent_reread --rc 0 --reason declined \
                             --subject "$TIP" --kind branch_sha --lineage <PROP> || \
               { echo "REFUSE — the change_intent_reread row did not record."; exit 1; } ;;
        yes) verdicts record --decision change_intent_reread --rc 1 --reason declined \
                             --subject "$TIP" --kind branch_sha --lineage <PROP> || \
               { echo "REFUSE — the change_intent_reread row did not record."; exit 1; }
             echo "REFUSE — <PROP> DECLINED its change-intent: citation on a PROTECTED-CLASS diff (classifier rc $ci_lsc, hand clauses '$ROW2_HAND'). Floor row 2 owed a citation: this is a FAILURE, not a decline."
             echo "  (the merge is already pushed — this stops the CLOSE in §6, it does not un-push §3.)"
             exit 1 ;;
        *)   verdicts record --decision change_intent_reread --rc 2 --reason cannot_assess \
                             --subject "$TIP" --kind branch_sha --lineage <PROP> || \
               { echo "REFUSE — the change_intent_reread row did not record."; exit 1; }
             echo "REFUSE — <PROP> DECLINED its change-intent: citation, and whether one was OWED could not be determined (classifier rc $ci_lsc, where 2 also means no landed merge of <PROP> carrying \$TIP was found — fetch origin main; hand clauses '$ROW2_HAND'). Unknown is not clear."
             exit 1 ;;
    esac
elif [ "$ci_bad" -eq 0 ]; then
    verdicts record --decision change_intent_reread --rc 0 --reason ok \
                    --subject "$TIP" --kind branch_sha --lineage <PROP> || \
      { echo "REFUSE — the change_intent_reread row did not record."; exit 1; }
else
    # the reason token comes from the reference, which maps the error text to the declared
    # vocabulary (canon_not_found / unknown_form / manifest_not_found / item_unresolved)
    verdicts record --decision change_intent_reread --rc 1 \
                    --reason "$(scripts/layerb-reference.sh decide change_intent_reread "$ci_badtok" "$TIP" |
                                sed -n 's/.*reason=//p')" \
                    --subject "$TIP" --kind branch_sha --lineage <PROP> || \
      { echo "REFUSE — the change_intent_reread row did not record."; exit 1; }
fi
```

⚠ **Read the loop's OUTCOME onto the row — do not record a pass because the loop finished.** Every
branch above except the zero-token one writes a `change_intent_reread` row (the zero-token branch
writes only the count row, and refuses); the row that does not exist is "ran nothing, recorded ok",
which is exactly how HZ-12370's three false greens were produced.

⚠ **And READ `verdicts record`'s own rc**, which every branch above now does. The store refuses a
reason token that is not declared for the decision, and an earlier cut of this fence walked straight
past that refusal: the operator got stderr and NO ROW, while the block continued as though the gate
had recorded. *A gate that fired and left no row did not fire* cuts both ways — the writer has to
check that the write happened.

⚠ **And the reason must come from the token that FAILED, which is why `ci_badtok` exists.** An
earlier cut of this fence classified the failure by `$tok` after the loop. **`$tok` is EMPTY there** —
the loop body does run in THIS shell (the heredoc is a redirect, not a pipe), but the `read` that
ENDS the loop hits EOF and clears the variable. Measured, same result in all three: bash 5.3.9, bash
3.2.57 and zsh 5.9 each print `tok=''` after the loop while `ci_badtok` holds `bad-first`. So the old
form asked the reference to classify an EMPTY token — it would have recorded `--rc 1` with whatever
reason an empty argument yields, on a proposal whose actual failure was `canon_not_found`. The rc was
right and the reason was unrelated to any token. `ci_badtok` keeps the FIRST failing one, captured
inside the loop where the value still exists. (`ci_bad` still counts them all — the count and the
classified token answer different questions.)

⚠ Do not reach for `$tok` after ANY `while read` loop for this reason; if you need a value from the
loop, capture it in the body.

⚠ **Re-read EVERY token, not the first.** The value is `;`-separated and an unrecognised form is
ITSELF a failure (CH-7020), so a value whose first token verifies and whose second does not reads as
clean if you stop at one — measured on PROP-6366, where three tokens gave rc 0, rc 1, rc 1.
⚠ **There is no slot in that grammar for a RATIONALE**: every `;`-separated element is parsed as a
citation, so citations go on the `change-intent:` line and prose goes below the declaration block.

The token sets are `scripts/layerb-decisions.conf`'s (`verdicts decisions` lists them); an
undeclared token is refused and that refusal is itself an rc-0 record — read `verdicts about
$SHA` if in doubt.

**`--lineage <PROP>` on every row is what makes the record findable, and it is not decoration**
(HZ-12133). A verdict has two names — the tip it was decided about and the proposal that carried
that tip — and these rows used to record only the first while `reviewit`'s independence row recorded
only the second. Measured on DGX2 for ONE event: `verdicts about dd02301…` returned five gate rows
and ZERO independence rows; `verdicts about PROP-6291` returned six independence rows and ZERO gate
rows, with nothing in either output saying the other half existed. `verdicts about <key>` now answers
on subject OR lineage and names what it cannot cover, so a row that carries both keys is reachable
from either question — and one that carries only a sha is reached from `about <PROP>` through a
row that carries both, which is the only way `about <PROP>` reaches it (`about <sha>` reaches that
row DIRECTLY — it is the proposal-keyed question that needs the hop). Give `about` a FULL 40-hex
sha: matching is EXACT, and an abbreviation that begins a recorded id refuses at rc 2 rather than
answering emptily (a short sha that begins nothing still answers rc 0, with a note).
**What dropping the flag costs, measured** (`about <PROP>` over a `reviewit` row carrying
`--lineage` plus these three gates without one): the gates are not invisible — they are DEMOTED
out of the answer into the joined block, printed under `── through <sha> ──` and headed *NOT
verdicts about `<PROP>`*, reached only because the independence row at that tip carried the
proposal. They are lost outright when no row at that tip carries a lineage — a legacy-shaped
independence row, or one recorded at an earlier tip. `verdicts about <PROP>` is the query a
reader asking *"was this reviewed?"* actually types, and a gate it can only reach second-hand is
one tip drift away from being unreachable.

## 6. Close the item — or CARRY the remainder yourself: there is no third exit

⚠ **This section used to be titled "Close or leave the remainder", and leaving one was a COMPLIANT
WAY TO STOP WITH WORK UNDONE.** That exit is REMOVED (HZ-12459, Captain 2026-09-15, verbatim):

> *"there should be no remainders, no handoffs — the agent is supposed to bring the work all the way
> to merge."*

Measured on the live store the day it was removed, reading the `title` FIELD of `nk list --json`
and counting OPEN rows of type `hazard` or `chore` whose title matches `\bPROP-[0-9]+\b`:
**97 such rows, 83 of them with no assignee**, at 2026-09-15 18:20 local. That is not a hypothetical
failure mode — it is the accumulated output of this section's old second branch. Each review found
real things, converted them into fresh unassigned board items, and landed clean; the pool grew, and
the next resident's §4 drain met someone else's remainder instead of target-on-target work. **That is
the kanban handoff, relocated from "who reviews this" to "who fixes what the review found."**

⚠ **That pair is a SCOREBOARD, not a constant, and it moved three times while this was being
written** — the same instrument read 90 / 75 and then 89 / 74 within the hour. Re-run it; never quote
this sentence's numbers (CH-11968):

```bash
scripts/remainder-carrier-check.sh --sweep
```

**THE GATE RUNS ON EVERY CLOSE, NOT ONLY ON A PARTIAL ONE.** ⚠ It was wired under the
`completes-item: no` branch in the first cut and that was exactly backwards: the 83 orphans above
came from proposals that COMPLETED their item and **landed clean** — `CH-12446`
(*"PROP-6352 r9 review AMENDs"*, backlog, no assignee) is one, and `PROP-6352`'s own close took the
`yes` branch. A gate wired only to the partial close never sees the population it exists for.

```bash
# The carrier gate, BEFORE either branch. The agent name is DERIVED, never typed: the predicate is
# exact string equality against the item's `assignee`, and the board carries BOTH shapes (`Mini` and
# `DGX1/s-758a38c7`), so a hand-typed value silently turns a CARRIED item into a refusal.
. "$(git rev-parse --show-toplevel)/scripts/agent-identity.sh"
AGENT="$(resolve_agent_name)"                     # never the bare resolve_agent "$(hostname)" table
# rc 0 carried/declared · rc 1 an ORPHAN or an undeclared handoff (DO NOT CLOSE) · rc 2 CANNOT ASSESS
scripts/remainder-carrier-check.sh <PROP> --closing-agent "$AGENT" <REMAINDER-ID>... \
                                                  >/tmp/rcc-<PROP>.log 2>&1; rc=$?
cat /tmp/rcc-<PROP>.log                           # READ it: the verdict line names the item and why
# A GATE THAT FIRED AND LEFT NO ROW DID NOT FIRE — so record the verdict on the item BEFORE the
# branch, while both outcomes still reach this line. Single-quoted: `nk comment`'s body is POSITIONAL
# and a double-quoted one would command-substitute the backticks this repo puts round every path.
nk comment <ID> 'remainder-carrier-check rc=<rc> on <PROP> — <the verdict lines, verbatim>'
[ "$rc" = 0 ] || exit 1                           # rc 2 is never a clean tree — fix the cause

# then, and only then, the close itself:
# completes-item: yes
nk move <ID> done --resolution completed --closed-by <PROP>
# completes-item: no — the remainder is CARRIED, and the gate above has already said so
nk move <ID> backlog && nk update <REMAINDER-ID> --relate remainderOf:<PROP>

git push origin --delete <branch>
git worktree remove --force /tmp/ap-<PROP>      # the scratch tree from step 3; leaving it strands a
                                                # worktree registration and counts against the cap
rm -f log log.err                               # §3's and §5's relative logs — run from the SAME cwd
                                                # they were written in; nothing reads them after this
```

The gate derives the outstanding set three ways and unions them: the ids you pass, the typed edges
(`ITEM --remainderOf--> PROP`, human-asserted; `PROP --leavesRemainder--> ITEM`, server-seeded from
the `completes-item:` line), and any OPEN item whose TITLE names this proposal — which is the shape
the measured population actually takes. Then, per item: terminal ⇒ skip · assignee == you ⇒ CARRIED
· assignee is another host AND the body declares a cause ⇒ DECLARED · anything else ⇒ refusal.
It reads `title` and `assignee` as FIELDS and never scrapes an id out of a rendered row.

⚠ **The row it leaves is an item COMMENT, not a `verdicts record` row, and that is a stated
limitation rather than an oversight.** `verdicts`' decision vocabulary is a CLOSED port contract
(`scripts/layerb-decisions.conf`) whose reference implementation is the `nusy-layerb-verdict` crate,
so a first-class `remainder_binding` decision means a conf row AND a Rust answer AND a differential
run — a rust build, which is not this change. The comment is durable, queryable with `nk show <ID>`,
and satisfies the property canon actually states (*"a gate that fired and left no row did not
fire"*); a `verdicts` decision is the better long-term home for it.

### The fold-or-carry test — two questions, and there is no third answer

This is the part a stranger has to be able to apply, so it is a test and not a sentiment.

1. **Does fixing it change the SAME mechanism this proposal changes?**
   **Yes ⇒ FOLD IT IN.** Fix it on this branch, re-review the delta, land it in this round. One more
   review round costs one spawned sub-agent. A board item costs an unbounded wait, and since Layer A
   was deleted there is nothing that ends the wait.
2. **No — it is a DIFFERENT mechanism, and folding it would turn a one-mechanism change into two.**
   **⇒ CARRY IT.** `nk create` it, `nk claim` it **in this session**, and drive it to its own merge
   before you stop. A separate PROPOSAL is correct and expected. A separate OWNER, later, unassigned,
   is not.

⚠ **"Out of scope" is not the third answer, and it is the one everybody reaches for.** Scope decides
which PROPOSAL a fix lands in. It never decides WHO does it. The moment you write a scope sentence
and stop, you have converted your own finding into somebody else's unassigned item — and there is no
somebody.

⚠ **The counter-argument, stated rather than argued away.** A strict no-remainder rule can widen a
focused diff without bound, and good judgement sometimes says not to. On 2026-09-15 the reviewer of
`PROP-6396` deliberately filed `HZ-12445` rather than fold a pre-existing, wider hole into a one-file
chore — under the old rule that was right, and under this one the FIRST half of it is still right.
What changed is the second half. Split the change, yes; drop the carrier, no. Question 2 is exactly
that case, and its answer is "your own next proposal", not "the pool".

### The ONLY legitimate other-host case is physical incapacity, and it must SAY SO

An item that genuinely needs M5, Air or a Spark is still fileable — but it is ASSIGNED to that host
and its body carries, on a line of its own:

```text
handoff-cause: <rust-delta|gpu-required|captain-authority> — <why that host and not you>
```

`rust-delta` — the bus host runs NATS, the single-writer store and CI, so Mini never drives a build
(`mini-watcher` carries the derived diff test). `gpu-required` — training, inference and eval
batteries run on a Spark. `captain-authority` — an act only the Captain performs; ⚠ this third token
is a WIDENING of `CMT-HZ-12459-25823` point 4, which names two cases, and it is FLAGGED on HZ-12459
for the Captain rather than presented as ruled (`handoff` carries the argument). **Nothing else is
a cause.** "This looks long", "this is out of scope", "someone else knows this area" are not on the
list, and the gate reads the token, so writing one of them there refuses exactly as an absent
declaration does. And a declared item is still ASSIGNED: unassigned names no recipient at all.

⚠ **THE TOKEN IS NOT THE PREDICATE — THE PAIR OF HOSTS IS** (round-4 BLOCKING 2). `rust-delta` counts
only when the CLOSING agent is Mini and the assignee is a build host; `gpu-required` only when the
assignee is a Spark and the closer is not; `captain-authority` only when the assignee is the Captain.
A legal token over a host that can do the work is `PREFERENCE`, rc 1, with both hosts named in the
refusal — measured at round 3, where DGX1 handing M5 a rust delta with *"I would rather not"* read
`DECLARED` rc 0.

### The 83 are PRE-EXISTING and this change does not touch them

They are not closed, assigned, retagged or otherwise made to disappear — most are real work that
needs disposition by somebody who reads them, and a bulk anything would be manufacturing a pass.
The gate above is scoped to the TRANSPORT precisely so that it is not red on arrival: it fires when
a proposal closes, over the items referencing THAT proposal, so it catches every new instance at the
moment of creation and ignores history by construction. A board-wide sweep would have refused on 83
rows nobody in the closing session filed, and this fleet knows what happens then — the worktree-COUNT
expression in `session-exclusivity-check.sh` read 39 on DGX2, refused unconditionally, was routinely
stepped over and protected nothing for a month (HZ-12006). The standing debt is REPORTED instead, and
who drains it is the Captain's call:

```bash
scripts/remainder-carrier-check.sh --sweep --limit 20   # a report, never a refusal
```
