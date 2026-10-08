---
id: CHORE-008
title: "Choose where the central probe runs at install time: locally or on a named host (Mini for the fleet) — Captain Q2"
type: chore
status: underway
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-08
tags: [v1.0, VOY-001, packaging]
depends_on: []
---

# Choose where the central probe runs at install time: locally or on a named host (Mini for the fleet) — Captain Q2

Part of VOY-001. The Captain's ruling on 2026-10-08, verbatim on SIG-001: "If this is FOSS, the config will have to ask to run locally or on another host. In this case, run on mini". EXP-001 shipped one launchd plist hard-wired to Mini (`/Users/admin/...`, `packaging/launchd/`).

## Definition of Done
1. An install step (`packaging/install.sh` or a `quotabus install` subcommand) that ASKS where the central probe runs, or takes it as a flag: `local` (this host, as the current user) or a named host over SSH. It renders the unit (launchd on macOS, a systemd user unit on Linux) for that target, with paths derived from the target's user and home, not hard-coded.
2. The fleet's choice is recorded in config or packaging docs as Mini. Installing for Mini produces a unit equivalent to today's plist.
3. No secret is in any unit or argv (keys still arrive only through the launcher's environment).
4. Tests: rendering for `local` and for a named host gives the right paths and target, plus a control showing the hard-coded `/Users/admin` path is gone. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T18:58:28+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T19:17:53+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ] .
```
