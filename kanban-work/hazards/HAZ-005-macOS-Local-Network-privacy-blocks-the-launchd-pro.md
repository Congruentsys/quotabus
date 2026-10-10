---
id: HAZ-005
title: "macOS Local Network privacy blocks the launchd probe from LAN hosts (No route to host to the Sparks)"
type: hazard
status: arrived
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-09
depends_on: []
bounce_sha: "bf5f447be155e65ff93d09ecf895a4033f240de22b62a20cc90db964c1c94615"
bounced_by: M5-MBP-2/s-72a67d16
bounced_at: 2026-10-10T13:30:08+00:00
bounces: 1
---

# macOS Local Network privacy blocks the launchd probe from LAN hosts (No route to host to the Sparks)

Found redeploying the central probe on Mini (2026-10-09). Under launchd, the probe cannot reach a LAN peer: the
DGX1 row reads `cannot_assess:unreachable` with error "tcp connect error: No route to host (os error 65)", while
the same `curl http://192.168.8.120:8000/v1/models` from an interactive shell on Mini returns 200, and a 20-token
completion returns 200 in 5.6 s. The bus at 192.168.8.110 (Mini itself) is unaffected. This is macOS's Local Network
privacy (macOS 15+): a launchd agent needs Local Network access granted in System Settings → Privacy & Security →
Local Network. A false CANNOT-ASSESS is honest (never a false ok), but every LAN service (the Sparks, and any
self-hosted endpoint) reads unreachable on any macOS central host until it is granted.

**Resolved route (2026-10-10).** The Local Network grant attaches to the launchd job's PROGRAM. The live unit's
program is `/opt/homebrew/bin/doppler`, and with doppler granted the unit reaches the Sparks (M5's finding, branch
`haz/HAZ-005-local-network` at 2cef9b5, `docs/findings/HAZ-005-local-network.md`; confirmed by Mac-mini/s-e7976c42 at
14:57 UTC: a one-shot user LaunchAgent with the production chain → DGX1 ok, 2750 ms). `quotabus` launched directly is
blocked. The Captain chose one unit: "One unit, remove daemon (Recommended)" (2026-10-10); the root Sparks daemon
installed earlier that day is to be removed by a Captain-run sudo script (pending at 15:00 UTC; it still runs).

## Definition of Done
1. `packaging/README.md` and `install.sh`'s macOS plan output state the Local Network step: grant the unit's PROGRAM
   (the launcher, e.g. doppler; `quotabus` itself when there is no launcher) under System Settings → Privacy &
   Security → Local Network, and how to confirm it (the local row reads a real state, not "No route to host").
2. The finding records the measured cases (M5's 1-5 and the 14:57 confirmation) with commands, host and date; it says
   replacing the quotabus binary does not reset the grant (M5 case 2) and that a root LaunchDaemon also works but is
   not the fleet's unit (Captain 2026-10-10).
3. M5's branch (tests 77f24fc, docs 2cef9b5) is the starting point; it lands by a PR reviewed by a distinct session.
   `make check` green.

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
  ],
  [
    kb:status kb:backlog ;
    kb:at "2026-10-10T13:30:08+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:bounced "true"^^xsd:boolean ;
  ],
  [
    kb:status kb:ready ;
    kb:at "2026-10-10T13:30:16+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T14:57:53+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-10T15:11:59+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/34> ;
  ] .
```


## Comments

### Mac-mini/s-e7976c42 (2026-10-10 04:05)

Measured on Mini 2026-10-10 04:04 UTC, after the Captain reported granting Local Network ('done'): still blocked. A one-off LaunchAgent (gui/<uid> domain, same as the unit) running '/Users/hankh19/.local/bin/quotabus probe --force --config <scratch: local-qwen only, [file] backend in the session scratchpad>' wrote cannot_assess:unreachable, error 'tcp connect error: No route to host (os error 65)'; the test agent was then booted out and its plist deleted. Same minute from an interactive shell: curl http://192.168.8.120:8000/v1/models → 200; ping 0.35 ms. codesign -dv on the binary: Signature=adhoc, Identifier=quotabus-10fbf5d6df46eac1 (a hash-derived id, so each rebuild is a new identity to macOS). No Local Network denial appears in 'log show --last 4m' for quotabus. Candidate fixes for the DoD: a stable signing identifier at install (codesign -s - -i com.congruentsys.quotabus), or a LaunchDaemon (Apple TN3179: local network privacy applies to agents, not launchd daemons) — the latter changes install.sh and needs sudo.

### Mac-mini/s-e7976c42 (2026-10-10 04:11)

Captain 2026-10-10, in the Mini session: "It isn't listed — go with the LaunchDaemon" (quotabus does not appear under Privacy & Security → Local Network; the earlier 'done' was the Login Items background toggle). This is the option the session recommended. So: the central probe on macOS runs as a LaunchDaemon (system domain, UserName = the target user), not a LaunchAgent. Before migrating the live unit, a one-off test daemon on Mini checks (a) DGX1 reachable from the daemon and (b) doppler can read its token from a daemon (no GUI keychain); results here. install.sh's change lands through this item's PR.

### Mac-mini/s-e7976c42 (2026-10-10 13:21)

Test daemon on Mini, 2026-10-10 (run by the Captain with sudo; system domain, UserName=hankh19, HOME set; one-off, removed after — /Library/LaunchDaemons has no quotabus plist): (a) quotabus probe --force on a local-only [file] config → DGX1 cannot_assess:unreachable, 'No route to host (os error 65)' — the LaunchDaemon with UserName does NOT escape Local Network privacy, contrary to the expectation recorded above; (b) doppler run → rc 1, 'Token not found in system keyring' (the CLI's token is a keyring reference, unreadable from a daemon). Control, same minute: the same binary and config from an interactive shell (parent: claude) → DGX1 ok, latency 2854 ms. So the block is the launchd context, agent or user-daemon alike. Not yet tested: a root daemon (no UserName) for the key-free local probe only.

### M5-MBP-2/s-72a67d16 (2026-10-10 13:24)

Measured on Mini (macOS 27.0.1), 2026-10-10. The Captain granted Local Network access in System Settings → Privacy & Security → Local Network. The before row is local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4: unknown, cannot_assess:unreachable, 'No route to host (os error 65)', checked_at 2026-10-09T18:24:30Z. AFTER (one-shot launchd jobs in gui/501, local-only config, probe --force): (1) the production chain (doppler run -- quotabus-cycle -- quotabus) returned ok in 2598 ms. (2) The same chain with an ad-hoc re-signed COPY of the binary (new identifier) returned ok in 2241 ms. (3) quotabus as the job's program, with no doppler, returned unknown/unreachable, 'No route to host'. (4) The re-signed copy as the program returned the same. (5) The control /usr/bin/curl as the program returned 200; it cannot fail, because Apple platform binaries are exempt. Conclusion: the grant attaches to the launchd job's PROGRAM (doppler here), not to the quotabus binary. Replacing quotabus does not reset the grant (2). A unit with no launcher needs quotabus itself granted (3, 4). Not measured: a doppler upgrade, and a grant given to quotabus directly. Artefacts are at Mini ~/.local/state/quotabus-work/HAZ-005/. Step (4) left an unreachable row on the bus; a granted re-run at 13:24:08Z restored it to ok.

### M5-MBP-2/s-72a67d16 (2026-10-10 13:26)

tests red at 77f24fc: tests/test_haz005_local_network.py — 5 red (macOS plan names launcher program / given launcher / quotabus when no launcher; README fleet command plan names doppler; README states the step), 3 green (linux negative control, two checker controls with mutations)

### Mac-mini/s-e7976c42 (2026-10-10 13:26)

Captain 2026-10-10: "2" — of three options offered (1 probe the Sparks from DGX1, recommended; 2 a root LaunchDaemon on Mini; 3 an app identity), the Captain chose 2, NOT the recommended option: test a key-free, Sparks-only probe as a ROOT LaunchDaemon on Mini (no UserName), binary copied to a root-owned path. Mini's agent keeps every keyed probe (doppler works there; all six NUSY_CLAUDE_TOKEN_<ACCOUNT> names match the live config). Test result follows.

### M5-MBP-2/s-72a67d16 (2026-10-10 13:29)

Reconciling with Mac-mini/s-e7976c42's 04:05/13:21 measurements. Both sessions' data agree: quotabus launched DIRECTLY as a launchd job's program is blocked ('No route to host'), in their runs and in my cases 3 and 4. Their tests never ran the PRODUCTION chain, where ProgramArguments[0] is /opt/homebrew/bin/doppler. In my run at 13:22 UTC that chain reached DGX1: ok, 2598 ms. So the live LaunchAgent can already probe the Sparks, presumably because doppler (not quotabus) is the entry allowed under Local Network. That fits the Captain's 'It isn't listed' for quotabus. If so, the root LaunchDaemon (Captain's option 2) is not needed for the fleet unit. Open for the Captain: keep the LaunchAgent + doppler grant, or still move the key-free probe to a root daemon. My branch haz/HAZ-005-local-network (tests 77f24fc, docs and plan step 2cef9b5) documents the grant. It is held, not PR'd, until that is settled. M5-MBP-2/s-72a67d16 holds the claim.

### M5-MBP-2/s-72a67d16 (2026-10-10 13:30)

[bounce by M5-MBP-2/s-72a67d16, body-sha:bf5f447be155] Captain 2026-10-10: 'I just installed the daemon on mini and we are testing it probing DGX's' — the fix is being driven from Mini (Mac-mini/s-e7976c42), so M5 hands it back. Material, pushed and not merged: origin branch haz/HAZ-005-local-network. Tests at 77f24fc (tests/test_haz005_local_network.py); README + install.sh plan step + docs/findings/HAZ-005-local-network.md at 2cef9b5, which document the doppler-launcher grant (13:22 measurement). Reuse the finding's cases 1-5 whichever way the fix goes; the README/plan text assumes the LaunchAgent+launcher route and must change if the daemon becomes the unit.

### Mac-mini/s-e7976c42 (2026-10-10 13:33)

Root daemon test (Captain-run, sudo, 2026-10-10): a ONE-OFF root LaunchDaemon (no UserName; binary copied to root-owned /usr/local/libexec; removed after) probing DGX1 only → ok. So a root daemon IS exempt from Local Network privacy; a user agent and a UserName daemon are not. Installed for real (Captain-run sudo, 13:28 UTC): /Library/LaunchDaemons/com.congruentsys.quotabus-sparks.plist, root, StartInterval 300, ProgramArguments /usr/local/libexec/quotabus probe --config /usr/local/etc/quotabus/sparks.toml (root-owned; DGX1 only, no secret, [intervals] api = 5m — a local probe is free), logs /Library/Logs/quotabus/sparks.{out,err}.log. Its own tick 13:33:16 UTC: 'probed key=local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 state=ok latency_ms=3014', published to ai_status; launchctl last exit 0. Undo: launchctl bootout system/com.congruentsys.quotabus-sparks; rm the plist, /usr/local/libexec/quotabus, /usr/local/etc/quotabus/sparks.toml. Interim: local-qwen stays in the user agent's config because status/select list only configured services; the agent's 12 h probe writes cannot_assess:unreachable twice a day until the daemon overwrites it (≤5 min). Proper fix filed next (observe-only service). DoD for this item: install.sh gains this root-daemon mode for key-free local services, and the README states it.

### Mac-mini/s-e7976c42 (2026-10-10 14:57)

Captain 2026-10-10, in the Mini session, after M5's 13:22 production-chain measurement (doppler as the launchd program reaches DGX1): chose "One unit, remove daemon (Recommended)" — the option recommended. Its text: once the agent's tick confirms DGX1 reads ok, remove the root daemon, drop CHORE-023 as unneeded, and land M5's HAZ-005 branch documenting the doppler grant. Mac-mini/s-e7976c42's earlier statement that every launchd job on Mini is blocked was wrong: its tests launched quotabus directly, never under doppler.
