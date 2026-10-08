# Review brief — CHORE-005 r1 (quotabus PR #6)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver; a fresh Opus sub-agent it spawned wrote the whole diff
(commit 3caa9a6). The driver wants this approved.

**What to review:** PR #6 of https://github.com/Congruentsys/quotabus, branch `docs/CHORE-005-design-ttl-toml-openai`,
at the tip that contains this brief (run `git ls-remote origin docs/CHORE-005-design-ttl-toml-openai` and review THAT
sha; call it <SHA>). The item is `kanban-work/chores/CHORE-005-*.md` (read it on `origin/main`). The PR body is
`gh pr view 6`. The diff touches only `docs/DESIGN.md` (§3 and §4) plus this brief.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-005 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-005/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-005`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The "Done when", point by point: DESIGN.md §3 and §4 are amended on all three points the item names (kv TTL
   recipe; sample TOML validity; OpenAI probe route), each citing its evidence.
2. Every citation is true at its primary line: open each cited path:line range
   (`docs/findings/EXP-001-per-key-ttl.md:3-4,22-29,38-41`, `src/probe.rs:247-261`, `reviews/EXP-001-r1.md:167-172`,
   `examples/quotabus.toml`) and say whether it supports the sentence that cites it. Check PR #2's comments
   (`gh pr view 2 --comments`) for the `gpt-5.6-sol ok` claim. A claim stronger than its source is a finding.
3. natscli claims: if `nats` is on PATH, run `nats --version`, `nats kv put --help`, `nats kv create --help`,
   `nats kv add --help` (help only — never a kv write) and compare with the text. If it is not, say so.
4. The sample TOML: extract the §3 fenced `toml` block at <SHA> and parse it with `.venv/bin/python -c 'import tomllib…'`;
   as a control, extract the same block at `origin/main` and show it fails. Also say whether the new block agrees with
   `examples/quotabus.toml` and with what `src/config.rs` accepts (a key the binary would reject is a finding,
   severity at your judgement — the item asks only for valid TOML).
5. Nothing else in DESIGN.md changed meaning: read the whole diff (`git diff origin/main...<SHA> -- docs/DESIGN.md`).
   No open SIG is written as decided.
6. Secrets / public repo: nothing in the diff or the PR body carries a key, a token, an email or a private detail.
7. Run the gate yourself in your worktree: `make check`. Record its rc. Use a `CARGO_TARGET_DIR` of your own outside
   the tree (e.g. /tmp/rv-CHORE-005-target, removed when done); if `cargo` is not on PATH use the rustup shim's
   absolute path.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-005-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-005-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it
commissioned the diff from a sub-agent, checked it, and gains from `approve`). Then numbered findings `F1…` each with
severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict: changes` only for
blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-005 switch -c rv-tmp && git
add reviews/CHORE-005-r1.md && git commit -m "CHORE-005: review r1" && git push origin
HEAD:docs/CHORE-005-design-ttl-toml-openai`), committing as this machine's configured git identity. If the push is
rejected, report it — do not force. Your final message: the verdict line and the finding titles.
