---
name: refine-idea
description: "Take a captured IDEA through review-and-refine to FILED work — validate the premise, split by measurement, define under the citation rule, and land the edges so the pass survives the session"
disable-model-invocation: false
argument-hint: "IDEA-XXXX"
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/refine-idea/SKILL.md` on
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

# Refine Idea — the planning step between capture and work

> **New session?** Read [`docs/AUTONOMOUS-FLEET.md`](../../../docs/AUTONOMOUS-FLEET.md) first — how to operate in the autonomous fleet: what a session is, who may review/revise/merge, and which older statements in this repo are retired.

**Captain-directed, 2026-08-13, verbatim:** *"this is the planning step we need to add to
kanban. IDEA -> review and refine -> HDD (turn into hypotheses, measures, expr) - Then define
work -> then ready (then the rest of the process we have)."*

This skill **assembles existing mechanisms**. It introduces no new judgement machinery — the
adversarial pass, the `/steer` classifier, `/hypothesize`, the CH-6369 citation rule and the
reviewer≠author definition pass all already exist. What did not exist is the **step that files
the result**, which is why the pass has historically produced nothing durable.

## Why this exists — the measured failure

The pipeline was run by hand on two ideas. It worked, and it produced **nothing**.

| | |
|---|---|
| definitions produced | **16** (9 + 7) — `CH`×4, `M`×4, `HZ`×2, `H`×2, `EXPR`×2, `SG`×1, `EX`×1 |
| of those, filed | **0** |
| ideas rescoped by the pass | **2 of 2** — neither was buildable as written |
| ideas killed by the pass | **0 of 2** (the review edits; it does not filter) |

The refine stage paid for itself: it caught a premise that was **measurably false**, a Captain
ruling made eight days earlier that the idea did not cite, and two crates the ideas proposed
to build that were **already in-tree**. All of that evaporated with the session.

⚠ **The lesson is not "review harder."** The review was good. There was no filing step, so
the analysis had nowhere durable to go. Phase 5 is the reason this skill exists; phases 1–4
are the part that already worked.

## Two shapes to expect, because both are counter-intuitive

**An idea fans out; it does not become one item.** Sixteen definitions from two ideas —
**half of them development-board work**, only **two** hypotheses. The old ontology comment
("promoted to Hypothesis") described the intended case, not the dominant one.

**So do NOT emit a hypothesis per idea.** Apply the classifier **per definition**, not per
idea. Observed rate: ~1 hypothesis per idea across 8 definitions. A pass that scaffolds an
H→M→EXPR trio by default is the **over-formalization failure** CLAUDE.md names explicitly
("forcing an EXPR onto them is the over-formalization failure mode") — treat it as a defect
in this skill, not as rigor.

## Required environment

```bash
alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'
```

---

## Phase 1 — Claim

```bash
nk move IDEA-XXXX refining --assign "$(resolve_session_name)"
```

`refining` is a legal status: it is in the engine's `VALID_STATUSES` and in
`.yurtle-kanban/config.yaml`'s `type_states.idea`. If this move is REFUSED with
`UNKNOWN_STATUS`, this machine's **server** predates that (validation is server-side —
CH-7280), and the remedy is the writer rebuild + swap, not `--force`.

**The claim is reversible and the release is one command.** A dead session must never wedge
an idea here:

```bash
nk move IDEA-XXXX captured        # ungated, always available
```

## Phase 2 — Validate the premise, adversarially

Spawn ONE fresh sub-agent (routine or strong tier by the idea's stakes) with the REFUTE mandate
below as its whole prompt, and write its findings to an artifact whose header names path
(subagent), model and status. The sub-agent is the path (SG-10917 / CH-11596); a Copilot-CLI
run is opportunistic and never required. **An artifact with no findings and no header is a pass
still owed, not a skip.**

**Mandate the reviewer to REFUTE AGAINST THE CODE, not against plausibility.** That is what
caught all three real findings on the worked pair. Concretely: does the premise reproduce?
Does the thing it proposes to build already exist? Has a ruling already settled it?

⚠ **A premise that sounds precise is the dangerous kind.** One idea claimed a surface
*"refuses cleanly rather than returning a wrong answer"*; running it returned a **wrong
answer**, silently. Nothing but execution would have caught that.

## Phase 3 — Verdict

One of three, recorded as a comment on the idea:

- **KILL** — go to Phase 6. This is a **success**, and the pass has produced **zero** so far
  across two ideas, which is itself a signal worth watching.
- **VALID-AS-WRITTEN** — rare (0 of 2).
- **VALID-BUT-RESCOPED** — the common case (2 of 2). Record *what changed and why*.

Apply `/steer`'s classifier to any decision the verdict needs: *could a measurement settle
it?* → science; *do the goals settle it?* → decide now; *does it change a FEATURE or a GOAL, or
need genuine human authority?* → escalate.

## Phase 4 — Define

**Per definition, ask the classifier** — *"could a measurement change the answer?"*

- **yes** → `/hypothesize "<claim>"` for **that definition** (H + M + EXPR trio).
- **no** → an ordinary work item. This is the **majority**; see the shapes above.

Write 6-element bodies under the **citation rule (CH-6369)**: never write an enumerated list
from memory — every gate number, lifecycle sequence and dependency claim is quoted with a
file+section citation, and campaign-scoped items carry a `Docs:` line.

Then the **definition review**: reviewer ≠ author, **session**-scoped (CH-7739), distinct
model. Adopt `/plan-phase` step 6's **Release split** verbatim — Captain-directed scope
**auto-releases** after the pass; only genuinely **new** scope stays in the `planning`
STATUS for a decision packet (the retired `pending-ratification` tag is inert).

## Phase 5 — FILE, and land the edge — ⚠ THE STEP THAT DID NOT EXIST

**File with the edge at CREATE time.** A bad edge refuses the whole create, so no orphan is
ever minted:

```bash
nk create <type> "<title>" --body-file <draft> --relate formalizedFrom:IDEA-XXXX --push
```

For an item that already exists, attach it:

```bash
nk update IDEA-XXXX --relate formalizedAs:<ID>
```

Both spellings resolve to the one stored edge — the server normalizes the inverse into the
stored direction — so write whichever is natural at the moment of filing.

Then ask the gate whether this idea may be marked filed:

```bash
nk show IDEA-XXXX     # read its `formalizedAs` edges; then `nk show <target>` for each — every target must resolve
#   0 PASS     >=1 formalizedAs edge AND every target resolves
#   1 REFUSE   no edges, or a target that does not exist (both named)
#   2 CANNOT-ASSESS  never a pass
```

⚠ **The second arm is not paperwork.** An unresolvable relate target is **accepted at write
time on purpose** (cross-board research targets must work), so a typo'd id would satisfy an
edge count while nothing was filed. The gate refuses; it does not move. On PASS:

```bash
nk move IDEA-XXXX formalized --resolution completed
```

**Two-way traceability is then a single question each:**

```bash
nk relation query IDEA-XXXX     # what did this idea become?
nk relation query <ITEM>        # which idea spawned this?
```

## Phase 6 — The death path, and why it is deliberately asymmetric

**Kill from `captured` — one command, no reason, no review:**

```bash
nk move IDEA-XXXX abandoned --resolution wont_do
nk comment IDEA-XXXX "[refine-idea] killed at capture — <one line>"
```

**Kill from `refining` — record WHICH objection killed it, with its citation.** Checked for
non-emptiness only.

**The asymmetry is the design.** From `captured` nobody invested anything. From `refining`
the **refutation IS the product** — losing it makes the next filer re-derive the whole
investigation, the shape that cost CH-6464 six separate investigations of one void premise.

**Killing must stay cheaper than advancing**, and resurrection cheaper still
(`nk move IDEA-XXXX captured`, ungated, from either state). A cheap kill is only safe when
resurrection is cheap — and the pipeline's measured defect is that it is a **ratchet** (3 of
39 ideas ever resolved), so anything that makes killing expensive makes the defect worse.

## Health

```bash
nk list --board research --type idea    # read the pipeline by eye; no script sweeps it since CH-11826
```

Report-only, never a gate: ideas claiming a stage with no graph record, unresolvable edge
targets, statuses outside the declared lifecycle, and the captured-pool count.

⚠ **Age is reported, never gated.** Canon forbids date-based criteria on work while naming
Work Item Age as a mandatory flow metric — consistent, because a deadline decides and a
metric informs.

## Authorization

Captain-directed (2026-08-13). No ratification gate on the pipeline itself; individual
definitions follow the normal `planning`-STATUS rules via the Release split in Phase 4
(the retired `pending-ratification` tag is inert).
