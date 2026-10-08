---
name: plan-phase
description: "Plan the next campaign phase in one pass: voyages + expeditions + linked H/M/EXPR research trios + the Captain ratification packet. Fired by odyssey-watcher's planning-runway scent (EX-12263) or by Captain direction."
disable-model-invocation: false
argument-hint: "<campaign-tag> (e.g. v19-lrm)"
---

> ⚠ **IMPORTED, NOT YET ADAPTED.** Copied verbatim from `nusy-product-team@27feda15ea:.claude/skills/plan-phase/SKILL.md` on
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

# Plan Phase — voyages, expeditions, and research trios in one pass

> **New session?** Read [`docs/AUTONOMOUS-FLEET.md`](../../../docs/AUTONOMOUS-FLEET.md) first — how to operate in the autonomous fleet: what a session is, who may review/revise/merge, and which older statements in this repo are retired.

The fleet's iteration-planning meeting, compressed: at agent speed the planning
cadence is **every 2–3 days**, triggered by the **upper loop's** runway scent
(`.claude/skills/odyssey-watcher/SKILL.md` §3 — `READY < N_AGENTS` OR `LANES < 2`,
no drafts waiting) or by the Captain directly. ⚠ That citation dangled from
2026-09-07 to 2026-09-12: it used to say "the resident loop", and `runway`
returns 0 hits in `campaign-watcher`, so no loop emitted the signal this skill
waits on (EX-12126, HZ-12079 duty 1; the detector is restored at EX-12263). This
skill encodes the FULL procedure so no step is dropped — it exists because the
manual runs missed one each time (Phase B2 shipped without research trios; new
measures sat in `backlog`).

**The watcher drafts, the Captain ratifies.** Everything this skill files sits in
the `planning` STATUS (not the retired `pending-ratification` tag) and is invisible
to the work sweep until the Captain rules; the move OUT of `planning` IS the
sprint-start.

## Required environment

```bash
alias nk='nusy-kanban --server "${NUSY_FLEET_KANBAN_SERVER:-nats://192.168.8.110:4222}"'
SCOPE_TAG="$ARGUMENTS"          # e.g. v19-lrm
PHASE_TAG="<scope>-phase-<n>"   # e.g. v19-phase-b2 — one per planning round
```

## Pipeline

### 1 — Runway-band gate (CH-6386, Captain 2026-07-23 — replaces one-phase-ahead)

The buffer is a BAND measured in claimable-ready items, not a phase count (at
fleet speed a "phase" is ~12 hours; the old cap made the Captain the refill
button). Never plan against an API that doesn't exist — the per-item
contract-exists rule is what caught real churn, and it STAYS.

```bash
N_AGENTS=5
# (a) The band: plan when claimable-ready < N_AGENTS; draft up to 3×N_AGENTS
#     claimable items (count EX/CH on the ready frontier — planning-status items are
#     structurally out of `--status backlog`, so no subtraction is needed). Above the high-water mark → do not draft more.
# (b) Per-item contract-exists (unchanged, non-negotiable): every drafted item's
#     depends-on names contracts that are MERGED or explicitly gated by edges —
#     read the campaign plan's "Depends on" lines and verify (nk show / git log).
# (c) NEW-SCOPE drafts outstanding: at most ONE unratified new-scope packet at a
#     time (see step 6's *Release split*) — ratified-scope refills are not counted.
# (d) Record the runway level (READY count before/after) in the planning
#     comment on the campaign node — the band is tuned by evidence (M-a).
```

### 2 — Read the sources (not memory)

- The campaign plan doc (e.g. `research/V19/V19-VOYAGES.md`) — the voyages are
  *sketched* there; this skill instantiates the next 1–2.
- The rulings ledger (`V19-RULINGS.md`) + open-decisions file
  (`V19-DECISIONS-OPEN.md`) — what's ruled, what would gate this phase.
- `nk roadmap --ready` + open-item sweep — what the fleet actually has left.

> **The citation rule (CH-6369 — non-negotiable).** Every enumerated list, gate
> number, lifecycle sequence, fault list, and dependency claim in a drafted body
> is **quoted from canon with a file+section citation** — never written from
> memory. The 2026-07-23 Wave-3 adversarial cycle caught an *invented* 8-fault
> list, wrong gate tags, and a dropped ruled split-element, all from
> memory-drafting. If you cannot cite it, open the doc and find it; if canon
> doesn't contain it, it does not go in the body.

### 3 — Draft the voyages (the Epic layer)

Per voyage (target **voyage WIP 2–3 fleet-wide** — plan only what restores it):

- **2+ expeditions** (sizing standard, Captain 2026-08-16; two IS a voyage. >~6 → consider splitting).
- Body: goal · why-now (what unblocked it) · expedition list with the
  **dependency graph** (note which expeditions are parallel — that's fleet
  throughput) · success criteria · `**PENDING CAPTAIN RATIFICATION**` header.
- Tags: `$SCOPE_TAG,$PHASE_TAG` (+ `foss` etc. as fits) — and file it `planning`
  (`nk move <ID> planning`), never tagged `pending-ratification` (that tag is retired).

### 4 — File the expeditions (the Feature layer)

Each expedition: **3–6 phases, exactly one PROP**, with the 6-element body
(Context / Inputs with real file paths / Phases / Verification / Constraints /
Output). Then wire the graph — this is what lets the fleet self-sequence:

```bash
nk update EX-<later> --depends-on "EX-<earlier>[,...]"   # REPLACES — set the full list
nk update VY-<id>    --related   "EX-...,EX-...,<upstream VY>"
```

- Mark `gpu-required` in tags where true (the only pre-assignment allowed).
- Every item carries `$SCOPE_TAG` (the watcher's in-scope test) and is filed
  `planning` (never tagged `pending-ratification` — retired).
- **`$SCOPE_TAG` must be ON the ratified `ORDER=` line, and this is now checkable (CH-8271).**
  A planned item whose tags intersect ORDER nowhere carries **no scope tag** — no session finds it
  by scope tag, not merely low priority (the `ORDER` walk died with Layer A, CH-11826). A whole phase can be drafted, ratified and
  then never offered to anyone:

  ```bash
  nk list --tag <campaign-tag> --ready              # the scope tags are CLAUDE.md's "Tags"; the ORDER
  nk show EX-<id>                                   # walk died with Layer A (CH-11826): check the tags by eye
  ```

  ⚠ **This bites planning harder than ad-hoc filing**, because a phase is a BATCH: one wrong
  scope tag orphans every item in it at once. Measured on the board: 247 of 411 claimable
  items carried no ORDER tag, including a fully-specified voyage (VY-7077 + five expeditions)
  the Captain had asked for by name. **A mis-ranked batch is worse than an untagged one** — it
  reads as healthy at a rank nobody is working, and no sweep can detect it; an untagged batch
  at least surfaces in `nk list --status backlog` with no campaign tag.
- **Expected-information-gain tie-breaker (CH-6853).** Among expeditions that
  are ALREADY dependency-ready **and** carry genuine empirical uncertainty (an
  open kill-assumption; a claim a measurement could settle — the same
  classifier as the research trio in step 5), sequence the one whose answer
  de-risks the most downstream work FIRST — early resolution of a real unknown
  is worth more than any equally-ready build-out. Three hard bounds, each the
  guard against a known failure mode:
  - It is a tie-breaker **WITHIN dependency order and priority, never an
    override** — it only orders items that were otherwise interchangeable, and
    must never reorder across a dependency edge.
  - It applies **only to the empirically-uncertain subset** of the ready
    frontier. Plain build-out expeditions keep their existing order untouched.
  - It adds **no annotation to plain build-out work** — a voyage with no
    empirical uncertainty gets no risk line (mandatory risk ceremony on
    engineering work is the over-formalization failure mode CLAUDE.md's
    "most work is not an experiment" names).
- **Run the near-duplicate check on each title before filing** (CH-7749 — advisory,
  ~3s per title). A planner files in BATCHES, which is exactly where a duplicate is
  cheapest to prevent and most expensive to miss: a duplicated item is claimed and
  built by someone before anyone notices the overlap.

  ```bash
  nk query "<title>"                                  # the store's own search; you decide
  ```

  Exit 2 is CANNOT-ASSESS, not "no duplicates". Where a candidate IS the same work,
  prefer widening the existing item's body over filing beside it.

### 5 — Research trio per voyage (the step history shows gets dropped)

For **every voyage whose acceptance is a measurement**, run the `/hypothesize`
pipeline INLINE (open `.claude/skills/hypothesize/SKILL.md` and execute it —
same execution-note pattern as workit/reviewit):

- **H** — falsifiable claim + target + refutation + why-it-matters; move to
  **`active`**.
- **M** — definition / instrument / target per metric; **move to `active`**
  (measures left in `backlog` are invisible to the results write-back — this
  was the second historical miss).
- **EXPR** — `planned`, **`--depends-on` the battery expedition** (so the
  watcher's Phase-4d sweep auto-runs it the moment the battery lands), with
  Runner + Results path (`research/shared/eval-data/<scope>-<topic>/`).
- **Cross-link both ways**: H↔M↔EXPR↔voyage↔battery-expedition (`--related`
  REPLACES — read current values first, then merge).

If no measurement could change a voyage's acceptance, say so explicitly in the
voyage body ("research trio: N/A — plain engineering") rather than silently
omitting it.

### 6 — Adversarial canon review (MANDATORY before the packet — CH-6369)

**Surface-change classification rides this step (CH-6856).** For each drafted
item that ships a consumer-facing surface, classify by AUDIENCE and note the
route in the item body so the executor's `/workit` 1.4b2 fires informed:
agent-consumed → the operability smoke-panel; human-consumed → the heuristic
pre-pass + Artifact prototype + async captain-decision (expiry +
kill-criterion); both → the dual-audience rule keyed to the CHANGED CONTRACT
(`.claude/docs/ux-study-protocols.md` §a–d — the methods live there, never
restated here). Not through `/hypothesize`: that is reserved for genuinely
empirical claims.

**Reviewer ≠ author extends to work definitions, not just code.** Drafted bodies
stay in the `planning` STATUS (they already are, per step 3/4) until an
independent adversarial pass has run and its findings are fixed. The Captain
ratifies *reviewed* drafts, never raw ones.

**Who reviews — pick by stakes:**

- **Charter-level / era plans / any batch the Captain will read** → an
  **external model** via the Copilot CLI (model ≠ model, the §6.2 battery rule
  extended to plans). Non-interactive invocation:

  ```bash
  # Write the adversarial prompt to a file: the canon reading list (real paths),
  # the item IDs + how to read them (nk show), and the REFUTE mandate.
  # ⚠ The MACHINE-AGNOSTIC wrapper (EX-6606) this step used to invoke was DELETED at
  # VY-11604 Commit C — spawn the sub-agent directly and stamp the header by hand. The
  # wrapper paragraph below describes what EX-11973 must REBUILD, not what runs today.
  # Spawn ONE fresh strong-tier sub-agent with /tmp/adversarial-plan-review.md as its whole prompt
  # (the subagent IS the default path since SG-10917 / CH-11596) and write its findings to
  # /tmp/adversarial-plan-findings.md with a header naming path, model, machine, repo_sha, status.
  # The wrapper: detects the CLI · pins the model by --tier (never `auto`, which
  # may resolve to Claude and break model != model) · passes --deny-tool write as
  # defense-in-depth on the report-only mandate · captures stderr to a FILE
  # (/tmp/adversarial-plan-findings.err), never /dev/null · runs from the repo root
  # so the reviewer can read canon and run nk itself · and stamps a provenance
  # header (path/model/machine/agent/sha) on the findings.
  #
  # EXIT CODE = whether the pass is complete. Do not ignore it:
  #   0  external-model pass ran; findings are in --out
  #   10 CLI absent HERE -> the artifact is a subagent-fallback stub and an
  #      independent subagent pass with a distinct model is STILL OWED
  #   3  CLI ran but failed/returned nothing -> NOT an all-clear; read the .err
  ```

  **Model routing (Captain, 2026-07-23 — MAX plan):**

  | Work | `$REVIEW_MODEL` |
  |---|---|
  | Architectural / complex / charter-level review | `gpt-5.6-sol` |
  | Routine review, batteries, phase batches | `gpt-5.5` |
  | Diversity pass / second opinion (add alongside, esp. when two GPT reviews would share a blind spot) | `kimi-k2.7-code` |

  Slugs verified live 2026-07-23; names drift — re-verify with
  `copilot -p "List the exact model identifiers I can pass to --model, one per
  line, nothing else." --model auto --allow-all-tools --no-color` (there is no
  `--list-models` flag). Never use `auto` for the adversarial review itself —
  auto may pick Claude and silently break model ≠ model.

- **Routine phase batches** → `gpt-5.5` via the same invocation. Reserve
  `gpt-5.6-sol` for the architectural/complex tier; add a `kimi-k2.7-code` pass
  when the batch is large enough that a shared-blind-spot miss would be costly.

- **Copilot CLI unavailable on this machine** → the wrapper detects this, exits
  **10**, and writes a *subagent-fallback* artifact carrying the prompt. The
  REQUIRED fallback **for every tier, charter-level included**, is ≥1 independent
  subagent (Agent tool) with a **distinct model pinned** (the Agent tool's `model`
  parameter — pick a non-primary model), same prompt shape. Paste its findings
  into the artifact and set `status: ok` + the model you actually pinned. **Exit 10
  is not a pass** — the pass is never skipped for want of the CLI; model ≠ model is
  the preference, an independent adversarial pass is the requirement. (The Copilot
  CLI path and its installers are retired with CH-11826; the sub-agent is the path.)

- **Which path ran is RECORDED, not implicit** — the rule is EX-6606's, but ⚠ the wrapper
  that *wrote* the header died with Commit C, so you stamp it by hand until EX-11973
  restores a producer. Every artifact carries a
  provenance header — `path:` (`copilot-cli` vs `subagent-fallback`), `model:`,
  `machine:`, `agent:`, `repo_sha:`, `status:`. Before citing a pass as done,
  read the header yourself — a missing header or `model: auto` means the pass is
  still owed (no script checks it since CH-11826). Quote `path` + `model` in
  the cycle record below so the Captain can see whether a given review had
  external-model diversity or was a subagent.

**The prompt shape (both paths):** list the canon docs for this campaign (for
v20: ARCHITECTURE, VISION, PRD, DOMAIN-COG-SPEC, EXECUTION-PLAN, KILL-SCREEN,
SCIENCE-AMENDMENTS, COG-LIFECYCLE-SECURITY, the design docs) · list the item
IDs + `nk show` access · mandate: **REFUTE each body** on (1) contradictions
with canon, (2) required-but-missing (ruled elements, gates, trios, Docs:
lines, kill-screen conditions), (3) scope creep vs deferrals, (4) dependency
edges, (5) PR-class declarations · findings as `[BLOCKING|AMEND|NOTE]` + canon
citation · report-only, no edits.

**Then:** fix every BLOCKING and AMEND finding in the bodies (`nk update
--body-file`), file anything the review exposed as unowned (a dropped split
element IS undone work), and record the cycle on the campaign node
(`nk comment CA-XXXX`/VY) — reviewer identity, finding counts, disposition.
**"Reviewer identity" means the artifact's `path` + `model`** (e.g. "copilot-cli /
gpt-5.6-sol" or "subagent-fallback / `<pinned model>`"), not just "reviewed" — an
external-model pass and a subagent pass are different evidence, and EX-6606 made
that distinction readable instead of implicit — until Commit C deleted its wrapper,
so record it by hand until EX-11973 restores one.

**Release split (CH-6386 — ratification gates SCOPE, not throughput):**

- **Ratified-scope refills** — items that INSTANTIATE already-ratified content
  (exec-plan waves, ruled comments, §5-style ratified backlogs, each body
  citing its ratifying source) — **AUTO-RELEASE now**: move them out of
  `planning` (`nk move <ID> backlog`) after the pass; no Captain packet. Record
  the release in the campaign-node comment.
- **New-scope voyages** — anything NO ratified doc sketches — stay held and go
  to step 7's packet; the packet SAYS "canon-verified by <model/agent>, N
  findings fixed." At most one new-scope packet outstanding (step 1c).

### 7 — One consolidated decision packet — classified FIRST

**Run the `/steer` classifier over every candidate before the packet is written**
(`.claude/docs/goal-hierarchy.md`; the gate is campaign-watcher step 5b). Ratifying
**new-scope** voyages is genuinely bucket 3 — it changes what we are aiming at — so
the packet stays. Everything else must be classified OUT of it, not into it:

- **Bucket 1 (a measurement settles it)** → file the H → M → EXPR (`/hypothesize`)
  and let the data decide. It never enters the packet.
- **Bucket 2 (technical, or the goals settle it)** → decide it now, comment the
  decision with its goal + evidence basis, flag it for Captain veto in the pass
  report, and route the unblocked work. It never enters the packet.
- **Bucket 3 (changes a FEATURE or a GOAL, or a named human authority)** → the
  packet, each row pre-filled with a recommendation so the call is a 30-second
  confirm. Name which trigger fires.

Then file **ONE** `Q(Captain)` Signal (tag `captain-decision,$SCOPE_TAG`) covering:

- The ratification ask (list the drafted voyages, what each unblocks, which
  expeditions are immediately claimable and by whom).
- **Only the D-rulings that gate THIS phase** — batch, don't dribble; defer
  later-phase rulings explicitly ("deferred to the Phase-<n+1> packet").
- One-word answerable where possible ("'approved' refills the fleet").
- **A one-line statement that the non-bucket-3 candidates were classified and
  excluded**, with their dispositions (filed as H/M/EXPR, or decided). Without that
  line the packet is a dumping ground: the classifier is unverifiable from the
  artifact it produced, which is how the queue the Captain sees fills with calls
  the fleet could have made.

Then close any open `[planning-needed]` signal (`nk move SG-XXXX done`).

### 8 — Ratification protocol (on the Captain's ruling)

The executing agent (whoever sees the ruling first):

1. **Move the voyages `planning` → `in_progress`** (`nk move VY-<id> in_progress`) —
   that status move IS the sprint-start (the retired `pending-ratification` tag is
   inert; there is no tag to strip).
2. **Move every child `planning` → `backlog`** (`nk move <child> backlog`) so the
   expeditions enter the claimable frontier.
3. Record the ruling **verbatim-faithful** in the rulings ledger + delete the
   answered rows from the open-decisions file; commit to main.
4. Comment + close the packet signal; note the ratification timestamp on the
   **cycle-time measure** (M-4759 pattern) — the phase clock starts here.
5. Anything the Captain overrode or split out → new focused signal, not a
   re-litigation in the packet.

### 9 — Completion checklist (assert before ending the planning pass)

- [ ] **Runway-band gate passed** (step 1, CH-6386 — *replaces* the retired
      one-phase-ahead cap): planning was triggered at claimable-ready
      `< N_AGENTS`; drafting stopped at `≤ 3×N_AGENTS` claimable items; **every**
      drafted item's depends-on names contracts that are MERGED or explicitly
      edge-gated (per-item contract-exists — verified, not assumed); and at most
      **ONE** unratified NEW-SCOPE packet is outstanding. Ratified-scope refills
      do **not** count toward that bound (step 6's *Release split*, CH-6386).
- [ ] Every voyage: 2+ expeditions, dependency graph wired, success criteria.
- [ ] Every expedition: 6-element body, 3–6 phases, scope tag, deps encoded,
      `gpu-required` where true.
- [ ] Every measured voyage: H (**active**) + M (**active**) + EXPR
      (**planned**, depends-on the battery EX), cross-linked both ways.
- [ ] Everything filed in the `planning` STATUS (`nk move <ID> planning`); work sweep cannot claim it.
- [ ] **Adversarial canon pass ran (step 6); every BLOCKING/AMEND fixed; cycle
      recorded on the campaign node; packet states the reviewer + finding count.**
      The pass artifact carries its provenance header and its `status:` is `ok` — a `subagent-required` artifact
      (wrapper exit 10) means the pass is still OWED, not done.
- [ ] **Every enumerated list in every body carries its canon citation (the
      citation rule) and every body carries its `Docs:` line.**
- [ ] Exactly ONE decision-packet signal; `[planning-needed]` closed.
- [ ] Voyage WIP after ratification will be ≤ 3.

## Guardrails

- **Drafts, never ratifies** — the Captain's ruling is the sprint boundary.
- **Never plan against an API that does not exist** — VY-E/VY-F waited for VY-D's
  contract, and that catch is why the rule stays. **But the thing that caught it
  was the per-item contract-exists check, NOT a phase count** — CH-6386 says so in
  terms: *"the per-item contract-exists dependency rule STAYS (it is what caught
  real API churn, not the phase count)."* So the **phase-count cap is retired**;
  what you must still verify, per drafted item, is that its depends-on names a
  MERGED or explicitly edge-gated contract. The only surviving *count* limit is on
  unratified **NEW-SCOPE packets** — at most one outstanding (step 1c).
- **No silent omissions** — a skipped step (battery deferred, trio N/A) is
  stated in the body, mirroring the no-silent-omission review standard.
