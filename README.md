# quotabus

**Which of my LLM keys and AI subscriptions work right now — and how much is left?**

quotabus probes every LLM provider key and AI subscription account you list, and publishes one status record per
service to a NATS KV bucket. Each record says whether the service works, has a bad key, names a missing model, is out
of quota or is rate-limited, plus the balance where the provider exposes one. Every record carries the time it was
checked and expires on its own, so a reader never mistakes an old "ok" for a current one: absent or expired reads
**unknown**, and an unreachable bus reads **cannot assess**.

Humans read it with `quotabus status`. Agents read it with `quotabus select --role review --exclude-family anthropic`,
to pick a healthy model before spending a long prompt on it.

> **Status: v1.0.0** — see [`CHANGELOG.md`](CHANGELOG.md). One Rust binary with four commands:
> - `quotabus probe --config quotabus.toml` runs what is due and writes one row per (service, model) to the NATS KV
>   bucket of `[bus]` (created with per-key TTL), or to `[file] dir` without one. It covers API keys (with balances
>   for DeepSeek and Kimi), subscription accounts read centrally (Claude by setup-token, GitHub Copilot; nothing runs
>   on other hosts), and local model servers, which discover the model they serve from `GET <base>/models`. Each
>   query kind has its own `[intervals]` entry and TTL. Run it under your key manager so keys arrive by environment
>   only: `doppler run -- quotabus probe` or `secretspec run -- quotabus probe`.
> - `quotabus status [--json] [--kind api|local|subscription] [--stale] [--check <service>]` reads the rows with the
>   freshness rule; `--stale` lists only the UNKNOWN (absent or expired) entries.
> - `quotabus select --role R --exclude-family F` picks a healthy model, or exits 3 when nothing qualifies.
> - `quotabus alert`, run after each probe cycle, files one `signal` per crossing (a service going bad into a state
>   in `[alert] states`) to nusy-kanban, yurtle-kanban, a webhook and stdout, deduplicated at `alert.<key>`; only a
>   measured `ok` re-arms it, it never closes anything, and local servers file no alert.
>
> The config is TOML or its Yurtle twin: see [`examples/quotabus.toml`](examples/quotabus.toml) and
> [`examples/quotabus.yurtle.md`](examples/quotabus.yurtle.md). [`packaging/install.sh`](packaging/) installs the
> central probe as a launchd or systemd unit on this host or a named one; build the binary from source (it is not on
> crates.io). The design is [`docs/DESIGN.md`](docs/DESIGN.md); the prior-art review is
> [`docs/PRIOR-ART.md`](docs/PRIOR-ART.md). Work is tracked on this repo's own board under
> [`kanban-work/`](kanban-work/) (yurtle-kanban).

## What v1.0 is

- **One Rust binary:** `probe` (a central host checks API keys, subscription accounts and local servers), `status`,
  `select`, `alert`.
- **Keys never leave your key manager:** they reach the binary only through environment injection
  ([secretspec](https://github.com/cachix/secretspec) — Doppler, 1Password, macOS Keychain, `pass`, Vault, SOPS, env —
  or `doppler run`), and a redactor scrubs every record, log and error.
- **Config:** TOML, with a Yurtle (Markdown + Turtle) twin.
- **Outputs:** NATS KV (with per-key TTL) and change subjects (`ai.status.changed.<key>`); a CLI table; alerts filed
  to yurtle-kanban or nusy-kanban, deduplicated so one outage files one item.
- **It reports; it does not act.** Not in v1.0 (CHANGELOG, Known limits): pausing a provider (EXP-007); `serve`, spend
  via `ccusage`, key expiry, the model-drift sweep and polish of the file backend for outsiders without NATS (EXP-005).

## Licence

MIT — see [LICENSE](LICENSE).
