---
id: CHORE-029
title: "Redeploy Mini's central probe from the v1.0.0 tag (rows read quotabus@1.0.0)"
type: chore
status: backlog
priority: medium
assignee: null
created: 2026-10-11
depends_on: []
---

# Redeploy Mini's central probe from the v1.0.0 tag (rows read quotabus@1.0.0)

## Why
v1.0.0 is released (PR #37, merge ab14fe1, tag v1.0.0, https://github.com/Congruentsys/quotabus/releases/tag/v1.0.0). Mini's `~/.local/bin/quotabus` was built from deeb435 (CHORE-027) and prints `quotabus 0.1.0`, so every row it writes carries `observed_by …/quotabus@0.1.0`. The code is the same as v1.0.0 (only the version, CHANGELOG and README changed), so this is a relabel, not a behaviour change. Captain 2026-10-11: "yes, file the chore for Mini to redeploy v1.0.0".

## Inputs (host-specific)
Mini (user hankh19, gui/501), reached by `ssh mini`, and its Doppler login (project nusy-product-team, config dev). A session that cannot reach Mini bounces this item with the reason.

## Plan
Measure lane, following CHORE-027's steps (`docs/findings/CHORE-027-mini-redeploy.md`) exactly, with these differences:
1. Build from the tag: a `git archive v1.0.0` of origin, `cargo build --release --locked` on Mini. Do not change the config; it was already updated in CHORE-027.
2. Back up the current binary as `~/.local/bin/quotabus.prev-20261011` and install the new one in place. The launchd unit's program is doppler, so HAZ-005's Local Network grant still applies.
3. Check that `quotabus --version` prints `quotabus 1.0.0`.
4. Run one forced cycle under gui/501 doppler, as in CHORE-027, then wait for the next regular tick.

## Definition of Done
- `docs/findings/CHORE-028+1-…`: a findings file `docs/findings/<ID>-mini-v1.0.0.md`, landed by a PR with a distinct reviewer who re-runs the read-only commands. It records:
  - the commands and their redacted output, the host, the date, and the tag's sha;
  - `quotabus --version` on Mini printing `quotabus 1.0.0`;
  - one regular launchd tick after the install exiting 0, with its alert summary line;
  - `nats kv get ai_status <a fresh key>` (or `status --json`) showing `observed_by` ending `quotabus@1.0.0`;
  - `quotabus status` with every API, local and subscription row present, and `select --role review --exclude-family anthropic` rc 0 or rc 3 with the reason;
  - the rollback: restore `quotabus.prev-20261011`.
- No secret appears anywhere. The only bus writes are quotabus's own rows in `ai_status`.