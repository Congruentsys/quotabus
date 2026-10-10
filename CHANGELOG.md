# Changelog

All notable changes to quotabus are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-10-10

The first release. It meets VOY-001's Definition of Done: `quotabus status` on the hub shows every configured API
key and every subscription account with ages; a stale or missing row reads unknown; `quotabus select --role review
--exclude-family anthropic` returns a healthy model or refuses with rc 3; an outage files exactly one alert. This was
measured on the fleet's live bus in CHORE-016 (#25) and re-read on 2026-10-10.

### Added
- `quotabus probe`: one probe cycle over the configured API services (anthropic and openai protocols), with balance
  adapters for DeepSeek and Kimi, writing one `ai-status/1` row per (service, model) (#2).
- Two backends: a NATS KV bucket created with per-key TTL, or a file backend when `[bus]` is omitted (#2).
- `quotabus status [--json] [--kind] [--check <service>]` with the freshness rule: an absent or expired row reads
  UNKNOWN, an unreachable bus reads CANNOT-ASSESS; `--check` exits 0 ok, 1 not ok, 2 CANNOT-ASSESS, 3 UNKNOWN (#2).
- Separate probe intervals per query kind in `[intervals]` (`api` and `balance` default 12 h) and per-kind TTLs in
  `[ttl]` (default 3 × the kind's interval); `probe` runs only the kinds that are due, `--force` runs all; a balance
  read writes its own `.balance` row (#9).
- Central subscription reads in `quotabus probe`: Claude accounts by setup-token (direct read of the
  `anthropic-ratelimit-unified-*` headers, with a `claude -p --output-format stream-json` fallback) and GitHub Copilot
  (`copilot_internal/user`), on `[intervals] subscription` (default 1 h). Rows carry windows with `used_pct`,
  `reset_at`, `reset_in_s` and `pace`. Both sources are undocumented and their rows are labelled so (#12).
- `quotabus select --role R --exclude-family F [--prefer] [--n] [--json]`: only a fresh measured `ok` row qualifies,
  an excluded family is never chosen; exit 0 chosen, 2 store unreadable, 3 nothing qualifies. A change subject
  `ai.status.changed.<key>` is published when a row's state changes (#14).
- `quotabus alert`: one item per crossing, deduplicated at `alert.<key>`, to nusy-kanban, yurtle-kanban, a webhook and
  stdout. Only a fresh measured `ok` re-arms it; it never closes anything (#15).
- The Yurtle config front-end: a `yurtle-table` reader, with `examples/quotabus.yurtle.md` loading to the same config
  as `examples/quotabus.toml` (#15).
- `[probe] warn_pct` (default 90): an otherwise-ok row with any window at or above it reads `degraded` (#21).
- `[alert] states`: which states file an alert. The default is `quota_exhausted`, `auth_failed`, `model_missing`,
  `unreachable` and `window_near_limit`; a near-limit row carries the reason `window_near_limit` (#27).
- A `kind = "local"` service with no `models` lists `GET <base>/models` and probes each served id; an unreachable box
  writes one `cannot_assess:unreachable` row (#31).
- `packaging/install.sh`: chooses where the central probe runs (`--where local` or `--host <h>`, asked on stdin
  otherwise) and renders a launchd plist (macOS) or a systemd user service and timer (Linux) on a 300 s tick;
  `--render-to` is a dry run (#9, #10).
- `make check` and CI: tests, `cargo fmt --check`, clippy `-D warnings`, `cargo test --locked` (#2).

### Changed
- The old `[probe] interval` and `ttl` keys are refused with an error naming the new keys; if the store cannot be
  read, `probe` exits 1 unless given `--force` (#9).
- Alerts file only on the states in `[alert] states`; latency-degraded and `rate_limited` rows stay on the bus and are
  refused by `select`, but file nothing (#27).
- Local services file no alert: `alert` skips every `kind = "local"` slot and never writes its `alert.<key>` entry.
  Local rows are still published and still refused by `select` when not ok (#30).
- The `alert` summary line reads `<N> keys checked, <M> local skipped, <F> crossings filed, <C> re-armed` (#32).
- The example config lists the local servers as discovering services with no secret and no models (#31).

### Fixed
- The direct Claude read sends Claude Code's system prompt, without which the endpoint answers 429; a 429 with no
  unified headers reads `unknown` (`cannot_assess:http_429_no_unified_headers`), never `ok` (#13).
- Subscription windows' `used_pct` is rounded to 2 decimals (#16).
- The fleet install command uses the target's real user and user-owned paths, loads in the GUI domain, and sets
  `QUOTABUS_CLAUDE_BIN` for the stream-json fallback under launchd (#24).
- macOS Local Network privacy: the packaging docs and the install plan name the launchd job's program as the one to
  grant (#34).
- The `quotabus-cycle` wrapper passes only `--config` to `alert`, so a forced one-shot run no longer fails the alert
  step (#35).

### Security
- Keys are read only from the environment by the names the config gives. A `Secret` type prints `***`, and a
  `Redactor` scrubs every row, log line and error of each resolved secret and of common key patterns (#2).
- Credentials in a `nats://` URL (`[bus] url` or `QUOTABUS_NATS_URL`) reach the connection and are redacted from
  every output, including async-nats's own trace output, which printed the password (#5).
- Alert sinks run with secrets removed from their environment, and every alert body passes the redactor (#15).

### Project
- Design records, measurement findings, flowback packets for nusy-product-team, the ported work skills and board
  process rules (#1, #3, #4, #6, #7, #8, #11, #17, #18, #19, #20, #22, #23, #25, #26, #28, #29, #33, #36).

### Known limits
- v1.0 reports and alerts only. Pausing a provider is not a v1.0 feature (EXP-007).
- `serve`, spend via `ccusage`, key expiry and the model-drift sweep are not built (EXP-005).
- The crate is not published to crates.io and there are no prebuilt binaries: build from source and install with
  `packaging/install.sh`.

[1.0.0]: https://github.com/Congruentsys/quotabus/releases/tag/v1.0.0
