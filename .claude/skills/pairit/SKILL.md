---
name: pairit
description: XP pair work for ONE item (Captain 2026-09-24; no-proposal lab model 2026-09-29). A test partner in a fresh context writes the TDD and BDD tests from the item body and proves them red; the driver writes code to green; a distinct reviewer session reviews ONCE and its findings are fixed at once; the driver merges. No nk pr proposal, no approveit. Replaces workit + reviewit + approveit for executable work. Doc-only goes straight to main.
argument-hint: "<ITEM-ID>"
disable-model-invocation: false
allowed-tools: Bash(nusy-kanban *), Bash(bash *), Bash(python3 *), Bash(git *), Bash(cargo *), Bash(verdicts *), Bash(scripts/*), Bash(NUSY_SESSION_ID=* claude *), Bash(claude *), Agent
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/pairit/SKILL.md` on
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

# pairit: tests from a partner, code from the driver, one reviewer, merge

**Captain, 2026-09-24:** *"One agent starts the work, spawns the sub-agent to write full TDD, BDD tests,
then writes code to pass the tests. Then spawns the same sub-agent reviewer we do today, then when that
passes, approve and merge."* The project was fast before the heavy skills and fast again without them.
This file stays short on purpose: when something goes wrong, fix the item or the code, not this file.

**Why a partner writes the tests.** On 2026-09-24 every defect a reviewer caught had passed its
author's own tests: a self-test that could not fail on the code it named (PROP-6619), a filter with no
test (Mini's rust-delta read), a type widening nobody asserted (CH-12992). Tests written in the same
context as the code share its blind spots. Tests written first, from the spec, by another context,
do not.

## Scope

| the change | path |
|---|---|
| doc-only | straight to `main`, no pair |
| executable, not protected-class | **this skill** |
| protected-class (any path a row of `scripts/safety-paths.conf` matches — read the conf, e.g. `nusy-cortex`, `nusy-dream`, `floor_gate.rs`; the hook, CI, the manifest, `Cargo.lock` member change, a deleted test: `workit` §5's list) | under `campaign-loop` for a campaign whose CLAUDE.md clause carries the ruling (today `CA-13266`; first `CA-12966`): **this skill**, with the protected-class clauses of *No proposal, one review round* (strong-tier reviewer, `change-intent:` in the merge message, a second review on a BLOCKING fix) — Captain 2026-09-24 and 2026-09-29. Elsewhere: `workit` → `reviewit` → `approveit` |
| `partOf CA-12764` | that campaign's clause in `CLAUDE.md` |

The being's own safety floors (never-launder, the provable gate, `refuse_if_protected()`) are the
product, not process. Nothing here touches them.

## The pair

```text
0. CLAIM     driver: nusy-kanban claim <ID>; worktree + branch from origin/main
1. TESTS     test partner (fresh Agent sub-agent): TDD + BDD tests from the item body → commit T, proven RED
2. CODE      driver: code to GREEN without touching T's test files; then the floor
3. PUSH      driver: push the branch (no proposal); the item on the board is the record
4. REVIEW    reviewer (distinct `claude -p` session): ONE round — approve, or findings fixed at once
5. MERGE     driver: compose in a scratch tree, check, push, ancestry, rows, close the item
```

The driver is the session running this skill. Under `campaign-loop`, step 2 goes to a fresh implementer
sub-agent so the loop stays one item = one context; standalone, the driver writes the code itself.

### 0. Claim

```bash
nusy-kanban claim <ID>
git fetch origin main && git worktree add -b <type>/<id>-<slug> /tmp/<id> origin/main
```

`<type>` is `chore`/`expedition`/`hazard`. Every later step names `/tmp/<id>` as a LITERAL path.

### 1. Tests: the partner's brief (Agent tool, fresh context, foreground so the Captain can watch)

Give it the item id, the worktree path and the branch, and these rules verbatim:

- *"Read `nusy-kanban show <ID>`. Write tests for what the body SAYS, not for how you would build it.
  Work only in `/tmp/<id>`; touch no other tree."*
- *"TDD: unit tests for each behaviour the phases name. BDD: one scenario test per line of the body's
  Refusal-and-control section and per acceptance bar, named and commented Given / When / Then, in the
  language's own runner (Rust `#[test]` in `crates/<crate>/tests/*.rs`, Python `test_*.py` under
  pytest). No new test framework. For a measurement runner the tests ARE its controls: a known-answer
  fixture, a negative control that must fail, and a mutation of the runner that must turn a test red."*
- *"Tests go in their OWN files, never inside the implementation file, so the driver can be held to not
  editing them. You may add minimal stubs (`todo!()`, `raise NotImplementedError`) outside the test
  files so the tests compile; the driver replaces them."*
- *"Commit the tests ALONE as `<ID>: tests (red)`. Run them and confirm they are RED for the RIGHT
  reason: an assertion or a stub panic, never a missing import or a typo. Return: the commit sha, the
  test file paths, and the red run's failing test names."*

The driver records the red run on the item (`nusy-kanban comment <ID> …`) before writing any code.

### 2. Code to green

- **The driver never edits T's test files.** If a test is wrong, send it back to the partner with the
  reason; the partner fixes it in its own commit. This is the rule that keeps the pair a pair.
- Green, then the floor exactly as `workit` §4 spells it (`cargo fmt`, clippy on touched crates,
  `cargo check --workspace`, touched-crate tests; arch-guard). A red you did not cause: prove it at
  `origin/main` in a clean scratch tree and file it. Carry it only if it BLOCKS this item.

## ⚠ No proposal, one review round (Captain 2026-09-29 — the labs' model)

The Captain, 2026-09-29: *"pull in the same rachael-loop being used in the other repos. That modifies the
process so you can approveit with a sub-agent and not take everything through kanban cycles."* So this
skill lands work the way `rachael-lab` and `rachael-neural-lab` do (their `pairit`, CHORE `819d26e`,
2026-09-28): **no `nk pr` proposal, no review slot, no `approveit` transport, and ONE review round** —
findings are fixed at once and merged on green, never sent for a second review. The `nk pr` lifecycle is the
"kanban cycle" the ruling removes.

⚠ **What it does NOT remove — Layer B, which is the product's floor, not process** (canon: *"speed must never
come from weakening Layer B"*):

- **reviewer ≠ author**: the reviewer is a DISTINCT session with its own id (`NUSY_SESSION_ID=sub-<ID>`) and
  records its own `reviewer_not_author` row. Without a proposal nothing in the store REFUSES on the id, so the
  row is the evidence; a merge with no row did not have a review.
- **the composed tree** (floor row 7): the merge composes in a scratch worktree and `cargo check --workspace`
  runs there when `scripts/lib/rust-delta.py` does not answer `rust-delta: no`; a `gate4a` row records it.
- **protected class** (floor row 2 — read `scripts/safety-paths.conf`, never a copy): the reviewer runs at
  strong tier, the merge message carries a `change-intent:` citation, and the reviewer re-reads every token
  (SOURCE `scripts/lib/change-intent.sh`, never execute it). A protected-class **BLOCKING** finding is the one
  exception to one round: its fix gets a second strong-tier review, because a safety fix that was wrong once is
  the case a second look exists for.
- **the being's own safety floors** (never-launder, the provable gate, `refuse_if_protected()`, genome/seal)
  are the product and are untouched by any of this.

### 3. Push — the branch, not a proposal

```bash
git -C /tmp/<id> push -u origin HEAD      # ⚠ LOAD-BEARING: the pre-push branch stage walks every non-merge
                                          # commit NOT already on an origin ref; pushing the branch first is
                                          # what takes the branch's commits out of that population
```

The record is the item itself: the red run (step 1), the review (step 4) and the landing (step 5) are
`nusy-kanban comment`s on it. There is no `nk pr create`, no `claim-review`, no `pr review`, no `pr merge`.

### 4. Review: ONE round, by a DISTINCT session, never this one

```bash
NUSY_SESSION_ID=sub-<ID> claude --dangerously-skip-permissions -p "<brief>" < /dev/null   # background it; it is slow
```

The brief carries these lines verbatim:

- *"Review item `<ID>`'s branch `<branch>` at tip `<SHA>` in your OWN scratch worktree (`git worktree add --detach
  /tmp/rv-<id> <SHA>`). Run anything longer than one tool call DETACHED and wait with bounded polling
  (`timeout 540 bash -c 'until [ -f <rcfile> ]; do sleep 30; done'`). Do not end your turn until the two
  mandatory steps are done."*
- *"This is a pair: the tests in commit `<T>` were written first by another context. Check (1) the tests match
  the item body, not the code; (2) `git diff <T>..<SHA> -- <test paths>` is EMPTY or each hunk is a partner
  commit with a stated reason; (3) the code does not special-case the tests; (4) mutate the code at two points
  and a test goes red each time, then restore; (5) the floor is green on a trial merge with origin/main."*
- protected class only: *"This diff is PROTECTED CLASS (`<the matching safety-paths.conf rows>`). Review at
  strong tier. Re-read every `change-intent:` token by SOURCING scripts/lib/change-intent.sh."*
- *"MANDATORY, in this order: (1) `verdicts record --decision reviewer_not_author --rc 0 --reason independent
  --subject <SHA> --kind branch_sha --lineage <ID>`; (2) `nusy-kanban comment <ID> "<text>"` (write the text to
  a file and pass `"$(cat <file>)"` — backticks in a double-quoted string are command substitution) whose FIRST
  line is exactly `reviewed-at-sha: <SHA>`, SECOND line `verdict: approve` or `verdict: changes`, then numbered
  findings, each marked BLOCKING / AMEND / NOTE with evidence."*

**ONE round** (Captain 2026-09-28 in the labs; 2026-09-29 here):

- `verdict: approve` → merge.
- `verdict: changes` → hand EVERY finding to the implementer in one message (a finding about a test goes to
  the test partner as a ruled test edit in its own named commit). Each fix commit names the finding it closes
  (`<ID>: fix r1 F<n> — …`); a behaviour fix carries a test that goes red without it. Green floor on the fixed
  tip → merge with `r1 findings fixed at <FIX-SHA>` in the merge message and a comment listing each finding
  and its fix commit. **No second review** — except a protected-class BLOCKING finding (see above).
- Escalate instead of merging only when a finding cannot be closed by a tested fix: it needs a Captain ruling,
  it changes the item's scope, or the driver disputes it. Then ask the Captain (AskUserQuestion).

### 5. Merge (bash, from the repo root, ONE Bash call)

```bash
ID=<ID>; B=<branch>; SHA=<the reviewed-at-sha, or the r1 fix tip>
git fetch origin main "$B" || exit 1
[ "$(git rev-parse "origin/$B")" = "$SHA" ] || { echo "branch tip is not the reviewed/fixed tip"; exit 1; }
. scripts/agent-identity.sh
git worktree add --detach /tmp/ap-$ID origin/main || exit 1
git -C /tmp/ap-$ID merge --no-ff "origin/$B" -m "$ID: <what landed> (reviewed-at-sha <SHA>; r1 <approve | findings fixed at FIX-SHA>)

Reviewer: $(resolve_agent_name)/sub-$ID  Merged-by: $(resolve_agent_name)
<protected class only: change-intent: item:<ID>; …>" || exit 1
( cd /tmp/ap-$ID && python3 scripts/lib/rust-delta.py origin/main HEAD | grep -qx 'rust-delta: no' \
    || cargo check --workspace ) || exit 1                # plus the touched crates' tests when rust changed
git -C /tmp/ap-$ID push origin HEAD:main || exit 1       # rejected (main moved)? remove the tree and redo from the worktree add
git fetch origin main && git merge-base --is-ancestor "$(git -C /tmp/ap-$ID rev-parse HEAD)" origin/main || exit 1
```

⚠ **The subject must NOT begin `Merge PROP-`**: that pattern is the pre-push attestation stage's population,
and there is no proposal for it to check. The `Reviewer:` line names the reviewer's own session id, which is
what the `reviewer_not_author` row carries.

Then:

1. Record the rows (`NUSY_VERDICT_CONF=scripts/layerb-decisions.conf`): `verdicts record --decision gate4a
   --rc 0 --reason clean --subject <merge sha> --kind merge_sha --lineage <ID>` (or the honest token), and for
   protected class one `change_intent_reread` row per token.
2. Close the item with its measurement and the command that produced it: `nusy-kanban move <ID> done
   --resolution completed` (no `--closed-by PROP`: there is none). A remainder is filed first and related with
   `leavesRemainder`.
3. Delete the branch (local and origin) and remove both worktrees.

**Done means that merge is an ancestor of `origin/main`.** A pushed branch alone is a third of the work.
