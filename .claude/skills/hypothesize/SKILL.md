---
name: hypothesize
description: "Scaffold a research question into the Hypothesis → Measure → Experiment trio so the rigorous path is cheaper than the ad-hoc one (AI developer-scientist discipline)"
disable-model-invocation: false
argument-hint: "<the claim a measurement could settle> | --discover IDEA-R-NNN"
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(PYTHONPATH= .venv/bin/yurtle-kanban *), Bash(git *), Read, Edit, Write, Agent
---

# Hypothesize — scaffold H → M → EXPR in one step

**Board writes reach `origin` at once** (CLAUDE.md § Syncing the board): yurtle-kanban 3.4.0 pushes `move`,
`comment` and `rank` by default; `create`/`update` take `--push` (with `ME` set in the same Bash call).

> Ported from nusy-product-team `hypothesize` (2026-10-02), nk → yurtle-kanban 3.2.0; adapted to quotabus 2026-10-08 (yurtle-kanban 3.4.0).
>
> ⚠ **quotabus has not configured the research (HDD) board this skill writes to** (CLAUDE.md § Skills). Until a
> question needs the full trio, a measure-first question is a chore in pairit § The measure lane; configuring the
> research board is a chore of its own.

Turn an empirical question into the three linked research-board items, so an agent never has to
choose between "do it scientifically" and "do it fast" — the scientific path *is* the fast path.

**When to use:** the moment a work item asks a question a *measurement* could settle — coverage,
accuracy, latency, "X reproduces Y", "scales to N triples", "zero hallucination". Apply the
classifier first:

> **"Could a measurement change the answer?"**
>
> - **No** → don't use this skill. Plain engineering, or a decision for the Captain.
> - **Yes** → scaffold the trio here, *before* implementing.

⚠ **Over-formalization fails in BOTH directions.** (a) A claim a measurement could settle, shipped
as plain engineering or sent to the Captain as a "decision", is an unfiled experiment — this skill
is the remedy. (b) A measurement demanded for something no measurement could settle — a values,
naming, licensing or safety-posture call — is the other failure; that is a decision, not an EXPR.
Most work (refactors, bug-fixes, infra, docs) is neither and ships through plain TDD with no
hypothesis.

## Required Argument

`$ARGUMENTS` — the **falsifiable claim** in one line, phrased so a number could refute it. Good:
"The forward-chain engine sustains derivation at 1M triples within 3× of oxigraph." Bad: "make the
engine fast."

## Required environment

Run from the repo root. The CLI is `.venv/bin/yurtle-kanban`. Pass `--agent "$ME"` on moves and comments so
they are attributed to this session, with `ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"`
set in the same command (CLAUDE.md § The board). Board writes are files in git, and every one lands on origin at once (`create`/`update` with `--push`;
`move` and `comment` push by default in 3.4.0).

## Pipeline

### 1. Sharpen the claim into a hypothesis

State, in the hypothesis file you will fill in step 4:

- **Claim** — `$ARGUMENTS`, the falsifiable statement.
- **Target** — the threshold that counts as validated (e.g. "coverage ≥ 0.9", "p50 latency ≤ 2×
  oxigraph").
- **Refutation** — what observation would *refute* it (forces falsifiability).
- **Why it matters** — the decision or claim it backs.

Two further lines are **OPTIONAL** — they make discovery-derived provenance durable without taxing
the direct path, which stays exactly as fast and may omit both:

- **Outcome** — `<behavior change, for whom>`: the north-star yardstick the hypothesis serves.
  Populated from the `--discover` Outcome stage when the H came through discovery; absent on the
  direct path.
- **Derived-by** — `/hypothesize --discover IDEA-R-NNN` or `direct`: how the claim was arrived at.
  Without it, the Idea an H came from is lost the moment the trio is scaffolded.

### 2. Check for an existing hypothesis first

```bash
.venv/bin/yurtle-kanban list --board research --type hypothesis 2>&1 | grep -i "<keyword>"
.venv/bin/yurtle-kanban query "<claim keywords>" --no-semantic 2>&1 | head
```

If one already covers this, **reuse it** — add your Measure/Experiment under it instead of filing a
duplicate.

### 3. Create the Measure(s) — FIRST, so the hypothesis can name them

One per metric that settles the claim. Keep them reusable. `--unit` and `--category` are required.

```bash
.venv/bin/yurtle-kanban measure create "<metric name>" --unit <ratio|ms|count|percent> --category <accuracy|performance|coverage> --push
# → note the new M-NNN id
```

### 4. Create the Hypothesis

```bash
.venv/bin/yurtle-kanban hypothesis create "<the claim, one line>" \
  --target "<threshold that = validated>" \
  --measures "M-NNN[,M-NNN]" \
  [--source-idea IDEA-R-NNN] [--literature LIT-NNN] \
  --push
# → note the new H-NNN id
```

The HDD subcommands (`measure`/`hypothesis`/`experiment create`) write a turtle block carrying the
links, which `hdd validate` reads. **Fill the generated template's sections in place** (Read + Edit
the file under `research/hypotheses/`): Claim, Target, Refutation (the template's "If this
hypothesis is false" predictions), Why it matters, the optional Outcome / Derived-by lines, and
`Status: draft (awaits experiment)`. Same for the measure (Description, Formula, Collection Method).
⚠ **Do NOT use `update --body` / `--body-file` on an HDD item** — it replaces everything under the
heading, turtle block included, and the links are silently lost (measured 2026-10-02). Commit the
edit and push.

### 5. Create the Experiment (the runnable arm)

```bash
.venv/bin/yurtle-kanban experiment create --hypothesis H-NNN --measures "M-NNN[,M-NNN]" --title "<EXPR title>" --push
# → note the new EXPR-NNN id; it also writes the inverse link into H-NNN's turtle block
```

Fill its template in place: **Purpose** (tests H-NNN), **Method** (participants/materials/
procedure/config), **Runner** (the exact command), **Arms + metrics** (which M-items, which arms),
**Pre-Registration** (protocol locked, git sha — before Run #1), **Data Location**. A run folder
is made with `.venv/bin/yurtle-kanban experiment run EXPR-NNN --being <subject> [--params k=v,…]`
(creates `research/runs/EXPR-NNN/<timestamp>/config.yaml`; the runner writes `metrics.json` there).

**For UX / human-agent surface claims:** When the hypothesis is about a UI, API, or data format
(usability, discoverability, task completion), the **Method** must include:

1. **A measured prototype** — task completion, time-to-answer, error rate, or whatever the Measure
   names. A prototype nobody measured is a demo.
2. **The UXR review + simulated study** (via `/uxr-review`) as a recorded artifact, with
   **participant personas declared** (who was simulated, and why those).
3. **The evidence-class caveat** — simulated participants are LLM-generated evidence about human
   behaviour (weakest class); never quote as measured human behaviour. Correct claim: *"study
   surfaced N problems; K confirmed"* — never *"users take 4.2s"*.

If the experiment needs a GPU, say so in its Method and tag it:
`.venv/bin/yurtle-kanban update EXPR-NNN --tag gpu-required --push`. ⚠ That is a fact about the work, not
a dispatcher — **nothing runs a filed EXPR on its own.** The expedition that answers the hypothesis
runs it (via `pairit` for any code it needs). An EXPR left `draft` on the expectation that
something picks it up will sit there.

### 6. Link the trio (and the expedition that answers it)

The HDD subcommands already linked H ↔ M ↔ EXPR in the turtle blocks. Add the frontmatter
`related` list too, so `show` displays it, and link the expedition both ways:

```bash
.venv/bin/yurtle-kanban update H-NNN    --related "M-NNN,EXPR-NNN" --push
.venv/bin/yurtle-kanban update EXPR-NNN --related "H-NNN,M-NNN" --push
# If an expedition answers this hypothesis:
.venv/bin/yurtle-kanban update EXP-NNN  --related "H-NNN,M-NNN,EXPR-NNN" --push
.venv/bin/yurtle-kanban hdd validate     # 0 errors, 0 warnings
```

⚠ `--related` **replaces** the list (pass the whole set), and accepts IDs that exist on no board —
`hdd validate` and `show <id>` are how you confirm a target resolves. Use `--related`, not
`--add-dep`, for provenance: a dependency makes the item unpickable until its target is done.

### 7. Hand off

Report the three ids. From here:

- The **expedition** that answers the hypothesis lands through `pairit`; when the run finishes it
  writes the result back into the EXPR's Results/Conclusion sections and the Measures' Historical
  Values, and a finding goes to `research/` stating the measurement and the command that produced it.
- The **Captain validates** the hypothesis from the linked results — agents gather evidence and
  record the status (VALIDATED / NOT YET / REFUTED) as a finding and a comment; they do **not**
  move the H-item to `complete` themselves.

## Guardrails

- **Don't over-formalize.** If no measurement could change the answer, this skill doesn't apply —
  ship plain engineering, or escalate a values/policy call as a Captain decision.
- **Don't auto-close.** Filing results ≠ validating the hypothesis; that's the Captain's call.
- **One hypothesis per claim.** Reuse an existing H-item rather than forking a parallel one (step 2).

## `--discover` mode — the front half

`/hypothesize --discover IDEA-R-NNN` turns a raw research-board **Idea** into a de-risked,
pre-registered test plan — the discover/validate front half the measure/learn back half above
presumes. A MODE, not a separate skill.

### Stage 1 — the kill-test entry classifier (GATES the mode)

Objective, in order — the first match exits:

1. **Can existing code / docs / a targeted test answer it?** → plain engineering. Exit; no science
   tax.
2. **Is it already a falsifiable claim + threshold?** → plain `/hypothesize` (the fast path above).
   Exit; scaffold the trio directly.
3. **Is there ONE unresolved kill-assumption that cannot be answered cheaply?** → enter `--discover`.

The classifier keeps this mode off normal features: plain engineering and plain claims never enter
it, and a hard budget bounds what does — **one Idea, one kill-assumption, a fraction of one agent
context, stop the moment the kill-assumption resolves.** Stages 3–5 are CONDITIONAL: skip any the
budget or an earlier answer makes moot.

### Stage 2 — Outcome (the yardstick)

State the north-star: the behavior change and for whom. This is a *yardstick, NOT a validated
claim* — it must not pre-suppose the result (source + question, nothing more committed than that).

### Stage 3 — Prior-art scan (reuse beats rediscovery)

The **research board FIRST** — `.venv/bin/yurtle-kanban list --board research --type hypothesis`
(and `--type literature`, `--type idea`), `.venv/bin/yurtle-kanban query "<terms>" --no-semantic` —
then wider literature. A prior H that already settles the kill-assumption ends the mode here.

### Stage 4 — Assumptions map + the ONE riskiest

Enumerate the assumptions the outcome rests on; name the single **kill-assumption** — the one whose
falsification kills the whole line. One, not a ranked list: the budget buys one answer.

### Stage 5 — the cheapest spike

An **early eval on a TINY labelled dataset** that resolves the ONE kill-assumption. Exploratory
only — see integrity rule (a).

### Stage 6 — GO / NO-GO / PIVOT

- **GO** → hand off to the normal trio pipeline above. Move the Idea along
  (`.venv/bin/yurtle-kanban move IDEA-R-NNN active --agent "$ME"`, then `complete --resolution completed` once the
  trio is filed — see `/refine-idea` Phase 5); create the H with `--source-idea IDEA-R-NNN`, plus
  the two optional provenance lines from step 1: `Outcome:` from Stage 2 and
  `Derived-by: /hypothesize --discover IDEA-R-NNN`.
- **NO-GO** → `.venv/bin/yurtle-kanban move IDEA-R-NNN abandoned --resolution wont_do --agent "$ME"`, with the
  provenance-stamped spike eval JSON as the warrant, named in a comment
  (`.venv/bin/yurtle-kanban comment IDEA-R-NNN --agent "$ME" --body "NO-GO: <measurement> — <command>; eval: <path>"`).
  **No H/M/EXPR ceremony for a dead line.**
- **PIVOT** → re-enter at Stage 4 with a revised riskiest assumption.

### The three integrity rules (non-negotiable)

- **(a) NO HARKing.** The spike is **EXPLORATORY and EXCLUDED from adjudication** — it never counts
  as evidence for or against the hypothesis. On GO, the pre-registered EXPR's **Run #1 is a FRESH
  run of the frozen protocol**, fixed *before* seeing spike results. *"Do NOT modify protocol after
  seeing results (that's HARKing)."* There is no "spike is Run #0".
- **(b) Reproducible spike artifact.** A throwaway spike's eval JSON that SURVIVES (e.g. as a NO-GO
  warrant) must carry a reproducible provenance block: dataset hash + committed runner + source
  commit + env — and **never a `git_sha` pointing at a deleted scratch branch**. A number whose
  script doesn't reproduce is worthless (the measured lesson: a registered 0.847 whose own cited
  script yields 0.8603).
- **(c) The classifier gates entry** (Stage 1), with conditional stages and the hard budget. Plain
  build-out work never gets a "riskiest assumption" line — the classifier keeps this off
  non-empirical items entirely.
