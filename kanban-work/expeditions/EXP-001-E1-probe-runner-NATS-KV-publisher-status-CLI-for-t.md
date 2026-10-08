---
id: EXP-001
title: "E1: probe runner + NATS KV publisher + status CLI for the API providers (replaces the hand probe)"
type: expedition
status: backlog
priority: high
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: []
---

# E1: probe runner + NATS KV publisher + status CLI for the API providers (replaces the hand probe)

Part of VOY-001. Design: `docs/DESIGN.md` §2 (the record), §3, §4 (probe catalogue), §5, §6, §9 row E1.

## Plan
1. Rust scaffold, CI (fmt, clippy -D warnings, tests), the MIT licence already in the repo.
2. TOML config (§3) and the status record (§2): state ∈ {ok, auth_failed, model_missing, quota_exhausted, rate_limited, degraded, unknown}, balance if exposed, rate-limit headroom, latency, `checked_at`, `observed_by`, redacted error, `ttl_s`.
3. Probe runner for the six key-bearing API services in our config (GLM, DeepSeek, Kimi, OpenAI, Together, local Qwen), with xAI's disabled key as the negative control; balance adapters for DeepSeek and Kimi. Keys only from the environment (`secretspec run --` / `doppler run --`); a `Secret` newtype and a redactor on every output.
4. KV publisher: a NEW bucket created with per-key TTL. **Measure first:** a key put with `--ttl` is ABSENT after expiry on Mini's nats-server 2.12.4 (no existing bucket there has per-key TTL).
5. `quotabus status` (table, `--json`, `--check` rc), and the launchd plist for the central probe on Mini.

## Definition of Done
- `quotabus status` on M5 shows the six rows from Mini's bucket, each with its age.
- A wrong key reads `auth_failed`; a retired model name reads `model_missing`.
- Bus down reads CANNOT-ASSESS with rc 2, never `ok`.
- The per-key TTL measurement is recorded with its command.