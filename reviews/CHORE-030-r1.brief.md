# Review brief — CHORE-030 r1 (quotabus PR #39)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver. It filed the item from the CHORE-029 r1 reviewer's side note.
A fresh Opus sub-agent (the test partner) wrote the tests at 13d5e2e. A second fresh Opus sub-agent (the implementer)
wrote the fix at 228240b. The driver wants this approved.

**What to review:** PR #39 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-030-resets-in-now`, at the
tip that contains this brief. Run `git ls-remote origin chore/CHORE-030-resets-in-now` and call that tip <SHA>. The
item is `kanban-work/chores/CHORE-030-*.md`; read it on `origin/main`. Read the PR body with `gh pr view 39`. The
design it cites is `docs/DESIGN.md` §2, the `headroom` row.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-030 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-030/.venv`.
Remove it when you are done (`git worktree remove --force /tmp/rv-CHORE-030`), after your review file is pushed.

**Check each of these. For every finding, quote the command and its output; a finding you did not run is not a finding.**
1. The tests at 13d5e2e match the item's "Done when" and Plan, not the code. `git diff 13d5e2e..<SHA> -- tests/` is
   empty.
2. The tests use real clock time with tolerance windows. Judge whether they can flake, e.g. on a slow CI runner or
   across a minute boundary. Run the test file at least 3 times: `CARGO_TARGET_DIR=/tmp/rv-CHORE-030-target
   /Users/hankh19/.cargo/bin/cargo test --test chore030_resets_in_now`.
3. The code does not special-case the tests. Check that AGE and "resets in" use the same `now`. Check that
   `status --json`, the stored row and the meaning of `reset_in_s` are unchanged. Check that `select` and `alert` are
   untouched. Check the edge cases: an unparseable `reset_at`, a row with neither field, a row with `reset_in_s` but
   no `checked_at`/age. Say what each renders.
4. Mutate the code at two points and show a test go red each time; restore the code afterwards. For example: drop the
   `- age_s(...)` in the fallback; change `s > 0` to `s >= -60`, or use `reset_in_s` ahead of `reset_at`.
5. Check the behaviour note in the PR body: a window with `reset_at` but no `reset_in_s` now shows reset text. Say
   whether that is right under DESIGN §2 and G1 ("never a false ok"; it is display only).
6. Secrets: nothing in the diff or the PR body carries a key, a token or an email.
7. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-030-target make check`. Record its rc, then remove that target dir.

**Rules for you:**
- Run EVERY command in the FOREGROUND: no `run_in_background`, no `&`, no `nohup`. This session ends when you stop,
  which kills any background work.
- Make NO board writes and NO GitHub writes.
- Never run `quotabus probe` and never touch launchctl or `~/.local/bin`.
- Make no commit other than your review file.
- Never print a secret.

**Output:** write `reviews/CHORE-030-r1.md` in your worktree, starting with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-030-r1.brief.md @ <SHA>
```
Next, a paragraph headed `Authorship:`. It says:
- All the work and this review were done by LLM agent sessions (name the model). No human reviewed it.
- "Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the same model
  family, briefed by the driver with this committed brief.
- The commissioning session is Mac-mini/s-e7976c42. Its stake: it filed the item, commissioned the tests and the fix,
  and gains from `approve`.

Then numbered findings `F1…`. Each gives its severity (blocker / should-fix / nit), the command, its output and the
fix you propose. Use `verdict: changes` only for blocker or should-fix findings.

Commit the review on a branch-tip checkout as this machine's configured git identity:
`git -C /tmp/rv-CHORE-030 switch -c rv-tmp && git add reviews/CHORE-030-r1.md && git commit -m "CHORE-030: review r1" && git push origin HEAD:chore/CHORE-030-resets-in-now`.
If the push is rejected, report it; do not force.

Your final message: the verdict line and the finding titles.
