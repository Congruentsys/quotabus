You are an INDEPENDENT code reviewer for quotabus PR #33 (https://github.com/Congruentsys/quotabus/pull/33), branch `chore/CHORE-025-pin-local-skipped`. You wrote none of this work. Review the tip of that branch, which is the commit adding this brief file. Call it <SHA>; get it with `git rev-parse origin/chore/CHORE-025-pin-local-skipped` after a fetch.

Work in your OWN scratch worktree: `git -C /Users/hankh95/Projects/quotabus fetch -q origin && git -C /Users/hankh95/Projects/quotabus worktree add -q --detach /tmp/rv-CHORE-025 origin/chore/CHORE-025-pin-local-skipped && ln -s /Users/hankh95/Projects/quotabus/.venv /tmp/rv-CHORE-025/.venv`. Remove it when you are done: `git -C /Users/hankh95/Projects/quotabus worktree remove --force /tmp/rv-CHORE-025`. Use CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-025 and add /Users/hankh95/.cargo/bin to PATH.

Item: kanban-work/chores/CHORE-025-*.md. It came from reviews/CHORE-024-r1.md F6 on main. The PR is TEST-ONLY. Its one commit is 5123e5f, adding tests/chore025_alert_local_skipped_discovering.rs. It pins behaviour that already exists, so the tests are green on today's code. Red was proven by mutation.

Check:
1. The tests match the item's Done when: M=1 when the store holds none of the service's rows; M = the served-id count when the newest cycle wrote several; and a control showing that an older cycle's ids are not counted.
2. `git diff origin/main...<SHA> --stat` shows only the test file and this brief. No production code changed.
3. Mutate src/main.rs `alert_with` to count local slots with `svc.slots()` instead of `slots_in`, and confirm the tests go red. Then mutate `slots_in` in src/config.rs to drop its newest-cycle filter, and confirm the older-cycle control goes red. Restore the code after each mutation.
4. No control passes by construction.
5. No real key, no bus, no network beyond loopback.
6. Run `make check` YOURSELF in your worktree and record the exit code.

Run EVERY command in the FOREGROUND: no run_in_background, no `&`, no nohup. Make NO board writes and NO GitHub writes. Make no commits other than the review file.

Write `reviews/CHORE-025-r1.md` in your worktree. Its first four lines are exactly:
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-025-r1.brief.md @ <SHA>

Then an `Authorship:` paragraph. It says:
- All the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
- "Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model family, briefed by the driver (the author) with this committed brief.
- The commissioning session is M5-MBP-2/s-72a67d16. Its stake: it drove the work and wrote no tests or code. The tests were written by a sub-agent.

Then the findings, numbered F1, F2 and so on. Each one quotes the command it ran and that command's output. Say whether each one blocks.

Commit only the review file, as `CHORE-025: review r1` by Hank Head <237287+hankh95@users.noreply.github.com>. Push it with `git push origin HEAD:chore/CHORE-025-pin-local-skipped`.
