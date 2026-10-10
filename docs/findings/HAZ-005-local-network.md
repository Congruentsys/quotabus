# HAZ-005: macOS Local Network privacy and the launchd probe

**Question.** Under launchd on Mini, the probe reads every LAN peer as `cannot_assess:unreachable`, "No route to
host (os error 65)", while an interactive shell reaches the same peer. After the Captain grants Local Network access,
what does the grant attach to, and does replacing the quotabus binary (a redeploy) reset it?

**Answer.**
- The grant attaches to the launchd job's FIRST program (`ProgramArguments[0]`). Under `doppler run --` that is
  doppler, not quotabus.
- Replacing the quotabus binary under a granted launcher did NOT reset it: an ad-hoc re-signed copy with a new
  identifier still reached the LAN.
- With no launcher, quotabus itself is the program: launched directly, quotabus got "No route to host (os error 65)"
  while doppler was granted. So a unit with no launcher needs quotabus allowed. That a grant to quotabus fixes it is
  inferred, not measured.
- Unmeasured: whether upgrading the launcher (e.g. `brew upgrade doppler`) resets the grant.
- Apple platform binaries are exempt: `/usr/bin/curl` as the job's program returned 200 with no grant involved, so a
  `curl` from a launchd job is not a valid check.
- A ROOT LaunchDaemon (system domain, no `UserName`) running quotabus directly is not blocked; a LaunchDaemon with
  `UserName` set is blocked like a user LaunchAgent. The root daemon works but is not the fleet's unit: the Captain
  chose one unit, the LaunchAgent under doppler (see "Which unit", below).

- **Host:** Mini (`Mac-mini.lan`, macOS 27.0.1, arm64), user `hankh19`, GUI domain `gui/501`.
- **Date:** 2026-10-10.
- **quotabus:** the deployed binary `~/.local/bin/quotabus`, ad-hoc signed (`codesign -dv`: `Signature=adhoc`, a
  hash-derived identifier).
- **Peer:** DGX1's local endpoint, row `local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4`.
- **Raw artefacts:** Mini `~/.local/state/quotabus-work/HAZ-005/`.

**Authorship.** All of this work was done by LLM agent sessions, Claude Opus 5.5. The driver `M5-MBP-2/s-72a67d16`
ran the measurement; the Captain granted the permission in System Settings (the GUI step no session can do). This
file was written by an implementer sub-agent of that driver from the driver's record on HAZ-005. Cases 6-9 and "Which
unit" were added by a docs sub-agent of `Mac-mini/s-e7976c42`, from that session's comments and raw logs; that session
ran those measurements, and the Captain ran the `sudo` steps. No human reviewed it.

## Method

Each case is a one-shot launchd plist bootstrapped into `gui/501` on Mini (the same domain as the installed unit),
running `quotabus probe --force` over a local-only config (the DGX1 row only), then booted out:

```sh
launchctl bootstrap gui/501 <one-shot plist>   # ProgramArguments differ per case, below
# ... the job runs quotabus probe --force --config <local-only config> once ...
launchctl bootout gui/501/<label>
```

## Results

| case | ProgramArguments[0] | DGX1 row |
|---|---|---|
| before the grant (the live unit) | doppler | unknown, `cannot_assess:unreachable`, "No route to host (os error 65)", checked_at 2026-10-09T18:24:30Z |
| 1. production chain via doppler | doppler | ok, 2598 ms |
| 2. chain via doppler, re-signed copy of quotabus (new identifier) | doppler | ok, 2241 ms |
| 3. quotabus directly, no launcher | quotabus | unknown / unreachable, "No route to host" |
| 4. the re-signed copy directly | the copy | unknown / unreachable, "No route to host" |
| 5. control: `/usr/bin/curl` to the endpoint | curl | HTTP 200 (exempt platform binary; cannot fail) |

Case 4 left an unreachable row on the bus; a granted re-run at 13:24:08Z restored it to ok.

### Cases measured by the Mini session

Measured by `Mac-mini/s-e7976c42` on Mini (macOS 27.0.1), 2026-10-10, each with a local-only config (the DGX1 row
only, a `[file]` backend in that session's scratchpad, no secret). Sources: the session's comments on HAZ-005
(`kanban-work/hazards/HAZ-005-macOS-Local-Network-privacy-blocks-the-launchd-pro.md`, lines 80, 88, 112), the
CHORE-023 closing commit `ad48dd1`, and, where they were kept, the raw logs in the session scratchpad on Mini
(`<scratchpad>/localnet/`); the result column quotes the raw log line where one exists.

| case | launchd job and command shape | DGX1 result |
|---|---|---|
| 6. user LaunchAgent, quotabus as the program (04:04 UTC) | one-shot plist in `gui/<uid>` (the unit's domain); `ProgramArguments`: `/Users/hankh19/.local/bin/quotabus probe --force --config <local-only>`; booted out and deleted after. The Captain's earlier 'done' was the Login Items toggle, not a Local Network grant (line 84) | `cannot_assess:unreachable`, "tcp connect error: No route to host (os error 65)" (comment, line 80; no raw log kept) |
| 7. LaunchDaemon with `UserName=hankh19` (13:20 UTC) | `/Library/LaunchDaemons/com.congruentsys.quotabus-daemon-test.plist`, system domain, `UserName` hankh19, `HOME` set, Captain-run with sudo, removed after; `ProgramArguments`: `/bin/sh daemon-test.sh`, which runs `quotabus probe --force --config <local-only>` and then `/opt/homebrew/bin/doppler run --project nusy-product-team --config dev -- /bin/sh -c '<print whether a key name is set>'` | probe: `state="unknown"`, `cannot_assess:unreachable` (raw `daemon.err.log`, 13:20:56Z; the comment, line 88, gives the error "No route to host (os error 65)"). doppler: `doppler rc=1`, "Doppler Error: secret not found in keyring" (raw log; the comment paraphrases it as 'Token not found in system keyring') |
| 8. ROOT LaunchDaemon, no `UserName` (13:27 UTC) | `/Library/LaunchDaemons/com.congruentsys.quotabus-root-test.plist`, system domain, Captain-run with sudo, removed after; `ProgramArguments`: a root-owned copy `/usr/local/libexec/quotabus-root-test probe --force --config <local-only>` | `state="ok"`, `latency_ms=Some(2679)` (raw `root.err.log`, 13:27:00Z; the comment, line 112, says "ok") |
| 9. the confirmation: user LaunchAgent, production chain (14:57 UTC) | one-shot user LaunchAgent; `ProgramArguments[0]` `/opt/homebrew/bin/doppler`: `doppler run --project nusy-product-team --config dev -- quotabus probe --force --config <local-only>` | `state="ok"`, `latency_ms=Some(2750)` (raw `chain.err.log`, 14:57:10Z; commit `ad48dd1`; item body lines 28-29) |

Read together with cases 1-5: what the Mini session measured as "every launchd job is blocked" (cases 6 and 7) was
quotabus launched directly (case 7's program was `/bin/sh`, a platform binary; quotabus under it was still
blocked, in a daemon, so that run does not separate the two). Under doppler (cases 1, 2 and 9) the agent reaches DGX1. Case 7 also shows a user-keyring launcher cannot
serve a daemon: doppler could not read its token there. Case 9 repeats case 1 independently, 95 minutes later, from
the other session.

## Which unit (Captain 2026-10-10)

- Captain 2026-10-10: "It isn't listed — go with the LaunchDaemon" (item line 84). quotabus was not listed under
  Local Network; this was the option the Mini session recommended. Case 7 then showed a `UserName` daemon is blocked
  too.
- Captain 2026-10-10: "2" (item line 100). Of three options offered (1 probe the Sparks from DGX1, the recommended
  one; 2 a root LaunchDaemon on Mini; 3 an app identity), the Captain chose 2, NOT the recommended option. That
  choice was made on the Mini session's incorrect premise that every launchd job on Mini was blocked: its tests had
  launched quotabus directly, never under doppler (item line 116). Case 8 confirmed a root daemon works, and one was
  installed at 13:28 UTC (`com.congruentsys.quotabus-sparks`, root, DGX1 only, every 300 s; item line 112).
- Captain 2026-10-10: "One unit, remove daemon (Recommended)" (item line 116), the option the session recommended,
  given after M5's case 1. Its text: once the agent's tick confirms DGX1 reads ok, remove the root daemon, drop
  CHORE-023 as unneeded, and land this branch documenting the doppler grant.

So a root LaunchDaemon also works, but it is not the fleet's unit. The fleet's macOS unit is the one user LaunchAgent
whose program is the launcher, with the launcher granted Local Network. Removing the root daemon is a Captain-run
`sudo` step; it was still loaded on Mini when this was written (`launchctl print
system/com.congruentsys.quotabus-sparks`: `state = not running`, `runs = 19`, `last exit code = 0`, 2026-10-10
15:00 UTC).

## What it changes

`packaging/install.sh` prints a Local Network step in its macOS plan, naming the unit's first program (the
`--launcher`'s program, else the `--quotabus` path) and how to confirm it; `packaging/README.md` states the same
facts beside the gui-domain note.
