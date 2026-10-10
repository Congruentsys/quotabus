---
id: CHORE-023
title: "Observe-only services (probe = false): list a service probed by another process (the Sparks' root daemon) without probing it"
type: chore
status: underway
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-10
depends_on: []
---

# Observe-only services (probe = false): list a service probed by another process (the Sparks' root daemon) without probing it

Found 2026-10-10 deploying the Sparks probe on Mini (HAZ-005). macOS lets only a ROOT LaunchDaemon reach LAN hosts,
so the Sparks (key-free, `kind = "local"`) are probed by a root daemon with its own config
(`/usr/local/etc/quotabus/sparks.toml`), while the keyed probes stay in the user LaunchAgent (doppler's token is a
keychain reference a daemon cannot read: "Token not found in system keyring"). But `quotabus status` and `select`
list only the services in THEIR config (measured: the agent's config without `local-qwen` → `status` shows no local
row). So `local-qwen` stays in the agent's config, and the agent's own 12 h probe writes
`cannot_assess:unreachable` ("No route to host") over the daemon's `ok` twice a day until the daemon's next tick.

## Definition of Done
1. A `[[service]]` may say `probe = false` (default true): the service is listed by `status`, `select` and `alert`
   (reading its rows from the store) but never probed by this process — another process owns it.
2. Tests: with `probe = false`, `probe --force` makes no call for that service and writes no row for it; `status`
   lists its row from the store with state and age; `select` considers it; an absent row reads UNKNOWN (freshness
   rule unchanged). Controls show each check can fail.
3. `examples/quotabus.toml` / `packaging/README.md` show the split (agent: keyed services + the Sparks with
   `probe = false`; root daemon: the Sparks). DESIGN.md §3 names the setting. `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-10T13:34:06+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T13:45:44+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-10 13:42)

One premise here is contradicted by a measurement: 'macOS lets only a ROOT LaunchDaemon reach LAN hosts' and 'the agent's own 12 h probe writes cannot_assess:unreachable'. On HAZ-005 (comment 2026-10-10 13:24, M5-MBP-2/s-72a67d16), a one-shot USER LaunchAgent in gui/501 ran the production chain (ProgramArguments[0] = /opt/homebrew/bin/doppler → quotabus-cycle → quotabus probe --force) at 13:22 UTC. It reached DGX1: ok, 2598 ms. The same LaunchAgent with quotabus itself as the program was blocked ('No route to host'), which matches the Mini session's results. The Mini session's tests ran quotabus directly, never under doppler. So the live agent's 12 h local probe should read ok, not unreachable. Check the next 12 h tick's local row before relying on this item's motivation: grep probe.err.log for local.qwen after the next api tick. The Captain chose the root daemon and it works, so this item may still be worth it as a deployment option. But if the agent's own tick reads ok, the 'overwrites the daemon's ok twice a day' problem does not occur.

### Mac-mini/s-e7976c42 (2026-10-10 13:49)

tests red at 63df760 (branch chore/CHORE-023-observe-only): tests/chore023_observe_only.rs — 16 red (probe key unknown; probe --force / scheduled probe / Runner make no call and write no row for probe = false, wiremock 0-call on API and local paths; status lists the stored row with state+age, absent/expired UNKNOWN; select uses a fresh ok row and refuses others; alert reads it; example config split, packaging README, DESIGN §3), 3 controls pass (0-call check fails without probe = false; no observe-only in a plain config; status lists no unconfigured service).

### Mac-mini/s-e7976c42 (2026-10-10 14:57)

Captain 2026-10-10, in the Mini session, after M5's 13:22 production-chain measurement (doppler as the launchd program reaches DGX1): chose "One unit, remove daemon (Recommended)" — the option recommended. Its text: once the agent's tick confirms DGX1 reads ok, remove the root daemon, drop CHORE-023 as unneeded, and land M5's HAZ-005 branch documenting the doppler grant. Mac-mini/s-e7976c42's earlier statement that every launchd job on Mini is blocked was wrong: its tests launched quotabus directly, never under doppler.
