# Review brief — CHORE-018 r1 (quotabus PR #27)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The commissioning session
is Mac-mini/s-e7976c42 (Claude Opus 5.5), the driver. It recorded the Captain's ruling on SIG-010 and filed this item.
A fresh Opus sub-agent (the test partner) wrote the tests at 2694eff, including ruled edits to existing tests; a second
fresh Opus sub-agent (the implementer) wrote the code and docs at 579c059. The driver wants this approved.

**What to review:** PR #27 of https://github.com/Congruentsys/quotabus, branch `chore/CHORE-018-alert-states`, at the
tip that contains this brief (run `git ls-remote origin chore/CHORE-018-alert-states` and review THAT sha; call it
<SHA>). The item is `kanban-work/chores/CHORE-018-*.md` and the ruling is on `kanban-work/signals/SIG-010-*.md` (its
comments) — read both on `origin/main`. The PR body is `gh pr view 27`.

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-018 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-018/.venv`.
Remove it when done (`git worktree remove --force /tmp/rv-CHORE-018`), after your review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The tests at 2694eff match the item's Definition of Done (points 1–5) and the ruling — not the code. In
   particular: the ruled edits to EXISTING tests (`exp004_alert.rs`, `exp004_r1.rs`, `exp004_config.rs`,
   `chore010_warn_pct.rs`): does each only replace an assertion of the every-bad-state rule SIG-010 overturned, or does
   any drop a property that still holds (e.g. a CANNOT-ASSESS never clearing; bad→bad re-filing)?
2. `git diff 2694eff..<SHA> -- tests/` is empty.
3. The code does not special-case the tests.
4. Mutate the code at two points (e.g. make an unlisted state arm the alert; drop the `window_near_limit` reason in
   `apply_warn_rule`) and show a test go red each time; restore it after.
5. G1 (never a false `ok`, UNKNOWN over a guess): can any path now CLEAR an alert without a measured `ok`, or report a
   near-limit/unreachable service less visibly than before for a state the ruling says should file? Does the
   `window_near_limit` reason survive the redactor and every reader (`status`, `select`)? Is `select` still refusing
   every degraded row?
6. DESIGN.md §2/§3/§10 and `examples/quotabus.toml`: the ruling is verbatim with its SIG cite; nothing else changed
   meaning; the example still loads.
7. Secrets: nothing in the diff, fixtures or PR body looks like a real key, a token or an email.
8. Run the gate yourself in your worktree: `CARGO=/Users/hankh19/.cargo/bin/cargo
   CARGO_TARGET_DIR=/tmp/rv-CHORE-018-target make check` (remove that target dir when done). Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-018-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-018-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the driver with this committed brief; commissioning session Mac-mini/s-e7976c42 and its stake: it filed
the item, commissioned the tests and code from two sub-agents, checked them, and gains from `approve`). Then numbered
findings `F1…` each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose.
`verdict: changes` only for blocker or should-fix findings. Commit it on a branch-tip checkout (`git -C
/tmp/rv-CHORE-018 switch -c rv-tmp && git add reviews/CHORE-018-r1.md && git commit -m "CHORE-018: review r1" && git
push origin HEAD:chore/CHORE-018-alert-states`), committing as this machine's configured git identity. If the push is
rejected, report it — do not force. Your final message: the verdict line and the finding titles.
