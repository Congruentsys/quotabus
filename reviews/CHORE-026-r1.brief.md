# Review brief — CHORE-026 r1 (quotabus PR #35)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver; it filed the item. A fresh Opus sub-agent (the test partner)
wrote the tests at 266f9ca; a second fresh Opus sub-agent (the implementer) wrote the fix at 5646fb5. The driver wants
this approved.

**What to review:** PR #35 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-026-cycle-force`, at the
tip that contains this brief (run `git ls-remote origin chore/CHORE-026-cycle-force`; call it <SHA>). The item is
`kanban-work/chores/CHORE-026-*.md` (read it on `origin/main`). The PR body is `gh pr view 35`.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-026 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-026/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-026`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The tests at 266f9ca match the item's "Done when" (not the code); `git diff 266f9ca..<SHA> -- tests/` is empty.
2. The wrapper is correct POSIX sh: run it yourself under `/bin/sh` and `dash` if present, against a stub quotabus on
   a scratch path (never the real binary), for `probe --config X`, `probe --force --config X`, `probe --config X
   --force`, `probe --config=X --force`, and edge cases: no `--config` at all; a config path containing spaces; a
   config path that is literally `--force`. Say what `alert` receives in each and whether any case is a regression
   from the old wrapper.
3. Mutate the wrapper at two points (e.g. pass "$@" to alert again; drop the `|| rc=$?`) and show a test go red each
   time; restore it after.
4. The README's sentences around the block are accurate; `tests/test_haz004_mini_paths.py` still holds.
5. Secrets: nothing in the diff or PR body carries a key, a token or an email.
6. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-026-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes, NO GitHub writes, never touch
`~/.local/bin/quotabus-cycle` or launchctl, and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-026-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-026-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it filed
the item, commissioned the tests and the fix, and gains from `approve`). Then numbered findings `F1…` each with
severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict: changes` only for
blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-026 switch -c rv-tmp && git
add reviews/CHORE-026-r1.md && git commit -m "CHORE-026: review r1" && git push origin
HEAD:chore/CHORE-026-cycle-force`), committing as this machine's configured git identity. If the push is rejected,
report it — do not force. Your final message: the verdict line and the finding titles.
