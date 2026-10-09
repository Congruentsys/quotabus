reviewed-at-sha: 4eb7021ba6f8bfb1d20f7f2a5c960a3d633663c6
verdict: changes
gate: make check rc=0 at 4eb7021ba6f8bfb1d20f7f2a5c960a3d633663c6 on Mac-mini
brief: reviews/CHORE-018-r1.brief.md @ 4eb7021ba6f8bfb1d20f7f2a5c960a3d633663c6

Authorship: LLM agent sessions did all of this work and wrote this review. Every session ran Claude Opus 5.5
(claude-opus-5-5). No human reviewed it. The commissioning session, Mac-mini/s-e7976c42, filed CHORE-018. It
commissioned the tests (2694eff) from one fresh Opus sub-agent and the code and docs (579c059) from a second, checked
both, and gains if this review approves. This review was written by a distinct reviewer: a separate `claude -p`
process with a fresh context that wrote none of the work. It is the same model family, and the driver briefed it with
the committed brief named above. It made no board or GitHub writes. Its only commit is this file.

## What was checked (all passed except F1)

- **Brief point 2.** `git diff 2694eff..HEAD -- tests/ | wc -l` printed `0`, so tests/ is unchanged since the red commit.
- **Brief point 1, the ruled edits to existing tests** (`git diff origin/main...2694eff -- tests/exp004_alert.rs tests/exp004_r1.rs tests/exp004_config.rs tests/chore018... chore010_warn_pct.rs`):
  - `exp004_alert.rs` swaps a bare `unknown` row for a `cannot_assess:not_found` row in two tests. The property "a
    CANNOT-ASSESS first row files nothing / never clears" still holds. The other bare-`unknown` uses (lines 405, 876)
    keep `cannot_assess:unspecified`, which never files.
  - `exp004_r1.rs` lists `degraded` and `rate_limited` in `[alert] states`, so the bad→bad dedup cases still exercise
    those states.
  - `exp004_config.rs` compares the alert config sink by sink and keeps the `assert_ne` control.
  - `chore010_warn_pct.rs` drops only the "files as any bad state" test, the rule SIG-010 overturned.
  - The CANNOT-ASSESS-never-clears and bad→bad re-filing properties are kept and extended in
    `a_cannot_assess_that_is_not_unreachable_still_files_nothing_and_never_clears`.
  - DoD points 1–5 each have tests.
- **Brief point 3, special-casing.** `git diff origin/main...HEAD -- src/ | grep -iE "^\+.*(cfg\(test|kimi|testhost|wiremock|127\.0\.0\.1|fake)"`
  returned rc=1 (no matches).
- **Brief point 4, two mutations.** Each was restored with `git checkout` afterwards. `git status --short` was clean.
  - **M1:** in `decide_listed`, `if !names.iter().any(|n| states.iter().any(|s| s == n)) {` was changed to
    `if names.is_empty() {`, so every unlisted bad state files. `cargo test -q --test chore018_alert_states` gave
    `test result: FAILED. 18 passed; 6 failed`. The six were:
    - `a_configured_list_is_the_whole_list_not_added_to_the_default`
    - `a_configured_list_with_rate_limited_files_it`
    - `degraded_in_the_list_files_a_latency_degraded_row`
    - `exp004_flapping_ok_and_latency_degraded_ten_cycles_files_zero`
    - `latency_degraded_and_rate_limited_do_not_arm_the_alert`
    - `under_the_default_latency_degraded_and_rate_limited_file_nothing`
  - **M2:** in `apply_warn_rule`, `r.reason = Some(WINDOW_NEAR_LIMIT.to_string());` was replaced with a comment. The
    result was `test result: FAILED. 23 passed; 1 failed`, the failure being
    `a_window_at_97_reads_degraded_with_reason_window_near_limit`. (See F3.)
- **Brief point 5, readers and redaction.** I put a `R::Near` row in the store with a temporary test, since removed,
  and ran the CLI:
  - `status --json` printed `"reason": "window_near_limit", "state": "degraded"`. The reason survives the redactor.
  - `select --role review --json` printed
    `[] | quotabus: nothing qualifies for role "review": no configured model with the role has a fresh ok row`, rc=3.
    `select` still refuses the degraded row.
  - `decide_listed` returns `Clear` only for a measured `ok`. Unlisted, other CANNOT-ASSESS and UNKNOWN verdicts all
    return `Nothing`, which neither files nor clears.
- **Brief point 6, docs.** DESIGN §2, the §3 alert row and the EXP-004 notes, §10 Q8 and `examples/quotabus.toml`
  quote "SIG-010: go with the recommended default, option 3" verbatim. The cite `SIG-010-*.md:37` on origin/main reads
  `Captain 2026-10-09: "SIG-010: go with the recommended default, option 3". ...`. Nothing else changed meaning.
  `the_example_config_shows_alert_states_with_the_default_and_loads` passes.
- **Brief point 7, secrets.** I grepped the diff for key, token and email patterns
  (`sk-…`, `ghp_`, `xox[bp]-`, `@host.tld`, bearer, `api_key="…"`): rc=1. The same grep over the PR body
  (`gh pr view 27 --json body`): rc=1.
- **Brief point 8, the gate.**
  `CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-018-target make check` gave rc=0, with
  354 tests passed and 0 failed across all `test result` lines.

## Findings

### F1 — should-fix: a Claude subscription at Anthropic's `allowed_warning` files nothing under the default, even with a window at 97 %

`apply_warn_rule` (`src/classify.rs`) writes `window_near_limit` only when it turns an `ok` into `degraded`. The
Claude unified-headers reader (`src/subscription.rs` `status_state`) already maps the provider's own near-limit status
word `allowed_warning` to `degraded`. So the warn rule never runs on that row, the row carries no reason, and
`listed_as` reads it as plain `degraded`. The default `[alert] states` does not list plain `degraded`, so the row
files nothing.

That holds even when a window is ≥ `warn_pct`, which is exactly the near-limit window the ruling says files. Before
CHORE-018 the same row filed, as every bad state did. This reports a near-limit service less visibly than before, for
a state the ruling says should file (G1).

Command: a temporary test added to a copy of `tests/chore018_alert_states.rs` and removed afterwards. It ran
`unified_row(0.97, "allowed_warning")`, put the row in a default rig and ran `alert`:
`cargo test -q --test zz_rv018 rv_ -- --nocapture`. Output:
```
RV state=degraded reason=None
RV filed_under_default=0
```
The test gate's own control, `unified_row(0.97, "allowed")`, reads `degraded` with reason `window_near_limit` and files.

Fix: tag the near-limit cause independently of the prior state. In `apply_warn_rule`, set
`reason = window_near_limit` whenever any window is ≥ `warn_pct` and the resulting state is `degraded`, not only when
the state was `ok`. Also have the unified reader's `allowed_warning` produce that reason, because it is the provider
saying the window is near its limit. A worse state (`quota_exhausted`, …) must stay untouched, as
`a_worse_state_at_97_is_not_labelled_near_limit` requires. Add partner tests:
- `unified_row(0.97, "allowed_warning")` files under the default.
- `unified_row(0.50, "allowed_warning")` files, or record a ruling that it should not.
- a latency-degraded row with no window still files nothing.

### F2 — nit: the `status` table does not show the near-limit reason

Command: the same temporary test ran `status` on an `R::Near` row. Output:
`kimi     kimi-k2.5  degraded  0s   official |` with an empty DETAIL column. Only `--json` shows `window_near_limit`.
This is no regression: before this PR there was no reason to show. But an operator reading the table cannot tell the
two kinds of `degraded` apart.

Fix (optional, or a follow-up chore): put `window_near_limit` (or the window and its %) in DETAIL for a `degraded` row.

### F3 — nit: no end-to-end probe → alert test for a near-limit window

The M2 mutation (no reason written) turns only the row-level test red. Every alert-level test builds its near-limit
row with the reason preset (`row(R::Near)`). A regression between the probe and the alert step would be caught only
indirectly.

Fix: when adding the F1 tests, have one of them run a probed `unified_row` through `alert` under the default, as the
F1 command above does.
