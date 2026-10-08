# Review brief — CHORE-004 r1 (quotabus PR #4)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The work was written by a
docs-author sub-agent of the resident session M5-MBP-2/s-72a67d16 (Claude Opus 5.5), which commissioned it, checked
it, also ran the CHORE-002 measurements it cites, and wants it approved.

**What to review:** PR #4 of https://github.com/Congruentsys/quotabus, branch `docs/CHORE-004-design-s4-measured`,
at the tip that contains this brief (run `git ls-remote origin docs/CHORE-004-design-s4-measured` and review THAT
sha; call it <SHA>). The item is `kanban-work/chores/CHORE-004-*.md` (read it on `origin/main`). The diff changes
`docs/DESIGN.md` only. Its sources are `docs/findings/CHORE-002-statusline-under-claude-p.md` and
`docs/findings/CHORE-002-zai-endpoint.md` (on main, reviewed in `reviews/CHORE-002-r1.md`), and the open signals
`kanban-work/signals/SIG-003-*.md` and `SIG-008-*.md`.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-004 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-004/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-004`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The Definition of Done, item by item (`git diff origin/main...<SHA> -- docs/DESIGN.md`).
2. Every fact the diff adds traces to a primary line in a findings file. Recount at least four numbers or claims
   (e.g. "three of the five endpoints", "35-token", "5 h and weekly", "epoch ms", "first render") against those lines.
   A claim stronger or weaker than its source is a finding.
3. **It records and decides nothing:** SIG-003 and SIG-008 are quoted as open everywhere. The Captain's Q9 decision is
   represented faithfully against what is on file (VOY-001 "Captain's rulings", EXP-002 step 2), with no invented
   quote. Nothing reads as a new design decision.
4. Nothing else in DESIGN.md now contradicts the change (e.g. other mentions of "measure first", Q9, statusLine,
   z.ai balance, §9's E2/C2 rows). Name any stale sentence.
5. Secrets / public repo: no key, token, email, account, order or agreement number, price or purchase date in the
   diff or the PR body.
6. Run the gate yourself in your worktree: `make check`. Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes, NO GitHub writes, and no commit other than your
review file. Never print a secret.

**Output:** write `reviews/CHORE-004-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-004-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the author with this committed brief; commissioning session M5-MBP-2/s-72a67d16 and its stake: it
commissioned and checked the diff, ran the measurements it cites, and gains from `approve`). Then numbered findings
`F1…` each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose.
`verdict: changes` only for blocker or should-fix findings.
Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-004 switch -c rv-tmp && git add reviews/CHORE-004-r1.md &&
git commit -m "CHORE-004: review r1" && git push origin HEAD:docs/CHORE-004-design-s4-measured`), committing as
`Hank Head <237287+hankh95@users.noreply.github.com>`. If the push is rejected, report it — do not force.
Your final message: the verdict line and the finding titles.
