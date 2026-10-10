# Packaging

`install.sh` installs the **central probe** (DESIGN §3, §5): a TICK, not an interval: the unit's
`StartInterval` (launchd) or timer period (systemd) is `--interval` seconds, and each `quotabus probe` run makes only
the calls that are due (each query kind on its own `[intervals]` entry in the config, measured from that kind's last
row in the store). The intervals live only in the config; the tick bounds how late a due kind can run. The fleet's
tick is 300 s. Where it runs is chosen at install time (DESIGN §10 Q2, SIG-001):
`--where local` (this host, as the current user) or `--host <name>` (a named host over SSH, with that host's
`--user` and `--home`). With neither flag it asks. It renders the unit for the target's OS with paths derived from
the target's home — a launchd agent on macOS (`<home>/Library/LaunchAgents/com.congruentsys.quotabus-probe.plist`,
logs in `<home>/Library/Logs/quotabus/`), a systemd user service and timer on Linux
(`<home>/.config/systemd/user/quotabus-probe.{service,timer}`) — then copies it into place and loads it
(`launchctl bootout` then `launchctl bootstrap gui/<uid>`, the uid computed on the target, so the target user must be
logged in at the GUI; or `loginctl enable-linger <user>` (so the timer outlives the session) and
`systemctl --user enable --now quotabus-probe.timer`; over `ssh`/`scp` for `--host`).
`--render-to <dir>` is a dry run: it writes the unit(s) into `<dir>`, prints the plan with each absolute install
path, and connects to, loads and writes under nothing else. `install.sh --help` lists every flag.

**Nothing runs on the other hosts.** The central probe is the only unit: it reads every subscription account too
(each Claude account by its own setup-token, Copilot by its token, all injected by the launcher like any API key;
DESIGN §4, EXP-002), so no agent, statusLine hook or timer is installed on any other host, and no host's
`~/.claude/settings.json` is touched (the Captain, 2026-10-08, rescoping EXP-002).

**No secret is in any unit or on argv.** Keys reach the binary only through the launcher that wraps it, given as
`--launcher "<command and args>"` — e.g. `doppler run --project … --config … --` or `secretspec run --` — which
injects the environment variables the config's `secret` names list.
The launcher string is stored in the unit and printed in the plan, so it must carry **no secret**: `install.sh`
refuses a launcher word that looks like one (`KEY=…`, `TOKEN=…`, `SECRET=…`, `PASSWORD=…`, `--token`, `sk-…`). Logs are redacted before they are written.

## The fleet: Mini

The fleet's central probe host is **Mini** (the Captain, 2026-10-08, on SIG-001: "If this is FOSS, the config will
have to ask to run locally or on another host. In this case, run on mini"). This command, run from a checkout on any
fleet host with SSH to Mini, installs the unit EXP-001 shipped for Mini as CHORE-007 changed it (the 300 s tick), field for field except
the user, home and paths, which HAZ-004 moved to the ones measured on Mini:

```
packaging/install.sh --host mini --user hankh19 --home /Users/hankh19 --os macos --launcher "/opt/homebrew/bin/doppler run --project nusy-product-team --config dev --" --quotabus /Users/hankh19/.local/bin/quotabus --probe-config /Users/hankh19/.config/quotabus/quotabus.toml --interval 300
```

Add `--render-to <dir>` to see the plist and the plan without touching Mini.

The user, home and paths are measured, not assumed (HAZ-004, from M5 over `ssh mini`, 2026-10-09): Mini has no
`admin` user (its only login is `hankh19`, uid 501), and `/usr/local/bin` and `/usr/local/etc` do not exist there, so
the binary goes to `~/.local/bin/` and the config to `~/.config/quotabus/`, user-owned paths that need no sudo.
The unit must load in the GUI domain (`gui/501`, the user logged in at the GUI), not from a plain SSH session:
Doppler's token is in the login keychain, and measured on Mini a `doppler run` over plain `ssh` fails with
"Unable to retrieve value from system keyring" while a job in `gui/501` reads it.

## The Sparks: a root LaunchDaemon, observe-only in the agent

On macOS, Local Network privacy blocks `quotabus` from reaching the Sparks (LAN hosts) when it runs as a user
LaunchAgent or as a LaunchDaemon with a `UserName`: measured on Mini 2026-10-10 (HAZ-005's comments), both wrote
`cannot_assess:unreachable`, "No route to host (os error 65)", while the same binary and config from an interactive
shell read `ok`. A **root** LaunchDaemon (no `UserName`) is exempt and reads `ok`. (M5 measured, the same day, that the
user agent also reaches DGX1 when its program is `doppler` holding the Local Network grant; the Captain chose the root
daemon.) A root daemon cannot read doppler's token ("Token not found in system keyring"), so the split is:

- **root daemon** — the key-free Sparks only. `/Library/LaunchDaemons/com.congruentsys.quotabus-sparks.plist`, root,
  `StartInterval` 300, `ProgramArguments` = `/usr/local/libexec/quotabus probe --config <root-owned sparks.toml>`
  (binary and config root-owned; the exact paths are in HAZ-005's 2026-10-10 13:33 comment), logs in
  `/Library/Logs/quotabus/sparks.{out,err}.log`. Its config lists only the `kind = "local"` services, with no `secret` and no launcher (a local probe is free, so
  `[intervals] api = "5m"`), and the same `[bus]` as the agent's config, so it writes the same rows.
- **user agent** (the unit above) — every keyed service, probed as before, plus the Sparks with `probe = false`
  (`examples/quotabus.toml`'s `local-qwen`): it never probes them and writes no row for them, but its `status`,
  `select` and `alert` list them, reading the daemon's rows under the normal freshness rule (no fresh row reads
  UNKNOWN).

Undo: `sudo launchctl bootout system/com.congruentsys.quotabus-sparks`, then remove the plist,
`/usr/local/libexec/quotabus` and the daemon's `sparks.toml`, and drop `probe = false` from the agent's
config.

## Alerts: `quotabus alert` after each probe cycle

`quotabus alert` (DESIGN §3 alert; EXP-004) runs **after each probe cycle, in the same unit**, so it reads the rows
the cycle just wrote and runs under the same launcher (the redactor then knows every key, and the kanban CLIs get the
environment they need). `install.sh` renders `<launcher> <quotabus> probe --config <file>`; chain the alert by
pointing `--quotabus` at a two-line wrapper, which receives `probe --config <file>`:

```sh
#!/bin/sh
# /Users/hankh19/.local/bin/quotabus-cycle — one probe cycle, then the alert over its rows; exits non-zero if either failed
shift                                   # drop "probe"; "$@" is now --config <file>
export QUOTABUS_CLAUDE_BIN=/opt/homebrew/bin/claude   # the unit's PATH has no /opt/homebrew/bin (HAZ-004)
/Users/hankh19/.local/bin/quotabus probe "$@"; rc=$?
/Users/hankh19/.local/bin/quotabus alert "$@" || rc=$?
exit $rc
```

and install with `--quotabus /Users/hankh19/.local/bin/quotabus-cycle` (every other flag as above). The probe's
stream-json fallback finds `claude` through `QUOTABUS_CLAUDE_BIN`, else `PATH`, and a unit's `PATH` is minimal: measured
on Mini (HAZ-004, 2026-10-09), `claude` is at `/opt/homebrew/bin/claude` while a launchd unit's default `PATH` is
`/usr/bin:/bin:/usr/sbin:/sbin`, so without the export the fallback records "no claude binary". Running the alert on every
tick, including ticks where nothing was due, is cheap and safe: it reads the store, files only a crossing (a service
going bad), keeps its dedup state at `alert.<key>` in the same bucket, and writes nothing else. A sink that fails
leaves the crossing unrecorded, so the next tick retries it; its rc is 1 and its line is in the unit's error log.

In `[alert.*]`, give each `command` as an **absolute path**: launchd and systemd units run with a minimal `PATH`.
`yurtle-kanban create … --push` files on the board of the directory it runs in (the unit's working directory), so
point its `command` at a wrapper that `cd`s into the board's repo first. A sink's environment is an allowlist only
(`PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `TERM`, each if set): none of the launcher's secrets reach it, and neither
does anything else, so the same wrapper sets what the CLI needs (nusy-kanban's `--server`, `SSH_AUTH_SOCK` for a
`git push` over SSH).
