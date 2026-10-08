reviewed-at-sha: 1f740d354f5915ec2c9f15cd716dc7a48da86275
verdict: changes
gate: make check rc=0 at 1f740d354f5915ec2c9f15cd716dc7a48da86275 on M5-MBP-2
brief: reviews/CHORE-007-r1.brief.md @ 1f740d354f5915ec2c9f15cd716dc7a48da86275

**Authorship:** All the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed
it. "Distinct" means this review ran in a separate `claude -p` process with a fresh context that wrote none of the work.
It is the same model family as the author, and the author briefed it with the committed brief above. The commissioning
session is M5-MBP-2/s-72a67d16. Its stake: it drove the test partner and the implementer, filed the item, and gains from
`approve`.

Host M5-MBP-2, 2026-10-08. Scratch worktree `/tmp/rv-CHORE-007` at the reviewed sha; `CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-007`.
The merge-base with `origin/main` is `aa97f73`. `origin/main` has since gained HAZ-001 (`4b08682`) and CHORE-005 (`07c1fa7`).

## Checks from the brief

1. **Tests against the DoD.** T covers all five DoD points and the Captain's rulings:
   - DoD 1 is `tests/per_kind_config.rs`: 12h defaults, TTL = 3 × the kind's own interval, per-kind `[ttl]`, old keys
     refused with the new keys named, `max_tokens`/`degraded_latency_ms` still accepted in `[probe]`.
   - DoD 2 is `tests/per_kind_schedule.rs` and `tests/cli_probe_due.rs`: fake clock plus file store; no row = due; API
     never reads the balance and vice versa; `--force` runs every kind. The control that fails on shared intervals is
     `control_the_reference_schedule_…` plus the `swapped_intervals_…` pair.
   - DoD 3 is `each_row_carries_its_own_kinds_ttl` and `an_overridden_ttl_is_the_rows_ttl_per_kind`.
   - DoD 4 and DoD 5 are `tests/tick_and_docs.rs`, which includes a known-answer control for the plist parser.

   T's edits to existing tests (`git diff aa97f73..0acfc02 -- tests/config_toml.rs tests/probe_runner.rs tests/cli_probe.rs tests/real_run_defects.rs tests/review_r1.rs`):
   - Each edit rewrites the retired `[probe] interval/ttl` keys as `[intervals]`/`[ttl]`.
   - Assertions on the 15m/45m defaults are rewritten as 12h/36h through `interval_for`/`ttl_for`.
   - The bad-duration refusal for `[probe] ttl = "forever"` is replaced by the same refusal on `[ttl] api` and
     `[intervals] balance`.

   None of these edits weakens a guard that is still valid.
2. `git diff 0acfc02..HEAD -- <T's nine test files> | wc -c` gave `0`. The implementer did not touch T's tests.
3. No test special-casing. `git diff 0acfc02..304e545 -- src | grep -n -E '^\+.*(test|QB_DS|FAKE|2031|cfg\(test\)|wiremock|127\.0\.0\.1|localhost)'`
   matched only two doc comments: probe.rs "so a test points it at a local stub" and "`now` is the clock (injectable for
   tests)". The code has no branch on a test value or path.
4. Both mutations went red. After each one, `git checkout -- <file>` restored the file and `git status --short` was empty.
   - **Balance read on the api interval.** `sed -i '' 's/due(QueryKind::Balance, &balance_key)/due(QueryKind::Api, \&balance_key)/' src/probe.rs`
     followed by `cargo test --locked --test per_kind_schedule` gave:
     ```
     test a_due_balance_read_never_calls_the_model ... FAILED   (left: (1, 1)  right: (1, 2))
     test a_due_api_probe_never_reads_the_balance ... FAILED    (left: (2, 2)  right: (2, 1))
     test swapped_intervals_swap_the_schedules ... FAILED       (left: [0, 200, 400]  right: [0, 75, 150, 225, 300, 375])
     test each_row_carries_its_own_kinds_ttl ... FAILED         (left: 32400  right: 10800)
     test each_kind_runs_on_its_own_interval ... FAILED         (left: [0, 75, 150, 225, 300, 375]  right: [0, 200, 400])
     test result: FAILED. 4 passed; 5 failed
     ```
   - **"No row" read as not due.** In `src/schedule.rs` `is_due`, `return true;` became `return false;`.
     `cargo test --locked --test cli_probe_due` failed
     `a_second_probe_straight_after_the_first_calls_nothing_and_force_calls_every_kind` (left: (0, 0), right: (1, 1)).
     `cargo test --locked --no-fail-fast --test per_kind_schedule` gave
     `test result: FAILED. 2 passed; 7 failed`; the failures include `an_empty_store_makes_every_kind_due`, with
     left: (0, 0) and right: (1, 1).
5. Design choices the item left open:
   - **Balance reads write their own `<kind>.<provider>.<account>.balance` row.** DoD 2 needs this: "that kind's last row
     in the store" requires a balance row. The design §2 record contract does not describe this row (F2).
   - **Due-ness is per row.** This is consistent with DoD 2. A model newly added to the config is due at once, which is
     correct.
   - **Rows written without a call (`secret_unset`, `no_endpoint`) leave the kind due.** That costs no calls, and §3 now
     documents it. It is consistent with "no row = due".
   - **An unreadable store refuses `probe` with exit 1 unless `--force`.** This is consistent with §2 ("never ok") and
     with the ruling to spend less. The exit code is the same as before, when an unreachable bus made `publish` fail. It
     has no test (F3).
   - **`status` ignores the balance row.** This is consistent: `status` reports configured (service, model) pairs, and
     the floor still reaches those rows as `quota_exhausted`.
6. **Secrets.** No key on argv, in a row, log, error or fixture. The fixture is `FAKE_SK = "sk-test-not-a-key-0123456789abcdef"`.
   - `git diff aa97f73..HEAD | grep -E '^\+.*(sk-[A-Za-z0-9_-]{16,}|xai-…|ghp_|gho_|AKIA|Bearer …|eyJ…|[0-9a-f]{32,})'`
     returned rc=1 (no match).
   - The new output paths go through the redactor: `nothing due` goes through `redactor.redact`, and
     `cannot read the store (…)` goes through `redact_error`.
   - The new `tracing::info!` logs only the record key slug and the state.
7. **Docs.** §3 and §10 Q5 quote both Captain rulings exactly as SIG-004 records them. §4's cost line and the
   `examples/quotabus.toml` / `packaging/README.md` text match the code.
   `grep -n -E '15 ?m|900|96 calls|interval' docs/DESIGN.md packaging/README.md` finds the 15-min figures only at
   DESIGN.md:296. That line is §10 Q5's original recommendation, kept as the question's record, with the ruling
   appended. The same grep over the DESIGN.md merged with `origin/main` (`git merge-tree` → `380e65a8`) also finds the
   15-min figures only in that Q5 line.
8. **Gate.** `make check CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-007` gave rc=0 at the reviewed sha. That covers 35
   pytest tests, `yurtle-kanban validate`, fmt, clippy `-D warnings` and every cargo test binary. **It fails once merged
   with current `origin/main`** (F1).

## Findings

**F1 — blocker: merging into current `main` breaks the gate.** HAZ-001's tests on main still write the `[probe] ttl`
key that this PR now refuses. PR CI is red for the same reason.

```
$ git merge-tree --write-tree --name-only origin/main HEAD      → 380e65a8…, rc=0 (clean textual merge)
$ git worktree add --detach /tmp/rv-CHORE-007-merge origin/main && git merge --no-commit --no-ff 1f740d35…
  Automatic merge went well; stopped before committing as requested
$ make check CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-007        → rc=2
$ cargo test --locked --no-fail-fast   (same merged tree)
  haz001_review_r1:      f2_a_server_list_with_credentials_connects_and_round_trips ... FAILED
    "… quotabus.toml: [probe] ttl is retired: set each kind's own in [ttl] (api, balance; default 3 × its interval)"
  haz001_url_credentials: 5 FAILED (a_down_bus_with_a_creds_url_leaks_no_password,
    creds_in_quotabus_nats_url_…, creds_in_the_bus_table_url_…, control_without_credentials_…, a_wrong_password_…)
$ grep -n -E '\[probe\]|ttl =' tests/haz001_*.rs
  tests/haz001_review_r1.rs:129:        "[probe]\nttl = \"2m\"\n\n\
  tests/haz001_url_credentials.rs:142:            "{bus}\n[probe]\nttl = \"2m\"\n\n\
$ gh run view 37830421791 --log-failed   (PR #9 CI at 1f740d35, check "rust" = FAILURE)
  test f2_a_server_list_with_credentials_connects_and_round_trips ... FAILED
  … [probe] ttl is retired: set each kind's own in [ttl] …
```

I tested a fix in the scratch merge tree only; nothing was committed and the tree is removed. In those two fixtures I
replaced `[probe]\nttl = \"2m\"\n` with `[ttl]\napi = \"2m\"\nbalance = \"2m\"\n`. Then
`cargo test --locked --no-fail-fast --test haz001_review_r1 --test haz001_url_credentials` gave
`test result: ok. 7 passed` twice. So no code conflicts with HAZ-001's credential handling; the only conflict is the
retired key.

**Fix:** merge `origin/main` into the branch. Make the same mechanical fixture edit to the two HAZ-001 test files, as a
test-partner edit under DoD 1 like T's other fixture edits. Re-run `make check` and CI on the new tip. The PR body's
"make check rc=0" is true only at the stale base and should say so.

**F2 — should-fix: the balance row is a new record shape that §2, the reader contract, does not describe.**
§2 says one record per (kind, provider, account, model). The new row puts the literal `balance` in the model slot, with
`probe.name = "balance"` and `state` `ok`/`quota_exhausted`. Only §3 describes it.

```
$ sed -n 30,31p docs/DESIGN.md
One record per **(kind, provider, account, model)**. JSON on the bus; the same struct in the library and the CLI.
$ grep -n 'balance row' docs/DESIGN.md
  (§3 cadence paragraph only)
```

Any reader that lists keys on the bus would read `api.deepseek.nusy-product-team.balance` as an `ok` model named
"balance". That includes the planned selector, the alert and `nats kv` users. `status` is not affected because it walks
the configured models. Config already refuses a real model whose slug is `balance` (src/config.rs, the new check in
`from_toml_str`).

**Fix:** add a sentence or table note in §2. It should say that each service with a balance endpoint also writes
`<kind>.<provider>.<account>.balance` (`probe.name = "balance"`), that this row is not a model, and that a reader
choosing models skips it. Alternatively, give the row a key segment that cannot be read as a model; that would be a
DESIGN change in this PR.

**F3 — nit: the store-unreadable refusal has no test.** This is a new behaviour: without `--force`, `probe` exits 1 and
makes no calls when the store cannot be read. It is stated in the PR body, but no test or control guards it.

```
$ grep -rn -E "cannot read the store|--force\"\]" tests/
tests/cli_probe_due.rs:91:    let out = probe(&cfg, &["probe", "--force"]).await;
```

The error is also printed under the label `quotabus: cannot publish:`, which is a misleading prefix for a read failure.

**Fix:** add a CLI test with an unreadable store, for example a `[file] dir` that is a regular file, or a down bus as in
`an_unreachable_bus_…`. It should assert rc 1, zero stub calls and the `--force` hint. Optionally, print the read
failure with its own prefix.
