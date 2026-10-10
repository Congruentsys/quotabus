---
id: CHORE-023
title: "Observe-only services (probe = false): list a service probed by another process (the Sparks' root daemon) without probing it"
type: chore
status: backlog
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