---
name: refine-idea
description: "Take a captured IDEA through review-and-refine to FILED work — validate the premise, split by measurement, define under the citation rule, and land the links so the pass survives the session"
disable-model-invocation: false
argument-hint: "IDEA-R-NNN"
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(PYTHONPATH= .venv/bin/yurtle-kanban *), Bash(git *), Bash(claude *), Read, Edit, Write, Agent
---

# Refine Idea — the planning step between capture and work

**Board writes reach `origin` at once** (CLAUDE.md § Syncing the board): `create`/`update` take `--push`; `move` and
`comment` go through `scripts/yk_push.sh` (with `ME` set in the same Bash call).

> Ported from nusy-product-team `refine-idea` (2026-10-02), nk → yurtle-kanban 3.2.0.

**Captain-directed, 2026-08-13, verbatim:** *"this is the planning step we need to add to kanban.
IDEA -> review and refine -> HDD (turn into hypotheses, measures, expr) - Then define work -> then
ready (then the rest of the process we have)."*

This skill **assembles existing mechanisms**: an adversarial pass, the measurement classifier,
`/hypothesize`, the citation rule and a reviewer ≠ author definition pass. What did not exist is
the **step that files the result**, which is why the pass historically produced nothing durable.

## Why this exists — the measured failure

The pipeline was run by hand on two ideas (in nusy-product-team). It worked, and it produced
**nothing**.

| | |
|---|---|
| definitions produced | **16** (9 + 7) — chores ×4, measures ×4, hazards ×2, hypotheses ×2, experiments ×2, signal ×1, expedition ×1 |
| of those, filed | **0** |
| ideas rescoped by the pass | **2 of 2** — neither was buildable as written |
| ideas killed by the pass | **0 of 2** (the review edits; it does not filter) |

The refine stage paid for itself: it caught a premise that was **measurably false**, a Captain
ruling made eight days earlier that the idea did not cite, and two components the ideas proposed to
build that were **already in-tree**. All of that evaporated with the session.

⚠ **The lesson is not "review harder."** The review was good. There was no filing step, so the
analysis had nowhere durable to go. Phase 5 is the reason this skill exists; phases 1–4 are the part
that already worked.

## Two shapes to expect, because both are counter-intuitive

**An idea fans out; it does not become one item.** Sixteen definitions from two ideas — **half of
them development-board work**, only **two** hypotheses.

**So do NOT emit a hypothesis per idea.** Apply the classifier **per definition**, not per idea.
Observed rate: ~1 hypothesis per idea across 8 definitions. A pass that scaffolds an H→M→EXPR trio
by default is the **over-formalization failure** ("forcing an EXPR onto them is the
over-formalization failure mode") — treat it as a defect in this skill, not as rigor. ⚠ It fails in
the other direction too: a definition that IS a claim a measurement could settle, filed as a plain
chore or sent to the Captain as a "decision", is an unfiled experiment.

## Required environment

Run from the repo root. The CLI is `.venv/bin/yurtle-kanban` (on a Spark, prefix `PYTHONPATH=`).
Name yourself on every board write: `--agent "$ME"`, with
`ME="${NUSY_AGENT_NAME:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"` set in the same command (shell
variables do not survive between tool calls; CLAUDE.md § The board). A move on an item held by someone is
refused unless you name yourself. `move`, `update` and `comment` commit locally: `git pull --rebase && git push` after each
phase (board files go straight to `main`).

**The idea lifecycle on this board** (`.venv/bin/yurtle-kanban states --board research`):

| this skill says | research-board status | how you get there |
|---|---|---|
| captured | `draft` | `idea create` puts it here |
| refining | `active` | `move … active --assign <agent>` |
| formalized | `complete` (`--resolution completed`) | Phase 5 |
| killed | `abandoned` (`--resolution wont_do`) | Phase 6 |

Legal moves: `draft → active | abandoned`, `active → complete | abandoned | draft`,
`abandoned → draft`. `complete` is terminal.

---

## Phase 1 — Claim

```bash
scripts/yk_push.sh move IDEA-R-NNN active --assign "$ME" --agent "$ME"
```

(`claim` does not apply: it takes only a `ready` item, and the research board has no `ready`
status.)

**The claim is reversible and the release is one command.** A dead session must never wedge an idea
here:

```bash
scripts/yk_push.sh move IDEA-R-NNN draft --agent "$ME"   # holder; a peer adds --take-over
```

## Phase 2 — Validate the premise, adversarially

Spawn ONE fresh sub-agent with the REFUTE mandate below as its whole prompt, and write its findings
to an artifact (`research/IDEA-R-NNN-refine-premise.md`) whose header names path (subagent), model
and status. **An artifact with no findings and no header is a pass still owed, not a skip.**

**Mandate the reviewer to REFUTE AGAINST THE CODE, not against plausibility.** That is what caught
all three real findings on the worked pair. Concretely: does the premise reproduce? Does the thing
it proposes to build already exist (here, in a sister repo, or in a pinned dependency)? Has a ruling
already settled it?

⚠ **A premise that sounds precise is the dangerous kind.** One idea claimed a surface *"refuses
cleanly rather than returning a wrong answer"*; running it returned a **wrong answer**, silently.
Nothing but execution would have caught that.

## Phase 3 — Verdict

One of three, recorded as a comment on the idea
(`scripts/yk_push.sh comment IDEA-R-NNN --agent "$ME" --body-file <a file>`; never `--body-file -`: a retried write would re-read an empty stdin):

- **KILL** — go to Phase 6. This is a **success**, and the pass produced **zero** across the first
  two ideas, which is itself a signal worth watching.
- **VALID-AS-WRITTEN** — rare (0 of 2).
- **VALID-BUT-RESCOPED** — the common case (2 of 2). Record *what changed and why*.

Apply the three-way classifier to any decision the verdict needs: *could a measurement settle it?*
→ science (`/hypothesize`); *do the goals settle it?* → decide now; *does it change a FEATURE or a
GOAL, or need genuine human authority?* → escalate to the Captain.

## Phase 4 — Define

**Per definition, ask the classifier** — *"could a measurement change the answer?"*

- **yes** → `/hypothesize "<claim>"` for **that definition** (H + M + EXPR trio).
- **no** → an ordinary development-board item (expedition, chore, hazard, signal). This is the
  **majority**; see the shapes above.

**Size by agent context, never by time.** A chore is a fraction of one context; an expedition is
what ONE agent can take from start to landed within its context; a voyage is 5–10 expeditions. An
item one agent cannot land before its context runs out is two expeditions. Never write a human time
estimate into a body.

Write 6-element bodies under the **citation rule**: never write an enumerated list from memory —
every gate number, lifecycle sequence and dependency claim is quoted with a file+section citation.

Then the **definition review**: reviewer ≠ author, a DISTINCT SESSION (a `claude -p` child that
wrote none of the definitions — the same reviewer `pairit` spawns), given the drafts and the
premise artifact, not the author's reasoning. Definitions that are Captain-directed scope may be
filed ready; genuinely **new** scope is filed in the backlog status (`harbor` / `draft`) with a
comment asking the Captain, not released.

## Phase 5 — FILE, and land the link — ⚠ THE STEP THAT DID NOT EXIST

**File each definition, citing the idea in its body** (an `Idea: IDEA-R-NNN` line):

```bash
# development board — expedition | chore | hazard | signal | voyage
.venv/bin/yurtle-kanban create <type> "<title>" --body-file <draft.md> --tags <tags> --push
# research board — via /hypothesize, with --source-idea IDEA-R-NNN on the hypothesis
```

Then record the formalizedAs set on the idea, and the inverse on each item. yurtle-kanban has no
typed edges (`formalizedFrom`/`formalizedAs`); the `related` list is the link, written both ways:

```bash
.venv/bin/yurtle-kanban update IDEA-R-NNN --related "<ID1>,<ID2>,…" --push   # REPLACES the list: pass the whole set
.venv/bin/yurtle-kanban update <ID>       --related "IDEA-R-NNN[,…]" --push
```

Do not use `--add-dep` for this: a dependency makes the item unpickable until the idea is done.

Then ask the gate whether this idea may be marked filed:

```bash
.venv/bin/yurtle-kanban show IDEA-R-NNN --json    # read `related`
.venv/bin/yurtle-kanban show <ID>                 # for EACH related id — rc 0 resolves, rc 1 does not
#   PASS           >=1 related id AND every one resolves
#   REFUSE         none, or a target that does not exist (name it)
#   CANNOT-ASSESS  the CLI errored — never a pass
```

⚠ **The second arm is not paperwork.** `update --related` accepts an ID that exists on no board
(measured 2026-10-02: `--related CHORE-999` was written without complaint), so a typo'd id would
satisfy a count while nothing was filed. The gate refuses; it does not move. On PASS:

```bash
scripts/yk_push.sh move IDEA-R-NNN complete --resolution completed --agent "$ME"
```

**Two-way traceability is then a single read each:** `show IDEA-R-NNN` lists what the idea became;
`show <ID>` lists the idea it came from. `hdd validate` checks the research-board half.

## Phase 6 — The death path, and why it is deliberately asymmetric

**Kill from `draft` (captured) — one command, no reason, no review:**

```bash
scripts/yk_push.sh move IDEA-R-NNN abandoned --resolution wont_do --agent "$ME"
scripts/yk_push.sh comment IDEA-R-NNN --agent "$ME" --body "[refine-idea] killed at capture — <one line>"
```

**Kill from `active` (refining) — record WHICH objection killed it, with its citation** (the
premise artifact path), in the comment. Checked for non-emptiness only.

**The asymmetry is the design.** From `draft` nobody invested anything. From `active` the
**refutation IS the product** — losing it makes the next filer re-derive the whole investigation
(one void premise once cost six separate investigations).

**Killing must stay cheaper than advancing**, and resurrection cheaper still
(`scripts/yk_push.sh move IDEA-R-NNN draft --agent "$ME"`, ungated, from `abandoned` or `active`). A cheap
kill is only safe when resurrection is cheap — and the pipeline's measured defect was that it is a
**ratchet** (3 of 39 ideas ever resolved), so anything that makes killing expensive makes the defect
worse.

## Health

```bash
.venv/bin/yurtle-kanban list --board research --type idea
.venv/bin/yurtle-kanban list --board research --type idea --older-than 14d    # age, reported
.venv/bin/yurtle-kanban hdd validate
```

Report-only, never a gate: ideas `complete` with no `related`, unresolvable `related` targets, and
the `draft` pool count.

⚠ **Age is reported, never gated.** A deadline decides; a flow metric informs.

## Authorization

Captain-directed (2026-08-13). No ratification gate on the pipeline itself; new scope waits in the
backlog for the Captain (Phase 4).
