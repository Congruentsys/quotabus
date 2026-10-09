---
id: HAZ-005
title: "macOS Local Network privacy blocks the launchd probe from LAN hosts (No route to host to the Sparks)"
type: hazard
status: backlog
priority: medium
assignee: null
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