---
name: hypothesize
description: "Scaffold a research question into the Hypothesis → Measure → Experiment trio so the rigorous path is cheaper than the ad-hoc one (AI developer-scientist discipline)"
disable-model-invocation: false
argument-hint: "<the claim a measurement could settle>"
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/hypothesize/SKILL.md` on
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

# Hypothesize — scaffold H → M → EXPR in one step

> **New session?** Read [`docs/AUTONOMOUS-FLEET.md`](../../../docs/AUTONOMOUS-FLEET.md) first — how to operate in the autonomous fleet: what a session is, who may review/revise/merge, and which older statements in this repo are retired.

Turn an empirical question into the three linked research-board items the
**AI Developer-Scientist Discipline** (CLAUDE.md) requires, so an agent never has
to choose between "do it scientifically" and "do it fast" — the scientific path
*is* the fast path.

**When to use:** the moment a work item asks a question a *measurement* could
settle — coverage, accuracy, latency, "X reproduces Y", "scales to N triples",
"zero hallucination". Apply the classifier first:

> **"Could a measurement change the answer?"**
>
> - **No** → don't use this skill. Plain engineering or a Captain ruling.
> - **Yes** → scaffold the trio here, *before* implementing.

**Canonical exemplar to copy:** **H-4694** (Triple-seam fidelity) → **EXPR-4697**
(seam-fidelity battery, all metrics 1.000, eval JSON linked).

## Required Argument

`$ARGUMENTS` — the **falsifiable claim** in one line, phrased so a number could
refute it. Good: "The Arrow forward-chain engine sustains derivation at 1M
triples within 3× of oxigraph." Bad: "make the engine fast."

## Required environment

```bash
alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'
```

## Pipeline

### 1. Sharpen the claim into a hypothesis

State, in the body you will pass to `nk`:

- **Claim** — `$ARGUMENTS`, the falsifiable statement.
- **Target** — the threshold that counts as validated (e.g. "coverage ≥ 0.9",
  "p50 latency ≤ 2× oxigraph").
- **Refutation** — what observation would *refute* it (forces falsifiability).
- **Why it matters** — the decision or launch claim it backs (e.g. "FOSS README
  scale claim").

Two further lines are **OPTIONAL** (CH-6850) — they make discovery-derived
provenance durable without taxing the direct path, which stays exactly as fast
and may omit both:

- **Outcome** — `<behavior change, for whom>`: the north-star yardstick the
  hypothesis serves. Populated from the `--discover` Outcome stage when the H
  came through discovery; free-form (or absent) on the direct path.
- **Derived-by** — `/hypothesize --discover IDEA-XXXX` or `direct`: how the
  claim was arrived at. Without it, the Idea an H came from is lost the moment
  the trio is scaffolded.

### 2. Check for an existing hypothesis first

```bash
nk list --board research --type hypothesis 2>&1 | grep -i "<keyword>"
nk query "<claim keywords>" 2>&1 | head
```

If one already covers this, **reuse it** — add your Measure/Experiment under it
instead of filing a duplicate.

### 3. Create the Hypothesis

```bash
nk create hypothesis "<short title>" \
  --body "Claim: $ARGUMENTS
Target: <threshold that = validated>
Refutation: <observation that would refute>
Why it matters: <decision / launch claim it backs>
Outcome: <behavior change, for whom — OPTIONAL, omit if none stated>
Derived-by: <'/hypothesize --discover IDEA-XXXX' | direct — OPTIONAL>
Status: draft (awaits experiment)." \
  --tags "<scope-tag>,research" \
  --push
# → note the new H-XXXX id
```

### 4. Create the Measure(s)

One per metric that settles the claim. Keep them ACF-style and reusable.

```bash
nk create measure "<metric name> (<unit>)" \
  --body "Definition: <how the metric is computed>.
Instrument: <runner / eval that produces it>.
Hypothesis: H-XXXX.
Target: <value that = validated>." \
  --tags "<scope-tag>,research" \
  --push
# → note the new M-XXXX id
```

### 5. Create the Experiment (the runnable arm)

```bash
nk create experiment "<EXPR title>" \
  --body "Purpose: tests H-XXXX.
Method: <participants/materials/procedure/config>.
Runner: <exact command, e.g. cargo run --release -p <crate> --example <name>>.
Arms + metrics: <which M-items, which arms>.
Results path: research/shared/eval-data/<scope>-<id>/.
Status: planned." \
  --tags "<scope-tag>,research" \
  --push
# → note the new EXPR-XXXX id
```

**For UX / human-agent surface claims:** When the hypothesis is about a UI, API, or data format (usability, discoverability, task completion), the **Method** must include:

1. **A measured prototype** — task completion, time-to-answer, error rate, or whatever the Measure names. A prototype nobody measured is a demo.
2. **The UXR review + simulated study** (via `.claude/skills/uxr-review/SKILL.md`) as a recorded artifact, with **participant personas declared** (who was simulated, and why those).
3. **The evidence-class caveat** — simulated participants are LLM-generated evidence about human behaviour (weakest class); never quote as measured human behaviour. Correct claim: *"study surfaced N problems; K confirmed"* — never *"users take 4.2s"*.

If the experiment needs GPU, add the queue triples to its turtle block
(`expr:runStatus \"queued\"`, `expr:requiresGPU \"true\"^^xsd:boolean`,
`expr:runOn \"DGX\"`) so a Spark can find it — the Sparks drain
`nk training list --status queued` each iteration (`campaign-watcher` §3).

⚠ **Do not read that as an executor.** The step that used to RUN a filed experiment was
`voyage-watcher` Phase 4d, and it was **not migrated** when that loop was cut:
`.claude/skills/voyage-watcher/SKILL.md` is a stub holding the last written copy of it, and
`campaign-watcher` has no equivalent. **HZ-12036 owns the question of who should own it.** Until
that lands, the EXPEDITION that answers this hypothesis runs the experiment itself, via `/workit`
Phase 4.4 — queueing the triples files the work, it does not dispatch it.

⚠ **`voyage-watcher` is not retired, and that bears on WHO eventually owns this.** SG-12234 (Captain
2026-09-12) amends EX-11924: it is the VOYAGE tier, `campaign-watcher` is the CAMPAIGN tier. It is
ruled but **NOT YET RESTORED** (`CH-12235`) — the skill file is still the stub described above, so
nothing in this section changes today. If the tier is restored it is the obvious home for Phase 4d,
which is exactly the question `HZ-12036` owns.

### 6. Link the trio (and the expedition that answers it)

```bash
nk update H-XXXX    --related "M-XXXX,EXPR-XXXX"
nk update EXPR-XXXX --related "H-XXXX,M-XXXX"
# If an expedition answers this hypothesis, link it both ways:
nk update EXP-YYYY  --related "H-XXXX,M-XXXX,EXPR-XXXX"
```

### 7. Hand off

Report the three ids. From here:

- The **expedition** that answers the hypothesis runs through `/workit`; its
  Phase 4.4 writes the result back to the EXPR + Measures + voyage.
- **No loop runs a filed EXPR today.** `voyage-watcher` Phase 4d did, and it was cut without a
  migration (`.claude/skills/voyage-watcher/SKILL.md` is the stub and the last written copy), so
  **HZ-12036** is open on who should own it. The expedition above is therefore the only executor —
  an EXPR left `planned` on the expectation that something picks it up will sit there.
  ⚠ SG-12234 (Captain 2026-09-12) amends EX-11924: `voyage-watcher` is the VOYAGE tier, **not
  retired** — ruled but NOT YET RESTORED (`CH-12235`). If it is restored it is the obvious home for
  this duty, which is exactly the question HZ-12036 owns. Either way the duty is unowned today.
- The **Captain validates** the hypothesis from the linked results (guardrail
  #6) — agents gather evidence and record the status (VALIDATED / NOT YET /
  REFUTED) as a finding; they do **not** auto-retire the H-item.

## Guardrails

- **Don't over-formalize.** If no measurement could change the answer, this skill
  doesn't apply — ship plain engineering, or escalate a values/policy call as a
  Captain ruling.
- **Don't auto-close.** Filing results ≠ validating the hypothesis; that's the
  Captain's call.
- **One hypothesis per claim.** Reuse an existing H-item rather than forking a
  parallel one (step 2).

## `--discover` mode — the front half (EX-6849, VY-6848's anchor)

`/hypothesize --discover IDEA-XXXX` turns a raw research-board **Idea** into a
de-risked, pre-registered test plan — the discover/validate front half the
measure/learn back half above presumes. A MODE, not a skill: a standalone
`/investigate` has no route in (adversarially validated fold-not-spawn).

### Stage 1 — the kill-test entry classifier (GATES the mode)

Objective, in order — the first match exits:

1. **Can existing code / docs / a targeted test answer it?** → plain
   engineering. Exit; no science tax.
2. **Is it already a falsifiable claim + threshold?** → plain `/hypothesize`
   (the unchanged fast path above). Exit; scaffold the trio directly.
3. **Is there ONE unresolved kill-assumption that cannot be answered cheaply?**
   → enter `--discover`.

The classifier is what keeps this mode off normal features: plain engineering
and plain claims never enter it, and a hard budget bounds what does — **one
Idea, one kill-assumption, ~30–60 min, stop the moment the kill-assumption
resolves.** Stages 3–5 are CONDITIONAL: skip any the budget or an earlier
answer makes moot.

### Stage 2 — Outcome (the yardstick)

State the north-star: the behavior change and for whom. This is a *yardstick,
NOT a validated claim* — it must not pre-suppose the result (the archived
Template 1 shape: source + question, nothing more committed than that).

### Stage 3 — Prior-art scan (reuse beats rediscovery)

The **nk research board FIRST** — `nk list --board research --type hypothesis`
/ `--type literature` / `--type idea`, `nk query "<terms>"` — then wider
literature (the archived Template 2's job). A prior H that already settles the
kill-assumption ends the mode here.

### Stage 4 — Assumptions map + the ONE riskiest

Enumerate the assumptions the outcome rests on; name the single
**kill-assumption** — the one whose falsification kills the whole line
(the archived Template 3's "claim + target" discipline applied *before* a
claim exists). One, not a ranked list: the budget buys one answer.

### Stage 5 — the cheapest spike

AI-native: an **early eval on a TINY labelled dataset** that resolves the ONE
kill-assumption. Exploratory only — see integrity rule (a).

### Stage 6 — GO / NO-GO / PIVOT

- **GO** → hand off to the normal trio pipeline above; promote the Idea
  (`captured` → `formalized`) and the created H carries
  `--relate related:IDEA-XXXX`, plus the two optional provenance lines from
  step 1: `Outcome:` from Stage 2's yardstick and
  `Derived-by: /hypothesize --discover IDEA-XXXX`.
- **NO-GO** → abandon the Idea (`--resolution wont_do`) with the
  provenance-stamped spike eval JSON as the warrant. **No H/M/EXPR ceremony
  for a dead line.**
- **PIVOT** → re-enter at Stage 4 with a revised riskiest assumption.

### The three integrity rules (non-negotiable — from the adversarial pass)

- **(a) NO HARKing.** The spike is **EXPLORATORY and EXCLUDED from
  adjudication** — it never counts as evidence for or against the hypothesis.
  On GO, the pre-registered EXPR's **Run #1 is a FRESH run of the frozen
  protocol**, fixed *before* seeing spike results. The archived canon,
  verbatim: *"Do NOT modify protocol after seeing results (that's HARKing)"*
  (`.claude/docs/hdd-agent-prompts.md`). There is no "spike is Run #0".
- **(b) Reproducible spike artifact.** A throwaway spike's eval JSON that
  SURVIVES (e.g. as a NO-GO warrant) must carry a reproducible provenance
  block: dataset hash + committed runner + source commit + env — and **never a
  `git_sha` pointing at a deleted scratch branch** (CH-6759
  figure-provenance; the measured lesson: a registered 0.847 whose own cited
  script yields 0.8603 — a number whose script doesn't reproduce is
  worthless).
- **(c) The classifier gates entry** (Stage 1), with conditional stages and
  the hard budget. Plain build-out work never gets a "riskiest assumption"
  line — the classifier keeps this off non-empirical items entirely.
- **(d) A deviation precedes the WINDOW it can admit, not just the check that
  reads it** (SG-12503, from PROP-6418's review of M-12438's DEVIATION 5, which
  was posted inside the window whose admissibility it decided):
  *"A deviation that changes a VALIDITY, ADMISSIBILITY or RE-DRAW rule must be
  posted before the first sample of any window it can admit or discard. A
  deviation posted inside such a window discards that window by construction;
  it may govern the next one. A deviation that changes only how a result is
  REPORTED may still be posted before the reporting step."* Writing an
  admissibility rule while the samples it judges are being drawn is a
  researcher degree of freedom even when no number has been read.

**Routed from the paved path**: `/workit` 1.4b (the empirical classifier) and
`/plan-phase` step 5 reference this mode; `/steer`'s refuted-narrow PIVOT
disposition creates the Ideas that enter here.
