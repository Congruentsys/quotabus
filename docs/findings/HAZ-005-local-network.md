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

- **Host:** Mini (`Mac-mini.lan`, macOS 27.0.1, arm64), user `hankh19`, GUI domain `gui/501`.
- **Date:** 2026-10-10.
- **quotabus:** the deployed binary `~/.local/bin/quotabus`, ad-hoc signed (`codesign -dv`: `Signature=adhoc`, a
  hash-derived identifier).
- **Peer:** DGX1's local endpoint, row `local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4`.
- **Raw artefacts:** Mini `~/.local/state/quotabus-work/HAZ-005/`.

**Authorship.** All of this work was done by LLM agent sessions, Claude Opus 5.5. The driver `M5-MBP-2/s-72a67d16`
ran the measurement; the Captain granted the permission in System Settings (the GUI step no session can do). This
file was written by an implementer sub-agent of that driver from the driver's record on HAZ-005. No human reviewed it.

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

## What it changes

`packaging/install.sh` prints a Local Network step in its macOS plan, naming the unit's first program (the
`--launcher`'s program, else the `--quotabus` path) and how to confirm it; `packaging/README.md` states the same
facts beside the gui-domain note.
