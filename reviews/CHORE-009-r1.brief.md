# Review brief — CHORE-009 r1 (quotabus PR #11)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver; a fresh Opus sub-agent it spawned wrote the whole diff
(commit 1666b4b). The driver also filed CHORE-009 (from the CHORE-006 review it commissioned) and wants this approved.

**What to review:** PR #11 of https://github.com/Congruentsys/quotabus, branch `docs/CHORE-009-design-review-nits`, at
the tip that contains this brief (run `git ls-remote origin docs/CHORE-009-design-review-nits` and review THAT sha;
call it <SHA>). The item is `kanban-work/chores/CHORE-009-*.md` (read it on `origin/main`); its source is
`reviews/CHORE-006-r1.md` F1–F3. The PR body is `gh pr view 11`. The diff touches only `docs/DESIGN.md` plus this
brief.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-009 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-009/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-009`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The "Done when", point by point: DESIGN.md is amended on F1, F2 and F3, each citing `reviews/CHORE-006-r1.md`.
2. Every citation is true at its primary line: `kanban-work/voyages/VOY-001-*.md:19`, `kanban-work/chores/CHORE-007-*.md:22-23`,
   `src/probe.rs:136-153`, `examples/quotabus.toml:111-122`, and the `reviews/CHORE-006-r1.md` line ranges. A claim
   stronger than its source is a finding (e.g. F1 says a paraphrased choice was "chosen by the Captain" — is that
   what the source supports, and is it marked as paraphrase?).
3. F2: does the central probe really probe the `local` kind (read the code, not only the citation)? Is the diagram
   still aligned and readable?
4. Nothing else changed meaning: read the whole diff (`git diff origin/main...<SHA> -- docs/DESIGN.md`). The §3 sample
   TOML still parses (`.venv/bin/python -c 'import tomllib…'` on the extracted block).
5. Secrets / public repo: nothing in the diff or the PR body carries a key, a token, an email or a private detail.
6. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-009-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-009-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-009-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it filed
the item, commissioned the diff from a sub-agent, checked it, and gains from `approve`). Then numbered findings `F1…`
each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict: changes`
only for blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-009 switch -c rv-tmp
&& git add reviews/CHORE-009-r1.md && git commit -m "CHORE-009: review r1" && git push origin
HEAD:docs/CHORE-009-design-review-nits`), committing as this machine's configured git identity. If the push is
rejected, report it — do not force. Your final message: the verdict line and the finding titles.
