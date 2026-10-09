# Review brief — CHORE-017 r1 (quotabus PR #26)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver; a fresh Opus sub-agent it spawned wrote the whole diff
(commit 7c31285). The driver wants this approved.

**What to review:** PR #26 of https://github.com/Congruentsys/quotabus, branch `docs/CHORE-017-cross-repo-chore`, at
the tip that contains this brief (run `git ls-remote origin docs/CHORE-017-cross-repo-chore` and review THAT sha; call
it <SHA>). The item is `kanban-work/chores/CHORE-017-*.md` (read it on `origin/main`). The PR body is `gh pr view 26`.
The diff touches `CLAUDE.md` and the `pairit`, `steer` and `quotabus-loop` skills, plus this brief.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-017 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-017/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-017`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The "Done when", point by point (1: the lane step, both command forms, the body contents, the id recorded on
   `stranded`; 2: CLAUDE.md states the sanctioned write with the Captain's words and keeps "never edit another repo's
   files"; 3: a test only if the skill tests pin lane steps).
2. The quote is verbatim against the item file, and the worked example (CHORE-001 → nusy-product-team CH-13371,
   related to IDEA-13333) is what CHORE-001's comments on `origin/main` say.
3. The commands are executable as written: `nusy-kanban create --help` and `.venv/bin/yurtle-kanban create --help`
   (help ONLY — never create anything) show every flag used.
4. Internal consistency across CLAUDE.md and all skills: grep for every other statement the new rule contradicts —
   in particular CLAUDE.md § Precedence says work here is tracked with yurtle-kanban "(never `nk`)"; does the new
   rule (a `nusy-kanban create` on the OTHER repo's board) read as consistent with it, or does Precedence need a
   clause? Also "read-only", "never write", "one hop" wording in `steer`, `pairit`, `quotabus-loop`, `quotabus-next`.
5. Nothing widens beyond the Captain's direction: the write is ONE chore per packet, no other write there.
6. Secrets / public repo: nothing in the diff or the PR body carries a key, a token, an email or a private detail
   beyond what the repo already carries.
7. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-017-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes on ANY board (no yurtle-kanban or nusy-kanban write
verb), NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a
secret.

**Output:** write `reviews/CHORE-017-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-017-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it
commissioned the diff from a sub-agent, checked it, and gains from `approve`). Then numbered findings `F1…` each with
severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict: changes` only for
blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-017 switch -c rv-tmp && git
add reviews/CHORE-017-r1.md && git commit -m "CHORE-017: review r1" && git push origin
HEAD:docs/CHORE-017-cross-repo-chore`), committing as this machine's configured git identity. If the push is
rejected, report it — do not force. Your final message: the verdict line and the finding titles.
