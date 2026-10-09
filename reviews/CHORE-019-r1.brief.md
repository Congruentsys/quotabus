# Review brief — CHORE-019 r1 (quotabus PR #28)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver. It filed CHORE-019 after closing CHORE-001 with `--force`, and
a fresh Opus sub-agent it spawned wrote the whole diff (commit ee8bccc). The driver wants this approved.

**What to review:** PR #28 of https://github.com/Congruentsys/quotabus, branch `docs/CHORE-019-stranded-close`, at
the tip that contains this brief (run `git ls-remote origin docs/CHORE-019-stranded-close` and review THAT sha; call
it <SHA>). The item is `kanban-work/chores/CHORE-019-*.md` (read it on `origin/main`). The PR body is `gh pr view 28`.
The diff touches `CLAUDE.md` and the `pairit`, `steer` and `quotabus-loop` skills, plus this brief.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-019 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-019/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-019`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The "Done when", line by line: pairit step 4 names the exact close command and why `--force` is needed; CLAUDE.md,
   steer and quotabus-loop agree with it.
2. The measured claims, re-run yourself in a THROWAWAY sandbox only — a clone whose `origin` is a throwaway bare repo
   under /tmp/rv-CHORE-019-sbx* (never this repo's origin; delete the sandbox after): a plain `move <ID> done` from
   `stranded` is refused with the quoted message; `move <ID> done --force --resolution completed` succeeds; the
   assignee is kept. Also read `cli.py` of `.venv`'s yurtle-kanban for the claim that `--force` implies
   `skip_gates` (lines 1040/1075) and `.kanban/config.yaml` for "no gates", and what `--force` does NOT override
   (the holder guard: which states does it cover?).
3. Safety of the rule (G1/board integrity): does the documented close let a session close an item whose landing it did
   NOT verify, or close another session's work in ways the old text forbade? Is the "read that shows it landed"
   requirement explicit? Is the scope limited to landed `stranded` other-repo items?
4. Consistency: grep CLAUDE.md and every skill for other statements about `--force`, `stranded`, "moves it back",
   and "Never `move … underway` by hand"; any left contradicting the new text is a finding.
5. Secrets / public repo: nothing in the diff or the PR body carries a key, a token, an email or a private detail.
6. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-019-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes against this repo's origin (sandbox writes against
your throwaway bare origin are fine), NO GitHub writes (no `gh pr review/comment/merge`), and no commit to the PR
branch other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-019-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-019-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it filed
the item after its own forced close of CHORE-001, commissioned the diff, and gains from `approve`). Then numbered
findings `F1…` each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose.
`verdict: changes` only for blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C
/tmp/rv-CHORE-019 switch -c rv-tmp && git add reviews/CHORE-019-r1.md && git commit -m "CHORE-019: review r1" && git
push origin HEAD:docs/CHORE-019-stranded-close`), committing as this machine's configured git identity. If the push
is rejected, report it — do not force. Your final message: the verdict line and the finding titles.
