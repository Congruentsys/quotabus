---
id: CHORE-028
title: "Remove the leftover alert.local.* dedup entry on Mini's bus (CHORE-027 r1 F5)"
type: chore
status: provisioning
priority: medium
assignee: null
created: 2026-10-10
depends_on: []
---

# Remove the leftover alert.local.* dedup entry on Mini's bus (CHORE-027 r1 F5)

From CHORE-027 r1 F5 (docs/findings/CHORE-027-mini-redeploy.md step 8). Mini's bus holds `alert.local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4` = {state ok, at 2026-10-10T13:26:51Z}. The pre-CHORE-021 binary wrote it. Since CHORE-021, alert never reads or writes a local service's entry, and dedup entries have no per-key TTL (DESIGN.md:268, bucket Maximum Age unlimited). So it stays forever as clutter. It is harmless: it is not a status row.

Inputs: Mini's bus (nats://192.168.8.110:4222, bucket ai_status: this repo's own bucket).

## Done when
1. This item authorises deleting exactly the `alert.local.*` keys in `ai_status` (CLAUDE.md rule 4: a delete of this repo's own key, only when an item says so). List them first with `nats kv ls ai_status | grep ^alert.local`, delete each one with `nats kv del ai_status <key> -f`, and list again: none left. Every other key is untouched (the count of non-alert.local keys is the same before and after).
2. The commands and their output are recorded on this item. Decide, and record on this item, whether `alert` itself should delete a local service's leftover entry in future. If it should, file that as a code item.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-10T20:42:19+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
