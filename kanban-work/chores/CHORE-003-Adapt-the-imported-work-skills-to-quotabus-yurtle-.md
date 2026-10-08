---
id: CHORE-003
title: "Adapt the imported work skills to quotabus — yurtle-kanban board, GitHub PRs, no fleet Layer-B tooling"
type: chore
status: backlog
priority: high
assignee: null
created: 2026-10-08
tags: [v1.0, VOY-001, skills]
depends_on: []
---

# Adapt the imported work skills to quotabus — yurtle-kanban board, GitHub PRs, no fleet Layer-B tooling

Part of VOY-001; FIRST, before EXP-001 is landed through a loop. Captain 2026-10-08: copy over our skills (campaign-loop or one of the loop skills, and the other work skills). Twelve were imported verbatim from nusy-product-team with a notice at the top of each (`.claude/skills/{campaign-loop,campaign-next,pairit,workit,reviewit,approveit,refine-idea,plan-phase,hypothesize,steer,uxr-review,editor-review}`); yurtle-kanban's own ten skills (work, review, done, …) were installed by `init` and stay as they are.

## Plan
1. **campaign-next** → a picker over `yurtle-kanban list --pickable --json` (or `claim --next`) scoped to a voyage/tag; drop the fleet cord, host routing, packets and NATS endpoint logic.
2. **campaign-loop** → pick, land through pairit, pick again; keep "never ScheduleWakeup / end the turn between items" and the clean-up step; replace the wait loop's bus reads with yurtle-kanban.
3. **pairit** → tests first by a partner, code by an implementer, ONE review by a distinct `claude -p` session posting its verdict on the GitHub PR (first line `reviewed-at-sha:`), findings fixed at once, merge via `gh pr merge`; no verdict rows, no scratch-worktree merge transport unless it helps.
4. **workit / reviewit / approveit** → keep only what a PR flow needs, or retire them in favour of pairit + yurtle-kanban's own `work`/`review` (state which).
5. **refine-idea, plan-phase, hypothesize, steer, uxr-review, editor-review** → swap nusy-kanban for yurtle-kanban; drop fleet-only references.
6. Remove each skill's IMPORTED notice once adapted; anything with no equivalent is cut, not left as instructions that cannot run.

## Definition of Done
Each kept skill runs here as written (a dry run of campaign-next prints a pick from this board); no skill text calls `nusy-kanban`, `nk`, `verdicts`, `scripts/lib/`, or a monorepo path as an instruction; the PR lists every skill kept, adapted or retired.