> **Origin.** nusy-product-team `research/literature/LIT-13334-ai-provider-status-monitor-prior-art.md` (commit `1064922dc7`, 2026-10-08), copied as the prior-art review behind this repo.

⚠ **TOOLING, FIRST LINE:** `WebSearch` and `WebFetch` were used for every external fact (all checked **2026-10-08**); `gh api` for licence / last-push / latest-release of every repository; the `nats` CLI against Mini's bus for the live KV facts; `git` in the M5 worktree for the fleet's own prior art. Nothing was installed or run against a provider. No probe was run. Items read: `IDEA-13333`, `LIT-13334`, `CH-10224`, `CH-10231`.

# LIT-13334 — Prior art for an AI provider & account status monitor: what exists, what we can adopt, and the gaps nobody fills (for IDEA-13333)

**M5, 2026-10-08.** Captain-directed. Read against `docs/external-review.md` §1/§1a (the hand probe of 2026-09-24 and the Copilot *"exceeded your monthly quota"* failure), the Mini KV bucket list, and the fleet's own deleted balance instrument (`scripts/fleet/provider-balance-scan.sh`, Commit C).

**Citation legend — every citation carries exactly one mark:**

| mark | meaning |
|---|---|
| **[WEB]** | fetched or searched this session, 2026-10-08; URL given |
| **[GH]** | `gh api repos/<owner>/<repo>` (+ `/releases/latest`) on 2026-10-08: SPDX licence · last push · latest release |
| **[REPO]** | this monorepo at `origin/main` (`298cb792cb`) or its history, read this session |
| **[BUS]** | measured on Mini's NATS (`192.168.8.110:4222`) this session |
| **[unverified]** | stated from a search snippet or memory only; not confirmed at the source |
| **[INFERENCE]** | this document's own conclusion |

---

## ⚠ The abstract — the answer in ten lines

1. **No FOSS tool does the whole of IDEA-13333.** The closest family is the 2026 wave of *coding-subscription quota trackers* (CodexBar, openusage, usagebar, agent-quota, AIQuotaBar): they already hold the hard-won provider adapters — Claude's OAuth usage endpoint, Copilot's internal quota endpoint, DeepSeek/Kimi/OpenRouter balances — but every one is a **single-user desktop app** (menu bar, tray, TUI) with at most a `127.0.0.1` HTTP API; none publishes off-host, none has freshness semantics for a machine reader, none files an alert into a work tracker, none offers a selector.
2. **The fleet built this once and it died with Layer A.** `provider-balance-scan.sh` (CH-10224/CH-10231/CH-10242, 2026-08-25) published `provider_balance_<p>` rows with `verdict ∈ {OK, LOW, EXHAUSTED, CANNOT-ASSESS}`, `checked_at`, `observed_by` into KV `fleet_alert_state`; `account_util_<host>` / `account_usage_<login>` rows carried Claude 5-hour/7-day percentages per host **[BUS] [REPO]**. Commit C (`f8b7f2f531`, 2026-09-07) deleted the scripts; the rows are 31–44 days stale and only `scripts/fleet/provider-balance.conf` survives **[REPO]**. The row contract is reusable prior art.
3. **Balance / quota endpoints exist for some providers and not others — so a probe is the only signal for the rest** (§2): DeepSeek and Kimi have documented balance endpoints; OpenRouter has per-key and per-account credit endpoints; Anthropic's Admin API gives historical usage/cost and configured limits but is *"unavailable for individual accounts"* and has no prepaid-balance endpoint; OpenAI has cost reports and no balance endpoint; z.ai has no documented balance endpoint; Copilot's personal quota is readable only through an internal endpoint the official SDK wraps.
4. **Claude Max state is readable per host two ways:** officially, the statusline stdin JSON (`rate_limits.five_hour.used_percentage` / `.resets_at`, `seven_day`, `spend_limit`; Pro/Max only); unofficially, `GET api.anthropic.com/api/oauth/usage` with the OAuth token (every tracker uses it; it 429s without the right `User-Agent`). The per-host capture pattern already exists (arturl95/claude-usage-monitor, 4 commits, MIT).
5. **Generic uptime monitors can host the API-key half but not the account half.** Gatus (Apache-2.0) has JSONPath conditions, header/body per endpoint, env-var secrets, a JSON API, Prometheus metrics, pushed "external endpoints" and a custom webhook alerter — but no NATS, no OAuth/keychain readers, no quota semantics. Uptime Kuma, blackbox_exporter and Alertmanager are the same shape with weaker conditions. None of them, nor Apprise (MQTT yes, NATS no), can publish to NATS.
6. **LiteLLM is a gateway, not a monitor:** `/health` makes a real call per model and reports healthy/unhealthy endpoints; the router's `allowed_fails` / `cooldown_time` is the right model for a selector, but cooldown state is not exposed and keys/budgets need Postgres. Its *semantics* are worth copying; the process is not worth running for this.
7. **The secret-source plugin layer should not be written:** `secretspec` (Apache-2.0, Rust, v0.21.1 2026-09-27) already unifies keyring/Keychain, 1Password, Doppler, `pass`, Vault, SOPS, Bitwarden, KeePass, Azure KV behind a `secretspec.toml` declaration and a Rust SDK; `vals` (Go, 30+ backends) and `teller` (Rust, slower cadence) are the alternatives.
8. **NATS gives freshness for free, once asked for.** Server 2.11 added per-message TTL and limit markers; Mini runs **2.12.4** but every existing bucket reports *"Per-Key TTL Supported: false"* — a new bucket must be created with the TTL option, and readers must treat an absent or expired key as UNKNOWN, never as healthy **[BUS]**.
9. **Nobody found publishes LLM provider/account status to NATS, and nobody offers a selector that answers "a healthy model for role=review, not family=anthropic".** Routers (LiteLLM, Portkey, Bifrost, CLIProxyAPI, claude-code-router) select *inside* a proxy you must call through; none exposes selection as a status query.
10. **Verdict: BUILD a thin core (probe runner → NATS KV with TTL → selector → kanban hook), ADOPT the adapters as a catalogue (the trackers' provider docs + our `provider-balance.conf` contract), ADOPT `secretspec` as the key-source layer, ADOPT the statusline capture for Claude Max and `ccusage` for spend; EXTEND nothing — Gatus is the one host worth naming as the fallback, and it carries only the key half.** Detail in §8–§10.

---

## 0. What we need, stated as requirements (from IDEA-13333)

| # | requirement | note |
|---|---|---|
| R1 | probe API keys: reachability, auth (401), model-name drift (404), quota/rate-limit (429 + headers), balance where exposed, latency; minimal-token calls | `docs/external-review.md` §1 already prescribes the 8-token smoke **[REPO]** |
| R2 | per-host coding-subscription state: Claude Max per machine (5 hosts, 5 logins), Copilot per `gh` login | the `claude` auth probe lies over SSH on macOS, so it must run locally **[REPO: IDEA]** |
| R3 | publish to NATS KV with freshness; stale reads UNKNOWN; a subject per change | 35 buckets live on Mini today, most last updated ~31 days ago **[BUS]** |
| R4 | pluggable secret sources (Doppler, 1Password, Keychain, pass, Vault, SOPS, env); never publish a key | |
| R5 | config as Yurtle (markdown + Turtle), with TOML/YAML for outsiders | |
| R6 | outputs: CLI table, optional UI, events → kanban (`nusy-kanban` / `yurtle-kanban`) | |
| R7 | a selector: "healthy model for role=review excluding family=anthropic" | the reviewer-family rule of `external-review.md` §0 **[REPO]** |
| R8 | budgets/spend vs threshold; key expiry reminders | |

## 1. LLM gateways / proxies with health checks or key management

| tool | licence · last push · release **[GH]** | what it does for us | what it lacks |
|---|---|---|---|
| **LiteLLM proxy** — https://github.com/BerriAI/litellm | MIT with an `enterprise/` carve-out (LICENSE file **[WEB]**) · 2026-10-08 · v1.104.2 2026-10-08 | `GET /health` runs *"a real test request against every configured model"* and returns `healthy_endpoints` / `unhealthy_endpoints` with error text; `background_health_checks: true`, `health_check_interval: 300` caches the result **[WEB: docs.litellm.ai/docs/proxy/health]**. Router: `allowed_fails` → `cooldown_time` (default 5 s), provider failover, context-window and content-policy fallbacks; provider budgets (`budget_limit` USD/day) **[WEB: docs/routing, docs/proxy/provider_budget_routing]**. 100+ provider adapters. | A gateway you must call *through*; virtual keys/budgets need Postgres; cooldown state is not exposed by any endpoint **[WEB]**; no subscription-account view (issue #18242 asks for upstream Copilot limits **[WEB]**); health = a paid call per model per interval |
| **Portkey Gateway** — https://github.com/Portkey-AI/gateway | MIT · 2026-05-25 · v1.15.2 2026-01-12 | fallbacks, retries, load-balancing, conditional routing, 1,600+ models **[WEB]** | routing only; no health/quota publication; Node runtime; release cadence slowed |
| **Bifrost** — https://github.com/maximhq/bifrost | Apache-2.0 · 2026-10-08 · ent-v2.2.6-base 2026-10-06 | Go gateway: adaptive load balancer, virtual keys, budgets **[WEB]** | same shape as LiteLLM; nothing account-facing |
| **Helicone AI Gateway** — https://github.com/Helicone/ai-gateway | **GPL-3.0** · 2025-11-21 · no release | — | stale; copyleft would bind a FOSS app that links it |
| **Kong** (`ai-proxy`) — https://github.com/Kong/kong · **Envoy AI Gateway** — https://github.com/envoyproxy/ai-gateway | Apache-2.0 · 2026-10-08 · 3.9.3 2026-06-17 · · Apache-2.0 · 2026-10-08 · v1.2.0 2026-10-07 | infrastructure gateways with generic upstream health checks | quota/account semantics absent [unverified: health-check detail not fetched] |
| **OpenRouter** (hosted, not FOSS) — https://openrouter.ai/docs/api_reference/limits | — | `GET /api/v1/key` with the ordinary key: `limit`, `limit_remaining`, `usage`, `is_free_tier`, `free_model_daily_requests{used,limit,remaining}`; `GET /api/v1/credits` with a **management key**: `total_credits`, `total_usage`; 402 carries `limit_source` (`openrouter_credits` / `openrouter_key_limit` / `openrouter_in_flight_budget`); 429 carries `X-RateLimit-*` + `Retry-After` **[WEB]** | a model-level reference for what a good status API looks like; we would probe it like any provider |

**Takeaway [INFERENCE]:** gateways solve *routing*, and their health checks are a by-product priced in real calls. The one thing to copy is LiteLLM's cooldown vocabulary (`allowed_fails`, `cooldown_time`, per-deployment not per-group) for R7.

## 2. What the providers expose — and where a probe is the only signal

| provider | balance / credit | usage & cost (historical) | live rate-limit / quota signal | verdict for us |
|---|---|---|---|---|
| **Anthropic API** | **none** found for prepaid credit [unverified: absence] | Admin API `GET /v1/organizations/usage_report/messages` (1m/1h/1d buckets, group by key/workspace/model), `GET /v1/organizations/cost_report` (USD, 1d); Admin key or `org:admin` OAuth; *"unavailable for individual accounts"*; data within ~5 min; poll ≤ 1/min **[WEB: platform.claude.com/docs/en/manage-claude/usage-cost-api]** | every Messages response: `anthropic-ratelimit-{requests,tokens,input-tokens,output-tokens}-{limit,remaining,reset}`, `retry-after`; spend-cap 429 has `error.details.error_code: enforced_spend_limit_reached` and **no** `retry-after` **[WEB: …/api/rate-limits]**. `GET /v1/organizations/rate_limits` returns *configured* limits per model group (not utilisation) **[WEB: …/manage-claude/rate-limits-api]** | probe + headers; Admin API only if we form an org |
| **Claude Max / Pro (subscription)** | n/a | `ccusage`-class log readers estimate spend from `~/.claude/projects/**/*.jsonl` | **official:** statusline stdin `rate_limits.five_hour.{used_percentage,resets_at}`, `.seven_day`, `.spend_limit` — *"appears only for claude.ai Pro and Max subscribers … only after the first API response"*; a window is dropped once `resets_at` passes **[WEB: code.claude.com/docs/en/statusline]**. **unofficial:** `GET https://api.anthropic.com/api/oauth/usage` (Bearer OAuth token from `~/.claude/.credentials.json` or Keychain item `Claude Code-credentials`, `anthropic-beta: oauth-2025-04-20`, `User-Agent: claude-cli/<ver>`), fields `five_hour`, `seven_day`, `seven_day_opus/sonnet`, `extra_usage`, `utilization` 0–100, `resets_at` **[WEB: CodexBar docs/claude.md; claude-code-statusline README]**; persistent 429s reported for Max users (anthropics/claude-code#30930 **[WEB]**) | per-host, local only (R2); official hook first, OAuth endpoint as fallback, both marked by source |
| **OpenAI API** | **none** — *"no API endpoint … to check the USD credit balance"*; legacy `/v1/dashboard/billing/credit_grants` is undocumented **[WEB: community.openai.com threads]** | `GET /v1/organization/costs`, `/v1/organization/usage/*` — **admin key** required **[WEB: developers.openai.com cookbook]** | `x-ratelimit-*` headers [unverified: not fetched] | probe; cost report if an admin key is provisioned |
| **DeepSeek** | `GET https://api.deepseek.com/user/balance` → `is_available`, `balance_infos[]{currency, total_balance, granted_balance, topped_up_balance}` **[WEB: api-docs.deepseek.com]** | — | 429 / suspension (the 2026-08-25 incident, CH-10224 **[REPO]**) | balance row (our conf already has it) |
| **Moonshot / Kimi** | `GET https://api.moonshot.ai/v1/users/me/balance` → `data.{available_balance, voucher_balance, cash_balance}`; ≤ 0 ⇒ `exceeded_current_quota_error` **[WEB: platform.kimi.ai/docs/api/balance]** | — | 404 on retired model ids (`kimi-k2.5`, 2026-09-24 **[REPO]**) | balance row + model-drift probe |
| **z.ai / GLM** | **no documented endpoint**; our conf: *"three candidates 404, measured 2026-08-25"* **[REPO]**; usagebar reads undocumented `GET /api/biz/subscription/list` and `GET /api/monitor/usage/quota/limit` with the API key (Coding-Plan 5-hour/7-day token quotas) **[WEB: usagebar docs/providers/zai.md]**; Coding-Plan quota is only spendable inside supported tools, general calls bill the wallet **[WEB]** | dashboard only | 429/401 | probe; optional undocumented adapter flagged as such |
| **GitHub Copilot** | n/a | official REST: `GET /users/{username}/settings/billing/premium_request/usage` — *"only applicable if the user has purchased their own Copilot plan"*; org/enterprise variants; per-user not exposed for enterprise-owned orgs **[WEB: docs.github.com/en/rest/billing/usage; community #184208]** | `GET https://api.github.com/copilot_internal/user` → `quota_snapshots.premium_interactions{entitlement, remaining, percent_remaining, credits_used, reset_date}` — the call the Copilot CLI makes at start (`gh api copilot_internal/user`) **[WEB]**; the Copilot SDK documents the same data as `account.getQuota()` → `entitlementRequests` (−1 = unlimited), `usedRequests`, `remainingPercentage`, `resetDate` **[WEB: docs.github.com copilot-sdk usage-and-billing]** | internal endpoint via the host's `gh` token, per login (R2); *"GitHub does not publish an included-credit entitlement on any documented endpoint"* (CodexBar) **[WEB]** |

**Takeaway:** two of our four metered providers (DeepSeek, Kimi) and OpenRouter have real balance endpoints; z.ai, OpenAI and the Anthropic prepaid balance have **none**, so for them *the probe is the status*. Subscription state (Claude Max, Copilot) is readable only on the host that holds the login.

## 3. Claude Code / coding-assistant usage and quota trackers (the closest prior art)

| tool | licence · push · release **[GH]** | platform · how it reads | has | lacks for us |
|---|---|---|---|---|
| **CodexBar** — https://github.com/steipete/CodexBar | MIT · 2026-10-08 · v0.73.0 2026-10-07 · 22.3k★ | macOS 14+ menu bar, Linux Qt, bundled CLI, `codexbar serve` local HTTP; **92 providers** (Claude, Codex, Copilot, Cursor, OpenRouter, DeepSeek, Kimi, z.ai, LiteLLM, Ollama…); reads local config/SQLite, browser cookies (opt-in), OAuth/device flow, API keys **[WEB]** | the richest adapter catalogue; per-provider docs (`docs/claude.md`, `docs/copilot.md`, `docs/deepseek.md`, `docs/openrouter.md`) spell out endpoints and fields | single user, single host; Swift app; no central publish, no TTL, no alerts to a tracker, no selector |
| **openusage** — https://github.com/robinebers/openusage | MIT · 2026-10-06 · v0.7.14 2026-10-06 · 4.3k★ | macOS 15; `ProviderRuntime` protocol (auth → fetch → normalise); `openusage` one-shot CLI with JSON; local HTTP `127.0.0.1:6736/v1/limits` **[WEB]** | a clean normalised-record model to copy | macOS only; local only |
| **usagebar** — https://github.com/luisleineweber/usagebar | MIT · 2026-08-28 · v0.1.1 2026-08-20 | Windows-first tray; 40+ providers incl. **Z.ai, Kimi Code, DeepSeek**; local HTTP API with bearer **[WEB]** | the z.ai undocumented endpoints | Windows; pre-release |
| **agent-quota** — https://github.com/torridfish/agent-quota | MIT · 2026-10-05 · no release · 4★ | **Python 3.11 TUI/CLI**, headless (`--watch`, `--only`, exit codes); Claude, Codex, Copilot, OpenCode, Z.ai, OpenRouter, DeepSeek, Kimi; cookies for Claude/Codex, keys for the rest **[WEB]** | the nearest "headless probe set" in a language the fleet uses | one host, stdout only; cookie-based Claude read |
| **AIQuotaBar** — https://github.com/yagcioglutoprak/AIQuotaBar | MIT · 2026-10-04 · v2.1.1 2026-10-04 | Python, macOS 12+; Claude/ChatGPT/Cursor/Copilot via browser cookies; `~/.claude` stats **[WEB]** | — | cookies; desktop |
| **ccusage** — https://github.com/ccusage/ccusage | MIT (LICENSE file **[GH]**; API says NOASSERTION) · 2026-10-08 · v20.0.26 2026-09-27 · 18.9k★ | reads local JSONL of **18** agent CLIs (Claude Code, Codex, Copilot CLI, Gemini CLI, Kimi, Qwen…); `daily/weekly/monthly/session/blocks`; `blocks` = 5-hour billing windows; `statusline`; `--json` **[WEB]** | spend estimation per host (R8), JSON out | *"reads token logs only"* — no rate-limit state, no publish |
| **Claude-Code-Usage-Monitor** — https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor | MIT · 2026-07-05 · v4.0.0 2026-06-27 · 8.7k★ | Python terminal monitor over the same logs; issue #202 asks to read the OAuth usage endpoint as *"authoritative window state"* **[WEB]** | — | estimates, not server state |
| **claude-usage-monitor** — https://github.com/arturl95/claude-usage-monitor | MIT · 2026-10-01 · 4 commits | **bash + jq**; a statusline hook writes `rate_limits` to `~/.claude/usage-state/<session>.json`; `claude-usage --json`, `--check` exit 0/3; injects a warning into the agent at 90 %/97 % **[WEB]** | exactly the per-host official capture pattern for R2 | tiny, local |
| **claude-code-statusline** — https://github.com/ohugonnot/claude-code-statusline | MIT · 2026-06-10 · v1.6.0 | stdin `rate_limits` first, `/api/oauth/usage` fallback, *"not an official API — it could change without notice"* **[WEB]** | the two-source pattern | display only |
| **ghcp-spend-tray** — https://github.com/DamianEdwards/ghcp-spend-tray | MIT · 2026-10-07 · v0.4.0 2026-10-04 | Copilot premium-request spend tray; issue #18: personal plans and *"unsupported quota responses"* **[WEB]** | the Copilot edge cases | desktop |
| **CLIProxyAPI** — https://github.com/router-for-me/CLIProxyAPI | MIT · 2026-10-08 · 54.5k★ | multiplexes OAuth coding accounts (Claude/Codex/Gemini) behind one OpenAI-compatible API; a fork's issue #4 asks for *"quota-aware ranking"* from the OAuth usage endpoint **[WEB]** | proof the "pick the account with headroom" need is live | a proxy; account pooling against ToS risk is theirs, not ours |
| **ccflare** — https://github.com/snipeship/ccflare · **claude-code-router** — https://github.com/musistudio/claude-code-router | MIT · 2026-04-19 · 1.0k★ · · MIT · 2026-10-08 · 37.6k★ | Claude account load-balancer; Claude Code → other providers router | routing, no status publication |

**Takeaway:** the adapters are solved many times over, in Swift, Python, TypeScript and bash; the *fleet* shape (N hosts → one bus → agents read) is solved by none.

## 4. Generic uptime / status monitors as a probe host

| tool | licence · push · release **[GH]** | can it carry an auth'd API probe with quota parsing? | can it publish to NATS? |
|---|---|---|---|
| **Gatus** — https://github.com/TwiN/gatus | Apache-2.0 · 2026-10-07 · v5.37.0 2026-09-24 · 12.3k★ | **Yes, best of the family:** per-endpoint `headers` and `body`; `${VAR}` substitution for secrets; conditions on `[STATUS]`, `[RESPONSE_TIME]`, `[BODY].json.path` with `len()`, `has()`, `pat()`, `any()`; results via `/api/v1/…` JSON and `/metrics`; **external endpoints** (`POST /api/v1/endpoints/{key}/external`, bearer) with heartbeat *"for detecting stalled updates"* **[WEB: README]** | No NATS alerter; alerting list is chat/pager providers plus **custom webhooks** **[WEB]** — a bridge would be a webhook → NATS shim |
| **Uptime Kuma** — https://github.com/louislam/uptime-kuma | MIT · 2026-10-08 · 2.5.5 2026-09-16 · 92.2k★ | HTTP(s) **JSON Query** monitor (`jsonPath` + expected value), request headers/body/auth; push monitors; Prometheus metrics; socket.io API **[WEB: wiki API-Documentation; search]** | No NATS among *"90+ notification services"* **[WEB]**; UI-and-SQLite config, not GitOps |
| **Statping-ng** — https://github.com/statping-ng/statping-ng | **GPL-3.0** · 2025-06-04 · v0.93.0 2025-06-04 | basic HTTP checks | stale and copyleft — discard |
| **Prometheus blackbox_exporter + Alertmanager** — https://github.com/prometheus/blackbox_exporter · https://github.com/prometheus/alertmanager | Apache-2.0 · 2026-10-08 · v0.29.0 2026-10-07 · · Apache-2.0 · v0.34.1 2026-09-17 | http prober: `headers`, `authorization`/`basic_auth`, `body`, `fail_if_body_matches_regexp`, `fail_if_body_json_matches_cel` / `…not_matches_cel` **[WEB: CONFIGURATION.md]**; but the exposed metric is `probe_success` — parsed values (a balance) do not become metrics | Alertmanager → webhook only; a Prometheus stack is heavier than the thing being monitored [INFERENCE] |
| **Apprise** — https://github.com/caronc/apprise | BSD-2-Clause · 2026-10-05 · v2.0.1 2026-10-03 | notification fan-out: `json://` webhook, `ntfy://`, **`mqtt://` yes, NATS no** **[WEB]** | useful for human alerts beside the bus |
| **llm-latency-tracker** — https://github.com/mazamaka/llm-latency-tracker | code MIT, data CC-BY-4.0 · 2026-10-08 | public TTFB/TTFT across 46 provider APIs from 4 regions; JSON API + MCP **[WEB]** | public health, *"not account credentials, quota status"* — a complement, not a substitute |

**Takeaway:** Gatus could host R1 (key probes, model-drift 404s, balance JSONPath floors) with no code, and its external-endpoint heartbeat is a freshness primitive. It cannot do R2 (OAuth/keychain reads on five hosts), R3 (NATS), R6 (kanban) or R7 (selector).

## 5. Secret-manager abstraction layers

| tool | licence · push · release **[GH]** | backends (as named by the project) | fit |
|---|---|---|---|
| **secretspec** — https://github.com/cachix/secretspec | Apache-2.0 · 2026-10-08 · v0.21.1 2026-09-27 · 1.6k★ | **keyring** (system keychain, recommended), **onepassword**, **doppler** (0.21+), **dotenv**, **pass** (GPG), protonpass, **vault**, **sops** (0.17+), bw / bws (Bitwarden), kdbx (KeePass), keeper, lastpass, dashlane, **akv** (Azure KV); declaration in a committed `secretspec.toml`; *"Type-Safe Rust SDK"* **[WEB: README via gh api]** | **Best fit for R4:** every source IDEA-13333 lists, Rust, Apache-2.0, active |
| **vals** — https://github.com/helmfile/vals | Apache-2.0 · 2026-10-06 · v0.47.0 2026-09-21 | Go library + CLI; `ref+vault`, `ref+sops`, `ref+doppler`, `ref+op` / `onepasswordconnect`, `ref+keychain`, `ref+awssecrets`, `ref+azurekeyvault`, `ref+bitwarden`, `ref+infisical`, `ref+envsubst`, `ref+file`, `ref+exec`, 30+ **[WEB]** | the Go alternative; URI-reference style |
| **teller** — https://github.com/tellerops/teller | Apache-2.0 (LICENSE.txt **[GH]**) · 2026-01-27 · v2.0.7 **2024-05-20** · 3.2k★ | Rust; Vault, AWS SM/SSM, Google SM, Consul, dotenv, *"and many more"*; `teller run`, `teller sh`, **`teller redact`** (scrub secrets from streams) **[WEB]** | the redaction idea is worth copying; release cadence is slow |
| **novops** — https://github.com/PierreBeucher/novops | **LGPL-3.0** · 2025-06-21 · v0.20.1 2025-06-21 | Vault, AWS, GCloud, Azure KV, SOPS, Bitwarden; 1Password *planned* **[WEB]** | copyleft and no Doppler/Keychain — discard |
| **keyring** (Python) — https://github.com/jaraco/keyring | (SPDX none via API; MIT [unverified]) · 2026-04-13 · v25.7.0 2025-11-16 | macOS Keychain, Windows Credential Locker, Secret Service | the Python route if the app is Python |
| **Doppler CLI** — https://docs.doppler.com/docs/cli | CLI open source at DopplerHQ/cli **[WEB]** (licence [unverified]) | `doppler run -- <cmd>`, `doppler secrets get NAME --plain`, service tokens, `--scope` per directory **[WEB]** | our three traps (`$HOME`, scopes, hosts without a login) are documented in `external-review.md` §2 **[REPO]** |
| **1Password CLI** — https://www.1password.dev/cli/secrets-environment-variables/ | proprietary CLI [unverified] | `op run --env-file`, `op://vault/item/[section/]field` references **[WEB]** | reachable through secretspec/vals rather than directly |

## 6. NATS-side prior art, and "pick a healthy model" selectors

- **Per-key TTL exists and is not enabled on our buckets.** nats-server v2.11.0 (2025-03-19) added the `Nats-TTL` header and `SubjectDeleteMarkerTTL` **[WEB: release notes]**; Mini's wire `INFO` reports `"version":"2.12.4"`, `"jetstream":true`, yet `nats kv info fleet_daemon_health` and `fleet_alert_state` both print **`Per-Key TTL Supported: false`** **[BUS]** — buckets predate the option. 35 buckets exist; of those listed, most show *Last Update* 30–50 days; `fleet_control` is 1 day old **[BUS]**. The IDEA's "readers cannot tell stale from current" is measured true: the `account_util_M5` row carries `"state":"stale"` *inside* the value, which only a reader that parses it can see **[BUS]**.
- **NATS services framework** (`$SRV.PING`, `$SRV.INFO`, `$SRV.STATS`) gives discovery, liveness and per-endpoint stats for free to any process that registers as a micro-service **[WEB: docs.nats.io/using-nats/developer/services]**; `natscli` v0.5.0 2026-09-17 **[GH]**. A per-host agent registered this way is pingable from the hub without a custom heartbeat.
- **No FOSS tool found by these searches publishes LLM provider or account status to NATS KV** (queries: "NATS KV service health status publisher", "LLM router … NATS", the gateway and monitor READMEs above) — a statement about these searches on 2026-10-08, not about the world.
- **Selectors:** every router (LiteLLM cooldown/fallbacks, Portkey, Bifrost adaptive LB, CLIProxyAPI, claude-code-router, OpenRouter's `models[]` fallback) chooses *while proxying a call*; none answers a status question an agent can ask before choosing its own transport, and none encodes a reviewer-family exclusion (R7).
- **The fleet's own contract** (reusable as-is): `provider_balance_<p>` = `{contract, provider, balance, currency, verdict: OK|LOW|EXHAUSTED|CANNOT-ASSESS, detail, checked_at, observed_by}`; `account-util/1.0` = `{host, spend_account, login_account, uuid_match, five_hour_pct, five_hour_resets_at, seven_day_pct, seven_day_resets_at, cache_age_min, state, reason, observed_at, intended_account, on_intended_account, drift}`; `account-usage/1.1` = `{account, spend_usd, window, sessions, hosts, reset_at_utc, percent_used, percent_used_5h, utilization_state, verdict: ESTIMATED, computed_at, observed_by}` **[BUS]**. Rules already ruled: *"an unreachable balance endpoint is CANNOT-ASSESS, never 'healthy'"*; *"only a MEASURED OK clears the push dedup key"*; report + pause only, never a top-up (CH-10224 **[REPO]**).

## 7. Comparison — requirements × the strongest candidate per family

| | R1 key probes | R2 per-host subscriptions | R3 NATS KV + freshness | R4 secret plugins | R5 Yurtle/TOML config | R6 CLI/UI + kanban | R7 selector | R8 budgets |
|---|---|---|---|---|---|---|---|---|
| **CodexBar / openusage / agent-quota** | partial (balances, quotas; no model-drift sweep) | **yes, but one host, one user** | no | keychain/cookies/env only | no | CLI/JSON, local HTTP | no | spend views |
| **ccusage** | no | spend only, from logs | no | n/a | no | CLI/JSON | no | per-day spend |
| **LiteLLM proxy** | `/health` real calls | no | no | env | YAML | API/UI | **inside the proxy** | **provider budgets** |
| **Gatus** | **yes** (headers, JSONPath floors, 404s) | no | JSON API, `/metrics`, heartbeat on pushed endpoints — **no NATS** | `${ENV}` | YAML | UI + custom webhook | no | floors as conditions |
| **blackbox_exporter + Alertmanager** | yes (CEL on JSON) but only `probe_success` out | no | no | headers | YAML | webhook | no | no |
| **secretspec** | — | — | — | **all of R4** | TOML | — | — | — |
| **our dead `provider-balance-scan` + `account_util`** | balances + verdicts | **yes, 5 hosts** | **KV rows with `checked_at`** (no TTL) | Doppler → `~/.nusy/providers.env` | conf table | push lines; pause actuator | no | floors in native currency |
| **IDEA-13333 target** | ✔ | ✔ | ✔ + TTL | ✔ | ✔ | ✔ | ✔ | ✔ |

## 8. The gaps no existing tool fills for us

1. **Subscription-account status per HOST, published centrally.** Every tracker reads one login on one machine for one human; we have five hosts, five Claude logins, one `gh` keyring per host, and an SSH-blind `claude` auth probe. A per-host agent that captures the statusline `rate_limits` (official) and `copilot_internal/user` (internal), keys the row by host, and publishes it is not in any repo found.
2. **Freshness semantics for an agent reader.** No tool emits "unknown because stale" as a first-class state; our own rows bury `state: stale` in the value. The fix is structural: a bucket created with per-key TTL so an expired key is *absent*, plus `checked_at` and `observed_by` in the value, plus the CANNOT-ASSESS verdict for an unreachable endpoint.
3. **A NATS KV / subject output.** Gatus, Uptime Kuma, Alertmanager and Apprise all stop at webhooks; nothing found writes KV or publishes a subject per change.
4. **Events → kanban.** None files an item; the hook is `nk create --body-file … signal "…"` (nusy-kanban) or a `yurtle-kanban` `create_item` hook (`kanban-hooks.yurtle.md` has `create_item` / `notify` actions; issue #47 asks for *"pluggable transport (beyond NATS)"* **[WEB]**) — dedup across runs is the only hard part, and CH-10224's "push once per crossing, only a measured OK clears it" rule is the answer.
5. **A selector with a family-exclusion rule.** "A healthy model for role=review, not family=anthropic, cheapest first" — a pure function over the KV rows plus a static model table (family, cost class, context). Routers do this inside a proxy; no status-side selector exists.
6. **Model-name drift as a probe.** Deprecation trackers (randomartifact/llm-status, the `llm-model-deprecation` action **[WEB]**) scan *code* against published calendars; nothing probes the live `/v1/models` or a 20-token call per configured model id and diffs the answer against the config.
7. **Yurtle config.** Yurtle v2.1 = YAML or Turtle frontmatter + markdown + fenced `yurtle` blocks; `yurtle-rdflib` v0.3.0 parses it in Python **[WEB; GH]**. Nothing outside the Congruentsys repos reads it; a TOML twin is mandatory for outsiders (secretspec's `secretspec.toml` is the model).

## 9. Recommendation — BUILD / ADOPT / EXTEND

- **BUILD the thin core**, nothing else: a probe runner driven by a config table (our `provider|url|jq_path|currency|floor` row, widened with `kind ∈ {messages-probe, balance, oauth-usage, statusline-capture, copilot-internal}` and a `family`), a NATS publisher (KV bucket created with per-key TTL; one subject per state change), a reader library (`freshness → UNKNOWN`), the selector, the kanban hook with crossing-dedup, and a one-screen CLI table. The three KV contracts in §6 are the starting schema.
- **ADOPT as a catalogue, not as code:** the provider adapter *knowledge* in CodexBar's `docs/<provider>.md`, openusage's normalised record, usagebar's z.ai endpoints, agent-quota's Python set — each MIT; re-implement the dozen HTTP calls rather than embed a Swift or Tauri app.
- **ADOPT `secretspec` for R4** (Apache-2.0, Rust SDK, Doppler/Keychain/pass/1Password/Vault/SOPS/env in one declaration); `vals` if the app is Go; `keyring` if Python. Copy `teller redact`'s idea: scrub every configured secret value from logs and rows before they leave the process.
- **ADOPT the Claude Max capture** as arturl95/claude-usage-monitor does it — a statusline hook writing `rate_limits` to a per-session file the host agent reads — with the OAuth usage endpoint as a flagged fallback; **ADOPT `ccusage --json`** for per-host spend (R8) rather than re-parsing JSONL.
- **EXTEND nothing.** The only host worth naming is Gatus for the API-key half (declarative, JSONPath floors, external-endpoint heartbeat, custom webhook → a NATS shim); it would still leave R2, R3, R6, R7 to build, so it buys little. LiteLLM is not the monitor; copy its cooldown vocabulary for the selector. Helicone's gateway, Statping-ng and novops are copyleft or stale — out.

## 10. What else would make it truly useful (features the good tools have that IDEA-13333's list misses)

| feature | seen in | why it matters for us |
|---|---|---|
| **Reset countdowns and pace** ("at this rate the 7-day window empties at HH:MM") | CodexBar, openusage, AIQuotaBar **[WEB]** | an agent can defer a long review rather than start one it cannot finish |
| **Warn the agent in-context before the cliff** (90 % / 97 % injection) | arturl95/claude-usage-monitor **[WEB]** | the loop skills can read the same KV row and stop claiming new items |
| **Two-source truth with provenance** (official hook vs undocumented endpoint, each labelled) | claude-code-statusline **[WEB]** | the fleet's own *"report the measurement and the command that produced it"* rule |
| **Per-key vs account limit distinction** (`limit_source`) | OpenRouter 402 **[WEB]** | "key capped" and "account empty" need different actions |
| **Background health with a cached last result, cost-bounded** | LiteLLM `background_health_checks` **[WEB]** | probes cost tokens; one probe per model per interval, never per read |
| **External/pushed endpoints with heartbeat** | Gatus **[WEB]** | hosts that cannot be probed from the hub (SSH-blind auth) push their own row; a missed heartbeat is itself a state |
| **Local HTTP + CLI JSON + exit codes** | openusage, agent-quota, usagebar **[WEB]** | scripts and humans read the same thing; `--check` exit codes slot into pre-flight checks |
| **Redaction of secrets from every output stream** | teller **[WEB]** | the "never publish a key" rule becomes a mechanism |
| **Configured-limit sync** | Anthropic Rate Limits API **[WEB]** | when an org exists, read limits instead of hard-coding them |
| **Public provider health as a second axis** | llm-latency-tracker (CC-BY data) **[WEB]** | separates "our key is dead" from "the provider is down" |
| **Services-framework registration** | NATS `$SRV.*` **[WEB]** | the hub can `nats micro ls` the fleet's agents without a bespoke census |

## 11. FOSS positioning

- **Is there a niche?** Yes, and it is specific: *multi-host, agent-readable, bus-published* account status. The desktop trackers (22k★ CodexBar, 4k★ openusage, 19k★ ccusage) prove the demand for the single-user half; CLIProxyAPI (54k★) and the LiteLLM issue asking for upstream limits prove the "which account still has headroom" need at the team scale. Nothing serves a team of agents on several machines.
- **Who else would use it?** Anyone running Claude Code / Codex / Copilot on more than one machine or under more than one login (small teams, CI runners, home labs), anyone fronting metered Chinese providers (DeepSeek, Kimi, GLM) where the only limit event is suspension, and NATS shops that want status on the bus rather than in a menu bar.
- **Licence families we would build on:** permissive throughout — MIT (the trackers, ccusage, Uptime Kuma, LiteLLM core, Portkey, nusy/yurtle-kanban, Yurtle), Apache-2.0 (secretspec, Gatus, blackbox_exporter, Alertmanager, NATS, vals, teller, Bifrost), BSD-2 (Apprise). The GPL/LGPL items (Helicone gateway, Statping-ng, novops) are excluded above, so a permissive licence for the app is unconstrained. Shipping/licence remains a separate question from this R&D (fleet memory: *no licence checks for R&D work*, Captain 2026-09-23).
- **Risk to state plainly:** the two most valuable account reads — Anthropic `/api/oauth/usage` and GitHub `copilot_internal/user` — are **undocumented**; every tracker carries the same risk and labels it. The design must degrade to "UNKNOWN (source gone)" rather than to a false all-clear, which is the CH-10224 rule again.

## 12. Verdict rows for `refine-idea`

| item | verdict |
|---|---|
| "Does a FOSS tool already do this?" | **No.** Partial coverage in three disjoint families; none crosses hosts, none publishes to a bus, none selects |
| Adopt | `secretspec` (R4) · statusline-hook capture (R2, Claude) · `copilot_internal/user` via `gh` token (R2, Copilot) · `ccusage --json` (R8) · the adapter catalogue in the trackers' docs (R1) · our own three KV contracts (R3) |
| Build | probe runner · NATS KV with per-key TTL + change subjects · freshness reader · selector · kanban hook with crossing-dedup · CLI table · Yurtle + TOML config |
| Extend | none required; Gatus is the named fallback host for the key half only |
| Measure first | that Mini's 2.12.4 bucket created with per-key TTL expires a key as expected; that the statusline hook fires on every host under `claude -p`; which z.ai endpoint, if any, answers an API key |
