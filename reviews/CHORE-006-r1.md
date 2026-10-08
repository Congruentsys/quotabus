reviewed-at-sha: b3a8ff1966d939d89867f1b696507fcf4ab66ec3
verdict: approve
gate: make check rc=0 at b3a8ff1966d939d89867f1b696507fcf4ab66ec3 on Mac-mini
brief: reviews/CHORE-006-r1.brief.md @ b3a8ff1966d939d89867f1b696507fcf4ab66ec3

Authorship: LLM agent sessions did all of this work and this review. The model was Claude Opus 5.5 (claude-opus-5-5). No
human reviewed it. A fresh Opus sub-agent wrote the diff (commit e99b027). The commissioning session (the driver) is
Mac-mini/s-e7976c42. It commissioned the diff from that sub-agent, checked it, wrote the brief, and gains from
`approve`. This reviewer is "distinct" in this sense: it is a separate `claude -p` process with a fresh context that
wrote none of the work. It is the same model family, and the driver briefed it with the committed brief named above.
The review was run on Mac-mini on 2026-10-08, in a detached worktree /tmp/rv-CHORE-006 at the SHA above.
`git ls-remote origin docs/CHORE-006-record-s10-rulings` printed `b3a8ff1966d939d89867f1b696507fcf4ab66ec3`.
The rulings were read on `origin/main` = `854eaf7`.

## Checks run (all pass)

**1. Verbatim quotes, SIG lines, recommended-or-not.** A script pulled every `**Captain 2026-10-08:** "…"
(…SIG-00N-*.md:L)` out of DESIGN.md at the SHA. For each, it checked that the quoted string occurs on line L of that
signal as `git show origin/main:<sig>` prints it:
```
SIG-001 26 "If this is FOSS, the config will have to ask to run locally or on another host. In this case, run on mini" OK
SIG-002 26 "Numbers + slugs" OK
SIG-003 30 "Off FOSS, on fleet" OK
SIG-004 26 "Slower - twice a day, but make it a config setting" OK
SIG-004 26 "API + balance only" OK
SIG-004 43 "for how often to query, make sure there are separate times for the different types of queries." OK
SIG-005 26 "TOML first, Yurtle in E4" OK
SIG-006 26 "Signal per crossing" OK
SIG-008 32 "Both + fallback" OK
SIG-007 26 "Defer to E5" OK
```
Q9 also cites SIG-008's recommendation at `:25-26`, and that is where the recommendation sits (`## Recommendation`
heading at :25, text at :26). I checked recommended-or-not against each signal's own Recommendation and ruling comment,
not against the item's summary:
- SIG-001:26 says "This is NOT the plain default". The doc says "Not the recommended default". Correct.
- SIG-004:26 says "This is NOT the default". The doc says "Not". Correct.
- SIG-002, 003, 005, 006 and 007 at :26/:30 each say "This is the recommended default". The doc agrees on all five.
- SIG-008:32 says "This is the recommended default", and SIG-008:26 recommends "Yes to 1 and 2, with the fallback
  kept". So the doc's "two differ (Q2, Q5)" is right. The item body's "Two rulings differ" lists three, and the third
  (Q9) is wrong: the doc correctly follows the signal, and the PR body says so.

The Origin note now records Q1, Q6 and Q11 as paraphrase, names SIG-001…SIG-007 and SIG-008, and says no §10 question
is open. That is consistent with the signals, which are all `status: arrived`.

**2. Stale sentences.** I ran:
`grep -nE "pending SIG|open as signal|\bopen\b.*SIG|15 ?min|15m|45m|\b96\b|5 ?min|every [0-9]|interval|cadence|twice|12 ?h" docs/DESIGN.md`
and a second grep for `[Rr]ecommend|Mini|§10|\bQ[0-9]+\b|ttl|TTL|stale|…|candidate`. The remaining hits for 15 min, 96
and 15m/45m are these:
- :221 and :326 are historical. One is "the 15 min default this line first named was 96"; the other is the Q5
  recommendation, kept on purpose.
- :173 states that the binary and `examples/quotabus.toml` still carry the shared `15m`/`45m`. That is true:
  `git show origin/main:examples/quotabus.toml` prints `11:interval   = "15m"` and `12:ttl        = "45m"`.

No "pending SIG" remains. The central probe's host is now config-or-Mini at :25, :73, :83-85 and :251. See F1 for one
Q1 leftover.

**3. Nothing claimed beyond the ruling.**
- Q5's "API + balance only" is read the way SIG-004:26 reads it: 12 h for API and balance, subscription stays 5 min.
- The per-kind intervals match SIG-004:43.
- The `[intervals]`/`[ttl]` sample is marked at :173 as "built by CHORE-007: E1's binary and `examples/quotabus.toml`
  still carry one shared `[probe] interval`… until it lands". It matches CHORE-007's Definition of Done (items 1-2:
  `[intervals]`, `[ttl]` defaulting to 3 × its own interval, "A due API probe never triggers a balance read, and vice
  versa").
- The `local` kind is explicitly left undecided (:87-88), which is honest.
- Two small attribution points are in F2 and F3.

**4. Sample TOML parses.**
`awk '/^```toml/{f=1;next}/^```/{if(f){f=0}}f' docs/DESIGN.md > /tmp/rv-CHORE-006-sample.toml; .venv/bin/python -c 'import tomllib,sys…' /tmp/rv-CHORE-006-sample.toml`
printed:
```
parsed OK
probe= {'max_tokens': 20}
intervals= {'api': '12h', 'balance': '12h', 'subscription': '5m'}
ttl= {}
kinds= ['api', 'subscription']
```
There is exactly one fenced `toml` block (`grep -c '```toml'` → 1).

**5. Arithmetic.** 24 h ÷ 12 h = 2 calls per model per day, so "≤ 2 calls × ~30 tokens per model per day" (:220-221)
is correct. For the old figure, 24 h ÷ 15 min = 96, which matches.

**6. Secrets / public repo.** I scanned the diff plus `gh pr view 8` title and body (247 lines) with:
`grep -nEi "sk-[a-z0-9]{8}|ghp_|gho_|xox[bp]-|AKIA|bearer [a-z0-9]{10}|<email regex>|BEGIN .*KEY|dp\.st\."`
It found nothing (rc=1). The only host detail is the bus IP, and it was already in the doc before this change.

**7. Gate.** I ran `PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR=/tmp/rv-CHORE-006-target make check` in the
worktree, and it returned rc=0. Its output included `35 passed in 6.24s` (pytest), `All work items valid.` (yurtle-kanban
validate), and every `cargo test` suite `ok`, the last being `15 passed; 0 failed`. `command -v cargo` resolved to
`/Users/hankh19/.nusy/gate-root/scripts/fleet/cargo-shim/cargo`.

## Findings

**F1 (nit): the §8 org row still offers `hankh95` as an option after Q1 was ruled.**
Command: `sed -n '291p' docs/DESIGN.md`
Output: `| **org** | `Congruentsys` (where `arrow-kanban` lives, …) or `hankh95` (yurtle, yurtle-kanban, nusy-kanban) — §10 Q1 |`
The PR updated the name row just above it (:290, "chosen by the Captain 2026-10-08, §10 Q1") and the Origin note
records Q1 as "`quotabus` under Congruentsys". The org row still reads as an open choice. Q1 is outside the Definition
of Done's list, so this is only a nit.
Fix: append "; Congruentsys chosen by the Captain 2026-10-08, §10 Q1".

**F2 (nit): the §3 diagram dropped the central probe's `local` kind.**
Command: `git diff origin/main...HEAD -- docs/DESIGN.md | grep -nE "^-.*local kinds"`
Output: `33:-   [Mini]  quotabus probe   (API + local kinds, every 15 min)`
It was replaced by `(API 12 h · balance 12 h)`. E1 (§9, :301) still has the central probe cover local Qwen. The prose
at :87-88 correctly says no default is ruled for `local`, but the diagram now suggests the probe does not run that kind
at all.
Fix: `(API 12 h · balance 12 h · local)`, or a footnote pointing to :87-88.

**F3 (nit): two of CHORE-007's design choices are stated without citing CHORE-007.**
Command: `sed -n '112p;222p' docs/DESIGN.md`
Output:
`[ttl]            # optional, per kind; each defaults to 3 x its own interval (3 missed = UNKNOWN)`
`endpoints are GETs, on their own interval (12 h default), never triggered by a messages probe or the reverse.`
The Captain ruled "separate times". The 3× per-kind TTL and the "never triggered … or the reverse" rule come from
CHORE-007's Definition of Done (items 1-2), not from a ruling. :173 credits CHORE-007 for the tables, but :222 cites
only §10 Q5. This is not wrong, since it is filed work, but a reader may take it for the Captain's words.
Fix: add "(CHORE-007)" after "the reverse" at :222.

No blocker or should-fix findings.
