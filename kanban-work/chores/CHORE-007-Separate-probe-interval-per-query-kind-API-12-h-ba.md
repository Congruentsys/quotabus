---
id: CHORE-007
title: "Separate probe interval per query kind (API 12 h, balance 12 h), each with its own TTL — Captain Q5"
type: chore
status: underway
priority: high
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001, config]
depends_on: []
---

# Separate probe interval per query kind (API 12 h, balance 12 h), each with its own TTL — Captain Q5

Part of VOY-001. EXP-001 shipped ONE `[probe] interval` (15m) and ONE `ttl` (45m) for every call a `quotabus probe` cycle makes. A launchd unit runs that cycle every 900 s (`packaging/launchd/…plist`). The Captain's rulings on 2026-10-08, verbatim on SIG-004:
- "Slower - twice a day, but make it a config setting" (scope: "API + balance only")
- "for how often to query, make sure there are separate times for the different types of queries."

This chore blocks EXP-002, which adds a third kind (subscription reads, 5 min) in the same shape.

## Definition of Done
1. Config: an `[intervals]` table with one key per query kind, `api` and `balance` (EXP-002 adds `subscription`), each a duration. Defaults: `api = "12h"`, `balance = "12h"`. Each kind's TTL defaults to 3 × its own interval and can be set per kind (`[ttl]` with the same keys). The old `[probe] interval` / `ttl` keys are refused with an error that names the new keys. `max_tokens` and `degraded_latency_ms` stay in `[probe]`.
2. Scheduling: a `quotabus probe` run makes a kind's calls only when that kind is DUE, i.e. its interval has elapsed since that kind's last row in the store (no row = due). A due API probe never triggers a balance read, and vice versa. `quotabus probe --force` runs every kind. A test with a fake clock and store shows each kind runs on its own interval, with a control that fails if the intervals are shared.
3. Each row's `ttl_s` is its own kind's TTL.
4. The launchd plist's `StartInterval` becomes a TICK (default 300 s): how often it checks what is due, documented as such in `packaging/README.md`. No interval is duplicated outside the config.
5. `examples/quotabus.toml` and `docs/DESIGN.md` §3/§4 (the cadence and the "≤ 96 calls" cost line) match. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T18:58:26+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T18:58:34+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
