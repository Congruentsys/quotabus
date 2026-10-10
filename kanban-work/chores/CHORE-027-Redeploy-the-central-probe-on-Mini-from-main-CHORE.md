---
id: CHORE-027
title: "Redeploy the central probe on Mini from main (CHORE-021/022/024 live; DGX1/DGX2 discovery config)"
type: chore
status: arrived
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-10
depends_on: []
---

# Redeploy the central probe on Mini from main (CHORE-021/022/024 live; DGX1/DGX2 discovery config)

## Why

Captain 2026-10-10: "create a chore for mini to redeploy it". Mini's central probe binary, `/Users/hankh19/.local/bin/quotabus` (quotabus 0.1.0, installed 2026-10-09 18:14), predates everything merged since. The live tick still prints the old summary, "alert: 16 keys checked, 0 crossings filed, 0 re-armed" (CHORE-026's close, 15:27:09 UTC). Not yet live on Mini:
- **CHORE-021** (#30): `kind = "local"` services file no alert.
- **CHORE-022** (#31): a local service with no `models` discovers what `/v1/models` serves; `status`/`select` read only its newest cycle.
- **CHORE-024** (#32): the summary line is `alert: <N> keys checked, <M> local skipped, <F> crossings filed, <C> re-armed`.
- **CHORE-025** (#33): tests only.

## Inputs (host-specific)

Mini only: the `hankh19` GUI login (the unit runs in `gui/501`; doppler's token is in the login keychain, `packaging/README.md`), the Rust toolchain or a build copied from another host, and the bus at `nats://192.168.8.110:4222`. A session that cannot reach Mini bounces this with the reason.

## Plan

1. Build `quotabus` from `origin/main` (record the sha). Use `cargo build --release --locked` with a `CARGO_TARGET_DIR` outside the tree.
2. Keep the current binary as `~/.local/bin/quotabus.prev-<date>`, then install the new one at `~/.local/bin/quotabus`.
   - Under the doppler launcher, replacing the binary does not reset the Local Network grant (HAZ-005 case 2, `docs/findings/HAZ-005-local-network.md`). Confirm this, don't assume it.
3. Change Mini's config `~/.config/quotabus/quotabus.toml` to match `examples/quotabus.toml` (CHORE-022):
   - replace the fixed-`models` `local-qwen` service with `dgx1-qwen` (`http://192.168.8.120:8000/v1`) and `dgx2-qwen` (`http://192.168.8.121:8000/v1`), both `kind = "local"` with no `secret` and no `models`;
   - keep a dated backup of the old config.
4. Run one forced cycle the way the unit does, inside `gui/501`. A plain ssh shell is not equivalent: the keychain, Local Network and PATH all differ (HAZ-004, HAZ-005). Use a one-shot LaunchAgent through doppler, like HAZ-005's, then let the regular 300 s tick run.

## Definition of Done

1. `~/.local/bin/quotabus` is built from a recorded `origin/main` sha. The previous binary and config are kept, dated.
2. The next REGULAR launchd tick after the swap exits 0 (`launchctl print gui/501/com.congruentsys.quotabus-probe` last exit code). Its alert line has the new form, `… keys checked, <M> local skipped, …`, with M ≥ 1.
3. `quotabus status` on Mini shows:
   - a DGX1 row for the model it serves (`ok`, or an honest state if the box is busy), keyed by the served id, not `local-qwen`;
   - a DGX2 row reading its real state;
   - every API and subscription row from before the swap, present and fresh.
4. The old `local.qwen.dgx1.qwen3` and `local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4` rows age out by their TTL; nothing purges the bucket (CLAUDE.md rule 4). Say how many hours remain on each.
5. The commands, their redacted output, the host, the date and the sha are recorded on this item (or in a short `docs/findings/CHORE-027-*.md`). No key is printed.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-10T20:27:27+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-10T20:27:35+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-10T20:42:13+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/36> ;
  ] .
```
