---
name: steer
description: The outer loop above quotabus-loop. Sweep everything on quotabus's board that waits on a decision (open captain-decision signals, harbor work items whose comment says they wait on one, stranded items), classify each (a measurement settles it → measure; the goals or an existing Captain ruling settle it → decide now; it changes a FEATURE or GOAL, or needs human authority → escalate), DECIDE everything in the first two buckets on the board, and put only the third to the Captain, batched, each with a named trigger and a recommended default. Run it whenever a signal is open or the loop stops on something that waits on a decision.
disable-model-invocation: false
allowed-tools: Bash(.venv/bin/yurtle-kanban *), Bash(.venv/bin/python *), Bash(git *), Bash(gh *), Bash(env *), Read, Agent
---

# steer (quotabus): decide what data and goals can decide; escalate only the rest

> **Source.** Lifted from https://github.com/hankh95/yurtle-kanban/blob/ecf76fa3dcf9f17c3c52c56b973ed0897f079866/.claude/skills/steer/SKILL.md
> (commit `ecf76fa3dcf9f17c3c52c56b973ed0897f079866`, MIT licence, Copyright (c) 2026 Hank Head / Congruent Systems LLC), itself a port of
> nusy-product-team's `/steer`. Adapted to quotabus 2026-10-08 (yurtle-kanban 3.4.0): GitHub issues and labels →
> this repo's board; yurtle-kanban's goals → quotabus's, drawn from `docs/DESIGN.md` and `CLAUDE.md`.

The loop stalls on questions it could answer itself. This pass answers them. **Only a decision that no measurement,
no goal and no earlier Captain ruling can settle goes to the Captain.** This is the one sanctioned way a session
decides a signal (`CLAUDE.md` § Skills, "Signals"): through bucket 1 or 2, recorded on the signal, open to the
Captain's veto. Bucket 3 stays the Captain's.

## Goals (the north star for every decision), in priority order

- **G1: honesty and safety.**
  - **Never a false `ok`.** A row that is absent or expired reads UNKNOWN, a failure to measure reads CANNOT-ASSESS,
    an unreachable bus is rc 2, "never `ok`, never `no rows`" (`docs/DESIGN.md` §2, the freshness rule); an
    undocumented source's 4xx "degrades to `unknown`, never to a false all-clear" (§6); the disabled xAI key and every
    bad-token control "must read `auth_failed`, never `ok`" (§4, §9 E2).
  - **UNKNOWN over a guess.** A probe that cannot measure writes `unknown` with `cannot_assess:<why>` (§2 `reason`);
    "a window the source did not carry is left out, never 0 %" (§4, Claude Max row).
  - **Never leak a secret.** The repo is PUBLIC (`CLAUDE.md`, top; rule 3); a key never leaves the process and every
    output passes the redactor (§6). A steer comment on the board is published too.
  - **Never write another bucket.** Only quotabus's own bucket(s) and subjects (`CLAUDE.md` rule 4; pairit's measure
    lane, step 4).
- **G2: the fleet's use.** The readers (`status`, `select`, `alert`; §3) and the review skills that choose a model
  with them (pairit, nusy-product-team's external review: §1 "Users: us", §9 E3) get a correct, fresh answer:
  published "where agents and humans on several machines can read it with a freshness rule they cannot skip" (§1
  Goal).
- **G3: simplicity and FOSS.** The smallest change: a thin core, adopting what exists (the design's header, LIT-13334
  verdict "BUILD a thin core, ADOPT …"; §7 "what earns its place"). Pinned FOSS over from-scratch (`CLAUDE.md` rule 5;
  §8). Useful without our fleet: no `[bus]` means a file backend with the same freshness rule (§8 "useful without our
  fleet").

G1 outranks G2 outranks G3. The §1 **non-goals** (not a gateway, not a spend ledger, not an actuator, not an RDF
engine, not a dashboard product) are not goals to weigh: crossing one is a feature change, bucket 3.

## The classifier (apply to every pending decision, in order)

0. **Ruled already?** Before classifying, read the WHOLE thread (`.venv/bin/yurtle-kanban show <ID>`, every
   comment, not only the latest) and `docs/DESIGN.md` §10. The Captain's rulings are recorded verbatim there and on
   the SIGs. A ruling that answers the question, or one that extends to it, makes it bucket 2 by construction.
1. **Bucket 1: could a measurement settle it?** → **measure**, then decide on the data. Either
   - a **quick read-only measurement** now (a file read, a `--help`, a `gh pr view`, a board read, `nats kv ls` /
     `kv info`; no real-key probe, no write to any bus), recorded per `CLAUDE.md` rule 1: the exact command, its
     redacted output, the host, the date and this repo's sha, in the `[steer]` comment; or
   - a **filed chore** in pairit's measure lane (`create chore … --push`, released to `provisioning` per the release
     rule) when it needs a real key (`doppler run` / `secretspec run`), the bus, or a findings file others will build
     on. The signal then waits on that chore; say so on both.

   "No reader relies on this shape" is data; a null is a result.
2. **Bucket 2: do the goals, or an EXISTING Captain ruling, settle it?** → **decide now.** Pick the option that best
   serves G1, then G2, then G3, weighing impact × reversibility. Applying an existing ruling to a new case (for
   example §10 Q5's "separate times for the different types of queries" to a new kind's interval) is bucket 2, never
   bucket 3. Where a SIG blocks nothing and the design carries a recommendation for it, that recommendation is the
   default (`CLAUDE.md` § Skills).
3. **Bucket 3: does it change a FEATURE or a GOAL, or need human authority?** → **escalate.**
   - a **feature change**: what quotabus does for the people who rely on it: a new command, record field, kind, sink
     or transport; crossing a §1 non-goal (actuation is §10 Q6's "a later expedition"); reversing or narrowing a §10
     ruling; a behaviour change a reader actually uses (measured, not guessed);
   - a **goal change**: anything that alters G1–G3 above;
   - **human authority**: the release, version, crate or publishing path, the org or the licence (§8: "Shipping/licence
     stays a separate question"); money or spend (token cost per read, cadence: §10 Q5 was the Captain's); a new
     credential, Doppler project or login; a bus bucket or stream no item names (rule 4); a terms-of-service, legal
     or ethical call (undocumented sources, §10 Q4; §7's account-pooling ToS exposure).

> Before escalating, **name the trigger**: the feature or goal it changes, or the authority it needs. If you can't
> name one, it is bucket 1 or 2: decide it.

A change in another repo is NOT bucket 3 by itself: it is pairit's other-repo lane (a packet, the item `stranded` on
that repo). Whether that repo has landed it is a bucket-1 read.

## One pass

Run from the main checkout (it only reads it and fast-forwards it); every write is a board verb, which lands on
`origin/main` (`CLAUDE.md` § Syncing the board). Set `ME` in the same Bash call as each write.

```bash
ME="${QB_AGENT:-$(hostname -s)}/s-${CLAUDE_CODE_SESSION_ID:0:8}"
git pull -q --rebase=merges || { echo "STOP: pull failed — every read below would be stale"; exit 1; }
.venv/bin/yurtle-kanban control status                     # mode halt => STOP, report the reason
# 1a. open signals tagged captain-decision (list has no --tag; JSON status is canonical: backlog = harbor)
.venv/bin/yurtle-kanban list -t signal --json | .venv/bin/python -c '
import json, sys
for i in json.load(sys.stdin):
    if i["status"] == "backlog" and "captain-decision" in (i.get("tags") or []):
        print(i["id"], i["title"])'
# 1b. harbor work items: read each one's comments for a wait on a decision
.venv/bin/yurtle-kanban list -s harbor --json | .venv/bin/python -c '
import json, sys
for i in json.load(sys.stdin):
    if i["item_type"] in ("expedition", "chore", "hazard"):
        print(i["id"], i["title"])'
# 1c. stranded (blocked) items: what each waits on
.venv/bin/yurtle-kanban list -s stranded
.venv/bin/yurtle-kanban show <ID>                          # per item: the body and EVERY comment
```

```text
1. SWEEP     1a–1c above; a harbor item counts only if its comment names a decision it waits on (one that only
             lacks a "Done when" waits on its author, not on a decision: leave it)
2. RULINGS   step 0: the whole thread and DESIGN §10; a long thread goes to a fresh sub-agent to read
3. CLASSIFY  bucket 1 → measure, then decide on the data; bucket 2 → decide; bucket 3 → collect
4. RECORD    bucket 1/2, on the signal:
               comment  `[steer] bucket-N: <decision> — <basis: G1/G2/G3, the ruling quoted with its file:line,
                         or the measurement's command and output>`
               close    .venv/bin/yurtle-kanban move <SIG> done --force --resolution completed --agent "$ME"
             on a work item: the same comment, then move it on (harbor → provisioning when it is now well defined,
             per the release rule; stranded → back per pairit's other-repo lane, citing the landing commit)
5. REFLECT   comment EVERY item the decision changes, saying what it changes for THAT item (a fresh session reads the
             comment, not your context); a decision that changes docs/DESIGN.md files a chore to amend it (a PR)
6. ESCALATE  bucket 3: a comment on each SIG, `[steer] bucket-3: <trigger> — recommended default: <option>`, the SIG
             left open; then ONE batched message to the Captain (under quotabus-loop: its stop report) listing only
             the bucket-3 items, each with its trigger and recommended default, so each answer is a quick confirm
7. REPORT    every bucket-1/2 decision as "decided — open to veto", with the SIG, the decision and its basis
```

**Why `--force` on the close.** Signals sit in `harbor`, and yurtle-kanban 3.4.0's transition table allows only
`harbor → provisioning` and `harbor → stranded` (`workflow.py`, `BACKLOG: [READY, BLOCKED]`); a plain
`move <SIG> done` is refused, rc 1: "Illegal move SIG-010: harbor → arrived. Legal from harbor: provisioning,
stranded." (measured 2026-10-08 on M5 in a throwaway clone with no origin, `--no-commit`; with `--force` the same move
printed "Moved SIG-010 to arrived", rc 0). `--force` skips the transition table and WIP
limits only; it does not override a holder (`--take-over`) or the gates (`--skip-gates`, never used here). Say so
in the close's comment so the forced move is not mistaken for a bypass.

If there are zero bucket-3 items, escalate nothing and say the queue is clear; quotabus-loop carries on.

## Guardrails

- **Decide aggressively in buckets 1–2**; over-escalation is the failure this loop exists to fix. But never launder a
  bucket-3 call as a decision: when unsure whether an option could produce a false `ok` or leak a secret, it is not
  decided by G3; when it touches release, spend, credentials or ToS, it is bucket 3. A bug that yields a false `ok`
  with a clear fix is bucket 2 (G1), and goes to the board as a hazard.
- **Every decision is recorded** on its signal (comment + basis) and reversible where possible; the Captain can veto
  any of them from the report. A vetoed decision is reopened with `move <SIG> harbor --force` (a plain move is
  refused: "Legal from arrived: none", same run; the forced reopen was not run) and its reflected comments are
  corrected, each citing the veto.
- **Never decide by silence.** A decision reached in this session's context and not written on the board is not made.
- **A decision is not a merge.** The work it unblocks still goes through pairit: tests by a partner, a distinct
  `claude -p` reviewer, a GitHub PR.
- **Secrets:** a `[steer]` comment quotes no key, auth header, raw provider error body or email; a measurement's
  output is redacted before it is pasted. The board is public.
- **Other repos are read-only** from here: measure there, never write; a change there is an other-repo packet.
