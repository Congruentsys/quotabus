reviewed-at-sha: 2933bfcc4289a2999535bd7e4ad6c3f8ec677077
verdict: approve
gate: make check rc=0 at 2933bfcc4289a2999535bd7e4ad6c3f8ec677077 on Mac-mini
brief: reviews/CHORE-011-r1.brief.md @ 2933bfcc4289a2999535bd7e4ad6c3f8ec677077

Authorship: All the work under review and this review were done by LLM agent sessions (Claude Opus 5.5). No human
reviewed any of it. The tests (98335d9, plus the formatting-only 27450cd) were written by a fresh Opus sub-agent (the
test partner), and the code (dd0413a) by a second fresh Opus sub-agent (the implementer). Both were commissioned by the
driver session Mac-mini/s-e7976c42. The driver commissioned the tests and the code, checked them, wrote the committed
brief for this review, and gains from `approve`. This reviewer is "distinct" in this sense: a separate `claude -p`
process with a fresh context that wrote none of the work. It is the same model family, and the driver briefed it with
the committed brief `reviews/CHORE-011-r1.brief.md`. The reviewer ran in its own worktree `/tmp/rv-CHORE-011` at the
sha above, on host Mac-mini, on 2026-10-08.

## Checks run (brief items 1–7)

1. **The tests match the DoD, not the code.** `tests/chore011_round_used_pct.rs` at 98335d9 drives all three sources
   through `Runner::run_due`:
   - unified headers (`unified_headers_0_14_is_stored_as_14`, `unified_headers_round_to_two_decimals`)
   - stream-json `utilization` (`stream_json_0_14_is_stored_as_14`, `stream_json_rounds_to_two_decimals`)
   - Copilot `100 − percent_remaining` (`copilot_percent_remaining_33_333_is_66_67`,
     `copilot_percent_remaining_86_is_14`), plus the entitlement fallback (`copilot_entitlement_fallback_is_rounded`)

   The known answers are 0.14 → 14.0, 0.005 → 0.5, 0.29 → 29.0, 0.123456 → 12.35, 33.333 → 66.67, 86.0 → 14.0 and
   100/300 → 66.67. Each is checked by `==` on the struct, the JSON value and the serialized text. There are two
   controls. `control_the_unrounded_product_is_not_the_known_answer` shows that `0.14*100` serializes as
   `"14.000000000000002"`. `control_the_2dp_predicate_rejects_unrounded_values` shows the predicate rejects 12.3456
   and 66.667 and accepts the known answers. The tests use only the public `Record` path, so they cannot see
   `round_2dp`.
2. **The test change after 98335d9 is formatting only.**
   `diff <(git show 98335d9:tests/chore011_round_used_pct.rs | tr -d ' \t\n') <(git show HEAD:tests/chore011_round_used_pct.rs | tr -d ' \t\n') && echo IDENTICAL`
   printed `IDENTICAL`. `git diff --stat 98335d9 dd0413a -- tests/` printed nothing, so the implementer's commit did
   not touch the tests. `git diff --stat 98335d9..HEAD -- tests/` printed `1 file changed, 5 insertions(+), 3
   deletions(-)`, all from 27450cd.
3. **There is one build point and no bypass.** `grep -rn "Window {" src/` finds only the struct definition and the
   literal inside `Window::measured` (`src/record.rs:104`). Every source calls `Window::measured`:
   `src/subscription.rs:259` (unified), `:399` (stream-json) and `:498` (Copilot). `window_pct` copies the top
   window's already-rounded `used_pct` (`src/subscription.rs:143`). Nothing in the code special-cases the tests.
4. **Mutations** (each run was `$C test -q --test chore011_round_used_pct`, and `git checkout -q src/record.rs`
   restored the code after each; `git status --short` was then empty):
   - M1 replaced `used_pct: round_2dp(used_pct),` with `used_pct,`. Result: `test result: FAILED. 3 passed; 6 failed`.
     All 6 source tests went red at `tests/chore011_round_used_pct.rs:79`.
   - M2 replaced `(x * 100.0).round() / 100.0` with `(x * 10.0).round() / 10.0`. Result: `test result: FAILED. 5
     passed; 4 failed`. These went red: `copilot_entitlement_fallback_is_rounded`,
     `copilot_percent_remaining_33_333_is_66_67`, `unified_headers_round_to_two_decimals` and
     `stream_json_rounds_to_two_decimals`.
5. **Edge cases and pace:** see F1 and F2. Computing `pace` from the unrounded value is sound: rounding is for the
   stored and displayed figure, and the rate derived from it should not carry rounding error. The choice is stated in
   a code comment (`src/record.rs`, "pace uses the UNROUNDED used_pct …") and in the PR body. No code in the repo
   compares `used_pct` or `window_pct` against a threshold. `grep -rn "used_pct\|window_pct" src/*.rs` outside
   record.rs and subscription.rs finds only the display at `src/main.rs:568` (`{:.0}%`) and `window_pct: None` in
   classify.rs. State comes from the status word, not from the pct.
6. **Secrets:**
   `git diff origin/main...HEAD | grep -niE "sk-ant|ghp_|gho_|github_pat|@[a-z0-9-]+\.(com|org|net)|token|bearer|api[_-]?key"`
   matched only prose ("fake tokens", line 116 of the diff, and the brief's own wording). The new test adds no token
   and uses the existing `tests/exp002` fakes (`FAKE_GH = "fakeQBghTOKEN9c1d2e3fNOTREAL"`, env var NAMES only) and
   the login `qb-fake-user`. The same grep over `gh pr view 16 --json body` matched nothing. No email address
   appears.
7. **Gate:** `CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-011-target make check` returned
   `rc=0`. Every `test result:` line reads `ok … 0 failed`, including `ok. 9 passed` for the new test file.

## Findings

**F1 (nit): values within 0.005 of 100 round to 100.0, and unified sources are not clamped.**
Command: `python3 -I -c` applying Rust's half-away-from-zero `round_2dp` to sample inputs.
Output: `99.995 -> 100.0`, `99.994 -> 99.99`, `100.004 -> 100.0`, `100.005 -> 100.01`.
A future reader that treats `used_pct >= 100.0` as exhausted would fire on a 99.995% window. The unified and
stream-json paths pass `u * 100.0` unclamped (`src/subscription.rs:259, :399`), so a value above 100 survives
rounding. Copilot clamps to 0–100. This does not affect the current code, because no reader compares the pct, and it
is within the DoD ("at most 2 decimals").
Proposed fix (optional, follow-up): document on `Window::used_pct` that a reader must not take 100.0 alone as
exhaustion, or clamp the unified sources as Copilot does.

**F2 (nit): a tiny negative utilization serializes as `-0.0`.**
Command: the same script. Output: `-0.001 -> -0.0`. serde_json writes negative zero as `-0.0`.
A reading this small and negative is implausible from the API, and the behaviour is harmless.
Proposed fix (optional): `round_2dp(x) + 0.0`, or clamp at 0, if it is ever seen.

**F3 (nit): no test pins the "pace from the unrounded value" choice.**
Command: `grep -n pace tests/chore011_round_used_pct.rs; echo "grep rc=$?"`. Output: `grep rc=1` (no match: the file asserts no pace value). The choice
is stated, but if the computation were moved to the rounded value, the difference (≤ 0.005 pp) would not be caught.
Proposed fix (optional): none needed for this chore. A known-answer pace test belongs with whatever item next relies
on pace precision.

No blocker or should-fix findings, so the verdict is approve.
