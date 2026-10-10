reviewed-at-sha: 29f9bf0140d5def353868d2507e86b47266ee694
verdict: approve
gate: make check rc=0 at 29f9bf0140d5def353868d2507e86b47266ee694 on M5-MBP-2
brief: reviews/CHORE-025-r1.brief.md @ 29f9bf0140d5def353868d2507e86b47266ee694

Authorship: All the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
"Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model
family, briefed by the driver (the author) with this committed brief. The commissioning session is
M5-MBP-2/s-72a67d16. Its stake: it drove the work and wrote no tests or code. The tests were written by a sub-agent.

Review date 2026-10-10, host M5-MBP-2, worktree `/tmp/rv-CHORE-025` (detached at the SHA above),
`CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-025`. Every mutation was reverted with `git checkout -- src/<file>`, and
`git status --short` printed nothing after each one.

## F1: the tests match Done when 1 (non-blocking, no defect)

`tests/chore025_alert_local_skipped_discovering.rs` runs `quotabus alert` through `run_cli` over one discovering
local service, `dgx1-qwen` (kind `local`, no `models`). The fixture asserts `parsed.services[0].discovers()`.
- `discovering_local_service_with_no_rows_skips_one_slot`: empty store, M=1 (and N=0).
- `discovering_local_service_skips_every_id_of_its_newest_cycle`: one cycle of three ids, M=3.
- `control_an_older_cycles_ids_are_not_counted`: an older cycle of {old-1, old-2, old-3, model-a} at now-60s and a
  newest cycle of {model-a, model-b} at now. It asserts M≠5, M≠4 and M=2.

Baseline:
```
$ cargo test --locked --test chore025_alert_local_skipped_discovering 2>&1 | grep -E "^test |test result"
test control_the_summary_parser_has_known_answers_and_can_fail ... ok
test discovering_local_service_skips_every_id_of_its_newest_cycle ... ok
test discovering_local_service_with_no_rows_skips_one_slot ... ok
test control_an_older_cycles_ids_are_not_counted ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

## F2: the diff is test-only (non-blocking, no defect)

```
$ git diff origin/main...HEAD --stat
 reviews/CHORE-025-r1.brief.md                     |  30 ++++
 tests/chore025_alert_local_skipped_discovering.rs | 203 ++++++++++++++++++++++
 2 files changed, 233 insertions(+)
$ git log --oneline origin/main..HEAD
29f9bf0 CHORE-025: review brief r1
5123e5f CHORE-025: tests (pin M for a discovering local service)
```
No file under `src/` changed.

## F3: mutation 1, `slots()` in `alert_with`, turns all three alert tests red (non-blocking, no defect; meets DoD 2)

```
$ sed -i '' 's/skipped += svc.slots_in(&listing.rows).len();/skipped += svc.slots().len();/' src/main.rs
$ cargo test --locked --test chore025_alert_local_skipped_discovering 2>&1 | grep -E "^test |test result|panicked|left:|right:"
test control_the_summary_parser_has_known_answers_and_can_fail ... ok
test discovering_local_service_with_no_rows_skips_one_slot ... FAILED
test discovering_local_service_skips_every_id_of_its_newest_cycle ... FAILED
test control_an_older_cycles_ids_are_not_counted ... FAILED
thread 'discovering_local_service_with_no_rows_skips_one_slot' (93308044) panicked at tests/chore025_alert_local_skipped_discovering.rs:154:5:
  left: 0
 right: 1
thread 'discovering_local_service_skips_every_id_of_its_newest_cycle' (93308043) panicked at tests/chore025_alert_local_skipped_discovering.rs:173:5:
  left: 0
 right: 3
thread 'control_an_older_cycles_ids_are_not_counted' (93308041) panicked at tests/chore025_alert_local_skipped_discovering.rs:202:5:
  left: 0
 right: 2
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
$ git checkout -- src/main.rs
```
The tests fail for the right reason: M=0, which is the dgx1-qwen regression F6 named.

## F4: mutation 2, no newest-cycle filter in `slots_in`, turns the older-cycle control red (non-blocking, no defect)

```
$ sed -i '' 's/\.filter(|r| Some(r.checked_at) == newest)/.filter(|_r| { let _ = newest; true })/' src/config.rs
$ cargo test --locked --test chore025_alert_local_skipped_discovering 2>&1 | grep -E "^test |test result|panicked|left:|right:"
test control_the_summary_parser_has_known_answers_and_can_fail ... ok
test control_an_older_cycles_ids_are_not_counted ... FAILED
test discovering_local_service_with_no_rows_skips_one_slot ... ok
test discovering_local_service_skips_every_id_of_its_newest_cycle ... ok
thread 'control_an_older_cycles_ids_are_not_counted' (93308791) panicked at tests/chore025_alert_local_skipped_discovering.rs:195:5:
  left: 5
 right: 5
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
$ git checkout -- src/config.rs
```
Only the control goes red, which is expected: the other two tests have a single cycle, where the filter has no effect.

## F5: the `assert_ne!(m, 4)` line in the control is dead (NON-BLOCKING)

The control's comment says the "older cycle [is] larger than the newest, so a count of … the older cycle reads a
different M". The test writes the older cycle as four ids, but `model-a` is shared with the newest cycle. The store
is keyed by record key, so the newest cycle overwrites `model-a`. That leaves 3 rows of the older cycle, not 4. An
oldest-cycle mutation therefore reads M=3. The `assert_ne!(m, 4)` line can never fire. The extra probe:
```
$ sed -i '' 's/let newest = own.iter().map(|r| r.checked_at).max();/let newest = own.iter().map(|r| r.checked_at).min();/' src/config.rs
$ cargo test --locked --test chore025_alert_local_skipped_discovering 2>&1 | grep -E "^test |test result|panicked|left:|right:"
test control_the_summary_parser_has_known_answers_and_can_fail ... ok
test discovering_local_service_with_no_rows_skips_one_slot ... ok
test discovering_local_service_skips_every_id_of_its_newest_cycle ... ok
test control_an_older_cycles_ids_are_not_counted ... FAILED
thread 'control_an_older_cycles_ids_are_not_counted' (93309561) panicked at tests/chore025_alert_local_skipped_discovering.rs:202:5:
  left: 3
 right: 2
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
$ git checkout -- src/config.rs
```
The `assert_eq!(m, 2)` line still catches the mutation, so the property is guarded. The `!= 4` line and its comment
are misleading, not harmful. A follow-up could change the line to `assert_ne!(m, 3)`, or drop the overlap. This does
not block.

## F6: no control passes by construction (non-blocking, no defect)

- The parser control checks a known answer, `Ok((3,2,1,0))`, and three inputs that must be rejected (a missing
  field, a non-number and empty stdout). `m_of` panics on an unparsable line, so a changed summary format cannot
  make M read 0 silently.
- The fixture asserts `discovers()`, so the service is really a discovering one.
- The alert tests went red under F3 and F4 for the expected values. The exception is the `!= 4` line (F5).

## F7: no real key, no bus, no network beyond loopback (non-blocking, no defect)

```
$ grep -nE "http|nats|KEY|key|bus|doppler|secret" tests/chore025_alert_local_skipped_discovering.rs
7://! alert: <N> keys checked, <M> local skipped, <F> crossings filed, <C> re-armed
10://! DoD 1: `quotabus alert` over a discovering local service reads M=1 when the store holds none of its rows (the
17://! Fakes only: the file backend in a temp dir, a fake `nusy-kanban`, no key, no bus.
28:use quotabus::{Backend, FileBackend, Kind, State, record_key};
32:fn local_key(model: &str) -> String {
33:    record_key(Kind::Local, "qwen", "dgx1", model)
54:    let cfg = dir.path().join("quotabus.toml");
58:         account = \"dgx1\"\nbase_url = \"http://127.0.0.1:1/v1\"\nprotocol = \"openai\"\n\
66:        quotabus::Config::from_toml_str(&s).unwrap_or_else(|e| panic!("config parses: {e}\n{s}"));
82:            .put(&common::record(&local_key(m), State::Ok, at, 600))
107:        "keys checked",
138:        summary("noise\nalert: 3 keys checked, 2 local skipped, 1 crossings filed, 0 re-armed\n"),
141:    assert!(summary("alert: 3 keys checked, 1 crossings filed, 0 re-armed").is_err());
143:        summary("alert: 3 keys checked, x local skipped, 1 crossings filed, 0 re-armed").is_err()
```
- The config has a `[file]` backend in a tempdir and no `[bus]`.
- The only URL is `http://127.0.0.1:1/v1`, and `alert` never probes it.
- The alert sink is a fake `nusy-kanban` shell script in the tempdir, and the run's PATH is `/usr/bin:/bin`.
- No env var holds a key.

## F8: the gate (non-blocking, no defect)

```
$ PATH=/Users/hankh95/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-025 make check > /tmp/rv-CHORE-025-make.log 2>&1; echo "make check rc=$?"
make check rc=0
$ grep -E "test result:" /tmp/rv-CHORE-025-make.log | awk '{p+=$4; f+=$6} END {print "cargo: " NR " binaries, " p " passed, " f " failed"}'
cargo: 51 binaries, 422 passed, 0 failed
```
pytest printed `86 passed in 4.97s`. `yurtle-kanban validate`, `cargo fmt --check` and clippy `-D warnings` passed
(rc=0 overall).

## Verdict

Approve. Done when 1 and 2 are met, the diff is test-only and the gate is green. F5 is a non-blocking cleanup.
