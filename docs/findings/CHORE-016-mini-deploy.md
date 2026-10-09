# CHORE-016: the central probe deployed on Mini, and VOY-001's Definition of Done on the live bus

**Question.** With the unit installed on Mini as `packaging/README.md` § The fleet: Mini says, does the live bus
meet VOY-001's Definition of Done?

> `quotabus status` on the hub shows every configured API key and every host's subscription state with ages; a stale
> or missing row reads unknown; `quotabus select --role review --exclude-family anthropic` returns a healthy model or
> refuses with rc 3; an outage files exactly one alert.

**Answer: yes.**
- Mini's launchd unit published 18 rows over two ticks, and `status` lists all 16 configured (service, model) rows
  with their ages.
- A missing row reads UNKNOWN: the empty-bus run before the first tick shows all 16 as `UNKNOWN … absent`. A stale
  row was not produced live here; the freshness tests cover it (`tests/freshness_rule.rs`).
- `select` returned `deepseek deepseek-v4-flash` (rc 0). With every family excluded it refused with rc 3.
- Four bad services crossed on tick 1 and filed four alerts. Tick 2 filed 0.

The run also found three bad keys and one low balance. They are listed under § What it changes.

- **Host:** Mini (`Mac-mini.lan`, macOS 27.0.1, arm64), user `hankh19`. Driven from M5 (`M5-MBP-2`) over `ssh mini`.
- **Date:** 2026-10-09, ticks at 03:12 and 03:17 UTC.
- **quotabus:** built on Mini at `a4cf26e` (origin/main) in a throwaway worktree `/tmp/qb-deploy` of Mini's
  checkout, `cargo build --release`. It reports `quotabus 0.1.0`.
- **Tools:** nats-server on Mini (bus `nats://192.168.8.110:4222`, read as `127.0.0.1:4222` on Mini), natscli
  `/opt/homebrew/bin/nats`, Doppler 3.75.2.
- **Raw artefacts:** `~/.local/state/quotabus-work/CHORE-016/` on M5: `probe.out.log`, `probe.err.log`,
  `status-after-tick2.txt`, `quotabus.toml`, `quotabus-cycle`.

**Authorship.** All of this work was done by an LLM agent session, Claude Opus 5.5: the deployment, the
measurements and this finding. The session is `M5-MBP-2/s-72a67d16`, and it wrote this file. No human reviewed it.
The Captain was GUI-logged-in on Mini, which the unit needs, and said "go ahead, ssh from here" (2026-10-09). The
review of this finding is by a distinct `claude -p` session (`reviews/CHORE-016-r1.md`).

## 1. What was installed

```sh
# build (on Mini)
ssh mini 'cd ~/Projects/quotabus; git fetch -q origin; git worktree add -q --detach /tmp/qb-deploy origin/main;
  cd /tmp/qb-deploy; CARGO_TARGET_DIR=/tmp/qb-target-deploy ~/.cargo/bin/cargo build --release -q;
  /tmp/qb-target-deploy/release/quotabus --version'
# → quotabus 0.1.0
# place binary, wrapper, config (the wrapper is the README's block, extracted from the README, not retyped)
ssh mini 'mkdir -p ~/.local/bin ~/.config/quotabus && cp /tmp/qb-target-deploy/release/quotabus ~/.local/bin/quotabus'
scp quotabus-cycle mini:.local/bin/quotabus-cycle; scp quotabus.toml mini:.config/quotabus/quotabus.toml
# the unit: the README's fleet command, with --quotabus pointed at the wrapper as § Alerts says
packaging/install.sh --host mini --user hankh19 --home /Users/hankh19 --os macos \
  --launcher "/opt/homebrew/bin/doppler run --project nusy-product-team --config dev --" \
  --quotabus /Users/hankh19/.local/bin/quotabus-cycle \
  --probe-config /Users/hankh19/.config/quotabus/quotabus.toml --interval 300
# → installed: com.congruentsys.quotabus-probe.plist on mini   (rc 0)
```

**The config's shape.** It is not in git. It holds no secret, only environment-variable NAMES, but it is the fleet's
own file. It was assembled from two sources:
- From `examples/quotabus.toml` at `a4cf26e`: the `[bus]`, `[intervals]` (api 12h, balance 12h, subscription 1h) and
  `[probe]` (`warn_pct = 90`) tables, and the seven API/local services: glm, deepseek, kimi, openai, together, xai
  and local-qwen.
- From EXP-002's fleet subscription config: six Claude accounts (`claude-hankh1844`, `-hankh19`, `-hankh1995`,
  `-hankh95`, `-hankxu95`, `-hanssantiago1995`; sources `unified_headers` then `stream_json`) and `copilot`
  (`copilot_internal`). Undocumented sources are on, for the fleet only (SIG-003).

That makes 14 `[[service]]` tables and 16 (service, model) rows. There is **no `[alert.*]` sink**, so alerts go to
stdout, which is the unit's log, until SIG-010 is ruled and a board sink is chosen.

Before the unit loaded, `status` against the empty bus read every one of the 16 rows `UNKNOWN … absent` (rc 0): a
missing row reads unknown.

**Doppler needs the GUI domain** (HAZ-004). Over a plain `ssh`, `doppler secrets --only-names` fails with "Unable to
retrieve value from system keyring", because the token is in the login keychain. A throwaway job bootstrapped into
`gui/501` counted the 7 subscription token names (count only, no values read), and the job was then removed. The
unit loads into `gui/501`.

## 2. Tick 1 (RunAtLoad, 03:12 UTC): rows and alerts

`~/Library/Logs/quotabus/probe.out.log` on Mini, verbatim (nothing in it needed redacting):

```
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
local.qwen.dgx1.qwen3 unknown (cannot_assess:secret_unset)
subscription.anthropic.hankh1844.claude-hankh1844 ok
subscription.anthropic.hankh19.claude-hankh19 ok
subscription.anthropic.hankh1995.claude-hankh1995 ok
subscription.anthropic.hankh95.claude-hankh95 ok
subscription.anthropic.hankxu95.claude-hankxu95 ok
subscription.anthropic.hanssantiago1995.claude-hanssantiago1995 ok
subscription.github.hankh95.copilot ok
published 18 rows to bucket ai_status
ALERT api.moonshot.nusy-product-team.kimi-k3 quota_exhausted (kimi kimi-k3): provider-status: kimi kimi-k3 is quota_exhausted
ALERT api.openai.nusy-product-team.gpt-5-6-sol auth_failed (openai gpt-5.6-sol): provider-status: openai gpt-5.6-sol is auth_failed
ALERT api.together.nusy-product-team.meta-llama-Llama-3-3-70B-Instruct-Turbo auth_failed (together meta-llama/Llama-3.3-70B-Instruct-Turbo): provider-status: together meta-llama/Llama-3.3-70B-Instruct-Turbo is auth_failed
ALERT api.xai.nusy-product-team.grok-4 auth_failed (xai grok-4): provider-status: xai grok-4 is auth_failed
alert: 16 keys checked, 4 crossings filed, 0 re-armed
```

The standing negative control, xai (its key is disabled), reads `auth_failed`, never `ok`. Every subscription read
used its first source (`probe=unified_headers` for Claude, `copilot_internal` for Copilot, per `probe.err.log`), so
the stream-json fallback was not exercised on this run.

## 3. `quotabus status` on the hub (03:12:24 UTC, 12 s after tick 1)

```
$ ssh mini '~/.local/bin/quotabus status --config ~/.config/quotabus/quotabus.toml'; echo rc=$?
SERVICE                  MODEL                                    STATE            AGE  SOURCE        DETAIL
glm                      glm-5.3                                  ok               12s  official
glm                      glm-5.2                                  ok               12s  official
deepseek                 deepseek-v4-pro                          ok               12s  official      balance 245.17 CNY
deepseek                 deepseek-v4-flash                        ok               12s  official      balance 245.17 CNY
kimi                     kimi-k3                                  quota_exhausted  12s  official      balance 17.41869 USD
openai                   gpt-5.6-sol                              auth_failed      12s  official
together                 meta-llama/Llama-3.3-70B-Instruct-Turbo  auth_failed      12s  official
xai                      grok-4                                   auth_failed      12s  official
local-qwen               qwen3                                    CANNOT-ASSESS    12s  official      cannot_assess:secret_unset
claude-hankh1844         claude-hankh1844                         ok               12s  undocumented  five_hour 0% (resets in 4h) seven_day 6% (resets in 22h)
claude-hankh19           claude-hankh19                           ok               12s  undocumented  five_hour 0% (resets in 4h) seven_day 0% (resets in 4d)
claude-hankh1995         claude-hankh1995                         ok               12s  undocumented  five_hour 1% (resets in 4h) seven_day 22% (resets in 4d)
claude-hankh95           claude-hankh95                           ok               12s  undocumented  five_hour 4% (resets in 4h) seven_day 20% (resets in 6d)
claude-hankxu95          claude-hankxu95                          ok               12s  undocumented  five_hour 1% (resets in 4h) seven_day 49% (resets in 5h)
claude-hanssantiago1995  claude-hanssantiago1995                  ok               12s  undocumented  five_hour 5% (resets in 2h) seven_day 20% (resets in 4d)
copilot                  copilot                                  ok               12s  undocumented  monthly 0% (resets in 22d)
rc=0
```

All 16 configured rows are present, each with a state and an age. None is missing.

## 4. `quotabus select` (same minute)

```
$ ssh mini '~/.local/bin/quotabus select --config ~/.config/quotabus/quotabus.toml --role review --exclude-family anthropic'; echo rc=$?
deepseek deepseek-v4-flash
rc=0
$ ssh mini '… select … --role review --exclude-family anthropic --exclude-family zhipu --exclude-family deepseek --exclude-family moonshot --exclude-family openai --exclude-family meta'; echo rc=$?
quotabus: nothing qualifies for role "review" excluding anthropic, zhipu, deepseek, moonshot, openai, meta: no configured model with the role has a fresh ok row
rc=3
```

The second call is the refusal control: once every family is excluded, the selector refuses rc 3 rather than
naming a bad model.

## 5. The bus

```
$ ssh mini '/opt/homebrew/bin/nats --server nats://127.0.0.1:4222 kv ls ai_status' | sort
alert.api.moonshot.nusy-product-team.kimi-k3
alert.api.openai.nusy-product-team.gpt-5-6-sol
alert.api.together.nusy-product-team.meta-llama-Llama-3-3-70B-Instruct-Turbo
alert.api.xai.nusy-product-team.grok-4
api.deepseek.nusy-product-team.balance
api.deepseek.nusy-product-team.deepseek-v4-flash
api.deepseek.nusy-product-team.deepseek-v4-pro
api.moonshot.nusy-product-team.balance
api.moonshot.nusy-product-team.kimi-k3
api.openai.nusy-product-team.gpt-5-6-sol
api.together.nusy-product-team.meta-llama-Llama-3-3-70B-Instruct-Turbo
api.xai.nusy-product-team.grok-4
api.zhipu.nusy-product-team.glm-5-2
api.zhipu.nusy-product-team.glm-5-3
local.qwen.dgx1.qwen3
subscription.anthropic.hankh1844.claude-hankh1844
subscription.anthropic.hankh19.claude-hankh19
subscription.anthropic.hankh1995.claude-hankh1995
subscription.anthropic.hankh95.claude-hankh95
subscription.anthropic.hankxu95.claude-hankxu95
subscription.anthropic.hanssantiago1995.claude-hanssantiago1995
subscription.github.hankh95.copilot
```

That is 22 keys: the 18 status rows and the 4 alert dedup marks. Only the `ai_status` bucket was written. Before
the deployment the bucket held 0 values (`kv info ai_status`, M5, 2026-10-09).

## 6. Tick 2 (03:17 UTC): exactly one alert per crossing

```
$ ssh mini 'launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E "runs|last exit"; tail -3 ~/Library/Logs/quotabus/probe.out.log; grep -c "^ALERT" ~/Library/Logs/quotabus/probe.out.log'
	runs = 2
	last exit code = 0
local.qwen.dgx1.qwen3 unknown (cannot_assess:secret_unset)
published 1 rows to bucket ai_status
alert: 16 keys checked, 0 crossings filed, 0 re-armed
4
```

On tick 2 the four services stayed bad and nothing new was filed, so the total stays at 4 ALERT lines over two
ticks. The per-kind schedule held: no API, balance or subscription call was due 5 minutes after the first. Only the
CANNOT-ASSESS `local-qwen` row was rewritten, and that costs no network call (`secret_unset`).

## 7. Secrets

```
$ ssh mini 'cd ~/Library; grep -cE "sk-[A-Za-z0-9]{8}|sk-ant-|Bearer |ghp_|gho_|github_pat_|x-api-key" \
    Logs/quotabus/probe.out.log Logs/quotabus/probe.err.log LaunchAgents/com.congruentsys.quotabus-probe.plist \
    ~/.config/quotabus/quotabus.toml ~/.local/bin/quotabus-cycle'
Logs/quotabus/probe.out.log:0
Logs/quotabus/probe.err.log:0
LaunchAgents/com.congruentsys.quotabus-probe.plist:0
/Users/hankh19/.config/quotabus/quotabus.toml:0
/Users/hankh19/.local/bin/quotabus-cycle:0
$ ssh mini '/usr/bin/plutil -extract ProgramArguments json -o - ~/Library/LaunchAgents/com.congruentsys.quotabus-probe.plist'
["\/opt\/homebrew\/bin\/doppler","run","--project","nusy-product-team","--config","dev","--","\/Users\/hankh19\/.local\/bin\/quotabus-cycle","probe","--config","\/Users\/hankh19\/.config\/quotabus\/quotabus.toml"]
```

No key appears on argv, because ProgramArguments carries none and keys reach the binary only through Doppler's
environment. None appears in the unit, the logs or the config either. A grep of one subscription row for
`sk-ant-|Bearer|@` matched only the writer field `"Mac-mini/quotabus@0.1.0"`.

## What it changes

- **VOY-001's Definition of Done is met on the live bus**, with alerts on stdout. Filing on a board waits on SIG-010,
  which is open.
- **Findings the fleet should act on.** Each is a provider or credential state, not a quotabus defect:
  - `openai` and `together` read `auth_failed`: their Doppler keys (`OPENAI_API_KEY`, `TOGETHER_API_KEY`) are set,
    but the provider rejects them.
  - `kimi`'s balance is 17.42 USD, below its configured floor of 100, so `kimi-k3` reads `quota_exhausted`.
  - `local-qwen`'s secret `NUSY_LOCAL_QWEN` is not in Doppler `nusy-product-team/dev`, so it reads
    `cannot_assess:secret_unset`.
- **Not exercised:** the stream-json fallback under launchd (`QUOTABUS_CLAUDE_BIN`), because every Claude account's
  direct read succeeded. It will be exercised the first time a direct read loses its headers.
