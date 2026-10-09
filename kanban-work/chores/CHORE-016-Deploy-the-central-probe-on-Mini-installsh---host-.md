---
id: CHORE-016
title: "Deploy the central probe on Mini (install.sh --host mini) and measure VOY-001's Definition of Done on the live bus"
type: chore
status: provisioning
priority: medium
assignee: null
created: 2026-10-09
depends_on: []
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
  ] .
```
