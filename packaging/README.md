# Packaging

- `launchd/com.congruentsys.quotabus-probe.plist` — the central probe on Mini (DESIGN §3, §5): a TICK, not an
  interval: `StartInterval` 300 runs `quotabus probe` every 300 s, and each run makes only the calls that are due (each
  query kind on its own `[intervals]` entry in the config, measured from that kind's last row in the store). The
  intervals live only in the config; the tick bounds how late a due kind can run. The plist holds **no secret**: keys reach the binary only through
  the launcher it wraps — `doppler run --project … --config … --` as written, or replace the first arguments with
  `secretspec run --` — which injects the environment variables the config's `secret` names list. Adjust the
  launcher path, project, config and `--config` path to the host, and replace the user `admin` in the
  `StandardOutPath` / `StandardErrorPath` (`/Users/admin/Library/Logs/quotabus/…`) with the account that runs it;
  create that directory first (`mkdir -p ~/Library/Logs/quotabus`) — launchd does not. Then `cp` the plist to
  `~/Library/LaunchAgents/` and `launchctl load` it. Logs are redacted before they are written.
