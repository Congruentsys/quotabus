> **Origin.** Written in nusy-product-team as `docs/design/IDEA-13333-PROVIDER-STATUS-MONITOR.md` (commit `27feda15ea`, 2026-10-08) and copied here as the starting design. Paths of the form `path:line` refer to that repository. The Captain answered every §10 question on 2026-10-08. Q1 `quotabus` under Congruentsys · Q6 no actuation in v1.0 (a later expedition) · Q11 work filed on this repo's board are on file as paraphrase (`kanban-work/voyages/VOY-001-*.md` "Captain's rulings", this repo). Q2, Q3, Q4, Q5, Q7, Q8 and Q10 were filed as signals SIG-001…SIG-007 and ruled the same day; Q9's first "yes" was conditional on C2, C2 measured the condition false, and it was re-asked as SIG-008 and ruled. Each of those eight rulings is quoted verbatim in §10 with its signal (`kanban-work/signals/`, this repo); two differ from the recommendation offered (Q2, Q5). No §10 question is open.

# IDEA-13333 — Design: an AI provider & account status monitor (standalone FOSS)

**M5, 2026-10-08.** Captain-directed. Inputs: `IDEA-13333` (the Captain's words and the measured problem), `LIT-13334`
(`research/literature/LIT-13334-ai-provider-status-monitor-prior-art.md`, verdict: BUILD a thin core, ADOPT
`secretspec` / the statusline capture / `gh copilot_internal` / the adapter catalogue), and the ecosystem read below.

**Measured at:** monorepo `origin/main` `1064922dc7`; the dead instrument at `f8b7f2f531^:scripts/fleet/provider-balance-scan.sh`
(318 lines); Mini's bus read-only with `nats kv ls` / `kv info` / `kv get` on 2026-10-08; natscli on M5 is **0.3.1**
(`nats --version`). Working name below: **`quotabus`** (§8 — a candidate when this was written; the Captain chose it on 2026-10-08, §10 Q1). Marks: **[unverified]** = not
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
| **Users: us** | 5 hosts / 5 Claude logins (`CLAUDE.md` "One session per machine"); M5 as the tmux hub reading everything; Mini as the bus host running the central probe (the Captain's words on the item; for anyone else, where the central probe runs is a config choice, local or a named host — §10 Q2); the loop skills (`pairit`, external review) choosing a reviewer model |
| **Users: others** | anyone with several LLM keys or coding subscriptions, on one machine or many: small teams, CI runners, home labs, agent fleets; NATS shops that want status on a bus, and non-NATS users who want a table and exit codes (§8) |
| **Non-goals** | not a gateway or proxy (we never route calls through it — LIT §1); not a spend ledger (`ccusage` owns per-host spend; §7 reads its JSON); not an actuator (it never pauses, tops up or switches accounts — §7, §10 Q6); not a Yurtle/RDF engine (it reads one block type, §3); not a dashboard product (the web page is one screen, optional) |

## 2. Data model — the status record and the freshness rule

One record per **(kind, provider, account, model)**. JSON on the bus; the same struct in the library and the CLI.

**The balance row is not a model.** Each service with a balance endpoint also writes one row per (kind, provider,
account) under `<kind>.<provider>.<account>.balance`, with `probe.name = "balance"`: the balance read's own record, so
that kind has a last row to be due from (§3). Its `state` is `ok`, `quota_exhausted` at or below the floor, or
`unknown` (`cannot_assess:balance_unreadable`); its `balance` is set unless `publish_balance = false`; its `ttl_s` is
the balance kind's. A reader choosing or listing models skips it (`probe.name == "balance"`); the floor reaches the
model rows themselves as `quota_exhausted`. Config refuses a model whose slug is `balance` on such a service.

**A subscription row is per account.** A `kind = "subscription"` service is one account (a Claude login, a Copilot
seat) and writes one row, `subscription.<provider>.<account>.<service id>`: the service id stands in the model slot, and
`account` is the configured label. Its `headroom.windows[]` carries the usage windows (EXP-002, §4).

| field | type | notes |
|---|---|---|
| `contract` | `"ai-status/1"` | versioned like the dead rows (`provider_balance/1.0`, `account-util/1.0`, LIT §6) |
| `key` | string | the KV key, `<kind>.<provider>.<account>.<model>` — slugs only (KV keys exclude `@`, so an email is never a key) [unverified: exact KV key charset] |
| `kind` | `api` \| `subscription` \| `local` | API key · subscription account (Claude Max, Copilot), read centrally with the account's own token · self-hosted endpoint (DGX1 Qwen) |
| `provider`, `account`, `model` | string | `account` is the LABEL from config (`nusy-product-team`, `hankh95`), never an email unless the operator chose one |
| `family` | string | `anthropic` \| `openai` \| `zhipu` \| `deepseek` \| `moonshot` \| `qwen` … — what the selector's exclusion rule reads (`external-review.md:20`) |
| `state` | enum | `ok` · `auth_failed` (401/403) · `model_missing` (404 on the model id) · `quota_exhausted` (402; 429 with no retry window; "exceeded your monthly quota"; balance ≤ floor) · `rate_limited` (429 with `retry-after`/reset) · `degraded` (200 but latency > threshold, or a window ≥ warn %) · `unknown` |
| `reason` | string | required when `state=unknown`: `cannot_assess:<why>` written by a probe that could not measure (unreachable, unparseable, no endpoint by design, token login exposes no usage); `absent` / `expired` are **reader-derived**, never written |
| `balance` | `{amount, currency, source}`? | only where a provider exposes one (§4); `source` names the endpoint |
| `headroom` | `{requests_remaining, tokens_remaining, reset_at, window_pct, window, windows}`? | from `x-ratelimit-*` / `anthropic-ratelimit-*` headers or a subscription's windows. `windows[]` (subscriptions, EXP-002): `{window: five_hour \| seven_day \| monthly, used_pct (0–100), reset_at (RFC 3339), reset_in_s, pace}`, `reset_in_s = reset_at − checked_at`, `pace = (used_pct/100) / (elapsed/len)` with `elapsed = len − reset_in_s` and `len` 5 h, 7 d, or the month before a `monthly` reset (1.0 empties the window exactly at its reset). Only the windows the source carried are written: an absent window is left out, **never 0 %**. `window` / `window_pct` / `reset_at` repeat the most-used window |
| `latency_ms` | int? | the probe's wall time |
| `probe` | `{name, source}` | `source ∈ {official, undocumented, file, header}` — LIT §10 "two-source truth with provenance" |
| `error` | string? | redacted, ≤ 200 chars (§6) |
| `checked_at` | RFC 3339 | when the probe ran |
| `ttl_s` | int | how long this record is trustworthy; chosen per kind (§3) |
| `observed_by` | string | `<host>/<binary>@<version>` — the probe host, for subscription rows too (they are read centrally, EXP-002) |

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
           keys (never on argv, never in a row): API keys, Claude setup-tokens, the Copilot token
   secretspec run -- / doppler run -- / env  ─┐
                                              v                       nothing runs on any other host
   [central host]  quotabus probe  (API 12 h · balance 12 h · local · subscription 1 h)
                 │  one row per (kind,provider,account,model); one subscription row per account
                 └──────────────► NATS KV  ai_status  (per-key TTL)
                                     │ change subject ai.status.changed.<key>
          ┌──────────────┬───────────┼───────────────┬──────────────────┐
     quotabus status   quotabus select   quotabus alert (crossing-dedup)   quotabus serve (optional)
     (table, --json,   (CLI + library)   → nusy-kanban signal · yurtle-     (127.0.0.1, JSON + one page)
      --check rc)                           kanban signal · webhook · stdout
```

**One binary, five subcommands.** `probe` is the central runner. Where it runs is a config choice made at install:
this host, or a named other host (§10 Q2; the install step is CHORE-008). The fleet sets Mini: a prebuilt binary is not
a build, so the bus host's no-build rule holds. **Each query kind has its own interval**, set on its own (§10 Q5): API
(messages) probes and balance reads default to every 12 h, subscription reads to every 1 h (the Captain, 2026-10-08, on EXP-002: "Hourly"); no interval is shared, and
a later kind (e.g. a quota-window read) gets its own. Every interval in this document is a default, not a constant. The
rulings name no default for the `local` kind's probe (DGX1 Qwen); that is not decided here. The central probe still runs that kind (`local-qwen` in `examples/quotabus.toml`), so the diagram shows `local` beside API and balance (`reviews/CHORE-006-r1.md:101-107`). **Subscription accounts are read by the same central probe** (EXP-002), each with its own token from the environment — the Captain, 2026-10-08, rescoping EXP-002: "Doppler has the API keys for all agents (includeing all of the claude agents that run on each machine. We should not need to run anything else on the other machines (just Mini or M5 depending on where we host this)". The per-host `agent`, the statusLine capture and the per-host units of the earlier plan are dropped: no read depends on a host's own login, so `claude auth status` over SSH on macOS returning a false "not logged in" (`f8b7f2f531^:scripts/fleet/account-util-publish.sh:7-17`), which is why a per-host reader was once planned, no longer matters. `status`, `select`, `alert`, `serve` are readers.

**Scheduling — a tick, and what is due.** The intervals live only in the config: `[intervals]` (one key per kind)
and `[ttl]` (one per kind; a kind left out is 3 × its own interval — CHORE-007's Definition of Done, `kanban-work/chores/CHORE-007-*.md:22`, not a Captain ruling). The launchd unit (§5) only ticks — `quotabus probe`
every 300 s — and each run makes a kind's calls only when that kind is **due**: its interval has elapsed since its last
row in the store (no row = due; a row written without a call, such as `secret_unset`, is not a run). Due-ness is per
row: each (service, model) for the API probe, each service's balance row (§2) for the balance read, and each subscription account's row for the subscription read, so a due API
probe never triggers a balance read and vice versa (CHORE-007, `kanban-work/chores/CHORE-007-*.md:23`; both citations per `reviews/CHORE-006-r1.md:109-117`). An API probe applies the floor and carries the balance from the
last fresh balance row. Until a `local` default is decided, a `local` service's probe runs on `[intervals] api`.
`quotabus probe --force` runs every kind now.

**Config — one model, two front-ends.** The binary's canonical format is TOML (outsiders; secretspec's `secretspec.toml`
is the model, LIT §8.7). The Yurtle twin is the same rows as a `yurtle-table` block (Yurtle v2.1,
`/Users/hankh95/Projects/yurtle/yurtle-spec.md:137-157`): the binary reads **only that block type** (fence + markdown
table + `@type`), ~150 lines, and ignores the rest of the file — the full Yurtle spec is explicitly not implemented here.

`quotabus.toml`:

```toml
[bus]            # omit the whole table and the file backend is used (§8)
url     = "nats://192.168.8.110:4222"   # or env QUOTABUS_NATS_URL
bucket  = "ai_status"

[probe]          # `interval` / `ttl` here are retired and refused, naming [intervals] / [ttl]
max_tokens = 20            # the 8-token smoke of docs/external-review.md:33

[intervals]      # one per query kind, each set on its own; none is shared (§10 Q5)
api          = "12h"       # messages probe, per model
balance      = "12h"       # balance GETs
subscription = "1h"        # subscription account reads, central (EXP-002; the Captain: "Hourly")

[ttl]            # optional, per kind; each defaults to 3 x its own interval (3 missed = UNKNOWN; CHORE-007)
# api          = "36h"
# balance      = "36h"
# subscription = "3h"

[[service]]
id         = "glm"
kind       = "api"
provider   = "zhipu"
family     = "zhipu"
account    = "nusy-product-team"
base_url   = "https://api.z.ai/api/anthropic"
protocol   = "anthropic"
models     = ["glm-5.3", "glm-5.2", "glm-4.6"]
roles      = ["review", "work"]
cost_class = "metered"
secret     = "NUSY_GLM"                               # a NAME; the value comes from the environment
balance    = "none"                                   # UNKNOWN by design (provider-balance.conf:20-22)

[[service]]
id         = "deepseek"
kind       = "api"
provider   = "deepseek"
family     = "deepseek"
account    = "nusy-product-team"
base_url   = "https://api.deepseek.com/anthropic"
protocol   = "anthropic"
models     = ["deepseek-v4-pro", "deepseek-v4-flash"]
roles      = ["review"]
cost_class = "metered"
secret     = "NUSY_DEEPSEEK"
[service.balance]
url      = "https://api.deepseek.com/user/balance"
path     = "balance_infos[0].total_balance"
currency = "CNY"
floor    = 150

[[service]]                                           # one per Claude account (the fleet has six)
id       = "claude-hankh95"                           # the row's model slot: subscription.anthropic.hankh95.claude-hankh95
kind     = "subscription"
provider = "anthropic"
family   = "anthropic"
account  = "hankh95"                                  # the account LABEL, not a host
models   = ["claude-haiku-4-5"]                       # the model of the 1-token direct read
secret   = "NUSY_CLAUDE_TOKEN_HANKH95"                # the account's setup-token, by NAME
sources  = ["unified_headers", "stream_json"]         # the fleet's: undocumented direct read, official fallback

[[service]]
id       = "copilot"
kind     = "subscription"
provider = "github"
family   = "openai"
account  = "hankh95"
secret   = "GITHUB_TOKEN"
sources  = ["copilot_internal"]                       # undocumented; labelled so in every row

[alert.nusy-kanban]
command   = "nusy-kanban"
item_type = "signal"
tags      = ["provider-status", "infra"]
[alert.yurtle-kanban]
command   = "yurtle-kanban"
item_type = "signal"                                  # SIG-006 "Signal per crossing", as on nusy-kanban
tags      = ["provider-status"]
[alert.webhook]
url = "https://example.invalid/hook"
```

One key per line: TOML has no `;` separator, so a line of `;`-separated keys does not parse; the block above loads with Python's `tomllib`. The `[intervals]` / `[ttl]` tables are the per-kind shape of §10 Q5, built by CHORE-007 for `api` and `balance`; EXP-002 added `subscription`, and the binary refuses any other key there. A subscription source runs only when `sources` lists it (`unified_headers`, `stream_json`, `copilot_internal`); `examples/quotabus.toml` uses the same layout, with the undocumented sources off (§10 Q4).

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
| #claude-hankh95 | subscription | anthropic | anthropic | hankh95 | https://api.anthropic.com    | claude-haiku-4-5         |              | NUSY_CLAUDE_TOKEN_HANKH95 |                                  |                                |          |       |
```
````

**The Yurtle front-end — what EXP-004 built** (`src/yurtle.rs`, `examples/quotabus.yurtle.md`, `tests/exp004_config.rs`).
`Config::load` reads a path ending in `.md` as Yurtle and anything else as TOML. No Rust Yurtle reader exists
(`docs/PRIOR-ART.md` names only the Python `yurtle-rdflib`), so the binary carries a small, strict reader for exactly
the block above — Yurtle v2.1's `yurtle-table` (`yurtle-spec.md:137-157`) — and nothing else; it is not a Yurtle
parser. It reads every fenced block whose info string is `yurtle-table` (a block shown inside another fence, as here,
is not read) and ignores frontmatter, prose, plain markdown tables and every other block. A block is one `@type <Type>`
line, then one markdown table whose first column is `@id` (`#<id>`); an empty cell is absence and a comma-separated
cell is a list, as the spec's rules 4-5 say. Cells are typed by column, not inferred (a `floor` must be a number,
everything else is a string). Refused, so a typo is an error and not a silently dropped value: an unknown `@type` or
column, a ragged row, a missing separator, a second `@type` line, `@prefix` / `@base` / prose inside a block, a
per-row `@type` column, an unclosed block. The rows become the same tables the TOML front-end parses, so one
validation path checks both. The rest of the config is more blocks of the same kind: `@type Bus` (`#bus`: `url`,
`bucket`), `File` (`#file`: `dir`), `Probe` (`#probe`: `max-tokens`, `degraded-latency-ms`), `Schedule` (`#api`,
`#balance`, `#subscription`: `interval`, `ttl`, i.e. `[intervals]` and `[ttl]`), and `AlertSink` (`#nusy-kanban`,
`#yurtle-kanban`, `#webhook`: `command`, `item-type`, `tags`, `url`). `Service` takes the columns above plus
`protocol`, `cost-class`, `sources`, `publish-balance` (`true`/`false`) and `context` (`<model>=<tokens>, …`). Those
names, and the split into several blocks, are EXP-004's [INFERENCE: the design shows only the `Service` block, and
"the binary reads only that block type" rules out a `yurtle` block for the rest]. `examples/quotabus.yurtle.md` loads
to the same config as `examples/quotabus.toml`.

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
| KV bucket `ai_status` | created with per-key TTL — natscli `nats kv add --marker-ttl=<d>` (bucket) and `nats kv create --ttl=<d>` (key) — in natscli 0.3.1 the per-key `--ttl` is on `kv create` ("Sets a TTL for the key"), `kv put --help` lists no TTL flag, and `kv add --ttl` is the bucket-wide max age, not a per-key TTL (`--help` of each, natscli 0.3.1 on M5 and Mini; `docs/findings/EXP-001-per-key-ttl.md:38-41`). `kv create` writes only a new or deleted key, so a hand-written TTL row over an existing key is a delete, then a create; the binary writes rows through the client library, not natscli; async-nats 0.50 `kv::Config.limit_markers: Duration` behind the `server_2_11` feature (https://docs.rs/async-nats/latest/async_nats/jetstream/kv/struct.Config.html). ⚠ Mini runs 2.12.4 but every existing bucket reports `Per-Key TTL Supported: false` (LIT §6; `kv info fleet_alert_state` today) — a NEW bucket, created with the option, is required. Measured by EXP-001: a new bucket made with `--marker-ttl` reports `Per-Key TTL Supported: true` and a 5 s key is absent at +8 s on Mini's 2.12.4 (`docs/findings/EXP-001-per-key-ttl.md:3-4,22-29`) |
| change subject | `ai.status.changed.<key>` published only when `state` differs from the previous row (a flapping latency does not spam the bus); the payload is the new row as JSON, on core NATS (no stream captures it). Built by EXP-003 in the bus backend's write, so every row `quotabus probe` writes goes through it. A key with no previous row (never written, or expired out of the bucket) or an unreadable one counts as a change [INFERENCE: the design is silent on the first row]; a failed announcement does not fail the write, the row is already stored [INFERENCE]. The write is conditional on the revision it read (`Nats-Expected-Last-Subject-Sequence`, 0 for an absent key, as async-nats's `Store::update` does): a concurrent writer that moved the key makes the server refuse it (`WrongLastSequence`), and the write re-reads and retries, up to 5 times, so the announce is decided against the row actually replaced and a concurrent change is never left unannounced (review r1 F2). After 5 refusals, or when the last revision cannot be read, the row is written unconditionally and announced: a duplicate announce is harmless, a missing one is not. The announce's flush is bounded at 2 s; past that a warning is logged and the write still returns ok (review r1 F4). The subject is a hint: a subscriber re-reads the row |
| CLI | `quotabus status [--json] [--kind api] [--stale]` — a one-screen table with AGE and SOURCE columns; `quotabus status --check <service>` exits 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN, for pre-flight checks in scripts (LIT §10) |
| selector | `quotabus select --role review --exclude-family anthropic [--prefer cheapest|fastest|largest-context] [--n 1]` prints `provider model` on line 1 (for `$(…)`), the ranked list with reasons under `--json`; rc 3 when nothing qualifies. Library: `quotabus::select(&rows, &Query, &Config) -> Vec<Candidate>` — a pure function over rows + the static model table (family, cost class, roles, context), unit-testable without a bus. Vocabulary copied from LiteLLM's cooldown (`allowed_fails`, `cooldown`), applied per model not per group (LIT §1) |
| alert | `quotabus alert` runs after each probe cycle: **crossing-dedup** — one alert per state crossing, keyed `alert.<key>` in the same bucket holding the last alerted state; **only a measured `ok` clears it, a CANNOT-ASSESS never does** (the dead script's rule, `provider-balance-scan.sh:289-295`). Sinks: `nusy-kanban create --tags … --body-file - signal "<title>"` (flags before positionals; `nusy-kanban create --help`), `yurtle-kanban create signal "<title>" --push --body-file -` (`yurtle-kanban/src/yurtle_kanban/cli.py:767-776`) — or, inside a yurtle-kanban repo, its own `create_item` hook action (`hooks.py:372-373, 473-495`) — a JSON webhook, and stdout. The item type is the sink's `item_type`: `signal` on both boards (§10 Q8, SIG-006). What EXP-004 built is below the table |
| web | `quotabus serve` on `127.0.0.1` by default: `GET /v1/status`, `GET /v1/select?…`, and one HTML page; feature-gated, off by default |

**Alert — what EXP-004 built** (`src/alert.rs`, `quotabus alert` in `src/main.rs`, `tests/exp004_alert.rs`).
- **What it reads.** The configured (service, slot) keys, as `status` does, each through the freshness rule (§2).
  Balance rows are not alerted on their own: the floor reaches the model rows as `quota_exhausted`.
- **Dedup at `alert.<key>`**, in the same store as the rows (bus: the same bucket; file backend:
  `<dir>/alert.<key>.json`). The value is JSON, `{contract: "quotabus-alert/1", key, state, at, pending?}`; `state`
  is the last alerted state. It is written with no per-key TTL (an expired dedup entry would file the same crossing
  again) and announces no change subject; every listing (`status`, `select`, `probe`'s due check) leaves `alert.*`
  keys out, so it never reads as a broken row.
- **A crossing** is a fresh measured state that is not `ok` while the key is armed (no entry, or an entry holding
  `ok`). It is filed to every configured sink and printed on stdout (`ALERT <key> <state> (<service> <model>): …`,
  plain text), then recorded. Once the key is alerted, every later bad reading — the same state or a different one —
  is the same outage and files nothing, across any number of cycles.
- **Only a measured `ok` clears it:** a fresh `ok` rewrites the entry as `state: "ok"` (re-armed; stdout says
  `CLEARED`) and files and closes nothing. A CANNOT-ASSESS (`unknown` row, unreadable row) and an UNKNOWN (absent,
  expired — an expired `ok` included) leave the entry as it is and file nothing.
- **It never auto-closes.** The only board call is `create`; a recovery is a stdout line, and the item is closed by
  hand.
- **Sinks.** `[alert.nusy-kanban]` runs `<command> create --tags <tags> --body-file - <item_type> "<title>"`;
  `[alert.yurtle-kanban]` runs `<command> create <item_type> "<title>" --push --tags <tags> --body-file -` (never
  `--no-push`, never `--assign`); the body (key, state, reason, error, `checked_at`, `observed_by`) is on stdin.
  `command` defaults to the sink's name, `item_type` to `signal`. The child's environment is an allowlist only —
  `PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `TERM`, each if set, the `claude` fallback's rule (`src/subscription.rs`
  `CHILD_ENV`; review r1 F3) — so nothing `doppler run` injects, configured `secret` or not, reaches a sink. A sink
  that needs more (nusy-kanban's server, an SSH agent for `git push`) is a wrapper script given as `command` that
  sets it. `[alert.webhook]` is a JSON POST of the crossing (key, service, model, state, reason, error,
  title …); a 2xx is success, and the URL is never printed (a webhook URL is often itself a credential). Title, body,
  JSON and stdout all pass the redactor.
- **A sink failure means retry:** the crossing is not recorded, rc is 1, and the next run files it again.
  **[INFERENCE] partial failure:** the crossing is recorded only when every sink succeeded; the sinks that did are kept
  in the entry's `pending: {state, filed: [...]}` (its `state` stays the previous one), so the next run retries only
  the sinks that failed and a failed webhook never files a second kanban item.
- **A bad→bad move files nothing** (`rate_limited` ↔ `quota_exhausted`, `model_missing` → `auth_failed`): the
  entry is left as it is, and only a measured `ok` re-arms the key — one item per crossing, "a service going bad"
  (ruled by the driver on review r1 F1, `reviews/EXP-004-r1.md`; it replaces this section's first [INFERENCE], under
  which a provider at its cap alternating between the two 429 readings filed a signal every cycle). A crossing still
  pending on some sinks is completed on the others whatever bad state it reads now. `degraded` and `rate_limited` are
  not `ok`, so each is a crossing too.
- **Exit codes:** 0 every sink called succeeded (or nothing crossed) · 1 a sink, or a dedup read/write, failed
  (retried next run) · 2 CANNOT-ASSESS: the store cannot be read (bus down, bucket or row directory missing), and
  nothing is filed. An `alert.<key>` that is not an alert entry skips that key with rc 1 (whether it was filed is
  unknown, so it neither files nor clears).
- **When it runs:** after each probe cycle, in the same unit (`packaging/README.md`).
- **One writer, and entries that outlive their service** (review r1 F4). `alert` assumes it is the only writer of
  `alert.*`: it runs once per probe cycle, on the probe host, and `put_entry` is a plain put, not the
  revision-conditional write the rows have (EXP-003), so two `alert` runs at once could each read "not alerted" and
  both file. Running it anywhere else, or twice per cycle, needs that conditional write first. An `alert.<key>` entry
  is never removed: a service taken out of the config leaves its entry in the bucket, where every reader skips it,
  so it is harmless; clean it with `nats kv del ai_status alert.<key>` (file backend: delete
  `<dir>/alert.<key>.json`) if wanted. Re-adding the service then starts it armed.

**Selector — what EXP-003 fixed** (`src/select.rs`, `tests/exp003_select.rs`, `tests/exp003_cli_select.rs`). The
pick is over the configured (service, model slot) pairs, each matched to its row by key; the model table is the config
(`family`, `cost_class`, `roles`, and `[service.context]`, `"<model>" = <tokens>`, for `largest-context`).
- **Only a fresh measured `ok` is chosen.** The freshness rule (§2) is applied at the query's `now`; an absent or
  expired row, a CANNOT-ASSESS (`unknown`) row, and every other state are refused, **`degraded` included**
  [INFERENCE: the design asks for "a healthy model" and says nothing that admits `degraded`].
- **A subscription seat is never a candidate** (`kind = "subscription"`, a Claude account or the Copilot seat), even
  fresh, `ok` and listing the role. This is the interim rule pending the Captain's SIG-009 (open: can a seat be a
  candidate, and as which model?): a seat's row is per ACCOUNT with the service id in the model slot (§2), so there is
  no model to print as `provider model` (review r1 F1, `reviews/EXP-003-r1.md`). A healthy model that a seat serves
  is chosen only when it is configured as its own API service.
- **The role must be listed** on the service. A family is refused when it is the service's or the row's `family`, compared without case; any number
  of `--exclude-family` may be given; a refused family is refused even when it is the only healthy one.
- **Ordering.** `--prefer` sets the first key, the other two break ties, then the row key, so the answer is total
  and repeatable: `fastest` is the row's `latency_ms`, lowest first; `largest-context` is `[service.context]`, largest
  first; `cheapest` is `cost_class` in the order `local`, then `subscription`/`free`, then `metered`, then anything
  else or unset [INFERENCE: the design names the classes, not their order]. An unmeasured latency or an unset context
  sorts last. With no `--prefer`, `cheapest` [INFERENCE].
- **Exit codes:** 0 chosen (`provider model` per line, `--n` lines, default 1; `--json` a JSON array of the first
  `--n` candidates, each `{service, provider, model, family, key, reason}`) · **2 CANNOT-ASSESS** when the store cannot
  be read (bus down, bucket or row directory missing) or `--prefer` is not one of the three words — never a pick
  (§2) · **3 nothing qualifies**, including when every candidate is `unknown`. On rc 2 and 3 stdout names no model
  (`[]` under `--json`), so `$(quotabus select …)` never yields a usable pick; the reason goes to stderr. It never
  writes the store (§10 Q6). nusy-product-team's wiring is `docs/wiring/nusy-product-team.md` (a separate item in
  that repo).

## 4. Probe catalogue — our providers

The cheapest probe for an Anthropic-protocol provider is the 8-token call already prescribed (`external-review.md:33,37`):
`POST <base>/v1/messages`, `max_tokens: 20`, thinking disabled; status → state, headers → headroom. Cost at the 12 h API default (`[intervals] api`, §10 Q5):
≤ 2 calls × ~30 tokens per model per day (24 h ÷ 12 h = 2; the 15 min default this line first named was 96; each
`--force` adds one). Balance endpoints are GETs, on their own `[intervals] balance` (12 h default), never triggered by a
messages probe or the reverse (CHORE-007, `kanban-work/chores/CHORE-007-*.md:23`, not §10 Q5; `reviews/CHORE-006-r1.md:109-117`).

| service | cheapest probe | reads | UNKNOWN by design |
|---|---|---|---|
| GLM / z.ai (`NUSY_GLM`) | messages probe per configured model. **Undocumented, labelled — off in the shipped config, on in the fleet's (Captain 2026-10-08, §10 Q4, SIG-003):** `GET /api/monitor/usage/quota/limit` answers the API key (Bearer or raw `Authorization`) with the Coding-Plan quota: two windows, 5 h and weekly, each with cap, remaining, % used and `nextResetTime` (epoch ms) — field meanings [inferred]; **unconfirmed:** a further 35-token call left `remaining` unchanged, so the counter either lags or does not register a call that small, and an adapter must not treat `remaining` as a live per-call meter. `GET /api/biz/subscription/list` gives plan status and renewal; its billing fields are never published (usagebar's endpoints, LIT §2; `docs/findings/CHORE-002-zai-endpoint.md`) | 200/401/404/429; latency. ⚠ a bad key gets **HTTP 200 with body `code: 401`** on three of the five endpoints (`/api/anthropic/v1/models`, both biz/monitor endpoints; the messages probe and `/api/paas/v4/models` give a real 401) ⇒ an adapter reads the body's `code`/`success`, never the HTTP status alone. No rate-limit headers on any response (`docs/findings/CHORE-002-zai-endpoint.md`) | balance — no documented endpoint, and no wallet balance on any endpoint probed 2026-10-08 (`docs/findings/CHORE-002-zai-endpoint.md`); three candidates 404'd 2026-08-25 (`scripts/fleet/provider-balance.conf:20-22`; the two live rows are `:25-26`). Headroom only from the quota endpoint (no headers) |
| DeepSeek (`NUSY_DEEPSEEK`) | `GET /user/balance` + messages probe | `balance_infos[0].total_balance`, `is_available` (LIT §2; conf row); the measured real case is a NEGATIVE balance (`-0.12 CNY` row on the bus today) ⇒ `quota_exhausted` | rate-limit headers [unverified] |
| Kimi / Moonshot (`NUSY_KIMI`) | `GET /v1/users/me/balance` + messages probe | `data.available_balance` (conf row); 404 on `kimi-k2.5`/`kimi-latest` ⇒ `model_missing` (`external-review.md:46`) | rate-limit headers [unverified] |
| OpenAI (`OPENAI_API_KEY`, Doppler `santiago`, root scope) | chat-completions call, `POST <base>/chat/completions` with `max_completion_tokens: 20`, on `gpt-5.6-sol` (`src/probe.rs:247-261`; other OpenAI-protocol providers get `max_tokens`, and the code's stated reason — OpenAI's own models refuse `max_tokens` on chat completions — is not measured in a finding). Chosen by E1 over the responses API this row first named; it measurably works: `openai gpt-5.6-sol` read `ok` on Mini's bus in EXP-001's real run, 2026-10-08 (PR #2 review and author comments; `reviews/EXP-001-r1.md:167-172`, F5) | 200/401/404/429; `x-ratelimit-*` [unverified] | balance — none exists (LIT §2); cost needs an admin key — deferred to E5 (Captain 2026-10-08, §10 Q10) |
| Together (`TOGETHER_API_KEY`) · xAI (`XAI_API_KEY`, disabled) | OpenAI-protocol chat probe | 200/401 — xAI's disabled key is the standing **negative control**: it must read `auth_failed`, never `ok` | balance [unverified] |
| local Qwen, DGX1 `192.168.8.180:30000` (`NUSY_LOCAL_QWEN`) | `GET /v1/models`, then a 20-token completion | reachability, model list, latency | balance (self-hosted). From M5 it is unreachable (`external-review.md:49`) — the row says `cannot_assess:unreachable` with `observed_by`, which is the honest answer (`kind = local`) |
| Anthropic API (no key today) | messages probe | `anthropic-ratelimit-{requests,tokens}-{limit,remaining,reset}`, `retry-after`; spend-cap 429 carries `enforced_spend_limit_reached` (LIT §2) | prepaid balance (none) |
| Claude Max, every account, from the probe host (EXP-002) | **direct, `unified_headers` (undocumented):** one `POST api.anthropic.com/v1/messages` per account with its setup-token (Doppler `nusy-product-team` holds `NUSY_CLAUDE_TOKEN_<ACCOUNT>` for six accounts, all `oat`-class setup-tokens) as `Authorization: Bearer`, `anthropic-beta: oauth-2025-04-20`, Haiku, `max_tokens: 1` (~35 input tokens), and `"system": "You are Claude Code, Anthropic's official CLI for Claude."` (required: the same token in the same minute without it got **429** `rate_limit_error` with no unified headers, with it 200 with `anthropic-ratelimit-unified-5h-utilization: 0.38`; measured on M5 2026-10-08T20:21Z, HAZ-003) → 200 with `anthropic-ratelimit-unified-{5h,7d}-{utilization,reset,status}`, `anthropic-ratelimit-unified-status` and `-overage-status`; all six accounts answered 200 with both windows (fake-token control: 401). **fallback, `stream_json` (official, Claude Code's own output):** when that reply carries no unified headers, `claude -p --output-format stream-json` runs once for the account with the token only in the child's environment (`CLAUDE_CODE_OAUTH_TOKEN`, never argv); its `rate_limit_event`, `rate_limit_info.unifiedWindows.{five_hour,seven_day}.{utilization,resetsAt}`, carried the same numbers as the headers (0.3 / 0.12, the same resets). The Captain, 2026-10-08: "Direct, stream-json fallback". **Not usable:** `GET api.anthropic.com/api/oauth/usage` answers a setup-token **403 permission_error**, "OAuth token does not meet scope requirement user:profile" (fake-token control: 401). All measured on M5, 2026-10-08 (EXP-002's body; raw artefacts kept out of git). Cost: ~36 tokens of the account's own quota per read, at the 1 h default 24 reads per account per day | utilization (0–1) → `used_pct`; reset (epoch s) → `reset_at`, `reset_in_s`, `pace`; `allowed` → `ok`, `allowed_warning` → `degraded` [INFERENCE: a status word the measurement did not see], `rejected` → `quota_exhausted`; 401/403 → `auth_failed`, never retried through `claude` | a reply with no unified headers and no `stream_json` listed (a 429 one reads `cannot_assess:http_429_no_unified_headers`, never `ok` or `quota_exhausted`), no `claude` binary, a failed run, or a stream with no `rate_limit_event` ⇒ `unknown`, `cannot_assess:<why>`; a window the source did not carry is left out, never 0 % |
| Copilot, from the probe host (EXP-002) | `GET api.github.com/copilot_internal/user` with the configured token (the fleet's: Doppler `GITHUB_TOKEN`) → 200, `quota_snapshots.premium_interactions.{entitlement, remaining, percent_remaining, unlimited}`, `quota_reset_date` (measured on M5, 2026-10-08) | a `monthly` window, `used_pct = 100 − percent_remaining`, `reset_at` from `quota_reset_date`; `requests_remaining`; `remaining == 0` or "exceeded your monthly quota" ⇒ `quota_exhausted`; 401/403 ⇒ `auth_failed` | undocumented endpoint (`copilot_internal`), labelled `probe.source = undocumented` in every row and off unless `sources` lists it; no model probe (a `copilot -p` call spends quota) |

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

**Packaging:** `packaging/install.sh` renders `com.congruentsys.quotabus-probe.plist` (the central host, chosen at install: this host or a named one, §10 Q2; the fleet's is Mini), or a systemd service and timer on Linux, and
nothing for any other host: subscription accounts are read by the central probe (EXP-002) — the fleet's existing shapes
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
| undocumented sources | off by default, on in the fleet's config (§10 Q4); every row they produce carries `probe.source = "undocumented"`; a 4xx from them degrades to `unknown`, never to a false all-clear (LIT §11) |

## 7. What else earns its place — and what is left out

| keep | why | where |
|---|---|---|
| **reset countdown and pace** (`window_pct`, `reset_at`; "at this rate the 7-day window empties at HH:MM") | an agent defers a long review it cannot finish (LIT §10) | in the record + `status` table; E2 |
| **`--check` exit codes** | the loop skills stop claiming when the row says so; scripts need no JSON parsing | E1 |
| **spend vs budget** — per host via `ccusage --json`, per provider via balance deltas | the Captain's original question was "usage left" | E5, reads `ccusage`, never re-parses JSONL |
| **key expiry / rotation reminders** — `expires = "2026-12-31"` per service → `degraded` 14 days out | a dead key is otherwise found by a failed call | E5, config-only |
| **model drift sweep** — `GET /v1/models` (where offered) diffed against configured ids; a 404 on a configured model is already `model_missing` | names drift (`kimi-k2.5`), nothing probes live ids (LIT §8.6) | E5 |
| **tmux session census from the hub** — `quotabus census` reads `tmux ls -F` per host over SSH and publishes `census/1.0`-shaped rows (the dead `fleet-session-census.sh` contract: `{identity, host, written_at, cap, live, sessions[{name,kind,age_s}]}`, read on the bus today) | the Captain named M5 as the hub; the row contract already exists | a SEPARATE optional module behind a feature flag, its own expedition; not in the core's config model |
| **NATS services registration** (`$SRV.PING/INFO/STATS`) for the central probe | `nats micro ls` lists a live probe without a bespoke heartbeat (LIT §6) | later; the per-host agent it was first meant for is dropped (EXP-002) [unverified: async-nats `service` feature name] |

**Deliberately out:** routing or proxying calls; pausing providers or switching accounts (the dead script's one act,
`:296-315` — actuation belongs to the reader); cookie-based reads of claude.ai / chatgpt.com (AIQuotaBar's method);
account pooling (CLIProxyAPI's ToS exposure); Prometheus exposition (a `/metrics` is a 40-line contribution if anyone
wants it); a persistent history store (the bus keeps `history = 1`; trends are `ccusage`'s or Grafana's job).

## 8. FOSS

| | |
|---|---|
| **name candidates** | **`quotabus`** (recommended, and chosen by the Captain 2026-10-08, §10 Q1: free on a GitHub exact-name search and `crates.io` 404, 2026-10-08; says bus + quota) · `llm-keywatch` (free) · `ai-status` [unverified]. Taken: `modelwatch` (wkwan, 28★), `keypulse`, `keystatus`, `provider-pulse` (`gh search repos --match name`) |
| **org** | **`Congruentsys`** (where `arrow-kanban` lives, MIT "Copyright (c) 2026 Congruentsys"; recommended, and chosen by the Captain 2026-10-08, §10 Q1 — on file as paraphrase only, `kanban-work/voyages/VOY-001-*.md:19`). The other option offered was `hankh95` (yurtle, yurtle-kanban, nusy-kanban). Amended per `reviews/CHORE-006-r1.md:93-99` |
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
| **E2** | expedition | central subscription reads in `quotabus probe` (the Captain's rescope, 2026-10-08): every Claude account by its Doppler setup-token — the direct `unified_headers` read, the `stream_json` fallback — and Copilot via `copilot_internal/user`, on `[intervals] subscription` (1 h) with its own TTL; reset countdown and pace; nothing installed on the other hosts | six `subscription.anthropic.*` rows and one `subscription.github.*` (Copilot) row on the bus, all `observed_by` the probe host, each with its windows, resets and age; a bad-token control reads `auth_failed`, never `ok` |
| **E3** | expedition | `select` library + CLI; change subjects; `pairit` / external-review doc wiring ("the reviewer is `$(quotabus select --role review --exclude-family anthropic)`") | the selector refuses a family the author uses; rc 3 when every candidate is `unknown`; unit tests without a bus |
| **E4** | expedition | `alert` with crossing-dedup; nusy-kanban, yurtle-kanban, webhook sinks; the Yurtle front-end (`yurtle-table` reader) and `examples/` | a Kimi 404 files ONE signal across ten cycles; a measured `ok` re-arms it; a CANNOT-ASSESS does not; the Yurtle and TOML examples load to identical configs |
| **E5** | expedition (optional) | `serve`; spend via `ccusage --json`; key expiry; model drift sweep; file backend polish for outsiders | one page on `127.0.0.1`; a `expires` 14 days out reads `degraded` |
| **E6** | expedition (optional, separate module) | tmux session census from the hub | `census/1.0` rows for five hosts, behind a feature flag |

E1 is the first slice because it replaces the hand probe immediately and nothing in E2–E6 can be designed against a
bus that holds no rows. E2 before E3: the selector is only as honest as the subscription rows it refuses on.

## 10. Questions for the Captain, and the rulings

Each question keeps the recommendation an agent offered; the Captain's 2026-10-08 ruling follows it, verbatim from the
comment on its signal in `kanban-work/signals/` (this repo), with whether it is that recommended default. Q1, Q6 and Q11
are on file as paraphrase only (Origin note).

1. **Name and org.** `quotabus` under `Congruentsys` (beside arrow-kanban) — *recommend yes*; `hankh95` if it should sit with yurtle/yurtle-kanban.
2. **Where the central probe runs.** Mini, as a prebuilt binary under launchd (no build on the bus host) — *recommend yes*; M5 runs `agent` + the hub's `status`/`serve`.
   **Captain 2026-10-08:** "If this is FOSS, the config will have to ask to run locally or on another host. In this case, run on mini" (`kanban-work/signals/SIG-001-*.md:26`). **Not the recommended default:** where the central probe runs is a config choice made at install (this host, or a named other host); the fleet sets Mini, as a prebuilt binary under launchd, as recommended. Built by CHORE-008.
3. **Publish balances and account labels to the bus.** Numbers yes, labels as configured slugs, never emails — *recommend yes*; `publish_balance` stays a per-service switch for outsiders.
   **Captain 2026-10-08:** "Numbers + slugs" (`kanban-work/signals/SIG-002-*.md:26`). **The recommended default.**
4. **Undocumented sources** (Anthropic `oauth/usage`, Copilot `copilot_internal/user`, z.ai biz endpoints and `/api/monitor/usage/quota/limit`). Off by default in the FOSS config; **on in our fleet config**, every row labelled `source = undocumented` — *recommend that split*.
   **Captain 2026-10-08:** "Off FOSS, on fleet" (`kanban-work/signals/SIG-003-*.md:30`). **The recommended default.**
5. **Cadence and spend.** API probes every 15 min (≤ 96 × ~30 tokens per model per day), subscription reads every 5 min (free), balance GETs every 15 min — *recommend these; make them config*.
   **Captain 2026-10-08:** "Slower - twice a day, but make it a config setting"; on scope: "API + balance only" (`kanban-work/signals/SIG-004-*.md:26`). Then, adding to it: "for how often to query, make sure there are separate times for the different types of queries." (`kanban-work/signals/SIG-004-*.md:43`). **Not the recommended default:** API probes and balance reads default to every 12 h (≤ 2 × ~30 tokens per model per day), subscription reads stay at every 5 min, and each query kind has its own config interval, set independently — none is shared (§3, §4). Built by CHORE-007.
   **Later, Captain 2026-10-08, on EXP-002:** subscription cadence "Hourly" (`kanban-work/expeditions/EXP-002-*.md`): `[intervals] subscription` defaults to 1 h, with its own TTL.
6. **Does it act?** The dead script paused a provider at a measured zero. *Recommend no*: publish + alert only; the loop skills and `select` are the actuator, which keeps the tool safe to FOSS.
7. **TOML first, Yurtle in E4** (the Captain's words lead with Yurtle). *Recommend TOML first*: it lands E1 fastest and is what outsiders expect; Yurtle follows as the same rows in a `yurtle-table` block, which Obsidian and `yurtle-rdflib` read today.
   **Captain 2026-10-08:** "TOML first, Yurtle in E4" (`kanban-work/signals/SIG-005-*.md:26`). **The recommended default.**
8. **Alert item type.** nusy-kanban `signal` with tag `provider-status`, one per crossing, never auto-closed — *recommend signal*; `hazard` only if the Captain wants a crossing to block work.
   **Captain 2026-10-08:** "Signal per crossing" (`kanban-work/signals/SIG-006-*.md:26`). **The recommended default.**
9. **A `statusLine` entry in every host's `~/.claude/settings.json`** — a fleet-wide Claude config change to get the official per-host read; the `~/.claude.json` fallback works without it. *Recommend yes*, installed by E2's unit, measured first by C2. The Captain's first answer, a "yes" on 2026-10-08, is on file only as paraphrase and was conditional on C2 confirming the hook fires under `claude -p` (`kanban-work/voyages/VOY-001-*.md` "Captain's rulings"). C2 (CHORE-002) measured that precondition **false** (§4 Claude Max row; `docs/findings/CHORE-002-statusline-under-claude-p.md`), so it was re-asked as SIG-008, whose recommendation was: install the statusLine for interactive hosts, add the stream-json `rate_limit_event` as a second source, keep the `~/.claude.json` fallback (`kanban-work/signals/SIG-008-*.md:25-26`).
   **Captain 2026-10-08:** "Both + fallback" (`kanban-work/signals/SIG-008-*.md:32`). **The recommended default (SIG-008's):** E2 installs the statusLine for interactive hosts, also captures the stream-json `rate_limit_event` from `claude -p` runs into the same cache, and keeps the `~/.claude.json` `cachedUsageUtilization` fallback.
   **Later, Captain 2026-10-08, rescoping EXP-002:** "Doppler has the API keys for all agents (includeing all of the claude agents that run on each machine. We should not need to run anything else on the other machines (just Mini or M5 depending on where we host this)" (`kanban-work/expeditions/EXP-002-*.md`). This replaces the per-host statusLine, its stream-json tee and the `~/.claude.json` fallback: every Claude account is read centrally (§4), and nothing is written to any host's `~/.claude/settings.json`.
10. **Admin keys** (Anthropic Admin API needs an org; OpenAI cost needs an admin key). *Recommend defer*: for these two the probe IS the status; revisit with E5's spend view.
   **Captain 2026-10-08:** "Defer to E5" (`kanban-work/signals/SIG-007-*.md:26`). **The recommended default.**
11. **Repo membership on the board.** A new voyage under the IDEA's refine (E1–E4 = 4 expeditions + 2 chores, a legitimate voyage), or chores under an existing campaign? *Recommend a voyage*, dual-tracked per §8.
