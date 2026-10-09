---
id: CHORE-018
title: "Alert only on configured states: [alert] states, default hard failures + near-limit window (Captain SIG-010)"
type: chore
status: arrived
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-09
depends_on: []
---

# Alert only on configured states: [alert] states, default hard failures + near-limit window (Captain SIG-010)

Part of VOY-001. Builds the Captain's SIG-010 ruling. Captain 2026-10-09: "SIG-010: go with the recommended default,
option 3" — the recommended default recorded on SIG-010 ([steer] bucket-3 comment, 2026-10-08 21:57): a configurable
`[alert] states`, defaulting to the hard failures (`quota_exhausted`, `auth_failed`, `model_missing`, unreachable)
PLUS a near-limit window. Latency-degraded and `rate_limited` stay on the bus and are refused by `select`, but file no
signal.

Today (EXP-004, `src/alert.rs`) every crossing from `ok` into ANY bad state files one signal; the EXP-004 review
measured `ok`↔`degraded` flapping at 5 signals in 10 cycles (`reviews/EXP-004-r1.md` F2).

**The distinction that matters:** a near-limit window and a slow 200 both read `degraded` (`src/classify.rs`:
`warn_state` for a window ≥ `[probe] warn_pct`, CHORE-010; the latency rule for a slow 200). The alert step must tell
them apart from the row itself (e.g. a distinct `reason` such as `window_near_limit` written by `warn_state`), never
by guessing.

## Definition of Done
1. Config: `[alert] states = [...]` (state names, plus a name for the near-limit window case); absent → the default
   above. An unknown name is refused with an error naming the valid ones.
2. `quotabus alert` files a signal only for a crossing into a state in `states`. A crossing into a state NOT in the
   list files nothing and does not arm the alert. The crossing-dedup rule is unchanged otherwise: one alert per
   crossing, and only a measured `ok` re-arms it (DESIGN §3 alert row).
3. A near-limit window is distinguishable from latency-degraded in the row (a `reason` or equivalent), set where
   `warn_state` degrades it; the latency-degraded row is not affected.
4. Tests: under the default, a crossing to `quota_exhausted` / `auth_failed` / `model_missing` / unreachable files;
   to a near-limit window files; to latency-degraded or `rate_limited` files nothing. The EXP-004 flapping case
   (ok↔latency-degraded, 10 cycles) files 0. A configured `states` that includes `rate_limited` files it. Controls
   show each check can fail.
5. DESIGN.md §3 alert row and §10 Q8 record the ruling verbatim with its SIG; `examples/quotabus.toml` shows
   `[alert] states` with the default. `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T13:38:37+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-09T13:49:54+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-09T14:13:29+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/27> ;
  ] .
```


## Comments

### Mac-mini/s-e7976c42 (2026-10-09 13:57)

tests red at 2694eff (branch chore/CHORE-018-alert-states): tests/chore018_alert_states.rs — 17 red (states key refused/absent, default set, unknown-name refusal, reason window_near_limit, only listed states file and arm, EXP-004 ok↔latency flapping 5 → must be 0, unreachable = unknown+cannot_assess:unreachable files under default, DESIGN §3/§10 Q8 quote SIG-010, example config), 7 green controls. Ruled edits to existing tests that asserted the overturned every-bad-state rule: exp004_alert.rs (CANNOT-ASSESS case now not_found), exp004_r1.rs (rig lists degraded/rate_limited), exp004_config.rs (per-sink compare), chore010_warn_pct.rs (dropped the decide-level 'files as any bad state' test; covered by the new file).
