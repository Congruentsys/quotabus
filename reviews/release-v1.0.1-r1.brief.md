You are an independent reviewer of quotabus PR #40, "chore: release v1.0.1 (CHORE-031)", on branch `chore/release-v1.0.1`. You wrote none of it. The tip you review is the commit that adds THIS brief. Get it with `git ls-remote origin chore/release-v1.0.1`; call it <SHA>.

Work in your OWN scratch worktree:
`cd /Users/<user>/Projects/quotabus && git fetch -q origin --tags && git worktree add -q --detach /tmp/rv-CHORE-031 <SHA> && ln -s /Users/<user>/Projects/quotabus/.venv /tmp/rv-CHORE-031/.venv`.
Remove it when you are done, after your review file is pushed.

Rules for you:
- Run EVERY command in the FOREGROUND: no background jobs, `&` or `nohup`.
- Make NO board writes and NO GitHub writes: no gh pr comment, review or merge, and no tags.
- The repo is PUBLIC: quote no key, auth header, raw provider error body or email.

The PR changes 4 files: the Cargo.toml/Cargo.lock version 1.0.0 → 1.0.1, a new CHANGELOG.md section, and README.md's status line. The item is `kanban-work/chores/CHORE-031-*.md` (Plan step 1). The fix it releases is CHORE-030, PR #39, code commit 228240b.

Review:
1. **Version.** `git diff origin/main..<SHA> -- Cargo.toml Cargo.lock` changes only quotabus's own version, and `publish = false` stays. Run `git grep -n '1\.0\.0'`. Every remaining hit must be history (reviews, findings, kanban-work, the CHANGELOG's 1.0.0 section), a dependency's own version, or a semver example. Nothing may assert that 1.0.0 is the current version.
2. **CHANGELOG accuracy.** Check every sentence of the 1.0.1 section against `git show 228240b -- src/main.rs`, `tests/chore030_resets_in_now.rs` and `gh pr view 39 --json title,body`, which is read-only and allowed. A claim that overstates, invents or misattributes something is a finding. Also check:
   - that `v1.0.0..<SHA>` contains no OTHER src/ change the CHANGELOG fails to mention (`git log --oneline v1.0.0..<SHA> --no-merges -- src Cargo.toml packaging`);
   - that the `[1.0.1]` link reference matches the `[1.0.0]` style;
   - that the format is still Keep a Changelog.
3. **README.** The changed line is true.
4. **Secrets.** Nothing in the diff or the PR body carries a key, an email address or a new account identifier.
5. **The gate.** Run `CARGO=/Users/<user>/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-031-target make check` in your worktree and record its rc. Run `/tmp/rv-CHORE-031-target/debug/quotabus --version` (it must print `quotabus 1.0.1`), then remove that target dir.

Write your review to `reviews/release-v1.0.1-r1.md` in your worktree. Its first four lines are:
- line 1: `reviewed-at-sha: <SHA>`
- line 2: `verdict: approve|changes`
- line 3: `gate: make check rc=<N> at <SHA> on <host>`, or `gate: NOT RUN — <reason>`
- line 4: `brief: reviews/release-v1.0.1-r1.brief.md @ <SHA>`

Then an `Authorship:` paragraph. It says:
- All the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
- "Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the same model family, briefed by the driver (the author) with this committed brief.
- The commissioning session is `<mini>/s-e7976c42`, which drives the release. A sub-agent wrote the bump, the CHANGELOG and the README.

Then numbered findings F1…. Each gives the command you ran and its output; a finding you did not run is not a finding. Mark each finding blocking or nit.

Commit ONLY that file to the branch, as this machine's configured git identity: `git -C /tmp/rv-CHORE-031 switch -c rv-tmp && git add reviews/release-v1.0.1-r1.md && git commit -m "release v1.0.1: review r1" && git push origin rv-tmp:chore/release-v1.0.1`. Then remove your worktree and the local `rv-tmp` branch.

Your final message: the verdict and the findings, in one paragraph each.
