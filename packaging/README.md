# Packaging

- `launchd/com.congruentsys.quotabus-probe.plist` — the central probe on Mini (DESIGN §3, §5): one `quotabus probe`
  cycle every 900 s (the 15 min `[probe] interval`). The plist holds **no secret**: keys reach the binary only through
  the launcher it wraps — `doppler run --project … --config … --` as written, or replace the first arguments with
  `secretspec run --` — which injects the environment variables the config's `secret` names list. Adjust the
  launcher path, project, config and `--config` path to the host, and replace the user `admin` in the
  `StandardOutPath` / `StandardErrorPath` (`/Users/admin/Library/Logs/quotabus/…`) with the account that runs it;
  create that directory first (`mkdir -p ~/Library/Logs/quotabus`) — launchd does not. Then `cp` the plist to
  `~/Library/LaunchAgents/` and `launchctl load` it. Logs are redacted before they are written.
