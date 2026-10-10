---
id: HAZ-005
title: "macOS Local Network privacy blocks the launchd probe from LAN hosts (No route to host to the Sparks)"
type: hazard
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: []
---

# macOS Local Network privacy blocks the launchd probe from LAN hosts (No route to host to the Sparks)

Found redeploying the central probe on Mini (2026-10-09). Under launchd, the probe cannot reach a LAN peer: the
DGX1 row reads `cannot_assess:unreachable` with error "tcp connect error: No route to host (os error 65)", while
the same `curl http://192.168.8.120:8000/v1/models` from an interactive shell on Mini returns 200, and a 20-token
completion returns 200 in 5.6 s. The bus at 192.168.8.110 (Mini itself) is unaffected. This is macOS's Local Network
privacy (macOS 15+): a launchd agent needs Local Network access granted in System Settings → Privacy & Security →
Local Network. A false CANNOT-ASSESS is honest (never a false ok), but every LAN service (the Sparks, and any
self-hosted endpoint) reads unreachable on any macOS central host until it is granted.

## Definition of Done
1. `packaging/README.md` (and `install.sh`'s plan output on macOS) state the Local Network step: after install, allow
   the probe in System Settings → Privacy & Security → Local Network (or the first prompt), and how to confirm it (the
   local row reads ok / a real state, not "No route to host").
2. Measured on Mini after the Captain grants it: the DGX1 row's state and error before and after, with the commands,
   recorded in a short finding or on this item. If replacing the binary (a redeploy) resets the grant, the README
   says so. `make check` green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T22:31:53+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T04:02:44+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### Mac-mini/s-e7976c42 (2026-10-10 04:05)

Measured on Mini 2026-10-10 04:04 UTC, after the Captain reported granting Local Network ('done'): still blocked. A one-off LaunchAgent (gui/<uid> domain, same as the unit) running '/Users/hankh19/.local/bin/quotabus probe --force --config <scratch: local-qwen only, [file] backend in the session scratchpad>' wrote cannot_assess:unreachable, error 'tcp connect error: No route to host (os error 65)'; the test agent was then booted out and its plist deleted. Same minute from an interactive shell: curl http://192.168.8.120:8000/v1/models → 200; ping 0.35 ms. codesign -dv on the binary: Signature=adhoc, Identifier=quotabus-10fbf5d6df46eac1 (a hash-derived id, so each rebuild is a new identity to macOS). No Local Network denial appears in 'log show --last 4m' for quotabus. Candidate fixes for the DoD: a stable signing identifier at install (codesign -s - -i com.congruentsys.quotabus), or a LaunchDaemon (Apple TN3179: local network privacy applies to agents, not launchd daemons) — the latter changes install.sh and needs sudo.

### Mac-mini/s-e7976c42 (2026-10-10 04:11)

Captain 2026-10-10, in the Mini session: "It isn't listed — go with the LaunchDaemon" (quotabus does not appear under Privacy & Security → Local Network; the earlier 'done' was the Login Items background toggle). This is the option the session recommended. So: the central probe on macOS runs as a LaunchDaemon (system domain, UserName = the target user), not a LaunchAgent. Before migrating the live unit, a one-off test daemon on Mini checks (a) DGX1 reachable from the daemon and (b) doppler can read its token from a daemon (no GUI keychain); results here. install.sh's change lands through this item's PR.

### Mac-mini/s-e7976c42 (2026-10-10 13:21)

Test daemon on Mini, 2026-10-10 (run by the Captain with sudo; system domain, UserName=hankh19, HOME set; one-off, removed after — /Library/LaunchDaemons has no quotabus plist): (a) quotabus probe --force on a local-only [file] config → DGX1 cannot_assess:unreachable, 'No route to host (os error 65)' — the LaunchDaemon with UserName does NOT escape Local Network privacy, contrary to the expectation recorded above; (b) doppler run → rc 1, 'Token not found in system keyring' (the CLI's token is a keyring reference, unreadable from a daemon). Control, same minute: the same binary and config from an interactive shell (parent: claude) → DGX1 ok, latency 2854 ms. So the block is the launchd context, agent or user-daemon alike. Not yet tested: a root daemon (no UserName) for the key-free local probe only.
