# CHORE-029: Mini's central probe redeployed from the v1.0.0 tag

**Question.** Mini's central probe is rebuilt from the `v1.0.0` tag and nothing else is changed. Does it run clean
under its regular launchd tick, and do its rows now read `observed_by …/quotabus@1.0.0`?

**Answer.** Yes.
- `quotabus --version` on Mini prints `quotabus 1.0.0`.
- Regular tick #716 (14:56 UTC) exited 0 and printed `alert: 15 keys checked, 2 local skipped, 0 crossings filed,
  0 re-armed`.
- Fresh rows on the bus carry `"observed_by":"<mini>/quotabus@1.0.0"`.
- Every row's state is unchanged from CHORE-027. As the item says, this is a relabel, not a behaviour change.

Authorship: the redeploy and this finding were done by an LLM agent session (Claude Opus 5.5),
`<mini>/s-e7976c42`, running on Mini itself (no ssh), with no sub-agent. No human reviewed it.
- **The request:** the Captain asked for the chore on 2026-10-11 ("yes, file the chore for Mini to redeploy
  v1.0.0"). Session `<m5>/s-72a67d16` filed it, and this session claimed it.
- **The review:** "distinct" means a separate `claude -p` process with a fresh context, which did none of this work.
  It is the same model family, briefed by this session with the committed brief.

- **Host:** `<mini>` (user `<user>`, launchd domain `gui/501`), arm64, macOS 27.0.1 (`sw_vers -productVersion`).
- **Date:** 2026-10-11.
- **quotabus:** the tag `v1.0.0` is annotated tag object `b1535264e81ec34c685d2075979e1b3185a6f90c`, which points to
  commit `ab14fe17e5cb09be1ee54632f50673aecac43ab5` (PR #37's merge). At the time of writing, this repo's `origin/main`
  was `001747a` (board commits only since the tag).
- **Bus:** `nats://<bus-host>:4222`, bucket `ai_status`. The only writes were the probe's own rows. Nothing was
  purged or deleted.
- **Raw artefacts:** on Mini, in `~/.local/state/quotabus-work/CHORE-029/` (`01-tag.txt` … `07-status-select.txt`,
  plus the one-shot plist and its logs).

## Commands and output (redacted: no key appears anywhere below; account names are the operator's config labels)

**1. The tag, read from origin.**

```
$ git -C ~/Projects/quotabus fetch -q origin --tags; git rev-parse v1.0.0 'v1.0.0^{commit}'
b1535264e81ec34c685d2075979e1b3185a6f90c
ab14fe17e5cb09be1ee54632f50673aecac43ab5
$ git ls-remote origin refs/tags/v1.0.0
b1535264e81ec34c685d2075979e1b3185a6f90c	refs/tags/v1.0.0
```

**2. Build.** The source was a `git archive v1.0.0` export into `/tmp/qb-build-CHORE-029/src`. That kept the build out
of the shared checkout and registered no worktree.

```
$ git archive v1.0.0 | tar -x -C /tmp/qb-build-CHORE-029/src
$ cd /tmp/qb-build-CHORE-029/src && CARGO_TARGET_DIR=/tmp/qb-build-CHORE-029/target ~/.cargo/bin/cargo build --release --locked -q
build rc=0
$ /tmp/qb-build-CHORE-029/target/release/quotabus --version
quotabus 1.0.0
```

Before the install, the new binary ran a read-only `quotabus status --config ~/.config/quotabus/quotabus.toml`, which
returned rc 0. The config was not changed: it is CHORE-027's, with `dgx1-qwen` and `dgx2-qwen`.

**3. Backup and install** (2026-10-11T14:52:00Z).

```
$ cp -p ~/.local/bin/quotabus ~/.local/bin/quotabus.prev-20261011
$ ~/.local/bin/quotabus.prev-20261011 --version
quotabus 0.1.0
$ install -m 755 /tmp/qb-build-CHORE-029/target/release/quotabus ~/.local/bin/quotabus.new && mv ~/.local/bin/quotabus.new ~/.local/bin/quotabus
$ ~/.local/bin/quotabus --version
quotabus 1.0.0
```

**4. One forced cycle, exactly as the unit runs.** It ran as a one-shot LaunchAgent `com.congruentsys.quotabus-chore029`
in `gui/501`, with these ProgramArguments:
`/opt/homebrew/bin/doppler run --project nusy-product-team --config dev -- ~/.local/bin/quotabus-cycle probe --force
--config ~/.config/quotabus/quotabus.toml`. This is HAZ-005's shape: doppler is the program launchd starts, so it is
the one holding the Local Network grant. The job was booted out afterwards.

```
	last exit code = 0
api.zhipu.nusy-product-team.glm-5-3 ok
api.zhipu.nusy-product-team.glm-5-2 ok
api.deepseek.nusy-product-team.deepseek-v4-pro ok
api.deepseek.nusy-product-team.deepseek-v4-flash ok
api.deepseek.nusy-product-team.balance ok
api.moonshot.nusy-product-team.kimi-k3 quota_exhausted
api.moonshot.nusy-product-team.balance quota_exhausted
api.openai.nusy-product-team.gpt-5-6-sol auth_failed
api.together.nusy-product-team.meta-llama-Llama-3-3-70B-Instruct-Turbo auth_failed
api.xai.nusy-product-team.grok-4 auth_failed
local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 ok
local.qwen.dgx2.dgx2-qwen unknown (cannot_assess:timeout)
subscription.anthropic.<acct-1>.claude-<acct-1> ok
subscription.anthropic.<acct-2>.claude-<acct-2> ok
subscription.anthropic.<acct-3>.claude-<acct-3> ok
subscription.anthropic.hankh95.claude-hankh95 ok
subscription.anthropic.<acct-4>.claude-<acct-4> ok
subscription.anthropic.<acct-5>.claude-<acct-5> ok
subscription.github.hankh95.copilot ok
published 19 rows to bucket ai_status
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
```

The same 19 keys and states as CHORE-027's step 4. DGX1 answered under the new binary, so the grant held across a
binary swap again (HAZ-005 case 2). A grep of both logs for `bearer|sk-|api[_-]?key=` counted 0.

**5. `observed_by` on the bus** (natscli, read-only).

```
$ nats --server nats://<bus-host>:4222 kv get ai_status api.deepseek.nusy-product-team.deepseek-v4-flash --raw | jq -c '{key,state,observed_by,checked_at,ttl_s}'
{"key":"api.deepseek.nusy-product-team.deepseek-v4-flash","state":"ok","observed_by":"<mini>/quotabus@1.0.0","checked_at":"2026-10-11T14:52:08.609242Z","ttl_s":129600}
$ nats --server nats://<bus-host>:4222 kv get ai_status local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 --raw | jq -c '{key,state,observed_by,checked_at,ttl_s}'
{"key":"local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4","state":"ok","observed_by":"<mini>/quotabus@1.0.0","checked_at":"2026-10-11T14:52:08.609242Z","ttl_s":129600}
```

**6. The next REGULAR tick.** It was waited for by polling `~/Library/Logs/quotabus/probe.out.log` until a new
`alert:` line appeared, then reading the unit:

```
2026-10-11T14:56:38Z
$ tail -3 ~/Library/Logs/quotabus/probe.out.log
nothing due
published 0 rows to bucket ai_status
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
$ launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E "runs|last exit code"
	runs = 716
	last exit code = 0
```

"nothing due" is expected: the forced cycle four minutes earlier refreshed every kind. The tick still ran the new
binary end to end (probe, then alert) and exited 0.

**7. Status and select on Mini.**

```
$ quotabus status --config /Users/<user>/.config/quotabus/quotabus.toml
SERVICE                  MODEL                                    STATE            AGE  SOURCE        DETAIL
glm                      glm-5.3                                  ok               4m   official
glm                      glm-5.2                                  ok               4m   official
deepseek                 deepseek-v4-pro                          ok               4m   official      balance 245.17 CNY
deepseek                 deepseek-v4-flash                        ok               4m   official      balance 245.17 CNY
kimi                     kimi-k3                                  quota_exhausted  4m   official      balance 17.41737 USD
openai                   gpt-5.6-sol                              auth_failed      4m   official
together                 meta-llama/Llama-3.3-70B-Instruct-Turbo  auth_failed      4m   official
xai                      grok-4                                   auth_failed      4m   official
dgx1-qwen                nvidia/Qwen3-32B-NVFP4                   ok               4m   official
dgx2-qwen                dgx2-qwen                                CANNOT-ASSESS    4m   official      cannot_assess:timeout
claude-<acct-1>         claude-<acct-1>                         ok               4m   undocumented  five_hour 0% (resets in 4h) seven_day 1% (resets in 5d)
claude-<acct-2>           claude-<acct-2>                           ok               4m   undocumented  five_hour 0% (resets in 4h) seven_day 0% (resets in 2d)
claude-<acct-3>         claude-<acct-3>                         ok               4m   undocumented  five_hour 2% (resets in 4h) seven_day 63% (resets in 2d)
claude-hankh95           claude-hankh95                           ok               4m   undocumented  five_hour 2% (resets in 87m) seven_day 37% (resets in 3d)
claude-<acct-4>          claude-<acct-4>                          ok               4m   undocumented  five_hour 0% (resets in 4h) seven_day 9% (resets in 4d)
claude-<acct-5>  claude-<acct-5>                  ok               4m   undocumented  five_hour 1% (resets in 4h) seven_day 25% (resets in 2d)
copilot                  copilot                                  ok               4m   undocumented  monthly 0% (resets in 20d)
rc=0
$ quotabus select --role review --exclude-family anthropic --config /Users/<user>/.config/quotabus/quotabus.toml
deepseek deepseek-v4-flash
rc=0
```

Every configured row is present: 8 API rows (the 2 balance keys are folded into DETAIL), 2 local rows, and 7
subscription rows.

## What this changes

Nothing in the design. The fleet's central probe now labels its rows `quotabus@1.0.0`, matching the released tag.

The one-off DGX1 check of 2026-10-10 20:25 UTC (HAZ-005, last comment) used the earlier `local-qwen` service. It was
superseded by CHORE-027's `dgx1-qwen` / `dgx2-qwen` config, which this deploy keeps.

To roll back on Mini (the config was not touched, so only the binary is restored):
```
mv ~/.local/bin/quotabus.prev-20261011 ~/.local/bin/quotabus
```
