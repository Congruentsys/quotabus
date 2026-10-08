---
name: uxr-review
description: UXR expert review + simulated user study for human/agent surfaces. A non-author agent role-plays a UXR expert, runs a simulated study with declared personas, saves the review with the evidence-class caveat, and the author makes edits — BEFORE the Captain reads. It EXTENDS the single-agent review pattern of /reviewit and /editor-review by adding declared personas, sub-agent participants and a study protocol.
disable-model-invocation: false
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/uxr-review/SKILL.md` on
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

> **New session?** Read [`docs/AUTONOMOUS-FLEET.md`](../../../docs/AUTONOMOUS-FLEET.md) first — how to operate in the autonomous fleet: what a session is, who may review/revise/merge, and which older statements in this repo are retired.

# UXR Review — UX expert review + simulated study before Captain read

The Captain's standing rule: **before he reads a human/agent surface (UI, API, data format), a different agent pretends to be a UXR expert and runs a simulated user study.** This front-loads usability and surface-design fixes so the Captain's reading time is spent on a near-shipable surface. It occupies the same PIPELINE POSITION as `/reviewit`.

**Pipeline position:** Build → prototype → **/uxr-review (this skill) → saved review + study results → author edits** → Captain reads → Captain feedback → revise → ship.

## Where this sits relative to existing canon — read before assuming it is "just another review skill"

**It EXTENDS the review pattern; it does not instance it.** `/editor-review` and `/reviewit` are single-agent desk reviews: one reader, one artifact, a fixed set of assessment criteria. This skill adds machinery neither has — **3–5 declared personas, sub-agents played as participants, a think-aloud protocol, and a results matrix**. Calling it "the same pattern as `/editor-review`" understates that by a wide margin, and the honest framing matters because the added machinery is exactly what the evidence-class caveat below exists to contain (DGX1's finding on PROP-4812).

**It is a standalone skill by RULING, not by oversight.** [`.claude/docs/ux-study-protocols.md`](../../docs/ux-study-protocols.md) (CH-6854) states that it is *"referenced by existing skills, never routed to by a skill of its own"*. **SG-8507** ruled that clause superseded for REVIEW skills, applying the Captain's later CH-8375 direction: this is a review skill, a different artifact class from the `/ux-study` skill that clause forbade. The supersession is recorded in that doc too, so a reader arriving from either direction finds it.

**That doc remains the measurement-protocol source of record, and this skill REFERENCES it rather than restating it.** Its tiering table (§d) decides how much study fires at what change size, and its §a operability protocol / §b heuristic pre-pass are the methods to reach for on an ordinary surface change. Reach for THIS skill when a surface warrants a full simulated study before the Captain reads it. **Do not copy protocol text out of that doc into here** — a second copy is the drift surface CH-6854 exists to prevent.

**Trigger:** An item has a **UI surface for humans** (dashboards, CLI, error messages, docs navigation) **or** a **data surface AI beings consume** (file formats, API shapes, path conventions). Most items have both — see `docs/DEVELOPMENT_PRACTICES.md` §3 for the dual-surface framing.

## Argument

`/uxr-review <ITEM-id | prototype path>` — the surface to review (e.g. `EXP-8372`, or a path to the prototype). Resolve its **target users** from the item body / linked hypothesis.

## Rules

- **Reviewer ≠ author** — a DISTINCT SESSION, the same bar `reviewit` and `editor-review` apply. If you built the surface, do not review it here: spawn a fresh `claude -p` child with its own `NUSY_SESSION_ID` and have IT review, exactly as `campaign-watcher` §7.2 spawns a code reviewer. ⚠ Not "another agent" and not "cross-agent": post-surgery the reviewer is a distinct session on the SAME machine, Layer A is deleted, and leaving a surface for another machine routes it to nobody (HZ-12459).
- **Role-play as a UXR expert** — adopt a UX researcher's lens: task completion, cognitive load, error recovery, discoverability. Don't give a generic code review.
- **Run a simulated user study** with sub-agents playing declared personas — record tasks attempted, completion rate, time-to-answer, errors encountered.
- **Save the review + study results** as a durable artifact — chat-only reviews are lost.
- **Apply the evidence-class caveat** (see below) — simulated participants are LLM-generated evidence about human behaviour, the weakest evidence class this fleet produces.
- **Be a real UXR:** give a decision (Ship / Minor revision / Major revision / Block) with the reasoning a UXR expert + simulated study would. Honesty over kindness — the goal is usability.

## The Evidence-Class Caveat (non-negotiable)

**Simulated participants are LLM-generated evidence about human behaviour.** Under the never-launder floor, LLM-derived findings are `certifiability_class="neural"` and barred from Proven. A simulated study's numbers must carry the same caveat and **must never be quoted as measured human behaviour**.

**Correct claim shape:** *"The simulated study surfaced N candidate problems; K were confirmed against the real surface"* — **never** *"users take 4.2s"* or *"75% of users failed X"*.

**What simulated studies CAN do:** surface candidate usability problems, identify unclear paths, reveal cognitive-load hot spots. **What they CANNOT establish:** that real humans hit the problem at rate X, or that real users behave this way.

## Pipeline

### 1 — Identify the surface + target users

```bash
alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'
nk show <ITEM-id>            # surface description, target users, linked H/M/EXPR
```

Read the **prototype** (or the built surface) and its **hypothesis** (what claim it validates). Confirm you are not the author (check the item's assignee / PR). Identify the target users (developers? agents? end-users? domain experts?).

### 2 — Declare participant personas

Based on the target users, declare **3–5 participant personas** who will be simulated in the study. Each persona must state:

1. **Role / background** — who they are (e.g. "junior dev, 6 months experience", "GPT-4-based agent with no repo context")
2. **Goals** — what they want from this surface
3. **Constraints** — time pressure, unfamiliarity, accessibility needs
4. **Why this persona** — why they were chosen (coverage of user diversity)

**Example personas for a fleet observability dashboard:**
- **Persona A**: Captain (executive, wants high-level status at a glance, time-constrained)
- **Persona B**: Debugging agent (needs exception drill-down, unfamiliar with the codebase)
- **Persona C**: New session (just spawned, zero context, needs onboarding surface)

### 3 — Design the simulated study protocol

Define **3–5 tasks** each persona will attempt:

1. **Task description** — what the participant is asked to do (e.g. "find the current blocker on VY-XXXX")
2. **Success criteria** — what counts as task completion
3. **Metrics** — completion (yes/no), time-to-answer (seconds/steps), errors encountered, help sought

**Example tasks for the dashboard:**
- Task 1: Identify which agent is currently blocked
- Task 2: Drill down to see why a specific exception happened
- Task 3: Find when a specific item was last claimed

### 4 — Run the simulated study

For each persona, spawn a **sub-agent** (via `Agent` tool) playing that persona's role. Give the sub-agent:

1. The persona card (role, goals, constraints)
2. The task list
3. Access to the prototype / surface
4. **Instructions to think-aloud**: narrate their thought process, where they look, what's unclear

Record for each task:
- Did they complete it? (yes/no)
- How long did it take? (steps / time estimate)
- What errors/confusion occurred?
- What did they try that didn't work?
- What help did they seek?

### 5 — Review as a UXR expert

Assess, in the voice of a UX researcher:

1. **Task completion rate** — % of tasks successfully completed across all personas
2. **Discoverability** — could participants find the surface's features? Were paths intuitive?
3. **Cognitive load** — were participants overwhelmed? Too much info at once? Unclear labels?
4. **Error recovery** — when participants made mistakes, could they recover? Were error messages helpful?
5. **Consistency** — did the surface follow its own conventions? Familiar patterns?
6. **Accessibility** — agent usability (can AI agents parse it?), human accessibility (screen readers, keyboard nav)
7. **Evidence-class caveat** — restate that findings are LLM-simulated, not measured human behaviour
8. **Decision + required revisions** — Ship / Minor / Major / Block, with a numbered list of the changes needed, and an honest **usability confidence** estimate

### 6 — Save the review + study results

Write it next to the surface, named by item:

```text
research/<area>/UXR-REVIEW-<ITEM-id>.md
```

**OR** if the surface is part of a voyage/expedition:

```text
nusy-kanban/work/<type>/<ID>/UXR-REVIEW-<ITEM-id>.md
```

Include:
- Surface reviewed (item ID, path)
- Reviewer (agent)
- Date
- **Declared participant personas** (the 3–5 personas + why chosen)
- Study protocol (tasks, metrics)
- Results table (persona × task, completion/time/errors)
- The 8 assessments above
- **Evidence-class caveat** (restated prominently)
- Decision
- Numbered required-revisions list
- Usability confidence estimate

#### ⚠ SEPARATE THE EVIDENCE TIERS IN THE ARTIFACT — three headed sections, never one findings list

The caveat above is stated three times in this skill and is still not sufficient on its own, because the ARTIFACT outlives the reading of it. A single merged findings list makes a simulated-participant observation and a heuristic-expert judgement typographically identical, and the next reader — quoting a number into a PROP body or a paper months later — cannot tell them apart. The existing doc already draws this line between its protocol (a) and protocol (b); the artifact must draw it too.

```markdown
## Findings — HEURISTIC-EXPERT (reviewer judgement)
   The UXR reviewer's own assessment. Strongest tier here. Not a measurement of
   anyone's behaviour, and does not claim to be.

## Findings — SIMULATED-PARTICIPANT (LLM-generated, `certifiability_class="neural"`)
   CANDIDATE problems surfaced by played personas. Barred from Proven. Every number
   in this section is a property of the simulation, never of humans. Correct shape:
   "the simulated study surfaced N candidate problems" — never "users take 4.2s".

## Findings — REAL-HUMAN (empty unless a real human was actually observed)
   Leave the section PRESENT and EMPTY when none exists. An absent heading reads as
   "not applicable"; an empty one reads as "none gathered", which is the true state
   and the thing a later reader needs to know.
```

**A finding that gets CONFIRMED against the real surface graduates from the simulated section to the heuristic one — move it, and say what confirmed it.** That migration is the only sanctioned path upward; nothing graduates into REAL-HUMAN without a real human.

Commit it (docs PR or directly per the research-doc convention).

### 7 — Author makes edits

Hand the saved review to the author (comment the ITEM with the review path + decision). The author addresses the required revisions and updates the surface. Re-run `/uxr-review` if the decision was Major-revision/Block and a re-review is warranted.

### 8 — Then the Captain reads

Only after the UXR-review + author edits does the Captain read. Comment the ITEM:
`nk comment <ITEM-id> "UXR-review complete: <decision>; edits applied; ready for Captain read. Review: <path>."`

## Guardrails

- Reviewer ≠ author; role-play as UXR expert; declare participant personas; save the review + study results (never chat-only).
- **Evidence-class caveat is mandatory** — simulated findings are LLM-generated, weakest evidence class, never quote as measured human behaviour.
- The Captain still reads + gives the final feedback — this does not replace his review, it precedes it.
- Ship go/no-go remains the **Captain's** call (guardrail #6 spirit: the agent assesses + recommends; the Captain decides to ship).

## Cross-reference

This skill complements `docs/DEVELOPMENT_PRACTICES.md` §3 *Usability Testing*:
- **§3 is the pre-merge developer checklist** — can someone use this at all? (fresh-clone test, docs-only test, error-path test)
- **`/uxr-review` is the surface-level review** — is this the right surface, and does it work for the person it is for?

Both run; they do different jobs. The checklist gates merge; the review validates design.
