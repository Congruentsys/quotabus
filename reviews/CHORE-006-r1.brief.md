# Review brief — CHORE-006 r1 (quotabus PR #8)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver; a fresh Opus sub-agent it spawned wrote the whole diff
(commit e99b027). The driver wants this approved.

**What to review:** PR #8 of https://github.com/Congruentsys/quotabus, branch `docs/CHORE-006-record-s10-rulings`, at
the tip that contains this brief (run `git ls-remote origin docs/CHORE-006-record-s10-rulings` and review THAT sha;
call it <SHA>). The item is `kanban-work/chores/CHORE-006-*.md` and the rulings are comments on
`kanban-work/signals/SIG-001…SIG-008-*.md` — read all of them on `origin/main` (the item's comment adds to Q5). The
PR body is `gh pr view 8`. The diff touches only `docs/DESIGN.md` plus this brief.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-006 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-006/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-006`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The Definition of Done, line by line, for EACH of Q2, Q3, Q4, Q5, Q7, Q8, Q9, Q10 and the Origin note: the ruling
   is quoted VERBATIM (diff each quote against its SIG line on `origin/main` yourself), cites its SIG at the right
   line, and says correctly whether it was the recommended default (read each SIG's recommendation, not the item's
   summary — the item body says "two rulings differ" and lists three; decide from the signals).
2. No stale sentence remains: grep DESIGN.md at <SHA> for "pending SIG", "15 min", "96", "recommend", interval and
   cadence words, the central probe's host, and Q-numbers; every sentence a ruling contradicts is a finding.
3. Nothing is claimed beyond its ruling: a paraphrase that adds a decision the Captain did not make (a default, a
   scope, a host) is a finding. Check especially Q5's "API + balance only" and the per-query-kind intervals, and the
   `[intervals]`/`[ttl]` sample config the doc adds (is it marked as not yet what the binary reads?).
4. The sample TOML in §3 still parses: extract the fenced `toml` block and parse it with `.venv/bin/python -c 'import
   tomllib…'`.
5. Arithmetic: §4's cost line (≤ 2 calls … per day at 12 h).
6. Secrets / public repo: nothing in the diff or the PR body carries a key, a token, an email or a private detail.
7. Run the gate yourself in your worktree: `make check`. Record its rc. Use a `CARGO_TARGET_DIR` of your own outside
   the tree (e.g. /tmp/rv-CHORE-006-target, removed when done); if `cargo` is not on PATH use the rustup shim's
   absolute path.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-006-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-006-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it
commissioned the diff from a sub-agent, checked it, and gains from `approve`). Then numbered findings `F1…` each with
severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict: changes` only for
blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-006 switch -c rv-tmp && git
add reviews/CHORE-006-r1.md && git commit -m "CHORE-006: review r1" && git push origin
HEAD:docs/CHORE-006-record-s10-rulings`), committing as this machine's configured git identity. If the push is
rejected, report it — do not force. Your final message: the verdict line and the finding titles.
