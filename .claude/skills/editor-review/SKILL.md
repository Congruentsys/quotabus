---
name: editor-review
description: Mock target-journal editor review of a paper draft. A non-author session role-plays the editor of the paper's target venue(s), reviews the draft as that editor would, saves the review, and the author makes edits — BEFORE the Captain reads. The research analogue of pairit's reviewer step.
disable-model-invocation: false
argument-hint: "<PAPER-NNN | draft path>"
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(PYTHONPATH= .venv/bin/yurtle-kanban *), Bash(git *), Bash(claude *), Read, Write, Edit
---

# Editor Review — mock target-journal review before Captain read

**Board writes reach `origin` at once** (CLAUDE.md § Syncing the board): yurtle-kanban 3.4.0 pushes `move`,
`comment` and `rank` by default; `create`/`update` take `--push` (with `ME` set in the same Bash call).

> Ported from nusy-product-team `editor-review` (2026-10-02), nk → yurtle-kanban 3.2.0; adapted to quotabus 2026-10-08 (yurtle-kanban 3.4.0).

The Captain's standing rule: **before he reads a paper draft, a different session pretends to be
the editor of the target journal and reviews it.** This front-loads venue-fit and rigor fixes so
the Captain's reading time is spent on a near-submittable draft. It is the paper analogue of the
distinct reviewer in `pairit`.

**Pipeline position:** LIT review → draft → **/editor-review (this skill) → saved review → author
edits** → Captain reads → Captain feedback → revise → submit.

## Argument

`/editor-review <PAPER-NNN | draft path>` — the paper to review (e.g. `PAPER-003`, or a path to the
draft markdown). Resolve its **target venue(s)** from the paper item / its LIT review.

## Rules

- **Reviewer ≠ author** — a DISTINCT SESSION, the same bar `pairit`'s reviewer meets. If you wrote
  the draft, do not review it here: spawn a fresh `claude -p` child, given the draft, the LIT review
  and this skill — never your own reasoning about the draft — and have IT review. A review is valid
  only if it comes from a context that wrote none of the work.
- **Role-play the actual editor** of the named venue — adopt that venue's scope, standards, and
  typical reviewer concerns (don't give a generic review).
- **Save the review** as a durable artifact — chat-only reviews are lost.
- **Be a real editor:** give a decision (Accept / Minor revision / Major revision / Reject) with the
  reasoning a desk editor + 2 reviewers would. Honesty over kindness — the goal is acceptance.

## Pipeline

### 1 — Identify the paper + venue

```bash
.venv/bin/yurtle-kanban show PAPER-NNN       # target venue, related LIT review + draft path
```

Read the **draft** and its **LIT review** (prior art /
publishability). Confirm you are not the author (`git log --format='%an %s' -- <draft path>`, and
the item's assignee). Identify the target venue(s).

### 2 — Review as the target-journal editor

Assess, in the voice of that venue's editor:

1. **Scope fit** — is this in-scope for the venue? Right audience?
2. **Novelty / contribution** — is the claimed contribution non-obvious vs the LIT review's prior
   art? What's the delta?
3. **Rigor / evidence** — are claims backed by the linked experiments and their eval data? Any
   unbacked empirical claim — one with no measurement and command behind it? Stats/baselines sound?
4. **Clarity / structure** — abstract, framing, figures, reproducibility.
5. **Honesty** — overclaim? Are limitations and costs stated?
6. **Decision + required revisions** — Accept / Minor / Major / Reject, with a numbered list of the
   changes needed to reach acceptance, and an honest **acceptance-likelihood** estimate.

### 3 — Save the review

Write it next to the paper, named by venue:

```text
docs/reviews/EDITOR-REVIEW-PAPER-NNN-<venue>.md
```

Include: venue, reviewer (session), date, the 6 assessments above, the decision, the numbered
required-revisions list, and the acceptance-likelihood. Commit it straight to `main` (findings are
docs).

### 4 — Author makes edits

Hand the saved review to the author:

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
.venv/bin/yurtle-kanban comment PAPER-NNN --agent "$ME" --body "Editor-review (<venue>): <decision>. Review: <path>."
```

The author addresses the required revisions and updates the draft. Re-run `/editor-review` if the
decision was Major-revision/Reject and a re-review is warranted.

### 5 — Then the Captain reads

Only after the editor-review + author edits does the Captain read. Comment the PAPER item (it lands on origin at once):

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
.venv/bin/yurtle-kanban comment PAPER-NNN --agent "$ME" --body "Editor-review (<venue>) complete: <decision>; edits applied; ready for Captain read. Review: <path>."
```

## Guardrails

- Reviewer ≠ author; role-play the real venue; save the review (never chat-only).
- The Captain still reads + gives the final feedback — this does not replace his review, it
  precedes it.
- Publication go/no-go remains the **Captain's** call: the agent assesses and recommends; the
  Captain decides to submit.
