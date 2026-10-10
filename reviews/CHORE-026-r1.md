reviewed-at-sha: d84a8fc62e6dbbd6890db83155df30ff34baf49b
verdict: changes
gate: make check rc=0 at d84a8fc62e6dbbd6890db83155df30ff34baf49b on Mac-mini
brief: reviews/CHORE-026-r1.brief.md @ d84a8fc62e6dbbd6890db83155df30ff34baf49b

Authorship: LLM agent sessions did all the work and this review; no human reviewed it. The model is Claude Opus 5.5
(claude-opus-5-5) throughout. The test partner (tests at 266f9ca) and the implementer (fix at 5646fb5) were fresh Opus
sub-agents commissioned by the driver, Mac-mini/s-e7976c42. This reviewer is "distinct": a separate `claude -p` process
with a fresh context that wrote none of the work. It is the same model family, and the driver briefed it with the
committed brief named above. The commissioning session has a stake: it filed the item, commissioned the tests and the
fix, and gains from `approve`.

Host Mac-mini, 2026-10-10. Scratch worktree /tmp/rv-CHORE-026 (detached at d84a8fc), scratch stubs in /tmp/rv26s.

## Checks that passed (no finding)

**1. The tests are unchanged since red and match "Done when".**
`git diff 266f9ca..HEAD -- tests/ | wc -l` → `0`. The tests run the README's own wrapper, extracted and never retyped,
under `/bin/sh` with `probe --force --config <file>` (force first and force last). They assert that the probe keeps
`--force` and that `alert` gets exactly `["alert","--config",<file>]`. The stub behaves like clap. This matches the
item. The item says "stub binaries on PATH", but the tests rewrite the wrapper's absolute path instead. That is
equivalent here, because the wrapper calls quotabus only by absolute path.

**2. The wrapper is POSIX sh, run under `/bin/sh` and `/bin/dash`, old wrapper vs new.**
Both runs used a logging stub at `/tmp/rv26s/bin/quotabus`, never the real binary. Command:
`for sh in /bin/sh /bin/dash; do for w in old new; do /bin/sh drive.sh $sh $w.sh; done; done`. `drive.sh` calls the
wrapper with each argv below; `new.sh` is the README block with its quotabus path pointed at the stub. Output was
identical under sh and dash:

| wrapper argv | old: `alert` got | new: `alert` got |
|---|---|---|
| `probe --config X` | `[--config] [X]` | `[--config] [X]` |
| `probe --force --config X` | `[--force] [--config] [X]` (exit 2 for real) | `[--config] [X]` |
| `probe --config X --force` | `[--config] [X] [--force]` (exit 2 for real) | `[--config] [X]` |
| `probe --config=X --force` | `[--config=X] [--force]` (exit 2 for real) | `[--config] [X]` |
| `probe` (no --config) | *(nothing)* | `[--config] []` ← **F1** |
| `probe --force` (no --config) | `[--force]` | `[--config] []` ← **F1** |
| `probe --config "/a b/c d.toml" --force` | `[--config] [/a b/c d.toml] [--force]` | `[--config] [/a b/c d.toml]` |
| `probe --config --force` (path literally `--force`) | `[--config] [--force]` | `[--config] [--force]` |
| `probe --config --config --force` | `[--config] [--config] [--force]` | `[--config] [--force]` (F3) |
| `probe --config A --config B` | all four | `[--config] [B]` |
| `probe --config=` | `[--config=]` | `[--config] []` (both fail; not a regression) |
| `probe --config "*"` | `[--config] [*]` | `[--config] [*]` (no globbing) |

The probe got the full argv, unchanged, in every case and in both wrappers. A config path with spaces survives. The
`--config=X` form works. Every case is fixed or unchanged except the no-`--config` case (F1).

**3. Two mutations each turn a test red.** Each mutation was applied to `packaging/README.md` with a python replace,
the test file was run with `.venv/bin/python -m pytest -q tests/test_chore026_cycle_wrapper.py`, and the README was
restored afterwards (`git status --short` empty):
- `alert --config "$cfg" || rc=$?` → `alert "$@" || rc=$?`:
  `FAILED ...test_forced_one_shot_gives_both_steps_valid_argv[force-first]`, `[force-last]` — `2 failed, 8 passed`.
- `|| rc=$?` dropped: `AssertionError: STUB_FAIL_ALERT: wrapper rc=0 although a step failed`,
  `FAILED ...test_wrapper_still_reports_a_failed_step` — `1 failed, 9 passed`.

**4. The README's claims are accurate, and HAZ-004 still holds.**
The README says `alert` "takes no other flag and exits 2 on `--force`". Checked against the repo build:
`env -i PATH=/usr/bin:/bin /tmp/rv-CHORE-026-target/debug/quotabus alert --force --config x.toml` →
`error: unexpected argument '--force' found ... Usage: quotabus alert [OPTIONS]`, `rc=2`. The contract "exits non-zero
if either failed" is kept (see the mutation above).
`.venv/bin/python -m pytest -q tests/test_haz004_mini_paths.py` → `8 passed`.

**5. No secrets.** I scanned `git diff origin/main...HEAD` and `gh pr view 35 --json body` with
`grep -i -E '(sk-|xai-|ghp_|github_pat|AKIA|api[_-]?key *=|token *=|<email regex>)'`. The diff matched only on lines
218 and 272 (`+@pytest.mark.parametrize(...)`), which are false positives of the email pattern. The PR body had no
match (rc=1). There is no key, token or email.

**6. The gate passed.** `CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-026-target make check` →
`make check rc=0`. Every `test result: ok`.

## Findings

**F1 — should-fix: with no `--config`, the new wrapper gives `alert` an empty `--config ""`, overriding
`$QUOTABUS_CONFIG` and `./quotabus.toml` (a regression from the old wrapper).**
`src/main.rs:23-25` makes `--config` an `Option<PathBuf>` with the help text "else $QUOTABUS_CONFIG, else
./quotabus.toml". The old wrapper called a bare `alert` when no `--config` was given, so it used that fallback. The
new one always passes `--config "$cfg"`, even when `cfg` is empty. This affects a hand one-shot run like
`quotabus-cycle probe --force` with the config in `$QUOTABUS_CONFIG` or the working directory, which is the use this
item exists for. In that run the probe works and the alert exits 2. Measured with the repo build (not the installed
binary) in an empty scratch directory:
```
$ env -i PATH=/usr/bin:/bin QUOTABUS_CONFIG=/tmp/rv26s/envcfg.toml $B alert           # old wrapper's call
CANNOT-ASSESS: cannot read /tmp/rv26s/envcfg.toml: No such file or directory (os error 2)   ← env honoured
$ env -i PATH=/usr/bin:/bin QUOTABUS_CONFIG=/tmp/rv26s/envcfg.toml $B alert --config ""  # new wrapper's call
error: a value is required for '--config <CONFIG>' but none was supplied
rc=2
```
**Fix:** pass `--config` only when one was found. This stays POSIX:
```sh
set --; [ -n "$cfg" ] && set -- --config "$cfg"
/Users/hankh19/.local/bin/quotabus alert "$@" || rc=$?
```
(the `set` goes after the probe line). I checked this under `/bin/sh` and `/bin/dash` with the same `drive.sh`:
`probe` and `probe --force` give a bare `alert`, every other row of the table above is unchanged, and the spaces path is
intact. With this change applied, the existing tests still pass (`tests/test_chore026_cycle_wrapper.py` and
`tests/test_haz004_mini_paths.py`: `18 passed`); the README was restored afterwards. Add a test: `probe --force` with
no `--config` must give `alert` no `--config`. Its control is the current wrapper, which gives `alert` `["--config",""]`.

**F2 — nit: no test covers the `--config=<file>` form, which the PR body says works.**
I deleted the `--config=*) cfg=${a#--config=} ;; ` branch from the README and ran
`.venv/bin/python -m pytest -q tests/test_chore026_cycle_wrapper.py` → `10 passed`, so the mutant survived. My manual
run (table, row 4) shows the form does work today. **Fix:** add `["probe", "--config=CFG", "--force"]` to the forced
parametrize, with the expected `alert` argv `["alert","--config",cfg]`.

**F3 — nit: a config path that is literally `--config` is mis-parsed (`--config --config --force` gives `alert`
`--force`).** The repo build rejects a hyphen-led value for the probe too, so the probe fails first and nothing is
lost. Command: `env -i PATH=/usr/bin:/bin $B probe --config --config --force` (and `$B probe --config --force`), each
→ `error: a value is required for '--config <CONFIG>' but none was supplied`, `rc=2`. This also covers the brief's
"config path that is literally `--force`" case: neither wrapper can pass it through, and the old wrapper behaved the
same way. No change is needed. I note it only because the brief asked about hyphen-led paths.
