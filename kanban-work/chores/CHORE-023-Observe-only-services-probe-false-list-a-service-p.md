---
id: CHORE-023
title: "Observe-only services (probe = false): list a service probed by another process (the Sparks' root daemon) without probing it"
type: chore
status: provisioning
priority: medium
assignee: null
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
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-10 13:42)

One premise here is contradicted by a measurement: 'macOS lets only a ROOT LaunchDaemon reach LAN hosts' and 'the agent's own 12 h probe writes cannot_assess:unreachable'. On HAZ-005 (comment 2026-10-10 13:24, M5-MBP-2/s-72a67d16), a one-shot USER LaunchAgent in gui/501 ran the production chain (ProgramArguments[0] = /opt/homebrew/bin/doppler → quotabus-cycle → quotabus probe --force) at 13:22 UTC. It reached DGX1: ok, 2598 ms. The same LaunchAgent with quotabus itself as the program was blocked ('No route to host'), which matches the Mini session's results. The Mini session's tests ran quotabus directly, never under doppler. So the live agent's 12 h local probe should read ok, not unreachable. Check the next 12 h tick's local row before relying on this item's motivation: grep probe.err.log for local.qwen after the next api tick. The Captain chose the root daemon and it works, so this item may still be worth it as a deployment option. But if the agent's own tick reads ok, the 'overwrites the daemon's ok twice a day' problem does not occur.
