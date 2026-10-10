reviewed-at-sha: 05e3005ec8cc466a7060080008b5af1a5b184c0b
verdict: approve
gate: make check rc=0 at 05e3005ec8cc466a7060080008b5af1a5b184c0b on M5-MBP-2
brief: reviews/release-v1.0.0-r1.brief.md @ 05e3005ec8cc466a7060080008b5af1a5b184c0b

Authorship: all the work and this review were done by LLM agent sessions (Claude Opus 5.5); no human reviewed it.
"Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the same model
family, briefed by the driver (the author) with the committed brief above. The commissioning session is
M5-MBP-2/s-72a67d16, which drove the release (a sub-agent wrote the bump, CHANGELOG and README).

Run on M5-MBP-2, 2026-10-10, in a detached worktree of origin/chore/release-v1.0.0 at 05e3005.

## Checks that passed (no finding)

1. Version. `git diff origin/main..HEAD -- Cargo.toml Cargo.lock` changes exactly two lines, `version = "0.1.0"` →
   `"1.0.0"` in `[package]` and in the lock's `name = "quotabus"` entry. `grep -n publish Cargo.toml` → `9:publish = false`.
   `grep -rn '0\.1\.0' --exclude-dir=.git --exclude-dir=.venv --exclude-dir=target .` (Cargo.lock excluded: its
   other hits are dependency versions) hits only history (docs/findings/CHORE-016, CHORE-027, docs/flowback/CHORE-020,
   reviews/EXP-002-r1, CHORE-016-r1, CHORE-027-r1, a CHORE-027 board item, this PR's brief) and two literal redactor
   samples (`src/redact.rs:19` doc comment, `tests/review_r1.rs:124-125` clean-text sample) whose point is the shape
   `host/quotabus@x.y.z`, not the crate version. The version reaches rows only via `src/lib.rs:35`
   `env!("CARGO_PKG_VERSION")`. Nothing asserts 0.1.0.
2. Gate. `PATH=/Users/hankh95/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/qb-target-rv-release make check` → `rc=0`
   (pytest, `yurtle-kanban validate`, fmt, clippy -D warnings, `cargo test --locked`, every `test result: ok`).
   `/tmp/qb-target-rv-release/debug/quotabus --version` → `quotabus 1.0.0`.
3. PR numbers. `for n in $(seq 1 36); do gh pr view $n --json number,title,state ...; done`: all 36 MERGED, and every
   cited number matches its entry, e.g. `2 EXP-001: probe runner + NATS KV publisher + status CLI`, `5 HAZ-001: pass
   nats:// URL credentials to the connection and keep them out of every output`, `9 CHORE-007: separate probe interval
   per query kind (api 12h, balance 12h), per-kind TTL, due-based probe`, `10 CHORE-008: install.sh chooses where the
   central probe runs`, `12 EXP-002: central subscription reads ...`, `13 HAZ-003: direct subscription read sends Claude
   Code's system prompt (429 without it)`, `14 EXP-003: the selector ... change subjects`, `15 EXP-004: alerts with
   crossing-dedup ... and the Yurtle config front-end`, `16 CHORE-011: round subscription windows' used_pct to 2
   decimals`, `21 CHORE-010: [probe] warn_pct = 90`, `24 HAZ-004: fleet install command uses Mini's real user ...
   GUI domain; QUOTABUS_CLAUDE_BIN`, `27 CHORE-018: alert only on configured states`, `30 CHORE-021: local services
   ... file no alert`, `31 CHORE-022: local Sparks discover the served model from /v1/models`, `32 CHORE-024: ...
   alert summary reports skipped local slots`, `34 HAZ-005: macOS Local Network`, `35 CHORE-026: quotabus-cycle passes
   only --config to alert`. The "Project" list (#1, 3, 4, 6, 7, 8, 11, 17-20, 22, 23, 25, 26, 28, 29, 33, 36) is all
   docs/process/test-only PRs by title.
4. CHANGELOG against code (12 entries checked):
   - Added, select/change subject: `src/backend.rs:414` `CHANGE_SUBJECT_PREFIX: &str = "ai.status.changed."`;
     `quotabus select --help` "exit 0 chosen · 2 CANNOT-ASSESS (store unreadable) · 3 nothing qualifies", flags
     `--role --exclude-family --prefer --n --json`.
   - Added, status: `quotabus status --help` "--check <CHECK> Exit 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN".
   - Added, intervals/TTL: `src/config.rs:239` "default interval of the API and balance kinds: twice a day",
     `:242` "subscription kind: hourly", `:245` "default TTL is this many of its own intervals: 3 missed runs";
     `quotabus probe --help` "--force Run every query kind now, due or not".
   - Added, warn_pct: `src/config.rs:289` `DEFAULT_WARN_PCT: f64 = 90.0`.
   - Added, alert states: `src/config.rs:198-204` `DEFAULT_ALERT_STATES` = quota_exhausted, auth_failed,
     model_missing, unreachable, window_near_limit — exactly the CHANGELOG's list.
   - Added, install.sh: `packaging/install.sh:4-22` `--where local | --host <name>`, asks on stdin with neither,
     `--interval` default `300`, `--render-to <dir> is a dry run`.
   - Added, make check/CI: Makefile `check:` runs pytest, `yurtle-kanban validate`, `cargo fmt --check`, `cargo clippy
     --all-targets --locked -- -D warnings`, `cargo test --locked`; `.github/workflows/*.yml:27-29` the same three cargo steps.
   - Changed, retired keys: `src/config.rs:549` "[probe] interval is retired: set each kind's own in [intervals]";
     probe on unreadable store: `src/main.rs:591` `Err(e) if !force => ... ProbeError::StoreUnread`, message
     "`quotabus probe --force` probes anyway", exit `PROBE_FAILED`.
   - Changed, summary line: `src/main.rs:349` `"alert: {checked} keys checked, {skipped} local skipped, {filed}
     crossings filed, {cleared} re-armed\n"`.
   - Fixed, 429: `src/subscription.rs:302` `"http_429_no_unified_headers"`.
   - Fixed, rounding: `src/record.rs:83-106` `round_2dp` applied to `used_pct`.
   - Security: `src/secret.rs:26` `f.write_str("***")`, `src/redact.rs:13` `MASK = "***"`; alert sink env scrubbed at
     `src/main.rs:421` `cmd.env_clear()` (and `src/subscription.rs:346` for the claude fallback).
5. Known limits. `.venv/bin/yurtle-kanban show EXP-007`: "Pause a provider ... NOT a v1.0 feature", harbor, tags
   post-v1.0. `show EXP-005`: "E5 (optional, post-v1.0): local web view, spend via ccusage, key expiry, model-drift
   sweep", harbor. DESIGN §9 row E5 (`docs/DESIGN.md:443`) and line 261 (`serve` feature-gated, off by default) agree.
   DESIGN line 99 records that the per-host `agent` was dropped (EXP-002 rescope), which is why README rightly no
   longer lists it.
6. README. `quotabus --help` lists exactly probe, status, select, alert. `status --help` `--kind <KIND> ... (api,
   local, subscription)` matches `--kind api|local|subscription`. "files one `signal`": `src/config.rs:303` default
   item type `"signal"`. "local servers file no alert" (#30). install.sh behaviour as in 4. Nothing promised is unbuilt;
   `serve` appears only under "Not in v1.0".
7. Secrets/public. `git diff origin/main..HEAD | grep -nEi '<email regex>|sk-|Bearer|ghp_|token=|hankh|/Users/'`: the
   only hits are in the brief (a `/Users/hankh95/...` path and the GitHub noreply address). Both are already on
   origin/main: `git grep -l '237287+hankh95@users.noreply' origin/main` → reviews/CHORE-002-r1.brief.md (and more);
   `git grep -l '/Users/hankh95' origin/main` → docs/DESIGN.md (and more). CHANGELOG and README changes carry none.

## Findings

F1 (nit). The preamble presents VOY-001's Definition of Done but rewords it. Command: `grep -n -A1 'Definition of
Done' kanban-work/voyages/VOY-001-*.md` → "`quotabus status` on the hub shows every configured API key and every
host's subscription state with ages; ...". CHANGELOG says "every subscription account with ages". The change is
justified (DESIGN:99, the EXP-002 rescope: subscriptions are read centrally, one row per account) but the text reads
as a quotation. Suggest: quote it verbatim and add "(subscriptions are read centrally since EXP-002, one row per
account)". Also "re-read on 2026-10-10" rests on docs/findings/CHORE-027-mini-redeploy.md (Date 2026-10-10, status and
select reads, alert summary line); citing it (#36) would make the claim checkable.

F2 (nit). `quotabus status --help` shows `--stale  Only entries that are UNKNOWN (absent or expired)`
(`src/main.rs:49`), but neither the CHANGELOG's status entry (`status [--json] [--kind] [--check <service>]`) nor the
README's status line mentions it. An under-statement, not an overstatement.

F3 (nit). Known limits paraphrases EXP-005 without its last item: `yurtle-kanban show EXP-005` → "...a sweep that
probes every configured model id for drift; polish of the file backend for outsiders without NATS." The CHANGELOG
and README list serve, ccusage, key expiry and drift but not the file-backend polish.

No blocking findings.
