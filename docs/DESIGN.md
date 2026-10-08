> **Origin.** Written in nusy-product-team as `docs/design/IDEA-13333-PROVIDER-STATUS-MONITOR.md` (commit `27feda15ea`, 2026-10-08) and copied here as the starting design. Paths of the form `path:line` refer to that repository. The Captain's answers to §10 (2026-10-08) are recorded in the kanban items: Q1 `quotabus` under Congruentsys · Q6 no actuation in v1.0 (a later expedition) · Q9 yes (conditional on C2; C2 measured the condition false — reopened as SIG-008, see §10 Q9) · Q11 work filed on this repo's board. The other §10 questions are open as signals.

# IDEA-13333 — Design: an AI provider & account status monitor (standalone FOSS)

**M5, 2026-10-08.** Captain-directed. Inputs: `IDEA-13333` (the Captain's words and the measured problem), `LIT-13334`
(`research/literature/LIT-13334-ai-provider-status-monitor-prior-art.md`, verdict: BUILD a thin core, ADOPT
`secretspec` / the statusline capture / `gh copilot_internal` / the adapter catalogue), and the ecosystem read below.

**Measured at:** monorepo `origin/main` `1064922dc7`; the dead instrument at `f8b7f2f531^:scripts/fleet/provider-balance-scan.sh`
(318 lines); Mini's bus read-only with `nats kv ls` / `kv info` / `kv get` on 2026-10-08; natscli on M5 is **0.3.1**
(`nats --version`). Working name below: **`quotabus`** (§8 — a candidate, not a ruling). Marks: **[unverified]** = not
confirmed at the source this session; **[INFERENCE]** = this document's own conclusion; **[inferred]** = a findings file's reading of measured output, not
documented.

## 1. Goal, non-goals, users

**Goal.** One small daemon-plus-CLI that answers, for every LLM key and coding subscription a team holds, *"does it work
right now, and how much is left?"* — and publishes the answer where agents and humans on several machines can read it with
a freshness rule they cannot skip. The reason it exists is measured: the Copilot subscription ran out and was found only
when a review failed (`docs/external-review.md:30,48`), and the only live record of which models answer is a hand probe
dated 2026-09-24 (`:35-50`).

| | |
|---|---|
| **Users: us** | 5 hosts / 5 Claude logins (`CLAUDE.md` "One session per machine"); M5 as the tmux hub reading everything; Mini as the bus host running the central probe (the Captain's words on the item); the loop skills (`pairit`, external review) choosing a reviewer model |
| **Users: others** | anyone with several LLM keys or coding subscriptions, on one machine or many: small teams, CI runners, home labs, agent fleets; NATS shops that want status on a bus, and non-NATS users who want a table and exit codes (§8) |
| **Non-goals** | not a gateway or proxy (we never route calls through it — LIT §1); not a spend ledger (`ccusage` owns per-host spend; §7 reads its JSON); not an actuator (it never pauses, tops up or switches accounts — §7, §10 Q6); not a Yurtle/RDF engine (it reads one block type, §3); not a dashboard product (the web page is one screen, optional) |

## 2. Data model — the status record and the freshness rule

One record per **(kind, provider, account, model)**. JSON on the bus; the same struct in the library and the CLI.

| field | type | notes |
|---|---|---|
| `contract` | `"ai-status/1"` | versioned like the dead rows (`provider_balance/1.0`, `account-util/1.0`, LIT §6) |
| `key` | string | the KV key, `<kind>.<provider>.<account>.<model>` — slugs only (KV keys exclude `@`, so an email is never a key) [unverified: exact KV key charset] |
| `kind` | `api` \| `subscription` \| `local` | API key · per-host login (Claude Max, Copilot) · self-hosted endpoint (DGX1 Qwen) |
| `provider`, `account`, `model` | string | `account` is the LABEL from config (`nusy-product-team`, `hankh95`), never an email unless the operator chose one |
| `family` | string | `anthropic` \| `openai` \| `zhipu` \| `deepseek` \| `moonshot` \| `qwen` … — what the selector's exclusion rule reads (`external-review.md:20`) |
| `state` | enum | `ok` · `auth_failed` (401/403) · `model_missing` (404 on the model id) · `quota_exhausted` (402; 429 with no retry window; "exceeded your monthly quota"; balance ≤ floor) · `rate_limited` (429 with `retry-after`/reset) · `degraded` (200 but latency > threshold, or a window ≥ warn %) · `unknown` |
| `reason` | string | required when `state=unknown`: `cannot_assess:<why>` written by a probe that could not measure (unreachable, unparseable, no endpoint by design, token login exposes no usage); `absent` / `expired` are **reader-derived**, never written |
| `balance` | `{amount, currency, source}`? | only where a provider exposes one (§4); `source` names the endpoint |
| `headroom` | `{requests_remaining, tokens_remaining, reset_at, window_pct, window}`? | from `x-ratelimit-*` / `anthropic-ratelimit-*` headers or a subscription window (`five_hour`, `seven_day`, `monthly`) |
| `latency_ms` | int? | the probe's wall time |
| `probe` | `{name, source}` | `source ∈ {official, undocumented, file, header}` — LIT §10 "two-source truth with provenance" |
| `error` | string? | redacted, ≤ 200 chars (§6) |
| `checked_at` | RFC 3339 | when the probe ran |
| `ttl_s` | int | how long this record is trustworthy; chosen per kind (§3) |
| `observed_by` | string | `<host>/<binary>@<version>` — the per-host agent's own host for subscriptions |

**The freshness rule, applied by EVERY reader (library, CLI, selector, UI, alert):**

```text
no row for the key                      -> UNKNOWN (reason=absent)
row.checked_at + row.ttl_s < now        -> UNKNOWN (reason=expired)     # even if the bucket did not expire it
row.state == unknown                    -> CANNOT-ASSESS (row.reason)   # a measured failure to measure
bus unreachable / bucket missing        -> CANNOT-ASSESS, rc 2          # never "ok", never "no rows" (HZ-11813 shape)
anything else                           -> the row's state, with its age printed
```

Two layers hold it: the bucket is created with per-key TTL so an expired key is physically **absent**, and the value
carries `checked_at` + `ttl_s` so a reader on a bucket without TTL support still applies the rule. The measured failure
this prevents: `provider_balance_deepseek` on Mini today still reads `"verdict": "EXHAUSTED"` with
`"checked_at": "2026-08-25T12:47:17Z"`, and `account_util_M5` buries `"state":"stale"` inside its value — a 44-day-old
alarm and a staleness flag only a parsing reader can see (both `nats kv get fleet_alert_state … --raw`, 2026-10-08).

## 3. Architecture

```text
           keys (never on argv, never in a row)                      per-host, local reads only
   secretspec run -- / doppler run -- / env  ─┐             ┌─ statusline hook file · ~/.claude.json
                                              v             │   gh token -> copilot_internal/user
   [Mini]  quotabus probe   (API + local kinds, every 15 min)        [every host] quotabus agent (every 5 min)
                 │  one row per (kind,provider,account,model)              │  rows keyed by its own host
                 └──────────────► NATS KV  ai_status  (per-key TTL)  ◄─────┘   + $SRV registration
                                     │ change subject ai.status.changed.<key>
          ┌──────────────┬───────────┼───────────────┬──────────────────┐
     quotabus status   quotabus select   quotabus alert (crossing-dedup)   quotabus serve (optional)
     (table, --json,   (CLI + library)   → nusy-kanban signal · yurtle-     (127.0.0.1, JSON + one page)
      --check rc)                           kanban issue · webhook · stdout
```

**One binary, five subcommands.** `probe` is the central runner (Mini: a prebuilt binary is not a build, so the bus
host's no-build rule holds). `agent` is the per-host reader of subscription state, which must run locally: `claude auth
status` over SSH on macOS returns a confident, false "not logged in" (`f8b7f2f531^:scripts/fleet/account-util-publish.sh:7-17`;
memory `feedback_claude_auth_probe_fails_over_ssh_on_macos`). `status`, `select`, `alert`, `serve` are readers.

**Config — one model, two front-ends.** The binary's canonical format is TOML (outsiders; secretspec's `secretspec.toml`
is the model, LIT §8.7). The Yurtle twin is the same rows as a `yurtle-table` block (Yurtle v2.1,
`/Users/hankh95/Projects/yurtle/yurtle-spec.md:137-157`): the binary reads **only that block type** (fence + markdown
table + `@type`), ~150 lines, and ignores the rest of the file — the full Yurtle spec is explicitly not implemented here.

`quotabus.toml`:

```toml
[bus]            # omit the whole table and the file backend is used (§8)
url     = "nats://192.168.8.110:4222"   # or env QUOTABUS_NATS_URL
bucket  = "ai_status"

[probe]
interval = "15m"
ttl      = "45m"           # 3 missed probes = UNKNOWN
max_tokens = 20            # the 8-token smoke of docs/external-review.md:33

[[service]]
id = "glm"; kind = "api"; provider = "zhipu"; family = "zhipu"; account = "nusy-product-team"
base_url = "https://api.z.ai/api/anthropic"; protocol = "anthropic"
models = ["glm-5.3", "glm-5.2", "glm-4.6"]; roles = ["review", "work"]; cost_class = "metered"
secret = "NUSY_GLM"                                   # a NAME; the value comes from the environment
balance = "none"                                      # UNKNOWN by design (provider-balance.conf:20-22)

[[service]]
id = "deepseek"; kind = "api"; provider = "deepseek"; family = "deepseek"; account = "nusy-product-team"
base_url = "https://api.deepseek.com/anthropic"; protocol = "anthropic"
models = ["deepseek-v4-pro", "deepseek-v4-flash"]; roles = ["review"]; cost_class = "metered"
secret = "NUSY_DEEPSEEK"
[service.balance]
url = "https://api.deepseek.com/user/balance"; path = "balance_infos[0].total_balance"; currency = "CNY"; floor = 150

[[service]]
id = "claude-max"; kind = "subscription"; provider = "anthropic"; family = "anthropic"; account = "{host}"
sources = ["statusline", "claude_json"]               # official first; "oauth_usage" is opt-in (undocumented)

[[service]]
id = "copilot"; kind = "subscription"; provider = "github"; family = "openai"; account = "{host}"
sources = ["copilot_internal"]                        # undocumented; labelled so in every row

[alert.nusy-kanban]
command = "nusy-kanban"; item_type = "signal"; tags = ["provider-status", "infra"]
[alert.yurtle-kanban]
command = "yurtle-kanban"; item_type = "issue"
[alert.webhook]
url = "https://example.invalid/hook"
```

`quotabus.yurtle.md` — the same rows, readable in Obsidian and queryable by `yurtle-rdflib` (one block; the rest is prose):

````markdown
---
yurtle: v2.1
type: quotabus-config
id: fleet/ai-status
---
# AI services the fleet holds

```yurtle-table
@type Service

| @id       | kind         | provider  | family   | account           | base-url                          | models                   | roles        | secret        | balance-url                                    | balance-path                   | currency | floor |
|-----------|--------------|-----------|----------|-------------------|-----------------------------------|--------------------------|--------------|---------------|------------------------------------------------|--------------------------------|----------|-------|
| #glm      | api          | zhipu     | zhipu    | nusy-product-team | https://api.z.ai/api/anthropic    | glm-5.3, glm-5.2         | review, work | NUSY_GLM      |                                                |                                |          |       |
| #deepseek | api          | deepseek  | deepseek | nusy-product-team | https://api.deepseek.com/anthropic | deepseek-v4-pro         | review       | NUSY_DEEPSEEK | https://api.deepseek.com/user/balance          | balance_infos[0].total_balance | CNY      | 150   |
| #kimi     | api          | moonshot  | moonshot | nusy-product-team | https://api.moonshot.ai/anthropic | kimi-k3                  | review       | NUSY_KIMI     | https://api.moonshot.ai/v1/users/me/balance    | data.available_balance         | USD      | 100   |
| #claude   | subscription | anthropic | anthropic| {host}            |                                   |                          |              |               |                                                |                                |          |       |
```
````

**Key sources.** The binary reads a secret **only from its environment**, by the NAME the config gives, and refuses to
probe a service whose name resolves empty (the `backend-env.sh` rule, `docs/external-review.md:66`). Injection is the
launcher's job: `secretspec run -- quotabus probe` covers keyring/Keychain, 1Password, Doppler (0.21+), `pass`, Vault,
SOPS, Bitwarden, `.env` from one committed `secretspec.toml` (https://github.com/cachix/secretspec README: `[defaults]
providers = [...]`, `[providers] local = "keyring://"`); `doppler run --scope … --project nusy-product-team --config dev --`
is what the fleet uses today (`external-review.md:63-65`). An in-process `secretspec` SDK path
(`secretspec_derive::declare_secrets!` + `SecretSpec::load(Provider::…)`, same README) is a later feature flag, not slice 1:
it declares secrets at compile time, and our list is config-driven. [INFERENCE]

**Outputs.**

| output | shape |
|---|---|
| KV bucket `ai_status` | created with per-key TTL — natscli `nats kv add --marker-ttl=<d>` and `nats kv put --ttl=<d>` (measured in `--help`, natscli 0.3.1 on M5); async-nats 0.50 `kv::Config.limit_markers: Duration` behind the `server_2_11` feature (https://docs.rs/async-nats/latest/async_nats/jetstream/kv/struct.Config.html). ⚠ Mini runs 2.12.4 but every existing bucket reports `Per-Key TTL Supported: false` (LIT §6; `kv info fleet_alert_state` today) — a NEW bucket, created with the option, is required and is the first thing to measure |
| change subject | `ai.status.changed.<key>` published only when `state` differs from the previous row (a flapping latency does not spam the bus) |
| CLI | `quotabus status [--json] [--kind api] [--stale]` — a one-screen table with AGE and SOURCE columns; `quotabus status --check <service>` exits 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN, for pre-flight checks in scripts (LIT §10) |
| selector | `quotabus select --role review --exclude-family anthropic [--prefer cheapest|fastest|largest-context] [--n 1]` prints `provider model` on line 1 (for `$(…)`), the ranked list with reasons under `--json`; rc 3 when nothing qualifies. Library: `quotabus::select(&rows, &Query, &Config) -> Vec<Candidate>` — a pure function over rows + the static model table (family, cost class, roles, context), unit-testable without a bus. Vocabulary copied from LiteLLM's cooldown (`allowed_fails`, `cooldown`), applied per model not per group (LIT §1) |
| alert | `quotabus alert` runs after each probe cycle: **crossing-dedup** — one alert per state crossing, keyed `alert.<key>` in the same bucket holding the last alerted state; **only a measured `ok` clears it, a CANNOT-ASSESS never does** (the dead script's rule, `provider-balance-scan.sh:289-295`). Sinks: `nusy-kanban create --tags … --body-file - signal "<title>"` (flags before positionals; `nusy-kanban create --help`), `yurtle-kanban create issue "<title>" --push --body-file -` (`yurtle-kanban/src/yurtle_kanban/cli.py:767-776`) — or, inside a yurtle-kanban repo, its own `create_item` hook action (`hooks.py:372-373, 473-495`) — a JSON webhook, and stdout |
| web | `quotabus serve` on `127.0.0.1` by default: `GET /v1/status`, `GET /v1/select?…`, and one HTML page; feature-gated, off by default |

## 4. Probe catalogue — our providers

The cheapest probe for an Anthropic-protocol provider is the 8-token call already prescribed (`external-review.md:33,37`):
`POST <base>/v1/messages`, `max_tokens: 20`, thinking disabled; status → state, headers → headroom. Cost at 15 min:
≤ 96 calls × ~30 tokens per model per day. Balance endpoints are GETs.

| service | cheapest probe | reads | UNKNOWN by design |
|---|---|---|---|
| GLM / z.ai (`NUSY_GLM`) | messages probe per configured model. **Undocumented, opt-in, labelled — pending SIG-003 (open):** `GET /api/monitor/usage/quota/limit` answers the API key (Bearer or raw `Authorization`) with the Coding-Plan quota: two windows, 5 h and weekly, each with cap, remaining, % used and `nextResetTime` (epoch ms) — field meanings [inferred]; **unconfirmed:** a further 35-token call left `remaining` unchanged, so the counter either lags or does not register a call that small, and an adapter must not treat `remaining` as a live per-call meter. `GET /api/biz/subscription/list` gives plan status and renewal; its billing fields are never published (usagebar's endpoints, LIT §2; `docs/findings/CHORE-002-zai-endpoint.md`) | 200/401/404/429; latency. ⚠ a bad key gets **HTTP 200 with body `code: 401`** on three of the five endpoints (`/api/anthropic/v1/models`, both biz/monitor endpoints; the messages probe and `/api/paas/v4/models` give a real 401) ⇒ an adapter reads the body's `code`/`success`, never the HTTP status alone. No rate-limit headers on any response (`docs/findings/CHORE-002-zai-endpoint.md`) | balance — no documented endpoint, and no wallet balance on any endpoint probed 2026-10-08 (`docs/findings/CHORE-002-zai-endpoint.md`); three candidates 404'd 2026-08-25 (`scripts/fleet/provider-balance.conf:20-22`; the two live rows are `:25-26`). Headroom only from the quota endpoint (no headers) |
| DeepSeek (`NUSY_DEEPSEEK`) | `GET /user/balance` + messages probe | `balance_infos[0].total_balance`, `is_available` (LIT §2; conf row); the measured real case is a NEGATIVE balance (`-0.12 CNY` row on the bus today) ⇒ `quota_exhausted` | rate-limit headers [unverified] |
| Kimi / Moonshot (`NUSY_KIMI`) | `GET /v1/users/me/balance` + messages probe | `data.available_balance` (conf row); 404 on `kimi-k2.5`/`kimi-latest` ⇒ `model_missing` (`external-review.md:46`) | rate-limit headers [unverified] |
| OpenAI (`OPENAI_API_KEY`, Doppler `santiago`, root scope) | responses-API call with a 20-token ceiling on `gpt-5.6-sol` | 200/401/404/429; `x-ratelimit-*` [unverified] | balance — none exists (LIT §2); cost needs an admin key — deferred (§10 Q10) |
| Together (`TOGETHER_API_KEY`) · xAI (`XAI_API_KEY`, disabled) | OpenAI-protocol chat probe | 200/401 — xAI's disabled key is the standing **negative control**: it must read `auth_failed`, never `ok` | balance [unverified] |
| local Qwen, DGX1 `192.168.8.180:30000` (`NUSY_LOCAL_QWEN`) | `GET /v1/models`, then a 20-token completion | reachability, model list, latency | balance (self-hosted). From M5 it is unreachable (`external-review.md:49`) — the row says `cannot_assess:unreachable` with `observed_by`, which is the honest answer, and DGX1's own `agent` can probe it locally (`kind = local`) |
| Anthropic API (no key today) | messages probe | `anthropic-ratelimit-{requests,tokens}-{limit,remaining,reset}`, `retry-after`; spend-cap 429 carries `enforced_spend_limit_reached` (LIT §2) | prepaid balance (none) |
| Claude Max, per host | **official:** a `statusLine` command that tees stdin JSON to `~/.cache/quotabus/claude-rate-limits.json` (`rate_limits.five_hour.{used_percentage,resets_at}`, `seven_day`; Pro/Max only; LIT §2) — no host has a `statusLine` today (`~/.claude/settings.json` keys measured). **fallback:** read `~/.claude.json` `cachedUsageUtilization` (works over SSH; memory above). **opt-in:** `GET api.anthropic.com/api/oauth/usage` (undocumented). **measured alternative, pending SIG-008 (open):** `claude -p --output-format stream-json` emits a `rate_limit_event`, `rate_limit_info.unifiedWindows.{five_hour,seven_day}.{utilization,resetsAt}` — it matched the statusLine to the unit in the same minute (0.19/0.09 ↔ 19/9 %; `docs/findings/CHORE-002-statusline-under-claude-p.md`) | ⚠ an `oauth_token` login never writes the cache (`account-util-publish.sh:38-40`) ⇒ `unknown`, `reason=cannot_assess:token_login_no_usage_panel`; the hook does **not** fire under `claude -p` (text or stream-json), only in an interactive session, so a host that runs only `claude -p` never writes the cache; `rate_limits` is absent on the first render (before an API reply) ⇒ absent = not yet known, never 0 % (`docs/findings/CHORE-002-statusline-under-claude-p.md`) |
| Copilot, per host | `gh api copilot_internal/user` with the host's `gh` login → `quota_snapshots.premium_interactions.{remaining, percent_remaining, reset_date}` (LIT §2) | `remaining == 0` or "exceeded your monthly quota" ⇒ `quota_exhausted` | undocumented endpoint, labelled; no model probe (a `copilot -p` call spends quota) |

## 5. Language, runtime, packaging

**Rust.** The ecosystem it joins is Rust + NATS (`nusy-kanban`/`arrow-kanban`, `nusy-kanban-server`, the monorepo);
`async-nats` is the client the fleet already pins (`arrow-kanban/Cargo.toml:29`); `secretspec` is Rust; one static
binary is what five hosts and a bus host that must not build want (`cargo install` or a release asset, no venv, no
runtime). The dead instrument is the counter-example: 318 lines of bash whose hardest bugs were the language's —
`${NUSY_LOCAL-QWEN38:-}` parsing as a default operator and probing with a garbage bearer (CH-10303;
`provider-balance-scan.sh:224-228`), a secret name `eval`'d after validation as a shell identifier, and `kv_put`
failures byte-identical to success until HZ-10330 (`:137,140`). A typed record, a typed client and a redaction newtype
remove that class. Python would mirror yurtle-kanban but is a per-host install of an interpreter plus deps; the Python
prior art (agent-quota) is a catalogue to read, not a base.

**Crates:** `clap`, `serde`/`serde_json`, `toml`, `reqwest` (rustls), `async-nats` (feature `server_2_11`), `tokio`,
`tracing`; optional features `serve` (axum, one page), `yurtle` (the table reader), `secretspec` (in-process SDK).

**Packaging:** `packaging/launchd/com.congruentsys.quotabus-probe.plist` (Mini), `…-agent.plist` (M5, Air),
`packaging/systemd/quotabus-agent.service` + `.timer` (DGX1/2) — the fleet's existing shapes
(`scripts/com.nusy.kanban-snapshot.plist`, `scripts/backup/t9-backup.timer`); GitHub release binaries for
`aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`. CI mirrors
`arrow-kanban/.github/workflows/ci.yml:1-40`: `cargo fmt --check`, build + test `--locked` across feature profiles, clippy.

## 6. Security

| rule | mechanism |
|---|---|
| a key never leaves the process | `Secret(String)` newtype: `Debug`/`Display` print `***`; the HTTP header is built from it in one place; no key on argv (the dead script's `-K -` lesson, `:148-162`), no key in a row, a log or a config dump (`quotabus config show` prints names) |
| redaction is a mechanism, not a rule | a `Redactor` holds every resolved secret value and scrubs every error body, log line and row before it leaves the process (`teller redact`'s idea, LIT §9); plus pattern scrubs (`sk-…`, `ghp_…`, `Bearer …`); error text capped at 200 chars |
| least privilege on the bus | the probe needs put/get on one bucket and publish on one subject prefix; readers need get/watch; NATS account/permissions when the deployment has them [unverified for Mini's bus]; the web page binds `127.0.0.1` unless told otherwise |
| least privilege on keys | the launcher injects only the names the config lists (`secretspec.toml` declares them); Doppler service tokens scoped per directory as today (`external-review.md:65`) |
| what a record MAY reveal | provider, model id, state, latency, reset times, window percentages, the configured account label and host label, a balance **when `publish_balance = true`** |
| what a record MAY NOT reveal | any secret, any URL query string, a raw error body, an email or org id unless the operator typed it as the label, a user's prompt (the probe prompt is a constant) |
| undocumented sources | off by default; every row they produce carries `probe.source = "undocumented"`; a 4xx from them degrades to `unknown`, never to a false all-clear (LIT §11) |

## 7. What else earns its place — and what is left out

| keep | why | where |
|---|---|---|
| **reset countdown and pace** (`window_pct`, `reset_at`; "at this rate the 7-day window empties at HH:MM") | an agent defers a long review it cannot finish (LIT §10) | in the record + `status` table; E2 |
| **`--check` exit codes** | the loop skills stop claiming when the row says so; scripts need no JSON parsing | E1 |
| **spend vs budget** — per host via `ccusage --json`, per provider via balance deltas | the Captain's original question was "usage left" | E5, reads `ccusage`, never re-parses JSONL |
| **key expiry / rotation reminders** — `expires = "2026-12-31"` per service → `degraded` 14 days out | a dead key is otherwise found by a failed call | E5, config-only |
| **model drift sweep** — `GET /v1/models` (where offered) diffed against configured ids; a 404 on a configured model is already `model_missing` | names drift (`kimi-k2.5`), nothing probes live ids (LIT §8.6) | E5 |
| **tmux session census from the hub** — `quotabus census` reads `tmux ls -F` per host over SSH and publishes `census/1.0`-shaped rows (the dead `fleet-session-census.sh` contract: `{identity, host, written_at, cap, live, sessions[{name,kind,age_s}]}`, read on the bus today) | the Captain named M5 as the hub; the row contract already exists | a SEPARATE optional module behind a feature flag, its own expedition; not in the core's config model |
| **NATS services registration** (`$SRV.PING/INFO/STATS`) for the per-host agent | `nats micro ls` lists live agents without a bespoke heartbeat (LIT §6) | E2 [unverified: async-nats `service` feature name] |

**Deliberately out:** routing or proxying calls; pausing providers or switching accounts (the dead script's one act,
`:296-315` — actuation belongs to the reader); cookie-based reads of claude.ai / chatgpt.com (AIQuotaBar's method);
account pooling (CLIProxyAPI's ToS exposure); Prometheus exposition (a `/metrics` is a 40-line contribution if anyone
wants it); a persistent history store (the bus keeps `history = 1`; trends are `ccusage`'s or Grafana's job).

## 8. FOSS

| | |
|---|---|
| **name candidates** | **`quotabus`** (recommended: free on a GitHub exact-name search and `crates.io` 404, 2026-10-08; says bus + quota) · `llm-keywatch` (free) · `ai-status` [unverified]. Taken: `modelwatch` (wkwan, 28★), `keypulse`, `keystatus`, `provider-pulse` (`gh search repos --match name`) |
| **org** | `Congruentsys` (where `arrow-kanban` lives, MIT "Copyright (c) 2026 Congruentsys") or `hankh95` (yurtle, yurtle-kanban, nusy-kanban) — §10 Q1 |
| **licence** | **MIT**, matching nusy-kanban, yurtle, yurtle-kanban, nusy-nano (`LICENSE` files read locally); every dependency is MIT or Apache-2.0 (async-nats, secretspec, Gatus-class references) — LIT §11; the copyleft items LIT found are excluded. Shipping/licence stays a separate question from this R&D (Captain 2026-09-23) |
| **layout** | mirror `arrow-kanban`: `Cargo.toml` (`edition = "2024"`, `license = "MIT"`), `LICENSE`, `README.md`, `CONTRIBUTING.md`, `.github/workflows/ci.yml`; add `SECURITY.md` and `CODE_OF_CONDUCT.md` from yurtle-kanban; `packaging/`, `examples/quotabus.toml`, `examples/quotabus.yurtle.md`, `secretspec.toml` |
| **dual-track** | `CLAUDE.md:673`: a standalone FOSS repo uses GitHub issues + PRs and dual-tracks an `nk` item. Per piece: a public gh issue, an nk chore recording issue # and PR #, the gh PR `Closes #n`; the nk item is `done` only when the **gh PR is merged**, and the resident's own loop reviews open FOSS PRs with a distinct session (memory `feedback_foss_repo_dual_tracking`). The campaign for this work is the IDEA's own refine output, not CA-13266 |
| **useful without our fleet** | `[bus]` omitted ⇒ a **file backend** (`~/.local/state/quotabus/<key>.json`) with the same freshness rule; TOML config; keys from env or `secretspec`; `status`/`select`/`--check` work against the file backend; webhook sink; the kanban sinks and NATS are optional features. One command for a newcomer: `quotabus init` writes an example config from the catalogue in §4 |

## 9. Phased plan — sized by agent context (no human time)

| slice | rung | what lands | done when |
|---|---|---|---|
| **E1** | expedition (one Opus context) | repo scaffold + CI + MIT; TOML config; the probe runner for the **six key-bearing API rows** (GLM, DeepSeek, Kimi, OpenAI, Together, local Qwen — xAI as the disabled negative control; this count is this document's reading of `external-review.md:24-31,63`); balance adapters for DeepSeek/Kimi from `provider-balance.conf`; the record (§2); redaction; KV publisher with `checked_at`/`ttl_s`, bucket created with per-key TTL; `status` table + `--json` + `--check`; `com.…quotabus-probe.plist` for Mini | `quotabus status` on M5 shows the six rows from Mini's bucket with ages; a wrong key reads `auth_failed`; `kimi-k2.5` reads `model_missing`; bus down reads CANNOT-ASSESS rc 2; a key put with `--ttl` is ABSENT after expiry on Mini 2.12.4 (the measure-first item) |
| **C1** | chore | `docs/external-review.md` §1a replaced by "run `quotabus status`"; the hand probe retired | doc-only, straight to main |
| **C2** | chore | measure: does the `statusLine` hook fire under `claude -p`; which z.ai endpoint answers an API key | two findings files, each with the command |
| **E2** | expedition | `agent`: Claude statusline capture + `~/.claude.json` fallback + Copilot `copilot_internal/user`; `$SRV` registration; units for all five hosts; reset countdown and pace | five `subscription.*` rows on the bus from five hosts, each `observed_by` its own host; a token-login host reads `cannot_assess:token_login_no_usage_panel`, never `ok` |
| **E3** | expedition | `select` library + CLI; change subjects; `pairit` / external-review doc wiring ("the reviewer is `$(quotabus select --role review --exclude-family anthropic)`") | the selector refuses a family the author uses; rc 3 when every candidate is `unknown`; unit tests without a bus |
| **E4** | expedition | `alert` with crossing-dedup; nusy-kanban, yurtle-kanban, webhook sinks; the Yurtle front-end (`yurtle-table` reader) and `examples/` | a Kimi 404 files ONE signal across ten cycles; a measured `ok` re-arms it; a CANNOT-ASSESS does not; the Yurtle and TOML examples load to identical configs |
| **E5** | expedition (optional) | `serve`; spend via `ccusage --json`; key expiry; model drift sweep; file backend polish for outsiders | one page on `127.0.0.1`; a `expires` 14 days out reads `degraded` |
| **E6** | expedition (optional, separate module) | tmux session census from the hub | `census/1.0` rows for five hosts, behind a feature flag |

E1 is the first slice because it replaces the hand probe immediately and nothing in E2–E6 can be designed against a
bus that holds no rows. E2 before E3: the selector is only as honest as the subscription rows it refuses on.

## 10. Open questions for the Captain

1. **Name and org.** `quotabus` under `Congruentsys` (beside arrow-kanban) — *recommend yes*; `hankh95` if it should sit with yurtle/yurtle-kanban.
2. **Where the central probe runs.** Mini, as a prebuilt binary under launchd (no build on the bus host) — *recommend yes*; M5 runs `agent` + the hub's `status`/`serve`.
3. **Publish balances and account labels to the bus.** Numbers yes, labels as configured slugs, never emails — *recommend yes*; `publish_balance` stays a per-service switch for outsiders.
4. **Undocumented sources** (Anthropic `oauth/usage`, Copilot `copilot_internal/user`, z.ai biz endpoints and `/api/monitor/usage/quota/limit`). Off by default in the FOSS config; **on in our fleet config**, every row labelled `source = undocumented` — *recommend that split*.
5. **Cadence and spend.** API probes every 15 min (≤ 96 × ~30 tokens per model per day), subscription reads every 5 min (free), balance GETs every 15 min — *recommend these; make them config*.
6. **Does it act?** The dead script paused a provider at a measured zero. *Recommend no*: publish + alert only; the loop skills and `select` are the actuator, which keeps the tool safe to FOSS.
7. **TOML first, Yurtle in E4** (the Captain's words lead with Yurtle). *Recommend TOML first*: it lands E1 fastest and is what outsiders expect; Yurtle follows as the same rows in a `yurtle-table` block, which Obsidian and `yurtle-rdflib` read today.
8. **Alert item type.** nusy-kanban `signal` with tag `provider-status`, one per crossing, never auto-closed — *recommend signal*; `hazard` only if the Captain wants a crossing to block work.
9. **A `statusLine` entry in every host's `~/.claude/settings.json`** — a fleet-wide Claude config change to get the official per-host read; the `~/.claude.json` fallback works without it. *Recommend yes*, installed by E2's unit, measured first by C2. **Captain 2026-10-08: yes** — the recommended default; on file only as paraphrase, conditional on C2 confirming the hook fires under `claude -p` (`kanban-work/voyages/VOY-001-*.md` "Captain's rulings"; EXP-002 step 2). C2 (CHORE-002) measured that precondition **false** (§4 Claude Max row; `docs/findings/CHORE-002-statusline-under-claude-p.md`), so the question is **open again as SIG-008**; the stream-json `rate_limit_event` is an agent's recommendation there, not a decision.
10. **Admin keys** (Anthropic Admin API needs an org; OpenAI cost needs an admin key). *Recommend defer*: for these two the probe IS the status; revisit with E5's spend view.
11. **Repo membership on the board.** A new voyage under the IDEA's refine (E1–E4 = 4 expeditions + 2 chores, a legitimate voyage), or chores under an existing campaign? *Recommend a voyage*, dual-tracked per §8.
