reviewed-at-sha: 50d1f820b8ca4a0614730aedfa8c2b4cce3f71d2
verdict: approve
gate: make check rc=0 at 50d1f820b8ca4a0614730aedfa8c2b4cce3f71d2 on Mac-mini (m4-mini.local)
brief: reviews/CHORE-019-r1.brief.md @ 50d1f820b8ca4a0614730aedfa8c2b4cce3f71d2

Authorship: all of this work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
"Distinct" means this review is a separate `claude -p` process with a fresh context that wrote none of the work. It
is the same model family as the author, and the driver briefed it with the committed brief above. The commissioning
session, Mac-mini/s-e7976c42, has a stake in the outcome: it filed CHORE-019 after its own forced close of CHORE-001
(5f570b8), it commissioned the diff (ee8bccc, written by a fresh Opus sub-agent), and it gains from `approve`, since
the change documents its earlier `--force` close as the sanctioned route.

## What was checked (host Mac-mini, 2026-10-09, quotabus 50d1f82)

**1. Done when.** `git diff origin/main...HEAD -- CLAUDE.md .claude/`:
- pairit step 4 now names the close command: `.venv/bin/yurtle-kanban move <ID> done --force --resolution completed
  --agent "$ME" -m "<ID> done: landed in <repo> as <their sha> … Check: <the command> → <its output>. Closed with
  --force: …"`.
- It says why `--force` is needed: from `stranded`, the table allows only provisioning, underway and harbor, and the
  only way through `underway` is the hand move CLAUDE.md forbids.
- CLAUDE.md (lines 83–86), steer (lines 117–118 and 131–136) and quotabus-loop (lines 18–19) all name the same
  command.
- `make check` is green (below). Met.

**2. The measured claims, re-run.** I cloned a bare repo from the reviewed tip to `/tmp/rv-CHORE-019-sbx-origin.git`
and made a clone of it. `git remote -v` → `origin /tmp/rv-CHORE-019-sbx-origin.git`. Since the clone came from a
detached tip, the sandbox's default branch was the PR branch, which does not affect the result. The parking session
was A=Mac-mini/s-e7976c42 and the closing session was B=rv/s-reviewer1:
```
$YK move CHORE-019 blocked --agent "$A" -m "park"            → park: pushed to origin/docs/CHORE-019-stranded-close  rc=0
  status: stranded / assignee: Mac-mini/s-e7976c42
$YK move CHORE-019 done --agent "$B" -m "plain"              → Error: Illegal move CHORE-019: stranded → arrived. Legal from stranded: provisioning, underway, harbor.  rc=1
$YK move CHORE-019 done --force --resolution completed --agent "$B" -m "forced close"
                                                             → forced close: pushed to origin/docs/CHORE-019-stranded-close  rc=0
  status: arrived / assignee: Mac-mini/s-e7976c42 / resolution: completed / line 52: kb:forcedMove "true"^^xsd:boolean ;
```
This reproduces every quoted claim. The real origin was untouched: `git ls-remote origin main` →
`b21fcd84… refs/heads/main`. The sandbox has been deleted.

Source reads (yurtle-kanban 3.4.0, `.venv`):
- `grep -n 'skip_gates=skip_gates or force' cli.py` → `1040:` and `1075:`. The claim is correct.
- `workflow.py` 64–68 → `WorkItemStatus.BLOCKED: [READY, IN_PROGRESS, BACKLOG]`. The claim is correct.
- `grep -n gates .kanban/config.yaml` → no output, rc=1, and `config.py:617` defaults `gates` to an empty dict.
  The claim that no gates are configured is correct.
- Holder guard, `service.py:5164–5182`: `if item.status != WorkItemStatus.IN_PROGRESS: return None`. It covers
  in-progress items only, so a `stranded` item is not guarded and any session can move it, forced or not. The new
  text says exactly this.
- I also checked whether `--force` gets past a halt. With the board halted in the sandbox (`control status` →
  `mode: halt`), both a forced move and a plain move to non-in-progress statuses succeeded. The only halt refusal is
  at `service.py:4747` ("Can't move … to in progress"). So the halt does not guard a close in either case, and this
  is not something `--force` changes.

**3. Safety.** Before this change, any session could already move a `stranded` item, because the holder guard covers
only in-progress items. The old text said "Whoever records the receiving repo's landing commit moves it back and
closes it", so the set of sessions allowed to close an item is the same as before. The new text requires the landing
evidence explicitly in pairit ("citing that commit and the read that shows the change is there", with the `Check:
<the command> → <its output>` slot in `-m`) and in CLAUDE.md ("<their landing sha and the read that shows it>").
The scope is limited by context to the other-repo lane: the pairit step, the CLAUDE.md other-repo bullet, and the
other-repo clauses in steer and the loop. Two wording nits follow (F1 and F2).

**4. Consistency.** I ran `grep -rn -e '--force' -e 'stranded' -e 'moves it back' -e 'by hand' CLAUDE.md
.claude/skills/`:
- "moves it back" no longer appears anywhere.
- `CLAUDE.md:46` "Never `move … underway` by hand" still stands and agrees with the new text, which avoids that hand
  move by going around `underway`.
- steer's old "does not override … the gates" is corrected at lines 131–134.
- Other `--force` mentions are steer:115, the SIG close, and steer:147, the veto reopen (unchanged and correct).
  `review/SKILL.md:114` is a `git push --force-with-lease`, which is unrelated.

No contradiction is left.

**5. Secrets.** `git diff origin/main...HEAD | grep -niE 'sk-|token|api[_-]?key|@…\.(com|net|org)|password|secret'`
matches only lines 118 and 125, which are the brief's own words ("a key, a token, an email"; "Never print a
secret"). `gh pr view 28 --json body -q .body | grep -niE …` → no match, rc=1. The diff and the PR body contain no
key, token, email or private detail.

**6. Gate.** `CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-019-target make check` → rc=0.
The log shows `86 passed in 16.37s` (pytest), `All work items valid.`, and every cargo `test result: ok.` with 0
failed. The target dir has been removed.

## Findings

**F1 (nit): steer and quotabus-loop cite only the landing commit, not the read.**
`grep -n 'citing the landing commit' .claude/skills/steer/SKILL.md` → line 118: "… `move <ID> done --force
--resolution completed`, citing the landing commit)". quotabus-loop:18–19 says only "once that repo lands it". Both
point to pairit step 4, which does require the read, so this is not a gap in the rule. But a session reading only
steer could cite a sha without checking that the change is on the other repo's main.
Fix: in both places, change "citing the landing commit" to "citing the landing commit and the read that shows it".

**F2 (nit): "any session may close a stranded one" reads wider than the rule.**
`sed -n 249p .claude/skills/pairit/SKILL.md` → "in-progress items only (`move --help`), so any session may close a
stranded one; the claimant stays the record." steer:135 says "A landed `stranded` item is closed the same way". Both
are scoped by their surrounding text, but taken alone each could be read as permission to force-close any
`stranded` item, including one stranded for a reason other than another repo.
Fix: in pairit, change it to "so any session that has made that read may close it". In steer, change it to "A
`stranded` other-repo item, once its landing is read, is closed the same way".

Neither finding is a blocker or should-fix, so the verdict is approve.
