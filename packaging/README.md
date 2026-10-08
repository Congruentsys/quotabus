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

**No secret is in any unit or on argv.** Keys reach the binary only through the launcher that wraps it, given as
`--launcher "<command and args>"` — e.g. `doppler run --project … --config … --` or `secretspec run --` — which
injects the environment variables the config's `secret` names list.
The launcher string is stored in the unit and printed in the plan, so it must carry **no secret**: `install.sh`
refuses a launcher word that looks like one (`KEY=…`, `TOKEN=…`, `SECRET=…`, `PASSWORD=…`, `--token`, `sk-…`). Logs are redacted before they are written.

## The fleet: Mini

The fleet's central probe host is **Mini** (the Captain, 2026-10-08, on SIG-001: "If this is FOSS, the config will
have to ask to run locally or on another host. In this case, run on mini"). This command, run from a checkout on any
fleet host with SSH to Mini, installs the unit EXP-001 shipped for Mini as CHORE-007 changed it (the 300 s tick), field for field:

```
packaging/install.sh --host mini --user admin --home /Users/admin --os macos --launcher "/opt/homebrew/bin/doppler run --project nusy-product-team --config dev --" --quotabus /usr/local/bin/quotabus --probe-config /usr/local/etc/quotabus/quotabus.toml --interval 300
```

Add `--render-to <dir>` to see the plist and the plan without touching Mini.
