# quotabus

**Which of my LLM keys and AI subscriptions work right now — and how much is left?**

quotabus probes every LLM provider key and AI subscription account you list, and publishes one status record per
service to a NATS KV bucket. Each record says whether the service works, has a bad key, names a missing model, is out
of quota or is rate-limited, plus the balance where the provider exposes one. Every record carries the time it was
checked and expires on its own, so a reader never mistakes an old "ok" for a current one: absent or expired reads
**unknown**, and an unreachable bus reads **cannot assess**.

Humans read it with `quotabus status`. Agents read it with `quotabus select --role review --exclude-family anthropic`,
to pick a healthy model before spending a long prompt on it.

> **Status: E1 landed — `probe` and `status` for the API services.** `quotabus probe --config quotabus.toml` runs one
> probe cycle and writes one row per (service, model) to the NATS KV bucket of `[bus]` (created with per-key TTL), or
> to `[file] dir` without one; run it under your key manager so keys arrive by environment only:
> `doppler run -- quotabus probe` or `secretspec run -- quotabus probe`. `quotabus status [--json] [--kind api]
> [--check <service>]` reads the rows with the freshness rule. `quotabus alert` (EXP-004), run after each probe
> cycle, files one `signal` per crossing (a service going bad) to nusy-kanban, yurtle-kanban, a webhook and stdout,
> deduplicated at `alert.<key>`; only a measured `ok` re-arms it, and it never closes anything. The config is TOML or
> its Yurtle twin: see [`examples/quotabus.toml`](examples/quotabus.toml),
> [`examples/quotabus.yurtle.md`](examples/quotabus.yurtle.md) and [`packaging/`](packaging/). `serve` is not built
> yet. The design is
> [`docs/DESIGN.md`](docs/DESIGN.md); the prior-art review is [`docs/PRIOR-ART.md`](docs/PRIOR-ART.md). Work is
> tracked on this repo's own board under [`kanban-work/`](kanban-work/) (yurtle-kanban).

## Planned shape (v1.0)

- **One Rust binary:** `probe` (a central host checks API keys), `agent` (each host reads its own subscription state —
  Claude Code, GitHub Copilot), `status`, `select`, `alert`; optional `serve`.
- **Keys never leave your key manager:** they reach the binary only through environment injection
  ([secretspec](https://github.com/cachix/secretspec) — Doppler, 1Password, macOS Keychain, `pass`, Vault, SOPS, env —
  or `doppler run`), and a redactor scrubs every record, log and error.
- **Config:** TOML, with a Yurtle (Markdown + Turtle) twin.
- **Outputs:** NATS KV (with per-key TTL) and change subjects; a CLI table; alerts filed to yurtle-kanban or
  nusy-kanban, deduplicated so one outage files one item.
- **It reports; it does not act.** Pausing a provider is a later, separate expedition.

## Licence

MIT — see [LICENSE](LICENSE).
