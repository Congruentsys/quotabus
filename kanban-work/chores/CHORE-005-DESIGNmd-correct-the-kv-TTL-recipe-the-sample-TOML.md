---
id: CHORE-005
title: "DESIGN.md: correct the kv TTL recipe, the sample TOML and the OpenAI probe route (found in EXP-001)"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-08
depends_on: []
---

# DESIGN.md: correct the kv TTL recipe, the sample TOML and the OpenAI probe route (found in EXP-001)

Found while landing EXP-001 (PR #2). `docs/DESIGN.md` is wrong or out of date in three places; a design change is its own PR (CLAUDE.md).

1. §3 Outputs names `nats kv put --ttl=<d>`; natscli 0.3.1 has the per-key TTL flag on `kv create`, not `kv put` (`docs/findings/EXP-001-per-key-ttl.md`, "What it changes" 1).
2. §3's sample `quotabus.toml` puts several keys on one line separated by `;`, which is not valid TOML; `examples/quotabus.toml` writes one key per line.
3. §4 says the OpenAI probe uses the responses API; E1 probes it with chat completions + `max_completion_tokens`, which measurably works (`gpt-5.6-sol` ok, PR #2 real run) — record that (review r1 F5).

## Done when
A PR amends DESIGN.md §3 and §4 on those three points, each citing its evidence; reviewed and merged.