---
id: SIG-004
title: "Q5: probe cadence — API 15 min, subscriptions 5 min, balances 15 min?"
type: signal
status: arrived
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
resolution: completed
---

# Q5: probe cadence — API 15 min, subscriptions 5 min, balances 15 min?

Design §10 Q5. Cadence and spend. **Recommendation:** API probes every 15 min (at most 96 × ~30 tokens per model per day), subscription reads every 5 min (free), balance reads every 15 min — all configurable.

## Comments

### M5/s-b1fd4c67 (2026-10-08 17:30)

Still open — not decided by any session. It blocks nothing in v1.0, so the work is built to docs/DESIGN.md §10's recommendation in EXP-001/EXP-002 (15 min API and balances, 5 min subscriptions — all config); a different ruling is a config change or a later PR.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:52)

Captain 2026-10-08: "Slower - twice a day, but make it a config setting"; on scope: "API + balance only". This is NOT the default. API probes and balance reads default to every 12 h, subscription reads stay at every 5 min, and every interval is a config setting.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:done ;
    kb:at "2026-10-08T18:52:46+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:forcedMove "true"^^xsd:boolean ;
    kb:resolution "completed" ;
  ] .
```

### M5-MBP-2/s-72a67d16 (2026-10-08 18:56)

Captain 2026-10-08, adding to the Q5 ruling: "for how often to query, make sure there are separate times for the different types of queries." So each query kind has its OWN config interval, set independently: API (messages) probe, balance read, and subscription read (and any later kind, e.g. a quota-window read). There is no shared interval. Defaults per the SIG-004 ruling: API 12 h, balance 12 h, subscription 5 min.

### M5-MBP-2/s-72a67d16 (2026-10-08 19:50)

Amended by the Captain 2026-10-08 (verbatim on EXP-002): 'We should not need to run anything else on the other machines (just Mini or M5 depending on where we host this)'. Claude usage is read centrally with each account's Doppler setup-token: 'Direct, stream-json fallback'. No statusLine install. Subscription cadence: 'Hourly' (1 h default, its own key).
