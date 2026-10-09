---
id: CHORE-016
title: "Deploy the central probe on Mini (install.sh --host mini) and measure VOY-001's Definition of Done on the live bus"
type: chore
status: arrived
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: [HAZ-004]
---

# Deploy the central probe on Mini (install.sh --host mini) and measure VOY-001's Definition of Done on the live bus

VOY-001's Definition of Done says `quotabus status` on the hub shows every configured API key and every subscription account with ages. Today nothing is deployed. Measured on M5, 2026-10-09: `nats --server nats://192.168.8.110:4222 kv ls ai_status` printed "No keys found in bucket", and `kv info` showed Values Stored: 0. The 7 subscription rows that EXP-002 published by hand have expired (their TTL is 3 h). The install path exists (CHORE-008, `packaging/install.sh --host mini …`, `packaging/README.md` § The fleet: Mini), and the Captain ruled where the probe runs (SIG-001: "run on mini").

**Inputs only some hosts have:** SSH to Mini as `admin`, and Doppler on Mini (project nusy-product-team, config dev) holding every key the fleet config names. A session without these bounces the item with the reason.

## Definition of Done
1. A release build of `quotabus` is installed on Mini at `/usr/local/bin/quotabus`, with the `quotabus-cycle` wrapper from `packaging/README.md` (probe, then alert).
2. The fleet config is at `/usr/local/etc/quotabus/quotabus.toml` on Mini. It lists every API service in `examples/quotabus.toml` plus the six Claude accounts and Copilot (EXP-002). Its alert sink is `stdout` only (it goes to the unit's log) until SIG-010 is ruled and a board sink is chosen. The config is not in git if it carries anything fleet-private; its shape is recorded in the finding.
3. The unit is installed with the README's exact `install.sh --host mini …` command (`--quotabus /usr/local/bin/quotabus-cycle`), loaded, and has run at least two ticks.
4. A findings file `docs/findings/<ID>-mini-deploy.md` (measure lane, reviewed like code) records, with the exact commands and redacted output:
   - `quotabus status` on Mini: every configured service, each with a state and an age, and none missing;
   - `quotabus select --role review --exclude-family anthropic`: its stdout and rc (0 with a model, or 3);
   - `nats kv ls ai_status`: the row count;
   - the alert lines in the unit's log for any bad row: exactly one per crossing over two ticks.
5. No key appears in the unit, in argv (`ps`), in the log or in the finding. Only the `ai_status` bucket is written.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T02:58:53+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-09T03:10:57+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-09T03:26:49+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/25> ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-09 02:59)

Depends on HAZ-004: the README's fleet command names user admin and /usr/local paths, and none of them exist on Mini (measured). Read paths 1-3 of the DoD as the corrected ones that HAZ-004 lands: binary under /Users/hankh19/.local/bin, config at /Users/hankh19/.config/quotabus/quotabus.toml.

### Mac-mini/s-e7976c42 (2026-10-09 18:19)

Redeployed on Mini 2026-10-09 18:14 UTC (Captain: "deploy the central probe on Mini"): /Users/hankh19/.local/bin/quotabus replaced by a release build of quotabus main 26f63b8 (cargo build --release --locked; sha256 bad715baa3bc82d1…), so the live probe now runs CHORE-011 (used_pct rounding) and CHORE-018 (Captain SIG-010: [alert] states, default hard failures + window_near_limit). Previous binary kept as ~/.local/bin/quotabus.prev-20261009 (rollback: mv it back). Unit, wrapper and config unchanged. Pre-swap check: new binary 'quotabus status --config ~/.config/quotabus/quotabus.toml' rc 0, 16 services listed. First tick on it, 18:19:24–32 UTC: launchctl exit 0, 'alert: 16 keys checked, 0 crossings filed, 0 re-armed' (no re-file of the already-alerted kimi quota_exhausted / openai, together, xai auth_failed), no error lines. No key on argv: the only 'sk-' match in ps is the binary name nusy-task-events. Open: local-qwen reads cannot_assess:secret_unset (NUSY_LOCAL_QWEN not in Doppler nusy-product-team/dev).
