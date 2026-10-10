# CHORE-027: redeploy of Mini's central probe from main

**Question.** Does Mini's central probe, rebuilt from `origin/main` with the DGX1/DGX2 discovery config, run clean
under its regular launchd tick and publish CHORE-021, 022 and 024's behaviour?

**Answer.** Yes. The binary is built from `deeb435`. Regular tick #496 (about 20:32 UTC) exited 0 and printed
`alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed`. DGX1 reads `ok` under the model id it
serves. DGX2 reads `cannot_assess:timeout`, its real state. Every API and subscription row is present and fresh.

Authorship: the redeploy and this finding were done by an LLM agent session (Claude Opus 5.5),
M5-MBP-2/s-72a67d16, over ssh from M5 to Mini, with no sub-agent. No human reviewed it. The Captain asked for the
chore (2026-10-10: "create a chore for mini to redeploy it"), then said "then continue in /quotabus-loop".

- **Host:** Mac-mini (user `hankh19`, launchd domain `gui/501`), arm64.
- **Date:** 2026-10-10.
- **quotabus:** built from `deeb435fbba833cb09604d3783b9739c4ed680b4` (`origin/main` at build time), version string
  `quotabus 0.1.0`.
- **Bus:** `nats://192.168.8.110:4222`, bucket `ai_status`. Only this repo's own bucket was written, by the probe
  itself. Nothing was purged or deleted.

## Commands and output (redacted: no key appears anywhere below)

**1. Build.** The source was a `git archive origin/main` export into `/tmp/qb-build-CHORE-027/src`. That kept the
build out of Mini's shared checkout and registered no worktree.

```
$ git -C ~/Projects/quotabus fetch -q origin; git -C ~/Projects/quotabus rev-parse origin/main
deeb435fbba833cb09604d3783b9739c4ed680b4
$ cd /tmp/qb-build-CHORE-027/src && CARGO_TARGET_DIR=/tmp/qb-build-CHORE-027/target ~/.cargo/bin/cargo build --release --locked -q
$ /tmp/qb-build-CHORE-027/target/release/quotabus --version
quotabus 0.1.0
```

**2. Backups.** Each is kept with a date suffix:
- `~/.local/bin/quotabus.prev-20261010`: the 2026-10-09 18:14 binary.
- `~/.config/quotabus/quotabus.toml.prev-20261010`: the previous config.

**3. Config.** The fixed-`models` `local-qwen` service was replaced by `dgx1-qwen` (`http://192.168.8.120:8000/v1`)
and `dgx2-qwen` (`http://192.168.8.121:8000/v1`). Both are `kind = "local"`, with no `secret` and no `models`, as in
`examples/quotabus.toml`. `diff` showed only the local-service block changed. The new binary ran a read-only
`quotabus status --config <new>` and got rc 0 before the config was installed.

**4. One forced cycle, exactly as the unit runs.** It ran as a one-shot LaunchAgent in `gui/501` with
ProgramArguments `/opt/homebrew/bin/doppler run --project nusy-product-team --config dev -- ~/.local/bin/quotabus-cycle
probe --force --config ~/.config/quotabus/quotabus.toml`. That is the HAZ-005 shape: doppler is the program
launchd starts, so it is the one that holds the Local Network grant. It was booted out afterwards. Its artefacts are on
Mini in `~/.local/state/quotabus-work/CHORE-027/`. Its last exit code was 0 and it wrote:

```
local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 ok
local.qwen.dgx2.dgx2-qwen unknown (cannot_assess:timeout)
… (10 api rows, 7 subscription rows; states as in step 6)
published 19 rows to bucket ai_status
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
```

The binary was replaced under the granted doppler launcher, and DGX1 still answered. This confirms HAZ-005 case 2:
the grant is not reset by a new quotabus binary.

**5. The next REGULAR tick.** The script polled `~/Library/Logs/quotabus/probe.out.log` until a new `alert:` line
appeared, then read `launchctl print gui/501/com.congruentsys.quotabus-probe`:

```
2026-10-10T20:32:48Z
alert: 16 keys checked, 0 crossings filed, 0 re-armed                       <- the last tick on the old binary
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed      <- the first regular tick on deeb435
	runs = 496
	last exit code = 0
```

M = 2: the two discovering local services, skipped as CHORE-021 rules. N fell from 16 to 15 because local slots no
longer count as checked (CHORE-024).

**6. Status and select on Mini** (`quotabus status|select --config ~/.config/quotabus/quotabus.toml`):

```
dgx1-qwen   nvidia/Qwen3-32B-NVFP4  ok             4m  official
dgx2-qwen   dgx2-qwen               CANNOT-ASSESS  4m  official  cannot_assess:timeout
$ quotabus select --role work --prefer cheapest
qwen nvidia/Qwen3-32B-NVFP4
```

The 8 API rows and 7 subscription rows are unchanged:
- glm ×2, deepseek ×2, and all six Claude accounts and Copilot read `ok`.
- kimi reads `quota_exhausted`, because its balance is below the floor.
- openai, together and the xai control read `auth_failed`.

They are as fresh as their own `[intervals]`: API 12 h, subscription 1 h.

**7. Old local rows.** Measured from M5 with natscli; each row's expiry is `checked_at + ttl_s`:

```
local.qwen.dgx1.qwen3                    unknown cannot_assess:secret_unset 2026-10-09T18:19:29 ttl_s=129600 left=9.8h
local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4   ok                                 2026-10-10T20:28:50 ttl_s=129600 left=35.9h
local.qwen.dgx2.dgx2-qwen                unknown cannot_assess:timeout      2026-10-10T20:28:50 ttl_s=129600 left=35.9h
```

- **A correction to the item body.** The item listed `local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4` as an old row to age
  out. It is not: discovery keys DGX1's row by the id it serves, so this is now the LIVE DGX1 key, refreshed by the
  new config.
- **The one stale row** is `local.qwen.dgx1.qwen3`, the 2026-10-09 `secret_unset` row from the retired config. Per-key
  TTL removes it in about 9.8 h, around 2026-10-11T06:19Z.
- **It does no harm meanwhile.** It is `unknown`, never `ok`, and no configured service lists the slot, so `status`
  and `select` do not read it. Nothing was purged (CLAUDE.md rule 4).

## What this changes

Nothing in the design. CHORE-021, CHORE-022 and CHORE-024 are now live on the fleet's central probe. DGX2 not
answering on `:8000` is the box's own state, not a probe fault: the same was seen from M5 at 13:50 UTC (CHORE-022's
real-path run).

To roll back on Mini:
```
mv ~/.local/bin/quotabus.prev-20261010 ~/.local/bin/quotabus
mv ~/.config/quotabus/quotabus.toml.prev-20261010 ~/.config/quotabus/quotabus.toml
```
