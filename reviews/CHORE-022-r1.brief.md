You are an INDEPENDENT code reviewer for quotabus PR #31 (https://github.com/Congruentsys/quotabus/pull/31), branch `chore/CHORE-022-local-discovery`. You wrote none of this work. Review the tip of that branch, which is the commit adding this brief file. Call it <SHA>; get it with `git rev-parse origin/chore/CHORE-022-local-discovery` after a fetch.

Work in your OWN scratch worktree: `git -C /Users/hankh95/Projects/quotabus fetch -q origin && git -C /Users/hankh95/Projects/quotabus worktree add -q --detach /tmp/rv-CHORE-022 origin/chore/CHORE-022-local-discovery && ln -s /Users/hankh95/Projects/quotabus/.venv /tmp/rv-CHORE-022/.venv`. Remove it when you are done: `git -C /Users/hankh95/Projects/quotabus worktree remove --force /tmp/rv-CHORE-022`. Use CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-022 and add /Users/hankh95/.cargo/bin to PATH.

Item: kanban-work/chores/CHORE-022-*.md. Commits:
- T = 4b341de: tests/chore022_local_discovery.rs and tests/chore022_example_design.rs.
- Ruled test edit by the test partner = 4d18e54: tests/examples_config.rs, which pinned the retired local-qwen example.
- Code = c66572d.

Check:
1. The tests in T and 4d18e54 match the item body's Definition of Done and DESIGN §4. Judge them against the body, not against the code.
2. `git diff 4d18e54..<SHA> -- tests/chore022_local_discovery.rs tests/chore022_example_design.rs tests/examples_config.rs` is empty.
3. The code does not special-case the tests.
4. Mutate the code at two points and confirm a test goes red each time. For example, skip discovery when models is empty, or send the Authorization header unconditionally. Restore the code after each.
5. G1 honesty (DESIGN §2): no path yields a false `ok`. Look at an unreachable box, a 404 list, an empty list, a malformed list, and a discovered model whose completion fails. Also check: does `status`/`select` reading slots from the store still apply the freshness rule to every row? Can a stale or expired row of a model the box no longer serves read ok?
6. Secret rules: no key on argv, in a row, a log, an error or a fixture. Nothing in the diff or the PR looks like a real key.
7. The changes to status and select in src/main.rs and src/select.rs are correct and minimal.
8. Run `make check` YOURSELF in your worktree and record the exit code.
9. Optional, read-only and keyless: `curl -s --max-time 5 http://192.168.8.120:8000/v1/models` from this host. If you run the probe, use a scratch config with a [file] backend in /tmp/rv-CHORE-022 only. NEVER a [bus] table, and never write Mini's bus.

Run EVERY command in the FOREGROUND: no run_in_background, no `&`, no nohup. Make NO board writes and NO GitHub writes. Make no commits other than the review file.

Write `reviews/CHORE-022-r1.md` in your worktree. Its first four lines are exactly:
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-022-r1.brief.md @ <SHA>

Then an `Authorship:` paragraph. It says:
- All the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
- "Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model family, briefed by the driver (the author) with this committed brief.
- The commissioning session is M5-MBP-2/s-72a67d16. Its stake: it drove the work and wrote no tests or code. The tests and code were written by sub-agents.

Then the findings, numbered F1, F2 and so on. Each one quotes the command it ran and that command's output; a finding you did not run is not a finding. Say whether each one blocks.

Commit only the review file, as `CHORE-022: review r1` by Hank Head <237287+hankh95@users.noreply.github.com>. Push it with `git push origin HEAD:chore/CHORE-022-local-discovery`.
