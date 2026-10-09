You are an INDEPENDENT reviewer, a fresh claude -p session that wrote none of this work, of quotabus PR #19 (https://github.com/Congruentsys/quotabus/pull/19), branch `docs/CHORE-012-seats-never`. Review the branch's tip (the commit that adds this brief; its parent is edab12a8e2dc206b95957ffeaf8dfe57747ce1c5), in your OWN scratch worktree: `cd /Users/hankh95/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-012 origin/docs/CHORE-012-seats-never && ln -s /Users/hankh95/Projects/quotabus/.venv /tmp/rv-CHORE-012/.venv`. Remove it when done (`git worktree remove /tmp/rv-CHORE-012`), after pushing your review from it.

This is a DOCS change to docs/DESIGN.md §3 and docs/wiring/nusy-product-team.md. The item is kanban-work/chores/CHORE-012-*.md. Its Definition of Done: "DESIGN §3 (the select section) and the wiring note state the rule as decided, citing SIG-009's steer comment and noting that a seat-routing feature would be a new item. No other text changes. `make check` is green."

Check:
1. Each DoD line is met (quote path:line evidence).
2. Every quote and citation is accurate against its PRIMARY source: the SIG-009 file's steer comment (kanban-work/signals/SIG-009-*.md), and the line range cited. Recount, don't trust.
3. Who decided is stated accurately: an agent session's steer pass (bucket 2), NOT a Captain ruling, open to the Captain's veto. Flag any wording that could be read as a Captain decision.
4. "No other text changes": `git diff origin/main...HEAD -- docs/` touches only the two paragraphs.
5. No secret, email or key appears anywhere in the diff.
6. Run the gate yourself in your worktree: `PATH=/Users/hankh95/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-012 make check`, in the FOREGROUND.

Rules: run every command in the FOREGROUND (no background, &, nohup). Make NO board writes (no yurtle-kanban writes), NO GitHub writes (no gh pr comment/review), and no commits other than the review file. Every finding quotes the command you ran and its output.

Write reviews/CHORE-012-r1.md in your worktree with:
line 1: `reviewed-at-sha: <the tip sha you reviewed>`
line 2: `verdict: approve` or `verdict: changes`
line 3: `gate: make check rc=<N> at <SHA> on <host>` (or `gate: NOT RUN — <reason>`)
line 4: `brief: reviews/CHORE-012-r1.brief.md @ <the tip sha>`
then an `Authorship:` paragraph: all the work and this review were done by LLM agent sessions (Claude Opus 5.5); no human reviewed it; "distinct" means a separate claude -p process with a fresh context that wrote none of the work, of the SAME model family, briefed by the driver (the author) with this committed brief; the commissioning session is M5-MBP-2/s-72a67d16, which drove the item and briefed the docs author (it wrote no text itself).
Then numbered findings F1… (each with the command and output), or "No findings".
Commit it as `CHORE-012: review r1` with author `Hank Head <237287+hankh95@users.noreply.github.com>` (git -c user.name="Hank Head" -c user.email=237287+hankh95@users.noreply.github.com commit), and push: `git push -q origin HEAD:docs/CHORE-012-seats-never`.
