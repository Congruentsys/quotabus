---
name: uxr-review
description: UXR expert review + simulated user study for human/agent surfaces. A non-author session role-plays a UXR expert, runs a simulated study with declared personas, saves the review with the evidence-class caveat, and the author makes edits — BEFORE the Captain reads. It EXTENDS the single-reviewer pattern of pairit's reviewer and /editor-review by adding declared personas, sub-agent participants and a study protocol.
disable-model-invocation: false
argument-hint: "<ITEM-ID | prototype path>"
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(PYTHONPATH= .venv/bin/yurtle-kanban *), Bash(git *), Bash(claude *), Read, Write, Edit, Agent
---

# UXR Review — UX expert review + simulated study before Captain read

**Board writes reach `origin` at once** (CLAUDE.md § Syncing the board): yurtle-kanban 3.4.0 pushes `move`,
`comment` and `rank` by default; `create`/`update` take `--push` (with `ME` set in the same Bash call).

> Ported from nusy-product-team `uxr-review` (2026-10-02), nk → yurtle-kanban 3.2.0; adapted to quotabus 2026-10-08 (yurtle-kanban 3.4.0).

The Captain's standing rule: **before he reads a human/agent surface (UI, API, data format), a
different session pretends to be a UXR expert and runs a simulated user study.** This front-loads
usability and surface-design fixes so the Captain's reading time is spent on a near-shippable
surface. It occupies the same pipeline position as `pairit`'s reviewer.

**Pipeline position:** Build → prototype → **/uxr-review (this skill) → saved review + study results
→ author edits** → Captain reads → Captain feedback → revise → ship.

## Where this sits — read before assuming it is "just another review skill"

**It EXTENDS the review pattern; it does not instance it.** `/editor-review` and `pairit`'s reviewer
are single-reader desk reviews: one reader, one artifact, a fixed set of assessment criteria. This
skill adds machinery neither has — **3–5 declared personas, sub-agents played as participants, a
think-aloud protocol, and a results matrix**. Calling it "the same pattern as `/editor-review`"
understates that by a wide margin, and the honest framing matters because the added machinery is
exactly what the evidence-class caveat below exists to contain.

**Scale the study to the change.** A small surface change needs a heuristic pass by the reviewer,
not a full simulated study. Reach for THIS skill when a surface warrants a full simulated study
before the Captain reads it.

**Trigger:** An item has a **UI surface for humans** (dashboards, CLI, error messages, docs
navigation) **or** a **data surface AI agents consume** (file formats, API shapes, path
conventions). Most items have both.

## Argument

`/uxr-review <ITEM-ID | prototype path>` — the surface to review (e.g. `EXP-004`, or a path to the
prototype). Resolve its **target users** from the item body / linked hypothesis.

## Rules

- **Reviewer ≠ author** — a DISTINCT SESSION, the same bar `pairit`'s reviewer and `/editor-review`
  meet. If you built the surface, do not review it here: spawn a fresh `claude -p` child, given the
  surface, the item and this skill — never your own reasoning about the surface — and have IT run
  the review. A review is valid only if it comes from a context that wrote none of the work.
- **Role-play as a UXR expert** — adopt a UX researcher's lens: task completion, cognitive load,
  error recovery, discoverability. Don't give a generic code review.
- **Run a simulated user study** with sub-agents playing declared personas — record tasks attempted,
  completion, steps-to-answer, errors encountered.
- **Save the review + study results** as a durable artifact — chat-only reviews are lost.
- **Apply the evidence-class caveat** (below) — simulated participants are LLM-generated evidence
  about human behaviour, the weakest evidence class this lab produces.
- **Be a real UXR:** give a decision (Ship / Minor revision / Major revision / Block) with the
  reasoning a UXR expert + simulated study would. Honesty over kindness — the goal is usability.

## The Evidence-Class Caveat (non-negotiable)

**Simulated participants are LLM-generated evidence about human behaviour.** LLM-derived findings
are neural-class evidence and are never promoted to proven; a simulated study's numbers carry the
same caveat and **must never be quoted as measured human behaviour**.

**Correct claim shape:** *"The simulated study surfaced N candidate problems; K were confirmed
against the real surface"* — **never** *"users take 4.2s"* or *"75% of users failed X"*.

**What simulated studies CAN do:** surface candidate usability problems, identify unclear paths,
reveal cognitive-load hot spots. **What they CANNOT establish:** that real humans hit the problem at
rate X, or that real users behave this way.

## Pipeline

### 1 — Identify the surface + target users

```bash
.venv/bin/yurtle-kanban show <ITEM-ID>       # surface description, target users, related H/M/EXPR
```

Read the **prototype** (or the built surface) and its
**hypothesis** (what claim it validates). Confirm you are not the author (the item's assignee, and
`git log --format='%an %s' -- <surface paths>`). Identify the target users (developers? agents?
end-users? domain experts?).

### 2 — Declare participant personas

Based on the target users, declare **3–5 participant personas** who will be simulated in the study.
Each persona must state:

1. **Role / background** — who they are (e.g. "junior dev, 6 months experience", "an LLM agent with
   no repo context")
2. **Goals** — what they want from this surface
3. **Constraints** — pressure, unfamiliarity, accessibility needs
4. **Why this persona** — why they were chosen (coverage of user diversity)

**Example personas for an observability dashboard:**
- **Persona A**: Captain (executive, wants high-level status at a glance, attention-constrained)
- **Persona B**: Debugging agent (needs exception drill-down, unfamiliar with the codebase)
- **Persona C**: New session (just spawned, zero context, needs an onboarding surface)

### 3 — Design the simulated study protocol

Define **3–5 tasks** each persona will attempt:

1. **Task description** — what the participant is asked to do (e.g. "find the current blocker on
   VOY-NNN")
2. **Success criteria** — what counts as task completion
3. **Metrics** — completion (yes/no), steps-to-answer, errors encountered, help sought

**Example tasks for the dashboard:**
- Task 1: Identify which agent is currently blocked
- Task 2: Drill down to see why a specific exception happened
- Task 3: Find when a specific item was last claimed

### 4 — Run the simulated study

For each persona, spawn a **sub-agent** (the `Agent` tool) playing that persona's role. Give it:

1. The persona card (role, goals, constraints)
2. The task list
3. Access to the prototype / surface
4. **Instructions to think aloud**: narrate their thought process, where they look, what's unclear

Record for each task:
- Did they complete it? (yes/no)
- How many steps did it take?
- What errors/confusion occurred?
- What did they try that didn't work?
- What help did they seek?

### 5 — Review as a UXR expert

Assess, in the voice of a UX researcher:

1. **Task completion rate** — % of tasks completed across all personas (a property of the
   simulation)
2. **Discoverability** — could participants find the surface's features? Were paths intuitive?
3. **Cognitive load** — were participants overwhelmed? Too much info at once? Unclear labels?
4. **Error recovery** — when participants made mistakes, could they recover? Were error messages
   helpful?
5. **Consistency** — did the surface follow its own conventions? Familiar patterns?
6. **Accessibility** — agent usability (can AI agents parse it?), human accessibility (screen
   readers, keyboard nav)
7. **Evidence-class caveat** — restate that findings are LLM-simulated, not measured human behaviour
8. **Decision + required revisions** — Ship / Minor / Major / Block, with a numbered list of the
   changes needed, and an honest **usability confidence** estimate

### 6 — Save the review + study results

Write it under `docs/reviews/`, named by item:

```text
docs/reviews/UXR-REVIEW-<ITEM-ID>.md
```

Include:
- Surface reviewed (item ID, path)
- Reviewer (session)
- Date
- **Declared participant personas** (the 3–5 personas + why chosen)
- Study protocol (tasks, metrics)
- Results table (persona × task, completion/steps/errors)
- The 8 assessments above
- **Evidence-class caveat** (restated prominently)
- Decision
- Numbered required-revisions list
- Usability confidence estimate

#### ⚠ SEPARATE THE EVIDENCE TIERS IN THE ARTIFACT — three headed sections, never one findings list

The caveat above is stated three times in this skill and is still not sufficient on its own,
because the ARTIFACT outlives the reading of it. A single merged findings list makes a
simulated-participant observation and a heuristic-expert judgement typographically identical, and
the next reader — quoting a number into a finding or a paper months later — cannot tell them apart.

```markdown
## Findings — HEURISTIC-EXPERT (reviewer judgement)
   The UXR reviewer's own assessment. Strongest tier here. Not a measurement of
   anyone's behaviour, and does not claim to be.

## Findings — SIMULATED-PARTICIPANT (LLM-generated, neural-class evidence)
   CANDIDATE problems surfaced by played personas. Never promoted to proven. Every number
   in this section is a property of the simulation, never of humans. Correct shape:
   "the simulated study surfaced N candidate problems" — never "users take 4.2s".

## Findings — REAL-HUMAN (empty unless a real human was actually observed)
   Leave the section PRESENT and EMPTY when none exists. An absent heading reads as
   "not applicable"; an empty one reads as "none gathered", which is the true state
   and the thing a later reader needs to know.
```

**A finding that gets CONFIRMED against the real surface graduates from the simulated section to
the heuristic one — move it, and say what confirmed it.** That migration is the only sanctioned path
upward; nothing graduates into REAL-HUMAN without a real human.

Commit it straight to `main` (findings are docs).

### 7 — Author makes edits

Hand the saved review to the author:

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
.venv/bin/yurtle-kanban comment <ITEM-ID> --agent "$ME" --body "UXR-review: <decision>. Review: <path>."
```

The author addresses the required revisions (code changes land through `pairit`) and updates the
surface. Re-run `/uxr-review` if the decision was Major-revision/Block and a re-review is warranted.

### 8 — Then the Captain reads

Only after the UXR-review + author edits does the Captain read. Comment the item (it lands on origin at once):

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
.venv/bin/yurtle-kanban comment <ITEM-ID> --agent "$ME" --body "UXR-review complete: <decision>; edits applied; ready for Captain read. Review: <path>."
```

## Guardrails

- Reviewer ≠ author; role-play as UXR expert; declare participant personas; save the review + study
  results (never chat-only).
- **Evidence-class caveat is mandatory** — simulated findings are LLM-generated, the weakest evidence
  class; never quote them as measured human behaviour.
- The Captain still reads + gives the final feedback — this does not replace his review, it
  precedes it.
- Ship go/no-go remains the **Captain's** call: the agent assesses and recommends; the Captain
  decides to ship.
