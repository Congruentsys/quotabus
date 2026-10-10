---
id: CHORE-022
title: "Local Sparks: no key, DGX1/DGX2 at :8000, probe the model /v1/models serves (example + DESIGN §4 are stale)"
type: chore
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: []
---

# Local Sparks: no key, DGX1/DGX2 at :8000, probe the model /v1/models serves (example + DESIGN §4 are stale)

Part of VOY-001. Found redeploying the central probe on Mini (2026-10-09, comment on CHORE-016). The Captain, the
same day: "Local qwen should be on http on DGX1 or DGX2 or both - should not need an API key - but does go up and down
as they do different work".

What is wrong in the repo (measured on Mini, 2026-10-09):
- `examples/quotabus.toml` and DESIGN.md §4 put local Qwen at `192.168.8.180:30000` with `secret = "NUSY_LOCAL_QWEN"`
  and model `qwen3`. The Sparks serve on `192.168.8.120:8000` (DGX1) and `192.168.8.121` (DGX2) per the spark-model
  skill (`~/.claude/skills/spark-model/SKILL.md`: "Everything else is on :8000"; Laguna-S on :8003, its proxy :8082).
  `curl -s http://192.168.8.120:8000/v1/models` → 200, `["nvidia/Qwen3-32B-NVFP4"]`; DGX2 answered on none of 8000,
  8003, 8082 at the time.
- A local service needs no key: `src/probe.rs` `auth()` already gives `Auth::None` to `kind = "local"` with no
  `secret`. With the secret line present and unset, the row reads `cannot_assess:secret_unset` and is never probed.
- The model a Spark serves changes with its work (`spark-model switch`), so a fixed `models = [...]` reads
  `model_missing` after every switch. DESIGN §4's local row already says "GET /v1/models, then a 20-token completion".

## Definition of Done
1. A `kind = "local"` service may omit `models`: the probe lists `GET <base>/v1/models` and probes what is served (one
   row per served model, keyed by its id), so a switch is not `model_missing`. A configured `models` keeps today's
   behaviour. An unreachable box is one `cannot_assess:unreachable` row per service (no model row invented).
2. `examples/quotabus.toml` has two local services, DGX1 `http://192.168.8.120:8000/v1` and DGX2
   `http://192.168.8.121:8000/v1`, with no `secret` and no `models`; DESIGN.md §4's local row matches, citing the
   spark-model skill and this item.
3. Tests with a local HTTP stub: the served model is discovered and probed; a changed served model produces a row for
   the new id and no `model_missing`; no request carries an Authorization header; an unreachable stub reads
   `cannot_assess:unreachable`. Controls show each check can fail. `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T22:31:51+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T13:42:36+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-10 13:45)

tests red at 4b341de: tests/chore022_local_discovery.rs + tests/chore022_example_design.rs — 10 red (no-models local service discovers /v1/models and probes served id; one row per served model; switch reads new id, no model_missing; no Authorization; unreachable box is one row; two boxes independent; example has DGX1/DGX2 :8000 no secret/models; no stale :30000/NUSY_LOCAL_QWEN; DESIGN §4 row), 7 green (controls + configured-models and unset-secret behaviour kept). A ruled test edit to tests/examples_config.rs (it pins the retired local-qwen example) is going to the partner.
