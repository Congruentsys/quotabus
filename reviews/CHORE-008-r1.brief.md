# Review brief — CHORE-008 r1 (quotabus PR #10)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver. A fresh Opus sub-agent (the test partner) wrote the tests at
8edf666; a second fresh Opus sub-agent (the implementer) wrote the code at b820c09. The driver wants this approved.

**What to review:** PR #10 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-008-install-target`, at
the tip that contains this brief (run `git ls-remote origin chore/CHORE-008-install-target` and review THAT sha; call
it <SHA>). The item is `kanban-work/chores/CHORE-008-*.md` (read it on `origin/main`); the ruling it builds is
`kanban-work/signals/SIG-001-*.md` and `docs/DESIGN.md` §3, §5 and §10 Q2. The PR body is `gh pr view 10`.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-008 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-008/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-008`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The tests at 8edf666 (`tests/test_install_target.py`, `tests/fixtures/chore008_mini_probe.plist`) match the item's
   Definition of Done (points 1–4) and the design sections it cites — not the code. Is the fixture really the plist
   EXP-001 shipped (`git show origin/main~N:packaging/launchd/com.congruentsys.quotabus-probe.plist` or the commit
   that deleted it)?
2. `git diff 8edf666..<SHA> -- tests/` is empty.
3. The code does not special-case the tests (e.g. detecting the fake tools, the fixture path, or test-only values).
4. Mutate `packaging/install.sh` at two points (e.g. hard-code the home in the plist paths; make `--render-to` call
   `launchctl`) and show a test go red each time; restore it after.
5. Dry-run it yourself: the README's Mini command with `--render-to /tmp/rv-CHORE-008-out` → diff against the
   fixture; `--where local --os linux --user alice --home /home/alice --render-to …` → read the service and timer. Is
   the real-install path (no `--render-to`; local and `--host` over ssh/scp) correct by reading — quoting, `set -e`
   behaviour, what it overwrites, launchctl/systemctl verbs? NEVER run it without `--render-to`. Run under
   `/bin/bash` (3.2) too.
6. Secrets: no key on argv, in a unit, a log or the plan; the `--launcher` string carries no secret; nothing in the
   diff or PR body looks like a real key, a token or an email.
7. The one-line DESIGN.md §5 change riding this branch: is it a pointer fix only (no change of meaning)?
8. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-008-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.
Never install a unit or touch `~/Library/LaunchAgents`, launchctl or systemctl.

**Output:** write `reviews/CHORE-008-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-008-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it
commissioned the tests and code from two sub-agents, checked them, and gains from `approve`). Then numbered findings
`F1…` each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose. `verdict:
changes` only for blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-008
switch -c rv-tmp && git add reviews/CHORE-008-r1.md && git commit -m "CHORE-008: review r1" && git push origin
HEAD:chore/CHORE-008-install-target`), committing as this machine's configured git identity. If the push is rejected,
report it — do not force. Your final message: the verdict line and the finding titles.
