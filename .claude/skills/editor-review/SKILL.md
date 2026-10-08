---
name: editor-review
description: Mock target-journal editor review of a paper draft. A non-author agent role-plays the editor of the paper's target venue(s), reviews the draft as that editor would, saves the review, and the author makes edits — BEFORE the Captain reads. The research analogue of /reviewit.
disable-model-invocation: false
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/editor-review/SKILL.md` on
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

# Editor Review — mock target-journal review before Captain read

The Captain's standing rule: **before he reads a paper draft, a different agent pretends to be
the editor of the target journal and reviews it.** This front-loads venue-fit and rigor fixes so
the Captain's reading time is spent on a near-submittable draft. It is the paper analogue of
`/reviewit`.

**Pipeline position:** LIT review → draft → **/editor-review (this skill) → saved review → author
edits** → Captain reads → Captain feedback → revise → submit.

## Argument

`/editor-review <PAPER-id | draft path>` — the paper to review (e.g. `PAPER-4873`, or a path to
the draft markdown). Resolve its **target venue(s)** from the paper item / its LIT review.

## Rules

- **Reviewer ≠ author** — a DISTINCT SESSION, the same bar `reviewit` applies. If you wrote the
  draft, do not review it here: spawn a fresh `claude -p` child with its own
  `NUSY_SESSION_ID` and have IT review, exactly as `campaign-watcher` §7.2 spawns a code reviewer.
  ⚠ Not "another agent" and not "cross-agent": post-surgery the reviewer is a distinct session on
  the SAME machine, Layer A is deleted, and leaving a draft for another machine routes it to nobody
  (HZ-12459).
- **Role-play the actual editor** of the named venue — adopt that venue's scope, standards, and
  typical reviewer concerns (don't give a generic review).
- **Save the review** as a durable artifact — chat-only reviews are lost.
- **Be a real editor:** give a decision (Accept / Minor revision / Major revision / Reject) with
  the reasoning a desk editor + 2 reviewers would. Honesty over kindness — the goal is acceptance.

## Pipeline

### 1 — Identify the paper + venue

```bash
alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'
nk show <PAPER-id>            # claim, target venue, linked LIT review + draft path
```

Read the **draft** and its **LIT review** (prior art / publishability). Confirm you are not the
author (check the draft's authoring expedition / PR). Identify the target venue(s).

### 2 — Review as the target-journal editor

Assess, in the voice of that venue's editor:

1. **Scope fit** — is this in-scope for the venue? Right audience?
2. **Novelty / contribution** — is the claimed contribution non-obvious vs the LIT review's prior
   art? What's the delta?
3. **Rigor / evidence** — are claims backed by the linked experiments/eval-data? Any unbacked
   empirical claim (developer-scientist bar)? Stats/baselines sound?
4. **Clarity / structure** — abstract, framing, figures, reproducibility.
5. **Honesty** — overclaim? Are limitations + the (e.g. proof-carrying overhead) stated?
6. **Decision + required revisions** — Accept / Minor / Major / Reject, with a numbered list of
   the changes needed to reach acceptance, and an honest **acceptance-likelihood** estimate.

### 3 — Save the review

Write it next to the paper, named by venue:

```text
research/<paper-area>/EDITOR-REVIEW-<PAPER-id>-<venue>.md
```

Include: venue, reviewer (agent), date, the 6 assessments above, the decision, the numbered
required-revisions list, and the acceptance-likelihood. Commit it (docs PR or directly per the
research-doc convention).

### 4 — Author makes edits

Hand the saved review to the author (comment the PAPER item with the review path + decision). The
author addresses the required revisions and updates the draft. Re-run `/editor-review` if the
decision was Major-revision/Reject and a re-review is warranted.

### 5 — Then the Captain reads

Only after the editor-review + author edits does the Captain read. Comment the PAPER item:
`nk comment <PAPER-id> "Editor-review (<venue>) complete: <decision>; edits applied; ready for Captain read. Review: <path>."`

## Guardrails

- Reviewer ≠ author; role-play the real venue; save the review (never chat-only).
- The Captain still reads + gives the final feedback — this does not replace his review, it
  precedes it.
- Publication go/no-go remains the **Captain's** call (guardrail #6 spirit: the agent assesses +
  recommends; the Captain decides to submit).
