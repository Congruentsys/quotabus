---
id: CHORE-020
title: "Tell every machine quotabus exists: fleet canon pointer + nusy-product-team CLAUDE.md (raw nats kv reads)"
type: chore
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: []
---

# Tell every machine quotabus exists: fleet canon pointer + nusy-product-team CLAUDE.md (raw nats kv reads)

## Why

Captain 2026-10-09: "Once working we should update the CLAUDE.md on every machine to know this tool exists".
quotabus is working: VOY-001's DoD was measured MET on Mini's live bus (docs/findings/CHORE-016-mini-deploy.md), and
the probe job runs every 300 s in gui/501 on Mini.

Captain's rulings for this item, 2026-10-09 (AskUserQuestion; the agent's recommended defaults were "install the
reader" and "both"):
- **How other machines read:** "Raw nats kv". The Captain chose this over the agent's recommended default (install
  the `quotabus` reader binary on each machine). Nothing is installed on other machines. Agents read the bus with natscli
  (`nats kv ls ai_status`, `nats kv get ai_status <key> --raw`).
- **Where the line goes:** "Both (Recommended)", the default the agent offered. That means ONE short pointer line in the
  fleet-managed block (nusy-product-team `docs/AGENT-MACHINE-CANON.md`, which syncs to `~/.claude/CLAUDE.md` on every
  machine), plus the detail in nusy-product-team's project `CLAUDE.md`, following AGENT-MACHINE-CANON's own rule "the
  block stays deliberately short".

## Lane

other-repo (nusy-product-team). The packet is written here. ONE chore is filed on nusy-product-team's board
(`CLAUDE.md` § Skills, cross-repo rule), and this item waits `stranded` on it. Each machine's hand re-sync of the block
is done by that repo's own recipe, never by us.

## What the packet must carry (G1: a raw read must still honour the freshness rule)

A raw natscli read skips the binary's freshness rule, so the text the packet asks for states that rule itself
(DESIGN §2):
- an ABSENT key (`nats: error: nats: key not found`, rc 1) is UNKNOWN, never ok. The bucket has per-key TTL, so an
  expired row is physically absent (`kv info ai_status`: "Per-Key TTL Supported: true");
- `checked_at + ttl_s < now` is UNKNOWN, even when the key is present;
- only `"state":"ok"` is usable; every other state, and an `unknown` with `cannot_assess:`, is not.
- The key scheme is `api.<provider>.<account>.<model>` and `subscription.<provider>.<account>.<service>`.
  `alert.*` keys are the alerter's dedup state, not status.

Measured from M5 on 2026-10-09 (natscli, `--server nats://192.168.8.110:4222`): `kv ls ai_status` lists the 18
status keys plus 4 `alert.*` keys. `kv get ai_status api.deepseek.nusy-product-team.deepseek-v4-flash --raw` returns
one ai-status/1 JSON row with `state`, `checked_at` and `ttl_s`. An absent key returns "key not found", rc 1. `quotabus`
is not installed on M5.

## Definition of Done

- [ ] `docs/flowback/CHORE-020-to-nusy-product-team.md` is landed by the docs lane. It gives (a) the exact pointer
      line for the fleet-managed block in `docs/AGENT-MACHINE-CANON.md` (one line, inside the BEGIN/END markers), and
      (b) the exact `CLAUDE.md` section text: what quotabus is, the natscli commands, the freshness rule above, the
      key scheme, and that probing runs only on Mini. It also states the base sha of each target file in
      nusy-product-team, why, and the Captain's words verbatim.
- [ ] The packet quotes no key, account email or raw provider error.
- [ ] ONE nusy-product-team chore is filed via `nusy-kanban … create chore` with `--relate related:IDEA-13333`,
      linking the packet, quoting the Captain and asking for the landing sha. Each machine's re-sync follows that
      repo's recipe.
- [ ] This item is moved `stranded`, naming that chore's id.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T18:13:50+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-09T18:13:52+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
