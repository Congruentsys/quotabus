# Review brief — CHORE-007 r1 (quotabus PR #9)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The work was written by two
sub-agents of the resident session M5-MBP-2/s-72a67d16 (Claude Opus 5.5): a test partner, which wrote the tests at
`0acfc02`, and an implementer, which wrote the code at `304e545`. That session drove both, filed the item and wants it
approved.

**What to review:** PR #9 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-007-per-kind-intervals`, at
the tip that contains this brief (run `git ls-remote origin chore/CHORE-007-per-kind-intervals` and review THAT sha;
call it <SHA>). The item is `kanban-work/chores/CHORE-007-*.md` (read it on `origin/main`, including its body and the
Captain's rulings it quotes from `kanban-work/signals/SIG-004-*.md`). The PR body is `gh pr view 9`. T = `0acfc02`;
T's test files are tests/per_kind_config.rs, tests/per_kind_schedule.rs, tests/cli_probe_due.rs,
tests/tick_and_docs.rs, tests/config_toml.rs, tests/probe_runner.rs, tests/cli_probe.rs, tests/real_run_defects.rs and
tests/review_r1.rs.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-007 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-007/.venv`.
Use `CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-007` (your own) and cargo at `/Users/hankh95/.cargo/bin/cargo` if it is
not on PATH. Remove the worktree when done (`git worktree remove --force /tmp/rv-CHORE-007`), after your review file
is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The tests in `0acfc02` match the item body's Definition of Done (all five points) and the Captain's rulings, not
   the code. T also edited pre-existing tests: check that each of those edits follows from the DoD and weakens no
   guard that is still valid.
2. Confirm that `git diff 0acfc02..<SHA> -- <T's test files>` is empty.
3. The code does not special-case the tests (e.g. no branching on test-only values or paths).
4. Mutate the code at two points and show a test go red each time, restoring after each:
   - make the balance read use the api interval (share the intervals);
   - make "no row" read as not due.
5. Design choices the item did not pin, which the implementer made. Judge each against the DoD and DESIGN.md:
   - balance reads write their own `<kind>.<provider>.<account>.balance` row;
   - due-ness is per row (per service/model for api, per service for balance);
   - rows written without a call (`secret_unset`, `no_endpoint`) leave the kind due;
   - an unreadable store refuses `probe` (exit 1) unless `--force`, so an unreachable bus now means no calls;
   - `status` ignores the balance row.
   Name any that contradicts the design or a reader's contract (§2 record, §3 freshness).
6. Secrets: no key on argv, in a row, a log, an error or a fixture; every output path goes through the redactor;
   nothing in the diff or the PR looks like a real key.
7. Docs: DESIGN.md §3/§4/§10 Q5, `examples/quotabus.toml` and `packaging/README.md` match the code and quote the
   Captain verbatim. No other DESIGN.md sentence still states the 15-min cadence or a shared interval
   (`grep -n -E '15 ?m|900|96 calls|interval' docs/DESIGN.md packaging/README.md`).
8. Run the gate ITSELF in your worktree, `make check CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-007`, and record its
   exit code on the review's third line. The driver's green run is the author's evidence, not yours.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session ends
when you stop, killing its background work and leaving a mutated tree. Make NO board writes, NO GitHub writes, and no
commit other than your review file. Run no real provider probe and touch no real bus. Never print a secret.

**Output:** write `reviews/CHORE-007-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-007-r1.brief.md @ <SHA>
```
Then a paragraph headed `Authorship:`. It says: all the work and this review were done by LLM agent sessions (name the
model); no human reviewed it; "distinct" means a separate `claude -p` process with a fresh context that wrote none of
the work, of the same model family, briefed by the author with this committed brief; the commissioning session is
M5-MBP-2/s-72a67d16, and its stake is that it drove the partner and implementer, filed the item, and gains from
`approve`.
Then numbered findings `F1…`, each with a severity (blocker / should-fix / nit), the command, its output, and the fix
you propose. `verdict: changes` only for blocker or should-fix findings.
Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-007 switch -c rv-tmp && git add reviews/CHORE-007-r1.md &&
git commit -m "CHORE-007: review r1" && git push origin HEAD:chore/CHORE-007-per-kind-intervals`), committing as
`Hank Head <237287+hankh95@users.noreply.github.com>`. If the push is rejected, report it — do not force.
Your final message: the verdict line and the finding titles.
