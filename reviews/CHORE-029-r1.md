reviewed-at-sha: befedc0793324fcebeb43dead514ece2031890a0
verdict: approve
gate: make check rc=0 at befedc0793324fcebeb43dead514ece2031890a0 on Mac-mini
brief: reviews/CHORE-029-r1.brief.md @ befedc0793324fcebeb43dead514ece2031890a0

Authorship: all the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
"Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model
family as the author, briefed by the driver (the author) with the committed brief named above. The commissioning
session is Mac-mini/s-e7976c42. Its stake: it ran the redeploy and wrote the finding. This review ran on Mini
(Mac-mini, macOS 27.0.1) on 2026-10-11, 15:00–15:04 UTC, in a detached scratch worktree at the SHA above.

## Definition of Done, line by line

| DoD line | Where in the finding | Holds? |
|---|---|---|
| Findings file, PR, distinct reviewer re-running the read-only commands | `docs/findings/CHORE-029-mini-v1.0.0.md`, PR #38, this review | yes |
| Commands + redacted output, host, date, tag sha | header + steps 1–7; tag b153526… → ab14fe1… | yes (F1) |
| `quotabus --version` prints `quotabus 1.0.0` | step 3 | yes (F2) |
| One regular launchd tick after install, exit 0, alert line | step 6, tick #716 | yes (F4; #717 since, also 0) |
| `observed_by` ending `quotabus@1.0.0` on a fresh key | step 5 | yes (F5) |
| `status` with every API, local and subscription row; `select … --exclude-family anthropic` rc 0/3 | step 7 | yes (F6) |
| Rollback restores `quotabus.prev-20261011` | "What this changes" | yes (F7) |
| No secret; only bus writes are quotabus's rows in `ai_status` | step 4 grep, bus line | yes (F8); bus writes not independently checkable read-only, no contrary evidence |

## Findings

**F1 — tag sha matches origin. Does not block.**
```
$ git -C /Users/hankh19/Projects/quotabus rev-parse v1.0.0 'v1.0.0^{commit}'
b1535264e81ec34c685d2075979e1b3185a6f90c
ab14fe17e5cb09be1ee54632f50673aecac43ab5
$ git -C /Users/hankh19/Projects/quotabus ls-remote origin refs/tags/v1.0.0
b1535264e81ec34c685d2075979e1b3185a6f90c	refs/tags/v1.0.0
```
Identical to the finding's step 1 and to `01-tag.txt` / `01b-tag-origin.txt`.

**F2 — installed binary, backup and versions. Does not block.**
```
$ ~/.local/bin/quotabus --version
quotabus 1.0.0
$ ~/.local/bin/quotabus.prev-20261011 --version
quotabus 0.1.0
$ ls -la ~/.local/bin/quotabus*
-rwxr-xr-x@ 1 hankh19  staff  12193376 Oct 11 14:52 /Users/hankh19/.local/bin/quotabus
-rwxr-xr-x@ 1 hankh19  staff       816 Oct 10 15:26 /Users/hankh19/.local/bin/quotabus-cycle
-rwxr-xr-x@ 1 hankh19  staff       444 Oct  9 03:12 /Users/hankh19/.local/bin/quotabus-cycle.prev-20261010
-rwxr-xr-x@ 1 hankh19  staff  12184272 Oct  9 03:12 /Users/hankh19/.local/bin/quotabus.prev-20261009
-rwxr-xr-x@ 1 hankh19  staff  12202960 Oct  9 18:14 /Users/hankh19/.local/bin/quotabus.prev-20261010
-rwxr-xr-x@ 1 hankh19  staff  12193104 Oct 10 20:28 /Users/hankh19/.local/bin/quotabus.prev-20261011
```
The installed mtime (14:52) matches the install time in step 3. The backup's mtime is 2026-10-10 20:28, which is
CHORE-027's install time, as `cp -p` preserves it. So the backup is CHORE-027's deeb435 build.

**F3 — the installed binary IS the tag's build, byte for byte. Does not block.**
```
$ git -C /Users/hankh19/Projects/quotabus archive v1.0.0 | tar -x -C /tmp/rv-CHORE-029-build/src
$ cd /tmp/rv-CHORE-029-build/src && CARGO_TARGET_DIR=/tmp/rv-CHORE-029-build/target ~/.cargo/bin/cargo build --release --locked -q
build rc=0
$ /tmp/rv-CHORE-029-build/target/release/quotabus --version
quotabus 1.0.0
$ shasum -a 256 /tmp/rv-CHORE-029-build/target/release/quotabus ~/.local/bin/quotabus
2a051d6a83456088b0eadd148bcffdd42c5f9077632b719915280a5123d96877  /tmp/rv-CHORE-029-build/target/release/quotabus
2a051d6a83456088b0eadd148bcffdd42c5f9077632b719915280a5123d96877  /Users/hankh19/.local/bin/quotabus
```
A match was not guaranteed: the target dir path differed (`/tmp/rv-CHORE-029-build` vs `/tmp/qb-build-CHORE-029`),
and that can change a binary. It matched anyway, on the same host and toolchain, so this is stronger than a
`--version` match. `/tmp/rv-CHORE-029-build` was removed afterwards.

**F4 — the regular launchd tick. Does not block.**
```
$ grep '^alert:' ~/Library/Logs/quotabus/probe.out.log | tail -3
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
$ launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E 'runs =|last exit code|program'
	program = /opt/homebrew/bin/doppler
	runs = 716
	last exit code = 0
	properties = runatload | inferred program
```
That was at 15:00:24Z, and it matches `06-regular-tick.txt`. Re-read at 15:03:27Z, a newer tick had run:
```
	runs = 717
	last exit code = 0
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
```
`program = /opt/homebrew/bin/doppler` confirms the HAZ-005 shape that the finding claims.

**F5 — `observed_by` on the bus. Does not block.**
```
$ nats --server nats://192.168.8.110:4222 kv get ai_status api.deepseek.nusy-product-team.deepseek-v4-flash --raw | jq -c '{key,state,observed_by,checked_at,ttl_s}'
{"key":"api.deepseek.nusy-product-team.deepseek-v4-flash","state":"ok","observed_by":"Mac-mini/quotabus@1.0.0","checked_at":"2026-10-11T14:52:08.609242Z","ttl_s":129600}
$ nats --server nats://192.168.8.110:4222 kv get ai_status local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 --raw | jq -c '{key,state,observed_by,checked_at,ttl_s}'
{"key":"local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4","state":"ok","observed_by":"Mac-mini/quotabus@1.0.0","checked_at":"2026-10-11T14:52:08.609242Z","ttl_s":129600}
```
Byte-identical to step 5 and `05-observed-by.txt`. The `checked_at` is still the forced cycle's: ticks #716 and #717
had nothing due.

**F6 — status and select. Does not block.**
```
$ ~/.local/bin/quotabus status --config ~/.config/quotabus/quotabus.toml
(17 rows: identical to the finding's step 7 in SERVICE, MODEL, STATE, SOURCE and DETAIL; AGE now 8m, was 4m)
rc=0
$ ~/.local/bin/quotabus select --role review --exclude-family anthropic --config ~/.config/quotabus/quotabus.toml
deepseek deepseek-v4-flash
rc=0
```
No row changed state since #716. There are 8 API rows, 2 local and 7 subscription, as the finding counts. The states
match CHORE-027 step 6: glm, deepseek, Claude ×6 and Copilot `ok`; kimi `quota_exhausted`; openai, together, xai
`auth_failed`; dgx2 `cannot_assess:timeout`. The forced cycle's 19 state lines (`forced.out.log`, state lines
only) also match step 4 line for line. The state files `01`…`07` match the finding's quoted blocks.

Side note, outside this PR's scope: the subscription DETAIL "resets in …" strings are unchanged between AGE 4m and
8m (e.g. `resets in 87m`). They appear to be rendered relative to `checked_at`, not to now. That is old behaviour,
not something this redeploy changed.

**F7 — config untouched, and the rollback. Does not block.**
```
$ stat -f '%Sm' -t '%FT%T%z' ~/.config/quotabus/quotabus.toml
2026-10-10T20:28:35+0000
```
That predates the 2026-10-11T14:52Z install and is CHORE-027's config install time, so the config was untouched. The
rollback `mv ~/.local/bin/quotabus.prev-20261011 ~/.local/bin/quotabus` restores the 0.1.0 binary (F2), and the
regular tick picks it up with no launchd change, because the unit's ProgramArguments name the path. The `mv`
consumes the backup, which is acceptable for a one-shot rollback. Not run, as it is not read-only.

**F8 — secrets. Does not block.**
```
$ grep -ciE 'bearer|sk-|api[_-]?key=' ~/.local/state/quotabus-work/CHORE-029/forced.out.log
0
$ grep -cE -i 'bearer|sk-[A-Za-z0-9]|api[_-]?key=|authorization:|x-api-key|@[a-z0-9.-]+\.(com|org|net|io)' <finding> <brief> <PR #38 body>
PR body:0  finding:1  brief:1
```
The finding's one hit is line 99, the text of the grep pattern itself, not a key. The brief's one hit is the GitHub
noreply commit identity, not a personal email. No key, auth header or email appears in the finding, the brief or the
PR body. The subscription account names (claude-hankh1844, -hankh19, -hankh1995, -hankh95, -hankxu95,
-hanssantiago1995, and `subscription.github.hankh95.copilot`) all already appear in
`docs/findings/CHORE-016-mini-deploy.md`, with counts 5/8/4/4/4/4 and 3. So the "operator's config labels, already
published" claim holds. The balances (245.17 CNY, 17.41737 USD) are not secrets, and CHORE-027 published the same
fields. `forced.err.log` was not read.

**F9 — gate. Does not block.**
```
$ CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-029-target make check
make check rc=0   (107 script tests passed; yurtle-kanban validate: All work items valid; all cargo test suites ok)
```
`/tmp/rv-CHORE-029-target` was removed afterwards.

## Verdict
approve. Every DoD line is met, every re-run read-only command reproduces the finding's output (allowing for newer
ticks), and the installed binary is byte-identical to a fresh `--locked` build of `v1.0.0`. No finding blocks.
