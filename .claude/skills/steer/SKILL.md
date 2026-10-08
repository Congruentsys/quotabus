---
name: steer
description: The PI / Product-leader OUTER loop. Run one pass of goal-driven fleet steering — sweep every pending decision/gate, classify each (measurement-settleable → science loop; goal-settleable → decide now; a FEATURE or GOAL change, or genuine human authority → escalate), DECIDE everything in the first two buckets, and stall the team only on the third. Wraps the science loop (HDD) and the execution loop (campaign-watcher). Invoke when the fleet appears "blocked on the Captain", when the captain-decision queue is growing, or at the top of an autonomous session to set direction before executing.
disable-model-invocation: false
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/steer/SKILL.md` on
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

# Steer — the PI / Product-leader outer loop

> **New session?** Read [`docs/AUTONOMOUS-FLEET.md`](../../../docs/AUTONOMOUS-FLEET.md) first — how to operate in the autonomous fleet: what a session is, who may review/revise/merge, and which older statements in this repo are retired.

The layer above execution and science (see `.claude/docs/goal-hierarchy.md`). The fleet was
stalling on decisions it could make itself; this loop makes them. **Only decisions that no
measurement and no goal can settle stall the team.**

One pass: set direction from the **goals**, drain the **decision** queue by deciding everything
that data or goals can decide, escalate **only** what changes a FEATURE or a GOAL or needs
genuine human authority, then hand the unblocked work to the execution loop.

## Required environment

```bash
alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'
```

Read `.claude/docs/goal-hierarchy.md` first — it is the north star (G1 medical → G2 general →
G3 strategy) and the 3-bucket classifier this skill applies. Everything below is the procedure.

## The classifier (apply to every pending decision, in order)

1. **Could a measurement settle it?** → **science loop**. File/point at the H→M→EXPR; the data
   decides. Never an escalation — an unfiled experiment.
2. **Do the goals settle it?** → **decide now**. Pick the option that best advances G1 (then G2,
   then G3), weighing impact × reversibility × blocking. Record the decision + its goal/evidence
   basis. Unblock the item.
3. **Changes a FEATURE or a GOAL, or needs genuine human authority?** (it alters what we build
   or what we are aiming at; or it needs a specific human signature / funding / external
   relationship / legal-ethical bedrock / public identity — name which) → **escalate**. Only
   these stall. *"If it changes features, goals or it can be well measured, then the data should
   decide"* — Captain, 2026-09-08; the features/goals half is the trigger an executing agent most
   often actually faces, and it is why the gate runs BEFORE a Captain-facing SG or CH is created,
   not after.

> Before escalating, name the specific trigger — the FEATURE or GOAL it changes, or the human
> authority required. If you can't name one, it's bucket 1/2 — decide it.

## Pipeline

### Phase 1 — Sync + load goals

```bash
# CH-9234: session-start sync needs main's COMMIT, not a checked-out named `main` — a
# spawned session's PRIMARY tree may hold that ref (CH-7738/HZ-8152), and a bare
# `git checkout main` FAILS OUTRIGHT there, not just "doesn't restore". Detaching onto
# `origin/main` reads the same commit in every tree, primary or linked, held or not — and
# nothing here commits, so there is no need to ever hold the branch name locally. BOTH rcs are
# checked DIRECTLY (never through `| tail`, CH-7898) so a real sync failure is never silent —
# the FETCH's too, not just the checkout's (CMT-17593 B2): a failed fetch leaves
# `refs/remotes/origin/main` at its STALE value, against which the checkout then SUCCEEDS, so
# guarding only the checkout would let the loop proceed on exactly the stale tree this
# sentence promises to refuse.
git fetch -q origin main \
    || { echo "[steer] FATAL: could not fetch origin/main (rc=$?) — refusing to proceed on a STALE origin/main ref" >&2; exit 1; }
git checkout -q --detach origin/main \
    || { echo "[steer] FATAL: could not sync to origin/main (rc=$?) — refusing to proceed on a stale/wrong tree" >&2; exit 1; }
# Read the goal hierarchy (the decision north star).
sed -n '1,80p' .claude/docs/goal-hierarchy.md
```

### Phase 2 — Sweep the decision queue

Gather everything that is *waiting on a decision* (not merely waiting on execution cycles):

```bash
# Open captain-decision signals — the explicit decision queue.
nk list --tag captain-decision 2>&1 | grep -E "SG-" | grep -ivE "done|complete|abandoned|retired|resolved"
# Blocked / gated work items (things reported as "Captain-gated" / "blocked-by").
nk blocked 2>&1 | head -30
# Approved-but-unmerged PRs whose merge was "gated on a ruling" (often a mis-filed bucket-2).
nk pr list 2>&1 | grep -iE "approved"
```

**Before classifying any of them from scratch, run the mechanical ruled-signal check
(CH-11083).** Two of five open `captain-decision` signals were measured already
carrying the Captain's ruling — verbatim, in their own comment threads — while a later
pass re-classified both bucket-3 because a fresh-context read of the item body and
recent comments has no structural way to see a ruling several comments up a long
thread (FA-E1). The classifier's own rule already covers this ("Applying an EXISTING
Captain ruling to a new case is bucket-2, not bucket-3"); the gap was that nothing
surfaced the ruling before the read.

```bash
nk list --tag captain-decision            # the queue; then per signal:
nk show <SG>                              # a [RULING] comment in its thread, or a ruledBy edge, answers it —
                                          # give the thread to a fresh sub-agent to read; the script died with CH-11826
```

A hit is bucket-2 by construction — go straight to Phase 3's *decide now* step, citing
the tool's output. **Still read the citation before closing an edge-only hit** — an
edge can be wired ahead of an actual close (the tool's own header documents a measured
counter-example); a `[RULING]`-comment hit needs no further reading. Anything the
sweep does not flag proceeds through the full classifier below exactly as before.

### Phase 3 — Classify + DECIDE (the core)

For each item from Phase 2, apply the classifier and act:

- **Bucket 1 (measurement):** if the settling experiment isn't filed, file it (`/hypothesize`)
  and let the execution loop run it (`/campaign-watcher`, step 7). If it *is* filed with on-disk
  eval that meets the target, **record the finding** — and per guardrail #6 (two-axis) close it
  when agent-delegable: **VALIDATED-narrow** (reviewer≠author + suite enrollment), **refuted-narrow**,
  or **obsolete/superseded**; a broad/AGI-relevant validated thesis **retires *into* the suite**
  (registry + `validation-suite`), never plain-close; **NOT-YET / values-laden / refuted-broad**
  stay Captain-retained (bucket 3).

  **REFUTED-narrow carries a DISPOSITION (CH-6851 — a refutation is a fork, not a dead end).**
  When closing a refuted-narrow hypothesis, record an explicit next-direction choice on it,
  in the adjudication comment:
  - **STOP** — the line is dead; abandon it. Creates **nothing**.
  - **PIVOT** — the line continues in a new direction: create exactly ONE research-board
    Idea, typed-edged to the refuted H, as the entry point for `/hypothesize --discover`:
    `nk create idea "<the new direction>" --relate related:H-XXXX --push`
  - **PERSEVERE-as-is** — the protocol or measurement was flawed, not the claim; retry the
    same claim. Creates **nothing**.

  Only PIVOT manufactures an artifact — never auto-create an Idea on every refutation (an
  Idea per dead line is backlog noise, and STOP is a real, common, artifact-free answer).
  Scope: **refuted-NARROW only** — refuted-broad is a strategic-direction call and stays
  Captain-retained exactly as above; this disposition never applies there.
- **Bucket 2 (goal):** **make the decision.** Write it as a comment on the item citing the goal
  it advances + the evidence, re-tag it out of `captain-decision` (the decision is made), and
  route the now-unblocked work (merge the PR, move the item to `in_progress`/`ready`, or hand it
  to the execution loop). If the decision needs a code change, that goes through branch + PR
  (execution loop) — the *decision* is still yours.
- **Bucket 3 (feature/goal change, or human authority):** leave it gated; collect it for one
  batched escalation. Name WHICH trigger fires — the feature or goal it changes, or the specific
  human authority required. If you cannot name one, it is bucket 1 or 2: decide it.

**Record every reclassification** so the queue's shrinkage is auditable: a one-line comment
`[steer] bucket-N: <decision + basis>` on each item you touch.

**Reflect the ruling onto the impacted items IN THIS SAME PASS — the required
checklist is `.claude/docs/decision-tracking.md` → *Ruling reflection (REQUIRED,
same pass)* (CH-6636).** Recording the decision is only step [a]; a bucket-1/2
ruling is not *applied* until you also [b] comment every impacted item with what
the ruling changes FOR THAT ITEM (never silent — a fresh executor reads the
comment, not your context), [c] lift the `dependsOn` edges it renders met
(`--unrelate dependsOn:…`, keep `related:`), and [d] verify with a
`nk list --tag <scope> --ready` sweep that the unblocked items actually surface.
The worked example (SG-6629) in that doc is the step-for-step reference.

### Phase 4 — One batched human escalation

File (or update) **ONE** consolidated `captain-decision` signal listing **only the bucket-3
items**, each naming its *specific* bucket-3 trigger — **the FEATURE or GOAL it changes**, or the
human authority required — and a recommended default. Both triggers file here: a feature/goal
change has no human authority to name, and demanding one would route the Captain's own stated
trigger (2026-09-08) straight back to bucket 1/2, which is the defect CH-11989 exists to fix. Do
**not** file separate signals per item, and do **not** include anything you decided in buckets 1–2.

```bash
nk create signal "[steer] Captain-gated decisions (N) — feature/goal changes and human authority" --tags "captain-decision,<scope>" --push
```

If there are **zero** bucket-3 items, file nothing — the team is fully unblocked; say so.

### Phase 5 — Hand off to execution

The decisions are made and the work is unblocked. Drop into the execution loop for the scope
(`/campaign-watcher <scope>`) to actually *do* the unblocked work — build,
review, merge, run experiments. Steer sets direction; the execution loop moves.

### Phase 6 — Self-schedule or exit

- If bucket-3 items remain **and** nothing else is executable → `ScheduleWakeup` a long fallback
  (the team genuinely waits on the human) and report the batched escalation.
- If executable work was unblocked → hand to the execution loop and let *it* schedule.
- If the goal is fully advanced and nothing is pending → exit (say the scope is done).

## Guardrails (this loop obeys them; see goal-hierarchy.md for the mapping)

- **Decide aggressively in buckets 1 & 2; escalate only a nameable bucket-3 trigger — the FEATURE
  or GOAL it changes, or the human authority required.** Over-escalation
  is the failure mode this loop exists to fix — but so is laundering a bucket-3 call as a decision.
  When genuinely unsure which bucket, treat safety/knowledge/legal as bucket 3.
- **The loop decides; it does not merge knowledge autonomously** (guardrail #2). Ontology / persona
  / safety-rule changes are drafted for the human to ratify.
- **Every decision is recorded** (comment with goal + evidence), reviewable, and reversible where
  possible — a decision is not a silent act.
- **Reviewer ≠ author still holds** for any PR the decision unblocks (execution loop).

## Cadence (CH-6515 — steer is scheduled, not summoned)

A standing mandate with no cadence still stalls: on 2026-07-25 a corroborated bucket-2 ruling
(SG-6508) sat ~3 watcher cycles undecided while the whole critical path serialized behind it — the
delegation existed, but nothing *scheduled* the deciding. So:

- **EVERY agent runs a mini-steer every campaign-watcher iteration** — `.claude/skills/campaign-watcher/SKILL.md`
  **step 5b, Steer-drain**: sweep the open `captain-decision` signals and adjudicate every
  bucket-1/2 one that carries a fleet recommendation + independent corroboration. Applying an
  EXISTING Captain ruling to a new case is bucket-2, not bucket-3.
  > **Re-pointed at CH-11989.** This mandate used to be scheduled against `voyage-watcher Phase
  > 0b-drain`. campaign-watcher became the resident loop every machine runs (EX-11924, Captain
  > 2026-09-08), and voyage-watcher's `0b`/`0b-drain` phases were cut with Layer A — so for the
  > duration the mandate pointed at a phase of a loop nobody ran, while the loop everybody ran never
  > mentioned deciding at all. The step-5b half is the same procedure, carried forward; only its
  > home moved.
  >
  > ⚠ None of that is a claim that `voyage-watcher` is RETIRED. SG-12234 (Captain 2026-09-12) amends
  > EX-11924 and rules it the VOYAGE tier, with `campaign-watcher` the CAMPAIGN tier. What was cut
  > here is the PHASE NUMBERING — `campaign-watcher` is not organised in those phases — so the
  > re-point stands either way. The tier is ruled but NOT YET RESTORED (`CH-12235`): the skill file
  > is still the retirement stub, so do not send a reader there expecting a loop.
  >
  > De-gated from "the designated planner (M5)" by Captain ruling 2026-08-15 — *planning is an
  > ephemeral task like any other*. The cadence argument above is the reason this had to change:
  > a mandate scheduled against ONE machine is only as reliable as that machine's uptime, which
  > is a weaker guarantee than the SG-6508 incident demanded. The classifier, the corroboration
  > requirement and the bucket-3 escalation are all unchanged — de-gating moved WHO, not WHAT.
- **SLA: no decidable signal survives more than one iteration, by any agent.** A signal with a
  written, corroborated recommendation is *fleet debt*, not Captain debt.
- **Bucket-3 signals get a pre-filled recommendation** (comment) so the Captain's ruling is a
  30-second confirm — the queue the Captain sees should contain only real calls, each with a
  proposed answer.
- Every planner adjudication is **flagged for Captain veto** in the session report — decided ≠
  silent (see Guardrails).

## Authorization

Captain-directed (2026-07-08): the fleet is authorized to run this outer loop and to **decide**
every bucket-1 and bucket-2 call autonomously, escalating only genuine bucket-3 calls — a change
to a FEATURE or a GOAL, or a call needing human authority.
This is a standing mandate, like the campaign-watcher's execution mandate — the point is that the
team stops stalling on decisions it can make from goals and data.
