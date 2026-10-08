# Review brief — CHORE-003 r1 (quotabus PR #1)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The author is the
resident session M5/s-b1fd4c67 (Claude Opus 5.5), which wrote every file in the diff and wants this approved.

**What to review:** PR #1 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-003-port-skills`, at the
tip that contains this brief (run `git ls-remote origin chore/CHORE-003-port-skills` and review THAT sha; call it
<SHA>). The item is `kanban-work/chores/CHORE-003-*.md` (read it on `origin/main`). The PR body is `gh pr view 1`.
The skills were ported from `Congruentsys/nusy-replicant-24@2de49870`; a read-only clone may exist at
/tmp/nr24-ref (if not: `gh repo clone Congruentsys/nusy-replicant-24 /tmp/rv-CHORE-003-src -- --depth 1`).

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-003 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-003/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-003`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The DoD, line by line: (a) `quotabus-next` run AS WRITTEN (copy its pass from the branch's
   `.claude/skills/quotabus-next/SKILL.md`, with `QB_AGENT=M5`) prints a pick from this board — read-only verbs only
   (`next`, `list`, `control status`, `show`); do NOT run `claim`; (b) no skill or script names nusy-replicant-24,
   LUM, `research/LUM` or `r24` (grep it yourself, do not only trust the test); (c) the PR lists kept / adapted / cut.
2. The central claim: yurtle-kanban 3.4.0 (`.venv/bin/yurtle-kanban`) pushes `move`, `comment`, `rank`,
   `voyage add` by default, so `scripts/yk_push.sh` can be retired. Verify it yourself against a THROWAWAY bare git
   origin under /tmp/rv-CHORE-003-sbx (never this repo's origin), including a write made from a feature branch.
3. The skills are internally consistent and executable: every command in `quotabus-next`, `quotabus-loop` and
   `pairit` uses flags that exist (`--help`), status names `move` accepts on this board's nautical theme, and
   `gh` flags that exist. Nothing still refers to a deleted file or a section that does not exist.
4. Nothing a quotabus rule needs was lost in the port: compare against the source skills; name any rule that was cut
   but still applies to this repo (and any that was kept but cannot apply).
5. Tests: `git diff 8d626cb..<SHA> -- tests/` should be empty (the tests were committed red at 8d626cb). Mutate
   `scripts/qb_clean.sh` at two points (e.g. drop the `data runs` artefact check; drop the commits-not-on-origin
   check) and show a test go red each time; restore it after. Mutate a skill (put `scripts/yk_push.sh` back into one)
   and show `tests/test_skills_port.py` go red; restore it.
6. Secrets / public repo: nothing in the diff or the PR body carries a key, a token, an email or a private host
   detail that should not be public.
7. Run the gate yourself in your worktree: `make check`. Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-003-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-003-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the author with this committed brief; commissioning session M5/s-b1fd4c67 and its stake: it wrote the
whole diff and gains from `approve`). Then numbered findings `F1…` each with severity (blocker / should-fix / nit),
the command, its output, and the fix you propose. `verdict: changes` only for blocker or should-fix findings.
Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-003 switch -c rv-tmp && git add reviews/CHORE-003-r1.md &&
git commit -m "CHORE-003: review r1" && git push origin HEAD:chore/CHORE-003-port-skills`), committing as
`Hank Head <237287+hankh95@users.noreply.github.com>`. If the push is rejected, report it — do not force.
Your final message: the verdict line and the finding titles.
