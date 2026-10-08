reviewed-at-sha: 76300e5af9f141e4d4c95fcb7ec1907052656f4e
verdict: changes
gate: make check rc=0 at 76300e5af9f141e4d4c95fcb7ec1907052656f4e on Mac-mini
brief: reviews/CHORE-008-r1.brief.md @ 76300e5af9f141e4d4c95fcb7ec1907052656f4e

Authorship: All of this work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
The tests (8edf666) were written by a fresh Opus sub-agent (the test partner). The code (b820c09) was written by a second
fresh Opus sub-agent (the implementer). Both were commissioned by the driver session Mac-mini/s-e7976c42, which
checked their work, wrote the brief and gains from `approve`. This review was written by a distinct reviewer: a separate
`claude -p` process with a fresh context that wrote none of the work. It is the same model family and was briefed by the
driver with the committed brief `reviews/CHORE-008-r1.brief.md`. The reviewer ran every command below on Mac-mini on
2026-10-08, in its own worktree `/tmp/rv-CHORE-008` detached at the reviewed sha.

## Summary of the checks (all passed unless a finding says otherwise)

1. **The tests match the Definition of Done, not the code.** DoD 1: local, named host, the stdin ask, launchd and
   systemd, paths from user and home. DoD 2: the README records Mini, and the Mini command renders the fixture field by
   field. DoD 3: no secret in a unit, the plan or argv. DoD 4: local and host renders, plus the `/Users/admin` control.
   Every checker has a control. The fixture is EXP-001's plist, byte for byte:
   `git show origin/main:packaging/launchd/com.congruentsys.quotabus-probe.plist | diff - tests/fixtures/chore008_mini_probe.plist && echo IDENTICAL_TO_ORIGIN_MAIN`
   → `IDENTICAL_TO_ORIGIN_MAIN`. `git diff -M --summary origin/main...HEAD` → `rename packaging/launchd/com.congruentsys.quotabus-probe.plist => tests/fixtures/chore008_mini_probe.plist (100%)`.
   The real-install path (no `--render-to`) is not exercised by any test, by design. It is reviewed by reading in F1
   and F2.
2. **The tests are untouched.** `git diff 8edf666..HEAD -- tests/ | wc -c` → `0`.
3. **No special-casing.** `grep -nE 'fakebin|sk-test|tester|chore008|fixture|pytest|alice|carol|buoy|dgx1|97' packaging/install.sh`
   → no output, `grep rc=1`.
4. **Mutations** (each one restored with `git checkout packaging/install.sh`; `git status --short` is empty after).
   Baseline: `.venv/bin/python -m pytest -q tests/test_install_target.py` → `24 passed`.
   - M1, `/Users/admin` hard-coded in the StandardOut/ErrorPath of `render_plist` → `5 failed, 19 passed`
     (test_local_macos_renders…, test_local_defaults_user_and_home…, test_named_host_renders…,
     test_local_render_carries_no_hard_wired_mini_path, test_hard_wired_path_absent_from_packaging_sources_except_readme).
   - M2, the `--render-to` branch also runs `launchctl list >/dev/null 2>&1 || true` → `5 failed, 19 passed`
     (both local renders, both named-host renders, test_installing_for_mini_yields_todays_plist: "a real-install tool was
     called during a dry run").
   - M3 (extra), `print_plan` also runs `env | grep _KEY || true` → `3 failed, 21 passed` (all three
     test_no_secret_in_units_plan_or_argv cases). The secret test can fail.
5. **Dry runs.** I ran the README's Mini command with `--render-to /tmp/rv-CHORE-008-out` under `bash` (5.3.9) and
   `/bin/bash` (3.2.57). Each run gave `rc=0`. `diff` against the fixture gave `diff rc=0` (byte-identical), and
   `plutil -lint` gave `OK`. The plan printed `target: host mini (over ssh, as admin)` and
   `unit: … -> /Users/admin/Library/LaunchAgents/com.congruentsys.quotabus-probe.plist`.
   I also ran `--where local --os linux --user alice --home /home/alice --launcher "secretspec run --" --quotabus "/opt/qb dir/quotabus" --render-to …`
   under both bashes. Both gave `rc=0`. The output was
   `ExecStart=secretspec run -- "/opt/qb dir/quotabus" probe --config /usr/local/etc/quotabus/quotabus.toml`, a
   `Type=oneshot` service and a timer with `OnBootSec=60 / OnUnitActiveSec=900 / WantedBy=timers.target`. The plan
   paths were under `/home/alice/.config/systemd/user/`.
   With no flag and empty stdin the script printed `install.sh: no target: answer "local" or a host name, …` with
   `rc=2`, and created no output dir.
   **Reading the real-install path.** It runs under `set -eu`. It stages the units in `mktemp -d`, which a trap removes.
   Quoting is correct for paths without a `'`. It overwrites `<home>/Library/LaunchAgents/<label>.plist` or the two
   systemd units, and nothing else. Over `--host` it uses `ssh user@host` for the mkdir and load and `scp -q` for the
   copy. F1 and F2 are the problems found.
6. **Secrets.** No key is in a unit, the plan or argv (M3 and the tests). The `--launcher` string is only a command, and
   the Mini one is `doppler run --project … --config … --` with no token. Scan:
   `(git diff origin/main...HEAD; gh pr view 10 --json body -q .body) | grep -nEi 'sk-[a-z0-9]{8,}|ghp_|gho_|dp\.(st|pt|ct)\.|xox[bp]-|AKIA[0-9A-Z]{12}|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[a-z]{2,}|token|password'`.
   It found only false positives: `@pytest.mark.parametrize` (×2), the brief's own sentence about "a token or an
   email", and the test regex `(KEY|TOKEN|SECRET|NUSY_GLM)=` (×2). No real key, token or email is present.
7. **DESIGN.md §5.** It is a one-line pointer fix. `packaging/launchd/com.congruentsys.quotabus-probe.plist` becomes
   `packaging/install.sh renders com.congruentsys.quotabus-probe.plist`. The rest of the line is unchanged (§10 Q2,
   "the fleet's is Mini"), so the meaning does not change. See nit F6.
8. **The gate.** `CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-008-target make check` →
   `make check rc=0`. It ran `59 passed` (pytest), every `cargo test` group `ok` and `yurtle-kanban validate`.

## Findings

**F1 — should-fix — the macOS install over `--host` uses the legacy `launchctl load`, whose target domain depends on the session it runs in.**
Command: `grep -n 'load_cmd=' packaging/install.sh`. Output:
`224:    load_cmd="launchctl unload '$dest_dir/$PLIST_NAME' 2>/dev/null || true; launchctl load -w '$dest_dir/$PLIST_NAME'"`.
For `--host mini`, this string runs via `ssh "$target" "$load_cmd"`, which is the fleet's own install path. Command:
`man launchctl | col -b | sed -n '353,362p;390,402p'`. Output: "LEGACY SUBCOMMANDS — Legacy subcommands select the
target domain based on whether they are executed as root or not … load | unload … Recommended alternative subcommands:
bootstrap | bootout | enable | disable … Relevant sessions are Aqua (the default), Background and LoginWindow …
Aqua agents are loaded only when a user has logged in at the GUI."
An ssh session is not an Aqua session, so loading this agent over ssh lands in whatever domain the legacy rules
choose, or fails. The `|| true` on the `unload` then hides a stale job. I did not run it, because the brief forbids
running launchctl. This finding comes from reading the code and the man page.
Fix: name the domain explicitly, in both the local and the ssh path:
`uid=$(id -u); launchctl bootout gui/$uid/com.congruentsys.quotabus-probe 2>/dev/null || true; launchctl bootstrap gui/$uid '<plist>'`.
Also say in the README that the target user must be logged in at the GUI. Alternatively, set
`LimitLoadToSessionType=Background` and bootstrap into `user/$uid`. Either way, the chosen domain is a decision the
script should make, not the session.

**F2 — should-fix — the Linux install over `--host` does not enable lingering, so the timer stops when the user's last session ends.**
Command: `grep -n 'systemctl --user' packaging/install.sh`. Output:
`227:    load_cmd="systemctl --user daemon-reload && systemctl --user enable --now $TIMER_NAME"`.
A `systemctl --user` timer runs only while that user's systemd manager runs. Without `loginctl enable-linger <user>`,
the manager stops when the ssh session that installed the timer closes. A headless `--host` target such as DGX1/2
then never probes. I read this; I did not run it, because there is no Linux host here and the brief forbids
running systemctl.
Fix: add `loginctl enable-linger "$user"` to the Linux `load_cmd`, or at least check
`loginctl show-user "$user" -p Linger` and print a warning in the plan.

**F3 — nit — `%` and `$` in a path reach `ExecStart=` unescaped.**
Command: `bash packaging/install.sh --where local --os linux --home /home/a --quotabus '/opt/100%/$HOME/qb' --render-to /tmp/rv-CHORE-008-lin >/dev/null; grep ExecStart …`.
Output: `ExecStart=/opt/100%/$HOME/qb probe --config /usr/local/etc/quotabus/quotabus.toml`. systemd expands `%`
specifiers and `$VAR` in ExecStart. Fix: in `systemd_quote`, write `%` as `%%` and `$` as `$$`.

**F4 — nit — `--help` prints the line `set -eu`.**
Command: `bash packaging/install.sh --help | tail -3`. Output: `# Portable bash (3.2+).` / `set -eu` / (blank).
Fix: `sed -n '2,20p'`, or `sed -n '2,/^set -eu/{/^#/p;}'`.

**F5 — nit — the plan echoes `--launcher` verbatim, and the unit stores it.**
`echo "launcher: $launcher"`. A user who writes `doppler run --token …` puts a token into the unit and into stdout. The
script cannot prevent every such leak, but it can refuse launcher words that look like credentials (`--token`, `*=*`
holding KEY/TOKEN/SECRET). Fix: refuse those words, or document that the launcher must not carry a credential.

**F6 — nit — DESIGN §5 now lists `…-agent.plist` (M5, Air) with no directory.**
`packaging/launchd/` no longer exists on this branch: the rename moved its only file. The `…` used to inherit
`packaging/launchd/` from the probe plist's path. This is not a change of meaning. Fix (optional):
`packaging/launchd/…-agent.plist` (future).
