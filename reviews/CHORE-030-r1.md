reviewed-at-sha: 7c0f4ca8497b7dd6e18eb3e7dd641d03011445bc
verdict: approve
gate: make check rc=0 at 7c0f4ca8497b7dd6e18eb3e7dd641d03011445bc on `<mini>`
brief: reviews/CHORE-030-r1.brief.md @ 7c0f4ca8497b7dd6e18eb3e7dd641d03011445bc

Authorship: All of this work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
"Distinct" here means a separate `claude -p` process with a fresh context that wrote none of the work. It is the same
model family, and the driver briefed it with the committed brief above. The commissioning session is
`<mini>/s-e7976c42`. Its stake: it filed the item, it commissioned the tests (13d5e2e) and the fix (228240b), and it
gains from `approve`.

Host `<mini>`, 2026-10-11. Worktree `/tmp/rv-CHORE-030`, detached at the SHA above. `CARGO_TARGET_DIR=/tmp/rv-CHORE-030-target`
was removed after the gate ran.

## The brief's checks

**1. The tests match the item, not the code; the tests are unchanged since 13d5e2e.**
```
$ git diff 13d5e2e..HEAD -- tests/ | wc -c
       0
```
Each "Done when" maps to a test:
- ≈50m, not 60m: `checked_ten_min_ago_shows_about_fifty_minutes`, band 47–50.
- The control at now shows 60m: `control_checked_now_shows_sixty_minutes`, band 59–60.
- No negative duration for a past reset: `past_reset_at_shows_no_negative_duration` and `past_reset_via_fallback_…`.
- `--json` and the stored row are byte-for-byte unchanged: `json_and_stored_row_unchanged`. It compares the file bytes
  before and after, checks that `row` equals the stored row and pins the key set.

The Plan's preference for `reset_at` and its fallback have their own tests: `reset_at_is_read_against_now` and
`fallback_without_reset_at_subtracts_age`. Every assertion is a band read against the item, and none relies on an
implementation detail.

**2. Real clock, tolerance bands: can they flake?** I ran the file three times:
```
$ for i in 1 2 3; do CARGO_TARGET_DIR=/tmp/rv-CHORE-030-target /Users/<user>/.cargo/bin/cargo test -q --test chore030_resets_in_now 2>&1 | grep -E "test result|FAILED|panicked"; done
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
```
My analysis: every expected value is relative (now ± offset), so a wall-clock minute boundary makes no difference.
`rfc3339` truncates `reset_at` to whole seconds, so a "+3600 s" reset can arrive as 3599.x s, which renders `59m`. Both
bands allow for that. Each band also leaves about 2–3 minutes of slack below the nominal value, and the CLI timeout is
`T` = 30 s, so a slow runner would hit the timeout before it left a band. The `checked` variable computed separately in
`checked_ten_min_ago…` differs from `sub_row`'s own `checked_at` by microseconds, which the band covers. I see no
realistic flake.

**3. The code.** `git diff 13d5e2e..228240b` changes only `src/main.rs`, and only inside `table_out` (+12/−2).
```
$ git diff origin/main...HEAD -- src/ | grep -n "fn \|select\|alert"
5:@@ -885,8 +885,18 @@ fn table_out(entries: &[Entry], now: chrono::DateTime<Utc>) -> String {
```
- **The same `now`:** `table_out(&entries, now)` (src/main.rs:756). AGE is `age_s(row, now)` and the new `left` uses
  that `now` for both `reset_at − now` and `age_s(row, now)`. They share a single clock read.
- **Nothing special-cases the tests:** the code has no ids, constants or test hooks. It parses `reset_at` as RFC 3339
  and falls back to `reset_in_s − age`.
- **`--json`, the stored row and `reset_in_s`:** `json_out` and the record code are not in the diff, and
  `json_and_stored_row_unchanged` is green. `select` and `alert` are untouched (the diff has a single hunk).
- **Edge cases.** I added a scratch test (`tests/zz_rv_scratch.rs`, deleted afterwards, never committed). It reuses
  the file's fixture. Each row was checked 600 s ago:
  ```
  SERVICE             MODEL               STATE  AGE  SOURCE    DETAIL
  garbage_at_with_in  garbage_at_with_in  ok     10m  official  five_hour 40% (resets in 50m)
  garbage_at_no_in    garbage_at_no_in    ok     10m  official  five_hour 40%
  neither             neither             ok     10m  official  five_hour 40%
  at_only             at_only             ok     10m  official  five_hour 40% (resets in 29m)
  in_only             in_only             ok     10m  official  five_hour 40% (resets in 50m)
  ```
  - An unparseable `reset_at` falls back to `reset_in_s − age` (50m), or shows no reset text when `reset_in_s` is
    absent too. It never panics and never prints the bad text.
  - With neither field, no reset text appears, as before.
  - The case of `reset_in_s` with no `checked_at`/age cannot occur. `Record.checked_at` is a required
    `DateTime<Utc>` (`src/record.rs:133`), and windows are only iterated from inside a row, so `age_s` is always
    `Some` here. If it were `None`, the `?` would produce no reset text rather than a wrong one.

**4. Mutations.** I restored the code after each one (`git checkout -q src/main.rs`; `git status --short` was empty at
the end).
```
== M1: drop - age_s in fallback
thread 'fallback_without_reset_at_subtracts_age' … panicked at tests/chore030_resets_in_now.rs:258:9:
thread 'past_reset_via_fallback_shows_no_negative_duration' … panicked at tests/chore030_resets_in_now.rs:291:9:
test result: FAILED. 6 passed; 2 failed; …
== M3: reset_in_s ahead of reset_at
thread 'reset_at_is_read_against_now' … panicked at tests/chore030_resets_in_now.rs:245:9:
test result: FAILED. 7 passed; 1 failed; …
== M2: s > 0 -> s >= -60
test result: ok. 8 passed; 0 failed; …
```
M1 and M3 are killed. M2 survives; see F1.

**5. The behaviour note** (`reset_at` without `reset_in_s` now shows reset text). This is right. DESIGN §2 (`headroom`
row) makes `reset_at` the stored fact and defines `reset_in_s` from it (`reset_in_s = reset_at − checked_at`), so
rendering from `reset_at` is the primary reading, not an invention. It affects display only. STATE and the verdict do
not depend on it, so it cannot turn any row into a false `ok` (G1). The scratch run's `at_only` row shows `resets in
29m` for a reset 30 min ahead. AGE stays beside it, so a reader can still tell that the row is old.

**6. Secrets.**
```
$ git diff origin/main...HEAD | grep -niE "sk-|ghp_|gho_|github_pat|token|bearer|@[a-z0-9-]+\.(com|org|io)|api[_-]?key"
37:+6. Secrets: nothing in the diff or the PR body carries a key, a token or an email.
142:+        tokens_remaining: None,
$ gh pr view 39 --json body -q .body | grep -niE "<same pattern>"   ; echo rc=$?
rc=1
```
Both diff hits are harmless (the brief's own wording and a struct field name). The PR body has no hits. The tests use
the fake secret name `QB_UNUSED` and no value.

**7. Gate.**
```
$ CARGO=/Users/<user>/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-030-target make check > /tmp/rv-CHORE-030-gate.log 2>&1; echo "make check rc=$?"
make check rc=0
```
The log shows 52 `test result: ok` lines and none FAILED. The script tests: `107 passed in 6.59s`. `yurtle-kanban
validate` ran. The target dir was then removed.

## Findings

**F1 (nit) — No test pins "reset due"; a reset that has already passed may render as `resets in 0s` and still pass.**
`check_past` accepts `resets in 0s` (`assert_eq!(check_past(Some("resets in 0s")), Ok(()))` in `checker_controls`).
`human_age` clamps negatives to `0s`. As a result, a mutant that drops the `reset due` branch entirely stays green:
```
== M2b: every Some(s) -> resets in human_age(s) (drops 'reset due')
-                Some(s) if s > 0 => detail.push_str(&format!(" (resets in {})", human_age(s))),
+                Some(s) => detail.push_str(&format!(" (resets in {})", human_age(s))),
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
```
(M2, `s >= -60`, also survives, for the same reason and because the past fixtures sit 600 s in the past.)
The item's "Done when" only asks for "no negative duration", and `resets in 0s` meets that, so this does not block.
The Plan does say "`(reset due)` or similar", though, and the code's `reset due` branch has no test.
**Proposed fix (follow-up, optional):** in the two `past_*` tests, assert that the text is `reset due` (or at least
that it is not a `resets in …` claim). Add a past-by-30-s fixture so the `s > 0` boundary is pinned.

No blocker or should-fix findings.
