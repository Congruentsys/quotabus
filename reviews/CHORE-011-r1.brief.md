# Review brief — CHORE-011 r1 (quotabus PR #16)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver. A fresh Opus sub-agent (the test partner) wrote the tests at
98335d9 and a formatting-only change to them at 27450cd; a second fresh Opus sub-agent (the implementer) wrote the
code at dd0413a. The driver wants this approved.

**What to review:** PR #16 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-011-round-used-pct`, at
the tip that contains this brief (run `git ls-remote origin chore/CHORE-011-round-used-pct` and review THAT sha; call
it <SHA>). The item is `kanban-work/chores/CHORE-011-*.md` (read it on `origin/main`). The PR body is `gh pr view 16`.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-011 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-011/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-011`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The tests at 98335d9 (`tests/chore011_round_used_pct.rs`) match the item's Definition of Done — all three sources
   (unified headers, stream-json `utilization`, Copilot `100 − percent_remaining`), known answers, and a control that
   fails on the unrounded value — not the code.
2. `git diff 98335d9..<SHA> -- tests/` changes only formatting (27450cd): show that stripping whitespace from both
   versions gives identical content.
3. The code does not special-case the tests; rounding happens where every source's window is built (check there is no
   source that builds a `Window` around the rounding).
4. Mutate the code at two points (e.g. drop the rounding in `Window::measured`; round to 1 decimal instead of 2) and
   show a test go red each time; restore it after.
5. Is rounding correct for the values a reader compares against thresholds (e.g. a value like 99.995 or a negative /
   >100 utilization)? Is the choice to compute `pace` from the unrounded value sound and stated?
6. Secrets: nothing in the diff, fixtures or PR body looks like a real key, a token or an email; test tokens are fakes.
7. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-011-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-011-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-011-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it
commissioned the tests and code from two sub-agents, checked them, and gains from `approve`). Then numbered findings
`F1…` each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict:
changes` only for blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-011
switch -c rv-tmp && git add reviews/CHORE-011-r1.md && git commit -m "CHORE-011: review r1" && git push origin
HEAD:chore/CHORE-011-round-used-pct`), committing as this machine's configured git identity. If the push is rejected,
report it — do not force. Your final message: the verdict line and the finding titles.
