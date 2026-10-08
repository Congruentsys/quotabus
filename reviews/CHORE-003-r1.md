reviewed-at-sha: 5347f9e078ff7014a677360a4025ff86c881dacb
verdict: changes
gate: make check rc=0 at 5347f9e078ff7014a677360a4025ff86c881dacb on M5-MBP-2.lan
brief: reviews/CHORE-003-r1.brief.md @ 5347f9e078ff7014a677360a4025ff86c881dacb

Authorship: LLM agent sessions did all of the work under review and this review. The model is Claude Opus 5.5
(`claude-opus-5-5`). No human reviewed it. "Distinct" means this review ran in a separate `claude -p` process with a
fresh context. That process wrote none of the diff. It is the same model family as the author, and the author briefed
it with the committed brief `reviews/CHORE-003-r1.brief.md`. The commissioning session is M5/s-b1fd4c67. It wrote the
whole diff (CLAUDE.md, the seven skills, `scripts/qb_clean.sh`, the tests, the Makefile and the brief), and it gains
from `approve`.

## Summary

Most of the brief checks out.
- The DoD picker pass prints a pick (resume CHORE-003 for the author's session) or the "nothing pickable"
  explanation (for a fresh session).
- The central claim holds against a throwaway bare origin, including writes made from a feature branch.
- Every yurtle-kanban and `gh` flag the skills use exists.
- The tests are unchanged since `8d626cb`, and both kinds of mutation turn them red.
- `make check` is green.

One blocker: the branch commits `.venv` as a symlink to an absolute path on the author's laptop. Merging it silently
deletes a checkout's real `.venv`.

## Findings

### F1 — blocker: `.venv` is committed as a symlink to `/Users/hankh95/Projects/quotabus/.venv`; merging it deletes the main checkout's virtualenv

The branch adds `.venv` as a tracked symlink in commit `8d626cb`. `.gitignore` has `.venv/`, and that pattern matches
only directories, so the symlink got committed.

```
$ git ls-tree HEAD .venv; git show HEAD:.venv
120000 blob d416cba8997f8e865f9635c47a286d24b9443dc3	.venv
/Users/hankh95/Projects/quotabus/.venv
$ git log --format='%h %s' --diff-filter=A -- .venv
8d626cb CHORE-003: tests (red) — clean step ported to quotabus paths, port guard; yk_push.sh retired ...
```

I simulated a main checkout that has a real (ignored) `.venv/` directory and merged the branch into it. Git treats
ignored files as expendable, so it **replaced the directory with the symlink and gave no warning**:

```
$ git clone -q ~/Projects/quotabus qb && cd qb && git fetch -q ~/Projects/quotabus refs/remotes/origin/chore/CHORE-003-port-skills:refs/heads/pr
$ mkdir -p .venv/bin && touch .venv/bin/yurtle-kanban
$ git merge --no-edit pr | tail -4; echo rc=$?
 create mode 100644 tests/test_skills_port.py
rc=0
$ ls -la .venv
lrwxr-xr-x ... .venv -> /Users/hankh95/Projects/quotabus/.venv
$ find . -path ./.git -prune -o -name yurtle-kanban -print
(nothing — the venv is gone)
```

Consequences:
- In the real main checkout (`~/Projects/quotabus`), the next `git pull` after this merges deletes the real `.venv/`
  and replaces it with a symlink to itself. Every skill then breaks, because each one calls `.venv/bin/yurtle-kanban`.
- On any other host or clone, the link dangles.
- The symlink publishes a local home path in a public repo.

It also breaks the worktree recipe in CLAUDE.md § Session start and pairit step 0 (`ln -s "$PWD/.venv"
/tmp/<ID>/.venv`). The tree already contains `.venv`, so BSD `ln` follows the existing link and puts the new link
INSIDE the main venv. Running the brief's own setup command did exactly that:

```
$ ls -la ~/Projects/quotabus/.venv/.venv
lrwxr-xr-x  1 hankh95  staff  38 Oct  8 17:24 /Users/hankh95/Projects/quotabus/.venv/.venv -> /Users/hankh95/Projects/quotabus/.venv
```

That link was created by this review's setup step, and I removed it after the review.

**Fix:**
- `git rm --cached .venv` on the branch.
- Change `.gitignore` line 1 from `.venv/` to `.venv` so the pattern also matches a symlink.
- Add a guard test, for example in `tests/test_skills_port.py`: `git ls-files .venv` is empty, with a control.

### F2 — nit: the public CLAUDE.md and the PR body name a private repo and its sha

```
$ gh repo view Congruentsys/nusy-replicant-24 --json isPrivate
{"isPrivate":true,"visibility":"PRIVATE"}
$ grep -rniE 'nusy-replicant|\bLUM\b|research/LUM|r24' .claude/skills scripts CLAUDE.md
CLAUDE.md:75:Ported 2026-10-08 from the yurtle-kanban skill set of `Congruentsys/nusy-replicant-24@2de49870` ...
```

(The other hits are in `tests/test_skills_port.py`, where the forbidden names are built from fragments.)

This does not fail the DoD: the DoD covers skills and scripts, and the hit is in CLAUDE.md. Still, it publishes the
name of a private repo. It also sits awkwardly with CLAUDE.md rule 5: "every lifted file names its source … in its
header". The ported skills have no provenance header, because the DoD forbids naming the source.

**Fix:** pick one of the following.
- Say "ported from a sibling Congruentsys repo's yurtle-kanban skill set (CHORE-003)" and leave the sha on the board
  item.
- Accept it as is, and narrow rule 5 so it covers FOSS only.

### F3 — nit: `quotabus-next` does not say what `next` prints when there is nothing to resume or pick

```
$ .venv/bin/yurtle-kanban next --agent "M5/s-65720c7d" --json; echo rc=$?
null
rc=7
```

The skill only covers the `{"kind":"resume"}` case ("else: the candidates"). A session reading the output literally
can misread rc 7 as an error. The source skill named rc 7 as "nothing pickable".

**Fix:** add one line: "`null`, rc 7 → no resume and no suggestion; go on to the candidates."

## Checks that passed (evidence)

1. **DoD (a), run as written with `QB_AGENT=M5`.** I ran it in a detached scratch worktree at origin/main `c856338`,
   not the shared checkout, using read-only verbs only.
   - `control status`: `mode: running`, rc 0.
   - `next --agent M5/s-65720c7d --json`: `null`, rc 7.
   - The pickable filter printed nothing.
   - `list --pickable --explain`: "Nothing pickable", and EXP-001 / CHORE-002 are waiting on CHORE-003, EXP-002 on
     EXP-001, EXP-003 on EXP-002, EXP-004 on EXP-003.
   - With the author's identity, `next --agent M5/s-b1fd4c67 --json` printed
     `{"id": "CHORE-003", "kind": "resume", ...}`.

   The pass prints a pick from this board. `claim` was not run.
2. **DoD (b).** My own grep over `.claude/skills scripts CLAUDE.md Makefile tests` finds no hits in any skill or
   script. Only CLAUDE.md:75 (F2) and the test's assembled patterns match. **DoD (c):** the PR body has Kept /
   Adapted / Cut sections.
3. **Central claim**, checked in the throwaway sandbox `/tmp/rv-CHORE-003-sbx` (bare `origin.git` plus a clone, using
   the repo's `.kanban/` config). This repo's origin was never touched.
   - From `main`: `move CHORE-004 provisioning`, `comment`, `rank CHORE-004 5` and `voyage add VOY-002 CHORE-004` each
     printed `…: pushed to origin/main`. The bare origin's log has a commit for each one
     (`d99f5a8 Move…`, `97538a5 Add comment…`, `60fb539 Rank…`, `6fe9f83 Link CHORE-004 → VOY-002`).
   - From branch `feat/x`, with a local unpushed commit: `move … underway`, `comment --body`, `rank 3` each printed
     rc 0 and `pushed to origin/main. Your checkout does not show this yet…`.
     - Branch HEAD `f8e0a27` was unchanged before and after.
     - The origin had `40dd397`, `0478007`, `7a0f881` on `main`.
     - The origin's item showed `status: underway`, `priority_rank: 3` and both comments.
     - The origin has only `main`, so the feature commit was not pushed.

   `scripts/yk_push.sh` can be retired.
4. **Flags and states.**
   - The `--help` output of `move` (`-m`, `--closed-by`, `--agent`), `comment` (`--body`, `--agent`), `claim`
     (`--take-over`, `--agent`), `list` (`--pickable`, `--explain`, `--stale`, `-s`, `-t`, `--json`), `next`
     (`--json`), `bounce`, `create --push` and `update --push`, `--depends-on`, `--related` lists them all.
   - In the sandbox, `move … blocked -m …` gave `status: stranded`, and `move … done --closed-by … -m …` gave
     `status: arrived` with `kb:closedBy`. `list -s harbor -t expedition` works (on main it lists EXP-005).
   - `gh` 2.89.0 has `pr merge --merge --delete-branch --subject`, `pr checks --watch`, `pr create --base --title
     --body-file`, `pr comment --body-file`, and `pr view --json headRefOid,mergeable,statusCheckRollup`.
   - Every `§` the skills cite exists: CLAUDE.md § Session start, § The board, § Syncing the board, § Skills, rule 4;
     pairit § Scope, § The docs/measure/other-repo lane; the loop's § Cleaning up between items.
   - The only missing paths are output directories the lanes create (`docs/findings/`, `docs/flowback/`,
     `docs/reviews/`).
   - No skill uses the deprecated positional `comment ID TEXT`.
5. **Comparison with the source.**
   - The cuts are correct for this repo:
     - the GPU host filter and the `gpu-required` tag;
     - the board-worktree teardown, which has no purpose now that the verbs push;
     - `push_main` and the merge worktree, replaced by `gh pr merge`;
     - the run protocol, P57 citations and the LIT closures;
     - the "tooling skips review" class, which this repo's CLAUDE.md rules out.
   - The source's "Which check (CHORE-030)" Rust gate is deferred to EXP-001, and the Makefile header says so.
   - I found no cut rule that still applies.
   - The one kept rule in tension is rule 5 (F2).
6. **Tests.** `git diff --stat 8d626cb..HEAD -- tests/` is empty. Mutations, each restored with `git checkout`:
   - Drop the `data runs` artefact check (`if [ -n "$arts" ] && ! is_disposable …` → `if false`): **8 failed**,
     19 passed. The failures include `test_each_ignored_artefact_dir_keeps_the_tree[runs/r1/log.txt]` and
     `[data/a.bin]`.
   - Drop the commits-not-on-origin check: **2 failed**, 25 passed
     (`test_commits_not_on_origin_keep_the_tree`, `test_disposable_does_not_override_commits_not_on_origin`).
   - Append ``scripts/yk_push.sh comment <ID>`` to `quotabus-next/SKILL.md`: **1 failed**
     (`AssertionError: {'.claude/skills/quotabus-next/SKILL.md': ['yk_push.sh']}`).
7. **Secrets.** A grep of `git diff origin/main...HEAD` and of the PR body for `@`, IPv4 addresses, `.lan`,
   `/Users/`, `sk-`, `ghp_`, `api_key` and `nats://` found:
   - no key or token (only `sk-test-not-a-key` as a documented fake);
   - one email, which is the commit noreply address in the brief;
   - the `/Users/hankh95/…` path in the `.venv` symlink (F1);
   - the private repo name (F2).
8. **Gate.** `make check` in `/tmp/rv-CHORE-003` at the reviewed sha: `32 passed`, `All work items valid. Checked 18
   items`, rc 0.
