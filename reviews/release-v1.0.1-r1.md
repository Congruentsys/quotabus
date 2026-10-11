reviewed-at-sha: ad45efb71e8770afcd5a8a33e58c4a1a30cbf870
verdict: approve
gate: make check rc=0 at ad45efb71e8770afcd5a8a33e58c4a1a30cbf870 on Mac-mini
brief: reviews/release-v1.0.1-r1.brief.md @ ad45efb71e8770afcd5a8a33e58c4a1a30cbf870

# Review r1: PR #40, release v1.0.1 (CHORE-031)

**Authorship:** LLM agent sessions (Claude Opus 5.5) did all the work and this review. No human reviewed it. "Distinct"
means a separate `claude -p` process from the same model family, with a fresh context that wrote none of the work. The
driver (the author) briefed it with the committed brief named on line 4. The commissioning session is
Mac-mini/s-e7976c42, which drives the release. A sub-agent wrote the version bump, the CHANGELOG section and the README
line.

Host Mac-mini, 2026-10-11, reviewed in a detached worktree at the SHA on line 1.

## Findings

**F1 (no defect). Version: only quotabus's own version changes, and `publish = false` stays.**
```
$ git diff origin/main..ad45efb -- Cargo.toml Cargo.lock
Cargo.toml:  -version = "1.0.0"  +version = "1.0.1"   (under [package] name = "quotabus")
Cargo.lock:  -version = "1.0.0"  +version = "1.0.1"   (under [[package]] name = "quotabus")
$ grep -n '^publish' Cargo.toml
9:publish = false
```
`git grep -n '1\.0\.0'` hits the following files outside kanban-work/ and reviews/:
- `.claude/skills/release{,-foss}/SKILL.md:2x`: the semver example "(0.9.0 → 1.0.0)".
- `CHANGELOG.md:17` and `:100`: the `[1.0.0]` section heading and its link reference, which are history.
- `Cargo.lock:25` and `:46`: `anstream` and `anstyle-parse` 1.0.0, which are dependency versions.
- `docs/findings/CHORE-029-mini-v1.0.0.md`: CHORE-029's v1.0.0 redeploy finding, which is history.

The remaining hits are in kanban-work/ (CHORE-029, CHORE-031, VOY-001) and reviews/ (CHORE-029-r1, release-v1.0.0-r1 and
this brief), all of them history or this release's own plan. Nothing states that 1.0.0 is the current version.

**F2 (no defect). CHANGELOG 1.0.1: every sentence is supported.** I checked it against `git show 228240b -- src/main.rs`
(`table_out` only), `tests/chore030_resets_in_now.rs` (added red at 13d5e2e) and `gh pr view 39 --json title,body`:
- "counted … from the row's `checked_at`, so it overstated the time left by the row's age". The old code printed
  `w.reset_in_s` directly. PR #39 cites DESIGN §2: `reset_in_s` = `reset_at − checked_at`. The claim is accurate.
- "counts from the reader's now (`reset_at − now`, else `reset_in_s − age`)". The new code does
  `reset_at` → `parse_from_rfc3339` → `(r − now)`, `.or_else(|| w.reset_in_s? − age_s(row, now)?)`. The claim is
  accurate.
- "renders a reset already past as '(reset due)'". The code has `Some(_) => " (reset due)"` for `s <= 0`, and the
  tests `past_reset_at_shows_no_negative_duration` and `past_reset_via_fallback_shows_no_negative_duration` cover it.
  The claim is accurate.
- "A window with `reset_at` but no `reset_in_s` … now shows its time left". The old code had `if let Some(s) =
  w.reset_in_s` and printed nothing otherwise. The new code takes the `reset_at` path first. PR #39's "Behaviour note"
  says the same. The claim is accurate. If that `reset_at` is already past, the window shows "(reset due)", which the
  previous sentence covers.
- "The stored row, `reset_in_s` and `status --json` are unchanged (#39)". The diff touches only `table_out`, and
  `json_out` (src/main.rs:835) is a separate path. The test `json_and_stored_row_unchanged` asserts that the stored
  file's bytes, `reset_in_s` = 3600 and the JSON key set are all unchanged. The claim is accurate, and the #39 credit is
  correct (PR #39, MERGED, merge commit afe953f).

No src/ change goes unmentioned:
```
$ git log --oneline v1.0.0..ad45efb --no-merges -- src Cargo.toml packaging
bdcbc2a release v1.0.1: version, CHANGELOG, README (CHORE-031)
228240b CHORE-030: status renders "resets in" against now, not checked_at
```
The link reference `[1.0.1]: https://github.com/Congruentsys/quotabus/releases/tag/v1.0.1` sits directly above
`[1.0.0]: https://github.com/Congruentsys/quotabus/releases/tag/v1.0.0` and follows the same style. The file is still
Keep a Changelog 1.1.0. The new section is `## [1.0.1] - 2026-10-11` with a `### Fixed` subsection, placed newest
first above `## [1.0.0] - 2026-10-10`. The format header is unchanged.

**F3 (no defect). README.** The diff changes one line, from `> **Status: v1.0.0**` to `> **Status: v1.0.1**`. The
line is true: Cargo.toml says 1.0.1, and the built binary prints `quotabus 1.0.1` (F5). The rest of that line ("One
Rust binary with four commands") is unchanged context, and v1.0.1 does not change the command set.

**F4 (no defect). Secrets.** I ran:
```
$ git diff origin/main..ad45efb | grep -nEi '@|sk-|bearer|token|api[_-]?key|authorization'
```
It printed only hunk headers (`@@`), one README context line ("Claude by setup-token") that is unchanged prose, and a
line of the brief. The diff has no key, no email address and no new account identifier. The PR #40 body, read with
`gh pr view 40 --json title,body`, has none either.

**F5 (no defect). Gate.**
```
$ CARGO=/Users/hankh19/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-031-target make check   # rc=0
  (runs .venv/bin/yurtle-kanban validate, the script tests and cargo test; summing the "test result" lines:
   passed=430 failed=0)
$ /tmp/rv-CHORE-031-target/debug/quotabus --version
quotabus 1.0.1
$ rm -rf /tmp/rv-CHORE-031-target
```

No finding is blocking. There are no nits.
