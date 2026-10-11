---
id: CHORE-031
title: "Release v1.0.1 (CHORE-030's status fix) and redeploy Mini from the tag"
type: chore
status: underway
priority: medium
assignee: Mac-mini/s-e7976c42
created: 2026-10-11
depends_on: []
related: [CHORE-029, CHORE-030]
---

# Release v1.0.1 (CHORE-030's status fix) and redeploy Mini from the tag

## Why
Since v1.0.0 (tag object b153526, commit ab14fe1), main carries one behaviour change: CHORE-030 (#39, merge afe953f). It makes `quotabus status` count "(resets in …)" from the reader's now, not from `checked_at`, and render a past reset as "(reset due)". Mini runs v1.0.0 (CHORE-029), so its `status` table still shows the old text. A redeploy from an untagged main would label rows with a version that names no release, so the fix ships as a patch release first.

The driver session offered "cut a v1.0.1 release with a CHANGELOG entry and then redeploy Mini from that tag, the same way as CHORE-029". Captain 2026-10-11: "yes" (the Captain chose the default the agent offered).

## Inputs (host-specific)
Mini (user hankh19, gui/501) and its Doppler login (project nusy-product-team, config dev), for part 2. A session that cannot reach Mini bounces this item with the reason.

## Plan
1. **Release (code lane, as v1.0.0's PR #37):**
   - bump `Cargo.toml`/`Cargo.lock` to 1.0.1 (`publish = false` stays);
   - add a `## [1.0.1] - <date>` CHANGELOG section with a `### Fixed` entry for CHORE-030 (#39);
   - set the README status line to v1.0.1;
   - review by a distinct `claude -p` session; merge;
   - push an annotated tag `v1.0.1` on the merge commit, and create a GitHub release from that CHANGELOG section.
   No crates.io publish.
2. **Redeploy (measure lane, CHORE-029's steps exactly):**
   - `git archive v1.0.1`, then `cargo build --release --locked` on Mini;
   - back up the current binary as `~/.local/bin/quotabus.prev-v1.0.0`, and install the new one in place;
   - run one forced cycle under the gui/501 doppler launcher, then wait for the next regular tick.
   The config is unchanged.

## Definition of Done
- PR merged with the 1.0.1 bump and the CHANGELOG entry. `git ls-remote origin refs/tags/v1.0.1` is non-empty, and `gh release view v1.0.1` exists.
- `docs/findings/CHORE-031-mini-v1.0.1.md` is landed by a PR with a distinct reviewer who re-runs the read-only commands. It records:
  - `quotabus --version` on Mini printing `quotabus 1.0.1`;
  - the forced cycle and one regular tick exiting 0, each with its alert line;
  - a fresh row's `observed_by` ending `quotabus@1.0.1`;
  - `status` with every configured row present and a subscription row whose "(resets in …)" counts from now;
  - `select --role review --exclude-family anthropic` rc 0 or rc 3 with the reason;
  - the rollback.
- No secret appears anywhere. The only bus writes are quotabus's own rows in `ai_status`.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-11T17:39:37+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-11T17:39:39+00:00"^^xsd:dateTime ;
    kb:by "Mac-mini/s-e7976c42" ;
  ] .
```
