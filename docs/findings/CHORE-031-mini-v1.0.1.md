# CHORE-031: v1.0.1 released, and Mini's central probe redeployed from its tag

**Question.** v1.0.1 releases CHORE-030's `status` fix, and Mini is rebuilt from the `v1.0.1` tag with no other
change. Does Mini's probe run clean under its regular launchd tick? Do its rows read `observed_by …/quotabus@1.0.1`?
Does `status` now count "(resets in …)" from now?

**Answer.** Yes, to all three.
- **Version:** `quotabus --version` on Mini prints `quotabus 1.0.1`.
- **Regular tick:** tick #751 (17:52 UTC) exited 0 and printed `alert: 15 keys checked, 2 local skipped, 0 crossings
  filed, 0 re-armed`.
- **Rows:** fresh rows carry `"observed_by":"<mini>/quotabus@1.0.1"`, and every row's state is unchanged from
  CHORE-029.
- **The fix, live:** a Claude window whose stored `reset_in_s` is 111 min shows `(resets in 107m)` at age about 4 min.
  107 min is `reset_at − now`.

Authorship: the release and the redeploy were run by an LLM agent session (Claude Opus 5.5), `<mini>/s-e7976c42`,
on Mini itself. A sub-agent wrote the release commit (version, CHANGELOG, README). That commit was reviewed by a
distinct `claude -p` session (PR #40, `reviews/release-v1.0.1-r1.md`). No human reviewed it.
- **The request:** the driver session offered to "cut a v1.0.1 release with a CHANGELOG entry and then redeploy Mini
  from that tag, the same way as CHORE-029". Captain 2026-10-11: "yes". That is the default the agent offered.
- **This finding:** reviewed by a distinct `claude -p` process with a fresh context that did none of this work. It
  is the same model family, briefed by this session with the committed brief.

- **Host:** `<mini>` (user `<user>`, launchd domain `gui/501`), arm64, macOS 27.0.1.
- **Date:** 2026-10-11.
- **Release:** PR #40, merged at `cdc656f994ae8cdb6985856b792184d0c4c45b74` (reviewed-at-sha `ad45efb`: approve).
  - Annotated tag `v1.0.1` is tag object `bb9a529771fbab40cd8f8814d17789e861e4fd18`, pointing to that commit.
  - GitHub release: https://github.com/Congruentsys/quotabus/releases/tag/v1.0.1.
  - No crates.io publish (`publish = false`).
- **This repo's `origin/main` when written:** `9a5362a`.
- **Bus:** `nats://<bus-host>:4222`, bucket `ai_status`. The only writes were the probe's own rows. Nothing was
  purged or deleted.
- **Raw artefacts on Mini:** `~/.local/state/quotabus-work/CHORE-031/`, holding `01-tag.txt` … `08-resets-from-now.txt`,
  plus the one-shot plist and its logs.

## Commands and output (redacted: no key appears anywhere below; account names are the operator's config labels)

**1. The tag, read from origin.**

```
$ git -C ~/Projects/quotabus fetch -q origin --tags; git rev-parse v1.0.1 'v1.0.1^{commit}'; git ls-remote origin refs/tags/v1.0.1
bb9a529771fbab40cd8f8814d17789e861e4fd18
cdc656f994ae8cdb6985856b792184d0c4c45b74
bb9a529771fbab40cd8f8814d17789e861e4fd18	refs/tags/v1.0.1
```

**2. Build.** It ran as `git archive v1.0.1 | tar -x -C /tmp/qb-build-CHORE-031/src`. That export kept the build out of
the shared checkout and registered no worktree.

```
$ cd /tmp/qb-build-CHORE-031/src && CARGO_TARGET_DIR=/tmp/qb-build-CHORE-031/target ~/.cargo/bin/cargo build --release --locked -q
build rc=0
$ /tmp/qb-build-CHORE-031/target/release/quotabus --version
quotabus 1.0.1
```

Before the install, the new binary ran a read-only `quotabus status --config ~/.config/quotabus/quotabus.toml`, which
returned rc 0. The config was not changed.

**3. Backup and install** (2026-10-11T17:48:35Z).

```
$ cp -p ~/.local/bin/quotabus ~/.local/bin/quotabus.prev-v1.0.0
$ ~/.local/bin/quotabus.prev-v1.0.0 --version
quotabus 1.0.0
$ install -m 755 /tmp/qb-build-CHORE-031/target/release/quotabus ~/.local/bin/quotabus.new && mv ~/.local/bin/quotabus.new ~/.local/bin/quotabus
$ ~/.local/bin/quotabus --version
quotabus 1.0.1
```

CHORE-029's backup, `~/.local/bin/quotabus.prev-20261011` (0.1.0), is still there.

**4. One forced cycle, exactly as the unit runs.** It ran as a one-shot LaunchAgent, `com.congruentsys.quotabus-chore031`,
in `gui/501`. Its ProgramArguments, read with `plutil -extract ProgramArguments json`:
`["/opt/homebrew/bin/doppler","run","--project","nusy-product-team","--config","dev","--","/Users/<user>/.local/bin/quotabus-cycle","probe","--force","--config","/Users/<user>/.config/quotabus/quotabus.toml"]`.
doppler is the program launchd starts, so it holds the Local Network grant (HAZ-005). The job was booted out
afterwards.

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

These are the same 19 keys and states as CHORE-029's step 4. A grep of both logs for `bearer|sk-|api[_-]?key=` counted
0 matches.

**5. `observed_by` on the bus** (natscli, read-only).

```
$ nats --server nats://<bus-host>:4222 kv get ai_status api.deepseek.nusy-product-team.deepseek-v4-flash --raw | jq -c '{key,state,observed_by,checked_at,windows:[.headroom.windows[]?|{window,reset_at,reset_in_s}]}'
{"key":"api.deepseek.nusy-product-team.deepseek-v4-flash","state":"ok","observed_by":"<mini>/quotabus@1.0.1","checked_at":"2026-10-11T17:48:36.351136Z","windows":[]}
$ nats --server nats://<bus-host>:4222 kv get ai_status subscription.anthropic.hankh95.claude-hankh95 --raw | jq -c '{key,state,observed_by,checked_at,windows:[.headroom.windows[]?|{window,reset_at,reset_in_s}]}'
{"key":"subscription.anthropic.hankh95.claude-hankh95","state":"ok","observed_by":"<mini>/quotabus@1.0.1","checked_at":"2026-10-11T17:48:36.351136Z","windows":[{"window":"five_hour","reset_at":"2026-10-11T21:20:00Z","reset_in_s":12683},{"window":"seven_day","reset_at":"2026-10-15T08:00:00Z","reset_in_s":310283}]}
```

**6. The next REGULAR tick.** It was waited for by polling `~/Library/Logs/quotabus/probe.out.log` until a new
`alert:` line appeared.

```
2026-10-11T17:52:17Z
$ tail -3 ~/Library/Logs/quotabus/probe.out.log
nothing due
published 0 rows to bucket ai_status
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
$ launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E "runs =|last exit code"
	runs = 751
	last exit code = 0
```

"nothing due" is expected, because the forced cycle four minutes earlier refreshed every kind. The tick still ran the
new binary end to end, probe then alert, and exited 0.

**7. Status and select on Mini.**

```
2026-10-11T17:52:22Z
$ quotabus status --config /Users/<user>/.config/quotabus/quotabus.toml
SERVICE                  MODEL                                    STATE            AGE  SOURCE        DETAIL
glm                      glm-5.3                                  ok               3m   official
glm                      glm-5.2                                  ok               3m   official
deepseek                 deepseek-v4-pro                          ok               3m   official      balance 245.17 CNY
deepseek                 deepseek-v4-flash                        ok               3m   official      balance 245.17 CNY
kimi                     kimi-k3                                  quota_exhausted  3m   official      balance 17.41715 USD
openai                   gpt-5.6-sol                              auth_failed      3m   official
together                 meta-llama/Llama-3.3-70B-Instruct-Turbo  auth_failed      3m   official
xai                      grok-4                                   auth_failed      3m   official
dgx1-qwen                nvidia/Qwen3-32B-NVFP4                   ok               3m   official
dgx2-qwen                dgx2-qwen                                CANNOT-ASSESS    3m   official      cannot_assess:timeout
claude-<acct-1>         claude-<acct-1>                         ok               3m   undocumented  five_hour 1% (resets in 107m) seven_day 1% (resets in 5d)
claude-<acct-2>           claude-<acct-2>                           ok               3m   undocumented  five_hour 0% (resets in 4h) seven_day 0% (resets in 45h)
claude-<acct-3>         claude-<acct-3>                         ok               3m   undocumented  five_hour 18% (resets in 87m) seven_day 66% (resets in 2d)
claude-hankh95           claude-hankh95                           ok               3m   undocumented  five_hour 1% (resets in 3h) seven_day 37% (resets in 3d)
claude-<acct-4>          claude-<acct-4>                          ok               3m   undocumented  five_hour 1% (resets in 107m) seven_day 9% (resets in 4d)
claude-<acct-5>  claude-<acct-5>                  ok               3m   undocumented  five_hour 1% (resets in 107m) seven_day 25% (resets in 2d)
copilot                  copilot                                  ok               3m   undocumented  monthly 0% (resets in 20d)
rc=0
$ quotabus select --role review --exclude-family anthropic --config /Users/<user>/.config/quotabus/quotabus.toml
deepseek deepseek-v4-flash
rc=0
```

Every configured row is present:
- 8 API rows (the 2 balance keys are folded into DETAIL);
- 2 local rows;
- 7 subscription rows.

**8. "(resets in …)" counts from now (CHORE-030, live).** Two five-hour windows were read from the bus. jq computed
both the time left from now and the stored `reset_in_s` in minutes, and the result was set against the `status` line:

```
now=2026-10-11T17:52:30Z
$ nats --server nats://<bus-host>:4222 kv get ai_status subscription.anthropic.<acct-1>.claude-<acct-1> --raw | jq -c --arg now "$NOW" '{checked_at, five_hour: (.headroom.windows[]|select(.window=="five_hour")|{reset_at,reset_in_s, from_now_min: (((.reset_at|fromdateiso8601) - ($now|fromdateiso8601))/60|floor), stored_min: (.reset_in_s/60|floor)})}'
{"checked_at":"2026-10-11T17:48:36.351136Z","five_hour":{"reset_at":"2026-10-11T19:40:00Z","reset_in_s":6683,"from_now_min":107,"stored_min":111}}
$ (the same, for subscription.anthropic.<acct-3>.claude-<acct-3>)
{"checked_at":"2026-10-11T17:48:36.351136Z","five_hour":{"reset_at":"2026-10-11T19:20:00Z","reset_in_s":5483,"from_now_min":87,"stored_min":91}}
$ quotabus status --config ~/.config/quotabus/quotabus.toml | grep -E "claude-<acct-1>|claude-<acct-3>"
claude-<acct-1>         claude-<acct-1>                         ok               3m   undocumented  five_hour 1% (resets in 107m) seven_day 1% (resets in 5d)
claude-<acct-3>         claude-<acct-3>                         ok               3m   undocumented  five_hour 18% (resets in 87m) seven_day 66% (resets in 2d)
```

`status` prints 107m and 87m, the times left from now. v1.0.0 would have printed the stored 111m and 91m, which is
what CHORE-029's reviewer saw stay fixed as the row aged. The stored row still carries `reset_in_s` counted from
`checked_at`, unchanged (DESIGN §2).

(The jq command recorded in the raw artefact `08-resets-from-now.txt` echoes a shorter filter than the one that ran.
The filter quoted above is the one that produced the output.)

## What this changes

Nothing in the design. The fleet's central probe runs the released v1.0.1, and its rows say so.

To roll back on Mini (the config was not touched, so only the binary is restored):
```
mv ~/.local/bin/quotabus.prev-v1.0.0 ~/.local/bin/quotabus
```
