reviewed-at-sha: 6687e99285435414f4cd76256064c39200ca6f4e
verdict: approve
gate: make check rc=0 at 6687e99285435414f4cd76256064c39200ca6f4e on M5-MBP-2
brief: reviews/CHORE-021-r1.brief.md @ 6687e99285435414f4cd76256064c39200ca6f4e

Authorship: LLM agent sessions (Claude Opus 5.5) did all of this work and this review. No human reviewed it.
"Distinct" here means a separate `claude -p` process with a fresh context that wrote none of the work. It is of the
SAME model family, and the driver (the author) briefed it with the committed brief named above. The commissioning
session is M5-MBP-2/s-72a67d16. Its stake: it drove the work and wrote no tests or code. Sub-agents wrote the tests
(5637ecc) and the code (b9d9073).

Worktree: `/tmp/rv-CHORE-021`, detached at the SHA above. CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-021. Date
2026-10-10, host M5-MBP-2.

## Findings

**F1 — the tests match the Definition of Done and the cited DESIGN sections (check 1). Does not block.**
Command: `cat kanban-work/chores/CHORE-021-*.md` and `cat tests/chore021_local_no_alert.rs`, read side by side.
- DoD 1 (local files no alert in any state, does not arm, does not clear):
  `a_local_service_files_nothing_whatever_its_state_under_the_default` and
  `…_even_when_every_state_is_listed` cover all 7 bad readings. `a_local_row_never_arms_an_alert_entry` checks
  that no `alert.<local key>` file is written. `a_local_ok_does_not_clear_an_entry_left_from_before` checks that a
  seeded entry stays byte-for-byte the same. Each has an API control that runs in the same rig.
- DoD 2 (still published; select still refuses): `an_unreachable_spark_is_still_published_and_files_nothing` runs
  `probe` end to end against a closed port. `select_still_refuses_a_local_row_that_is_not_ok` runs all 7 bad
  readings. Its known-answer control is `control_select_picks_the_local_row_first_when_it_is_ok`.
- DoD 3 (several cycles, 0 for local, API per SIG-010, controls): there are 4-cycle flapping tests for unreachable
  and for model_missing, plus a mixed sequence. Each expects api == 4. `api_rows_keep_sig_010s_default` pins the
  default list: 5 states file and latency and rate_limited do not.
- DoD 4 (DESIGN §3 alert row and §10 Q8): `records_the_ruling` checks the verbatim label, "Captain 2026-10-09", the
  local kind, and "recommended" outside the quoted label. It has negative controls for each part.
- Gap (not blocking): DoD 1 says "API **and subscription** rows are unchanged", but no integration test uses a
  `kind = "subscription"` service. Subscription is covered only by the unit test `only_local_services_are_skipped`
  in `src/alert.rs` (`assert!(alerts_for(Kind::Subscription))`). The code's only branch is `kind != Kind::Local`, so
  the risk is low.

**F2 — the tests are unchanged since T (check 2). Does not block.**
```
$ git diff 5637ecc..6687e99 -- tests/chore021_local_no_alert.rs | wc -c
       0
```

**F3 — the code does not special-case the tests (check 3). Does not block.**
```
$ git diff 5637ecc..b9d9073 -- src | grep '^+' | grep -nE 'qwen|dgx|kimi|moonshot|chore021|tests/|a_local_|control_' ; echo grep-rc=$?
grep-rc=1
```
The whole change is `alerts_for(kind) = kind != Kind::Local` and one `continue` in `alert_with`.
`grep -n 'alert_with' src/*.rs` shows that `alert_with` is the only alert loop, with call sites at main.rs:176 (bus)
and main.rs:180 (file). Both backends therefore take the skip.

**F4 — two mutations each turn tests red (check 4). Does not block.**
M1: `sed -i '' 's/    kind != Kind::Local$/    let _ = kind; true/' src/alert.rs`, then
`cargo test -q --test chore021_local_no_alert`:
```
test result: FAILED. 9 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```
(failed: a_local_ok_does_not_clear_an_entry_left_from_before, an_unreachable_spark_is_still_published_and_files_nothing,
a_local_service_files_nothing_whatever_its_state_under_the_default, …_even_when_every_state_is_listed,
a_local_row_never_arms_an_alert_entry, both flapping tests, the mixed test.) Restored with `git checkout -- src/alert.rs`.

M2: `sed -i '' 's/if !quotabus::alert::alerts_for(svc.kind) {/if false \&\& !quotabus::alert::alerts_for(svc.kind) {/' src/main.rs`
removes the skip. Same command:
```
test result: FAILED. 9 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
```
The same 8 tests failed. Restored with `git checkout -- src/main.rs`. After both restores, `git status --short`
printed nothing.

**F5 — secrets (check 5). Does not block.**
```
$ git diff origin/main...HEAD | grep -nEi 'sk-[a-z0-9]{10,}|api[_-]?key\s*=|[A-Za-z0-9]{32,}' | head
(no output)
$ grep -n 'FAKE_PLAIN' tests/common/mod.rs | head -3
16:pub const FAKE_PLAIN: &str = "fakeQBKEY7f3a9c0dNOTREAL";
$ gh pr view 30 --repo Congruentsys/quotabus --json body -q .body | grep -nEi 'sk-|key' | head
5:- Its `alert.<key>` entry is never written, and an entry left from before stays byte-for-byte unchanged.
```
The tests pass the fake key only through the environment (`(API_SECRET, FAKE_PLAIN)` given to `run_cli`), never on
argv. The only "key" in the PR body is the `alert.<key>` entry name.

**F6 — API and subscription alerting are unchanged; local rows are still published and still refused (check 6). Does not block.**
The code diff (`git diff origin/main...HEAD -- src`) leaves `decide_listed` and the `[alert] states` default
untouched. It skips only `Kind::Local`. The evidence is the green tests named in F1 (`api_rows_keep_sig_010s_default`,
the api == 4 / == 2 controls, the published-row test and the select tests) in the gate run in F8. One observation,
not blocking: `checked += 1` runs before the skip, so the summary line `alert: N keys checked, …` still counts local
keys. That is arguably right, since they were looked at, but the summary does not say they were skipped.

**F7 — DESIGN quotes the ruling verbatim, matching the item body, and says it was recommended (check 7). Does not block.**
I ran a `python3 -` script that joins the body's line breaks and searches DESIGN for the ruling's full quoted text:
```
in body (newlines joined): True
occurrences in DESIGN: 2
```
Both places (the §3 alert row and §10 Q8, "**Captain 2026-10-09, on local services**") read:
`Captain 2026-10-09: "No alert for local (Recommended)", the recommended option, whose text was: "kind = "local" rows
are still published … API and subscription alerts keep your SIG-010 default."` They cite `kanban-work/chores/CHORE-021-*.md`.

**F8 — gate (check 8). Does not block.**
```
$ cd /tmp/rv-CHORE-021 && PATH=/Users/hankh95/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-021 make check > /tmp/rv-CHORE-021-make.log 2>&1; echo "make check rc=$?"
make check rc=0
```
chore021 suite: `test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s`.
No suite in the log reports a failure.

## Verdict
approve. No finding blocks. F1's gap (no subscription service in the integration tests) and F6's summary-count
observation are optional follow-ups.
