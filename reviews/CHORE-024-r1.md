reviewed-at-sha: 33e1c232d9bf50dfcbfaeb0352839fd82870ebd0
verdict: approve
gate: make check rc=0 at 33e1c232d9bf50dfcbfaeb0352839fd82870ebd0 on M5-MBP-2
brief: reviews/CHORE-024-r1.brief.md @ 33e1c232d9bf50dfcbfaeb0352839fd82870ebd0

Authorship: all the work under review and this review were done by LLM agent sessions (Claude Opus 5.5); no human
reviewed it. "Distinct" here means a separate `claude -p` process with a fresh context that wrote none of the work, of
the SAME model family, briefed by the driver (the author) with the committed brief named above. The commissioning
session is M5-MBP-2/s-72a67d16. Its stake: it drove the work and wrote no tests or code; the tests (607f280) and the
code (797526f) were written by sub-agents. Host M5-MBP-2, 2026-10-10, worktree /tmp/rv-CHORE-024 (detached at the sha
above), CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-024.

## Findings

**F1 — the gate is green (not blocking).**
```
$ make check > /tmp/rv-CHORE-024-check.log 2>&1; echo "make check rc=$?"
make check rc=0
$ grep chore024 -A1 /tmp/rv-CHORE-024-check.log | grep "test result"
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
```
Every other `test result` line in the log reads `ok`, including the existing exp004 / chore018 / chore021 alert suites,
so api and subscription alerting and "local files nothing" still hold.

**F2 — the tests were not edited after T (not blocking).**
```
$ git diff 607f280..HEAD -- tests/ | wc -c
       0
```

**F3 — the tests match the Definition of Done (not blocking).** Read against the item body, not the code:
- DoD 1: `sub_and_local` drives a `kind = "subscription"` service through `quotabus alert` with the file backend and a
  fake `nusy-kanban` (argv recorded, `item_type = "signal"`, per §3 alert row / §10 Q8 SIG-006), over the same
  crossings as tests/chore021_local_no_alert.rs (flapping ok↔unreachable, ok↔model_missing, the same ten-step mixed
  sequence, each default state and each quiet state), checked against SIG-010's default (hard failures +
  window_near_limit file; latency-degraded and rate_limited do not). The control: the local service fed the SAME
  readings in the SAME run is rejected by the same checker, and `control_the_sig010_checker_…` shows a zero-filing
  outcome is rejected. Mutation M3 below shows the control bites end to end.
- DoD 2: the summary format is pinned (`alert: <N> keys checked, <M> local skipped, <F> crossings filed, <C> re-armed`,
  last line), with a parser control that rejects today's three-field line; four tests pin N, M, F and C.
- The item body itself cites no DESIGN sections; the brief's §3 alert row / §10 Q8 are honoured as above.

**F4 — the code does not special-case the tests (not blocking).**
```
$ git show 797526f -- src/main.rs | grep -nE "copilot|qwen|kimi|chore0|tmp|/state|test" ; echo rc=$?
rc=1
```
The change moves the `alerts_for(svc.kind)` skip above the per-slot loop and adds `skipped += svc.slots_in(&listing.rows).len()`.

**F5 — mutations go red (not blocking).** Each applied to src/main.rs with perl, run, then restored (`git status --short` empty after):
```
M1 (count local into N): s/skipped += svc.slots_in(&listing.rows).len();/checked += svc.slots_in(&listing.rows).len();/
$ cargo test -q --test chore024_subscription_alert_summary
test result: FAILED. 9 passed; 4 failed   — the_summary_counts_local_slots_as_skipped_not_checked,
  the_summary_with_only_local_services_checks_nothing, the_summary_counts_filings_and_re_arms_with_local_skipped,
  the_summary_checked_count_does_not_move_with_the_local_slot_count
M2 (report M as 0): s/{skipped} local skipped/0 local skipped/
test result: FAILED. 9 passed; 4 failed   — the same four tests
M3 (subscription skipped like local): s/if !quotabus::alert::alerts_for(svc.kind) {/if svc.kind != quotabus::Kind::Api {/
test result: FAILED. 4 passed; 9 failed   — all six a_subscription_* tests and three summary tests
```

**F6 — M is right for a discovering local service, but no test pins it (not blocking).** T's `rig_full` omits the
local service when its model list is empty, so no test covers a CHORE-022 discovering service (no `models`), which is
what the shipped example's dgx1-qwen / dgx2-qwen are. I measured it with a scratch test (tests/zz_rv_scratch.rs, not
committed, deleted after): one `kind = "local"` service with no `models`, run `alert` with an empty store, then with
rows `local.qwen.dgx1.a` and `.b` at now and `.old-model` 30 s older.
```
$ cargo test -q --test zz_rv_scratch -- --nocapture
NO ROWS: alert: 0 keys checked, 1 local skipped, 0 crossings filed, 0 re-armed
NEWEST CYCLE a,b (+ older old-model): alert: 0 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
```
Correct per `slots_in` (its id alone with no rows; the newest cycle's ids otherwise), matching `status`. But with the
mutation `s/skipped += svc.slots_in(&listing.rows).len();/skipped += svc.slots().len();/` the scratch test reads
`0 local skipped` in both cases, and T stays green:
```
$ cargo test -q --test chore024_subscription_alert_summary
test result: ok. 13 passed; 0 failed
```
A follow-up test for the discovering case would close this; the DoD does not name it, so it does not block.

**F7 — secrets (not blocking).**
```
$ git diff origin/main...HEAD | grep -nE "sk-[A-Za-z0-9]{8,}|ghp_|gho_|github_pat_|AKIA|xox[bp]-|Bearer [A-Za-z0-9]"; echo rc=$?
rc=1
$ gh pr view 32 --repo Congruentsys/quotabus --json body,title -q '.title + "\n" + .body' | grep -cE "sk-[A-Za-z0-9]{8,}|ghp_|gho_|github_pat_|AKIA|xox[bp]-"
0
```
The tests pass keys only through the environment (`QB_KIMI`, `QB_GH_TOKEN` = `FAKE_PLAIN`, "fakeQBKEY7f3a9c0dNOTREAL");
the config names the variable, never the value; nothing goes on argv.

**F8 — no doc quotes the summary line as current behaviour (not blocking).**
```
$ grep -rn "keys checked\|crossings filed" . --exclude-dir=target --exclude-dir=.git --exclude-dir=.venv
```
Hits are only the item itself, the new test, src/main.rs, and recorded past output (docs/findings/CHORE-016-mini-deploy.md,
reviews/CHORE-016-r1.md, reviews/CHORE-021-r1.md, the CHORE-016 board comment), which are history and stay as they
are. docs/DESIGN.md and README do not quote the line.
