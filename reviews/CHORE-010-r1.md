reviewed-at-sha: 0988c88ebe1013d01286862868d78870b761dc31
verdict: approve
gate: make check rc=0 at 0988c88ebe1013d01286862868d78870b761dc31 on M5-MBP-2
brief: reviews/CHORE-010-r1.brief.md @ 0988c88ebe1013d01286862868d78870b761dc31

Authorship: all the work and this review were done by LLM agent sessions (Claude Opus 5.5); no human reviewed it.
"Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model
family, briefed by the driver (the author) with the committed brief above. The commissioning session is
M5-MBP-2/s-72a67d16, which drove the item, briefed the test partner and the implementer and merged origin/main (it
wrote no tests or code). Reviewed 2026-10-09 in a detached scratch worktree `/tmp/rv-CHORE-010`,
`CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-010`.

## Checks

1. **Tests match the item and DESIGN, not the code.** `tests/chore010_warn_pct.rs` (f8ae737) pins DoD 1 (default 90;
   0 / 0.0 / -5 / 100.01 / 101 / 1000 / nan / inf / a string are refused, naming `warn_pct` and NOT as an unknown field),
   DoD 2 (the known answers 89.9 → ok, 90 → degraded, 97 → degraded; the `warn_pct = 100` control; any window, top
   `window_pct` or `windows[]`; six worse states not softened; through unified headers, stream-json and
   copilot_internal; rejected and exhausted rows stay `quota_exhausted`), DoD 3 (select refuses with a same-row ok
   control; alert files `degraded`, ok files nothing) and DoD 4 (DESIGN §2/§3 and the example, with a section-reader
   control). Every value comes from the item body or DESIGN §2; none is derived from the code.
2. **The red tests are untouched.** `git diff f8ae737..0988c88 -- tests/chore010_warn_pct.rs | wc -l` → `0`.
3. **Ruled edit 37b4b93 keeps the control.** `git show 37b4b93`: the event the rejected test uses is unchanged
   (`fallback_with_status` → `fallback_with(status, 1.0)`). The allowed twin with only `status` differing now asserts
   `!= QuotaExhausted` AND `== Degraded`, plus the 100 % and 12 % windows. A new below-threshold twin (five-hour 0.5)
   still asserts `ok`. The property (the status word, not the window, decides quota_exhausted) is pinned on both
   sides, and nothing is weakened: `ok` became the stricter exact `degraded`.
4. **No special-casing; every row; never softened.** `git diff f8ae737..1d8ff4e -- src`: `warn_state` returns any
   non-`Ok` state unchanged, then checks `window_pct` chained with every `windows[].used_pct` using `>=`. It is called
   once at the end of `subscription::read` (after the provider match, so on every subscription source, unmeasured
   rows included) and once in `probe::probe_model` after headroom is set (API rows). `grep -rn "headroom: Some\|headroom = \|\.headroom =" src`
   lists only those paths (subscription.rs:303/409/459 all go through `read`; probe.rs:422). Balance and unmeasured rows
   carry no headroom. No test constant is in `src`. The yurtle config path gains a `warn-pct` Float column that feeds
   the same table, so the range check applies there too.
5. **Mutations** (each restored with `git checkout`; `git status --short` is empty after):
   - M1 `.any(|p| p >= warn_pct)` → `p > warn_pct` in `src/classify.rs`:
     `cargo test -q --test chore010_warn_pct` → `test result: FAILED. 23 passed; 3 failed`. Failing:
     `control_warn_pct_100_leaves_97_ok`, `unified_5h_0_90_reads_degraded`, `window_pct_known_answers`.
   - M2 the `warn_state` line in `subscription::read` replaced by a comment:
     `test result: FAILED. 20 passed; 6 failed`. Failing: `copilot_monthly_97_reads_degraded_and_89_9_ok`,
     `stream_json_0_97_reads_degraded`, `unified_5h_0_90_reads_degraded`, `unified_5h_0_97_reads_degraded`,
     `unified_7d_0_97_reads_degraded_with_5h_low`, `unified_warn_pct_is_read_from_config`.
   - M3 the `warn_state` line in `probe::probe_model` replaced by a comment: no test fails. See F1.
6. **Secrets.** `git diff f8ae737~1..HEAD -- src tests examples docs/DESIGN.md | grep -nE '^\+.*(sk-[A-Za-z0-9]{10,}|ghp_|gho_|github_pat_|Bearer [A-Za-z0-9]{16,}|[A-Za-z0-9_-]{32,})'`
   matches only test function names (e.g. `+fn an_out_of_range_warn_pct_is_refused_naming_the_key() {`). The new
   output in `src` is two `ConfigError(format!(…))` strings that carry only the `warn_pct` value. Fixtures use
   `qb-fake-user` / `QB_UNUSED`. No key reaches argv, a row, a log or an error.
7. **Docs.** DESIGN §2's `state` row: "… is ≥ `[probe] warn_pct`, default 90; a worse state is never softened.
   CHORE-010, decided by steer bucket 2 on 2026-10-08, open to the Captain's veto; whether a near-limit `degraded`
   files an alert is SIG-010, open". §3's config block: `warn_pct = 90 # … CHORE-010, steer bucket 2`.
   `examples/quotabus.toml`: "decided by an agent session's steer pass, bucket 2, 2026-10-08, open to the Captain's
   veto … SIG-010, still open". None calls it a Captain ruling. `grep -n "^status" kanban-work/signals/SIG-010-*.md`
   → `5:status: backlog`, so "open" is accurate. The §3 SIG-009 hunk in the branch diff came in with the origin/main
   merge (CHORE-012), not from this item.
8. **Gate.** `make check` in the worktree → `rc=0`. The tail ends `test result: ok. 0 passed; 0 failed` (doc-tests);
   `grep -E "FAILED|failed"` finds only `0 failed` lines.

## Findings

**F1 (non-blocking, equivalent mutant): the API call site is not guarded by any test, and today it cannot be.**
M3 (dropping `r.state = warn_state(…)` in `src/probe.rs:426`) leaves the whole suite green:
`cargo test -q 2>&1 | grep -E "test result" | awk '{p+=$4; f+=$6} END {print "passed="p" failed="f}'` →
`passed=330 failed=0` (plus `warning: unused import: warn_state`). The reason is that no API row can carry a window
yet. `sed -n 113,131p src/classify.rs` shows `headroom_from_headers` always writes `window_pct: None`,
`windows: Vec::new()`, so `warn_state` is a no-op on every API row the probe builds today. The API rule is pinned only
through the pure function (`window_pct_known_answers`, `an_ok_api_row_at_97_through_the_rule_is_refused_by_select`),
as the test file's header says. The call is correct and makes DoD 2's "every row" hold when an API source starts to
fill `window_pct`. No change is required now. When an item makes an API source write `window_pct` or `windows[]`, it
should add a `Runner::run_due` test for an API row at ≥ `warn_pct`.

**F2 (nit, non-blocking):** DESIGN §2 says "decided by steer bucket 2" without "an agent session's", which the example
config says. It is accurate (steer is an agent-session pass, and the text calls it neither a ruling nor the Captain's)
and its veto clause is present, so no edit is required.

No blocking findings.
