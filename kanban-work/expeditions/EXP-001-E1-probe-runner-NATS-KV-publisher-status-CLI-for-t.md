---
id: EXP-001
title: "E1: probe runner + NATS KV publisher + status CLI for the API providers (replaces the hand probe)"
type: expedition
status: arrived
priority: high
assignee: M5/s-b1fd4c67
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [CHORE-003]
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

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T17:22:06+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T17:31:08+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-08T18:40:02+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/2> ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Released with depends_on CHORE-003. Open signals it touches are built to docs/DESIGN.md §10's recommendation, none blocking: SIG-001 (central probe on Mini as a prebuilt binary → the launchd plist targets Mini), SIG-002 (balances and account labels published as slugs, publish_balance per service), SIG-004 (cadence 15 min API / balances, as config), SIG-005 (TOML first), SIG-007 (no admin keys; the probe is the status). Any of them can be reversed by config or a later PR if the Captain rules otherwise.

### M5/s-b1fd4c67 (2026-10-08 17:33)

Measure-first step done: a per-key TTL put EXPIRES on Mini's nats-server 2.12.4 (5 s key present at +2 s, absent at +8 s; plain key survives; bucket reports Per-Key TTL Supported: true). Only a temporary bucket qb_measure_exp001 was created and then deleted. natscli 0.3.1 has the TTL flag on 'kv create', not 'kv put' (design §3 recipe corrected in the finding). Finding: docs/findings/EXP-001-per-key-ttl.md on branch exp/EXP-001-probe-kv-status.

### M5/s-b1fd4c67 (2026-10-08 17:53)

tests red at ade2ebd (partner sub-agent): 80 Rust tests, 73 failing on todo!() stubs or exit-code asserts, none on a compile error; fmt/clippy/build green; the 35 script tests still pass. Controls shown able to fail (leak checker mutant; plain bucket vs limit_markers).
