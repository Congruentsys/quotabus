# Review brief — CHORE-002 r1 (quotabus PR #3)

You are an independent reviewer: a fresh `claude -p` session that wrote none of this work. The author is the
resident session M5-MBP-2/s-72a67d16 (Claude Opus 5.5), which ran every measurement, wrote both findings files and
wants this approved.

**What to review:** PR #3 of https://github.com/Congruentsys/quotabus, branch `measure/CHORE-002-statusline-zai`, at
the tip that contains this brief (run `git ls-remote origin measure/CHORE-002-statusline-zai` and review THAT sha;
call it <SHA>). The item is `kanban-work/chores/CHORE-002-*.md` (read it on `origin/main`); the design sections it
cites are `docs/DESIGN.md` §4 (the GLM / z.ai and Claude Max rows) and §10 Q9. The PR body is `gh pr view 3`. The
deliverables are `docs/findings/CHORE-002-statusline-under-claude-p.md` and `docs/findings/CHORE-002-zai-endpoint.md`.
The author's raw artefacts are on this host at `~/.local/state/quotabus-work/CHORE-002/` (read them; do not edit).

**Setup:** work ONLY in your own scratch worktree:
`cd ~/Projects/quotabus && git fetch -q origin && git worktree add -q --detach /tmp/rv-CHORE-002 <SHA> && ln -s ~/Projects/quotabus/.venv /tmp/rv-CHORE-002/.venv`.
Put your own re-run artefacts under `~/.local/state/quotabus-work/CHORE-002/rv1/` (never overwrite the author's
files one level up). Remove the worktree when done (`git worktree remove --force /tmp/rv-CHORE-002`), after your
review file is pushed.

**Check, and quote the command and its output for every finding (a finding you did not run is not a finding):**
1. The Definition of Done, line by line: two findings files under `docs/findings/`, each with the exact command and
   its output; (1) statusLine under `claude -p` — fires or not, with the JSON it receives; (2) the z.ai endpoint that
   answers, and what it reports.
2. **Re-run finding (1) yourself** with copies of the finding's `hook.sh`, `settings.json` and `interactive.py`
   pointed at your `rv1/` dir (edit the path in your copies only): the `-p` text run, the `-p` stream-json run, and the
   interactive control. Do NOT touch `~/.claude/settings.json` (record its keys before and after). Compare: does the
   hook stay silent under `-p`; does the control fire; does `rate_limits` appear; does stream-json carry a
   `rate_limit_event` agreeing with the statusLine's numbers?
3. **Re-run finding (2) yourself**, read-only: copy `zai_probe.py` from the finding into `rv1/` (edit the path),
   run the FAKE control, then the real key ONLY as `cd ~/Projects/nusy-product-team && doppler run --project
   nusy-product-team --config dev -- python3 <your copy> NUSY_GLM real`. It costs one 35-token messages call.
   Compare status codes, the HTTP-200-with-body-401 claim, the quota-window shape and the absence of rate-limit
   headers. Never print the key, a raw body of the subscription list, or any header carrying the key.
4. Every number and claim in the two files matches the raw artefacts (counts, epoch→UTC conversions, the 19 %/9 %
   vs 0.19/0.09 agreement, "11 models"). Check that "[inferred]" is used where a meaning is not documented, and
   that no claim is softened or overstated.
5. **Secrets / public repo:** nothing in the diff or the PR body carries a key, a token, an email, a z.ai account,
   customer, order or agreement number, a price or a purchase date. The subscription block must show field NAMES
   only, apart from the values the file lists as kept. Grep the diff yourself.
6. The "What this changes" sections follow from the output and match design §4 as written on `origin/main`.
7. Run the gate yourself in your worktree: `make check`. Record its rc.

**Rules for you:** run EVERY command in the FOREGROUND (no `run_in_background`, no `&`, no `nohup`): this session
ends when you stop, killing background work. Make NO board writes (no yurtle-kanban write verb against this repo),
NO GitHub writes (no `gh pr review/comment/merge`), and no commit other than your review file. Never print a secret.

**Output:** write `reviews/CHORE-002-r1.md` in your worktree with exactly this header:
```
reviewed-at-sha: <SHA>
verdict: approve|changes
gate: make check rc=<N> at <SHA> on <host>
brief: reviews/CHORE-002-r1.brief.md @ <SHA>
```
then a paragraph headed `Authorship:` (all work and this review by LLM agent sessions, model named; no human reviewed
it; "distinct" = a separate `claude -p` process with a fresh context that wrote none of the work, same model family,
briefed by the author with this committed brief; commissioning session M5-MBP-2/s-72a67d16 and its stake: it ran the
measurements, wrote both findings and gains from `approve`). Then your re-run results (redacted), then numbered
findings `F1…` each with severity (blocker / should-fix / nit), the command, its output, and the fix you propose.
`verdict: changes` only for blocker or should-fix findings.
Commit it on a branch-tip checkout (`git -C /tmp/rv-CHORE-002 switch -c rv-tmp && git add reviews/CHORE-002-r1.md &&
git commit -m "CHORE-002: review r1" && git push origin HEAD:measure/CHORE-002-statusline-zai`), committing as
`Hank Head <237287+hankh95@users.noreply.github.com>`. If the push is rejected, report it — do not force.
Your final message: the verdict line and the finding titles.
