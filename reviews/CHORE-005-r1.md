reviewed-at-sha: fc2f7064f2f1e90a104743bac62d1d3bb1b23c1e
verdict: approve
gate: make check rc=0 at fc2f7064f2f1e90a104743bac62d1d3bb1b23c1e on Mac-mini
brief: reviews/CHORE-005-r1.brief.md @ fc2f7064f2f1e90a104743bac62d1d3bb1b23c1e

Authorship: all of the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed
it. The diff (commit 3caa9a6) was written by a fresh Opus sub-agent that the commissioning session
Mac-mini/s-e7976c42 spawned. That session checked the diff, wrote this review's brief, and gains from `approve`.
This review comes from a distinct session: a separate `claude -p` process with a fresh context that wrote none of
the work. It is the same model family, and the driver briefed it with the committed brief named above. It ran on
Mac-mini on 2026-10-08, in a detached scratch worktree at the sha above.

## Checks

**1. Done when, point by point.** `git diff origin/main...fc2f706 -- docs/DESIGN.md` touches three places:
- the §3 sample TOML (one key per line, plus a sentence on why);
- the §3 Outputs row "KV bucket `ai_status`" (`kv add --marker-ttl` + `kv create --ttl`, citing
  `docs/findings/EXP-001-per-key-ttl.md:38-41` and `:3-4,22-29`);
- the §4 OpenAI row (chat completions + `max_completion_tokens`, citing `src/probe.rs:247-261`,
  `reviews/EXP-001-r1.md:167-172` and PR #2).

All three points the item names are amended, and each one cites evidence. The PR has not merged yet, so the item's
"reviewed and merged" part is still open.

**2. Citations at their primary lines**
- `docs/findings/EXP-001-per-key-ttl.md:3-4`: "A bucket created with `--marker-ttl` reports `Per-Key TTL Supported:
  true`; a key written with a 5 s TTL was readable at +2 s and absent … at +8 s". Lines 22-29 are the command
  sequence (`kv add … --marker-ttl=2s`, `kv create … --ttl=5s`, `key not found rc=1 at +8s`), and lines 12-13 place
  it on Mini's 2.12.4. This supports the sentence that cites it.
- `:38-41`: "natscli 0.3.1 has no `kv put --ttl` … the flag is on `kv create` ("Sets a TTL for the key"), and
  `kv put --help` lists none … `nats kv create` (which fails when the key exists) — or a delete then create." This
  supports the put/create claim and the delete-then-create sentence. It does not cover the `kv add --ttl` claim or
  "on … Mini" (see F1).
- `src/probe.rs:247-261`: `Protocol::Openai`. When `svc.provider == "openai"`, the ceiling field is
  `max_completion_tokens`, otherwise `max_tokens`. Line 261 is `.post(format!("{base}/chat/completions"))`. The
  comment at :248 is the "stated reason". This supports the sentence, and the DESIGN text correctly marks that
  reason as not measured.
- `reviews/EXP-001-r1.md:167-172`: F5, "Mini's bus shows the call works (`openai gpt-5.6-sol ok 4m official`)". This
  supports the sentence.
- `gh pr view 2 --comments`: the review comment has "openai ok" (Mini, read-only check 7, 2026-10-08T18:18:17Z).
  The author's reply has "F5 … kept — it measurably works (`gpt-5.6-sol` ok)" and "Real probe re-run at `f58ad86`
  under `doppler run`: … gpt-5.6-sol ok". This supports the claim.
- `examples/quotabus.toml`: one key per line. This supports "uses the same layout".

**3. natscli** (this host, Mac-mini): `nats --version` → `0.3.1`. `nats kv put --help` lists no flags except the
global ones, so it has no `--ttl`. `nats kv create --help` → "Puts a value into a key only if the key is new or it's
last operation was a delete … `--ttl=DURATION  Sets a TTL for the key`". `nats kv add --help | grep -i ttl` →
`--ttl=DURATION How long to keep values for` and `--marker-ttl=DURATION Enables Per-Key TTLs and Limit Markers`.
All of these match the text. I ran help only and wrote nothing to kv.

**4. Sample TOML.** I extracted the first fenced `toml` block of DESIGN.md and loaded it with `.venv/bin/python -I`
and `tomllib.loads`:
```
== SHA (git show HEAD:docs/DESIGN.md | … )
OK ['alert', 'bus', 'probe', 'service'] ['glm', 'deepseek', 'claude-max', 'copilot']
rc=0
== origin/main (git show origin/main:docs/DESIGN.md | … )
FAIL TOMLDecodeError Expected newline or end of document after a statement (at line 11, column 11)
rc=1
```
The control fails as expected.

Binary acceptance: I took the same block, swapped `[bus]` for `[file] dir = "/tmp/rv-CHORE-005-scratch/rows"` so
nothing touches the bus, and ran `env -u QUOTABUS_NATS_URL /tmp/rv-CHORE-005-target/debug/quotabus --config
/tmp/rv-CHORE-005-scratch/sample.toml status`:
```
quotabus: CANNOT-ASSESS: row directory /tmp/rv-CHORE-005-scratch/rows does not exist
SERVICE   MODEL              STATE          AGE  SOURCE  DETAIL
glm       glm-5.3            CANNOT-ASSESS  -    -       cannot_assess:bucket_missing
… (5 rows: glm ×3, deepseek ×2)
rc=2
```
The config loads: rc 2 is only the missing row directory, not a config error. Every `[[service]]` key is a field of
`raw::Service`, which has `deny_unknown_fields` (`src/config.rs:200-218`), and `kind = "subscription"` is a valid
`Kind` (`src/record.rs:25-29`). The block agrees with `examples/quotabus.toml` on layout and on every shared key.
It is an illustrative subset (adds `glm-4.6`; no kimi/openai/together/xai/local-qwen), and that is fine for a design
sketch. See F2 for `[alert.*]`.

**5. Rest of DESIGN.md.** The diff has exactly the three hunks above, and nothing else changes meaning. The old
"is the first thing to measure" is correctly replaced with EXP-001's measured result. The GLM row's "pending SIG-003
(open)" is unchanged context, and no SIG is written as decided.

**6. Secrets / public repo.** I grepped `git diff origin/main...HEAD` plus `gh pr view 6` for
`sk-…|ghp_|Bearer …|xai-…|email|org-…|<n> CNY/USD`. There were 2 hits, both unchanged context lines from §4 that
are already on `main` (the word "Bearer" in prose; the `-0.12 CNY` example). No added line and nothing in the PR
body carries a key, token, email or private detail.

**7. Gate.** I ran `PATH=$HOME/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/rv-CHORE-005-target make check` in
/tmp/rv-CHORE-005 at fc2f706. Result: rc=0. The tail shows `test result: ok. 15 passed; 0 failed` and doc-tests 0/0.

## Findings

### F1 — nit — two natscli claims are stronger than the finding they cite
Command: `sed -n '12,14p;38,41p' docs/findings/EXP-001-per-key-ttl.md`. Line 12 places the measurement on "M5
(macOS, natscli **0.3.1**)". Lines 38-41 say nothing about `kv add --ttl`. The DESIGN text says "`kv add --ttl` is
the bucket-wide max age … (`--help` of each, natscli 0.3.1 on M5 and Mini; `…EXP-001-per-key-ttl.md:38-41`)". Both
claims are true: this review measured them on Mini (`nats kv add --help` → `--ttl=DURATION How long to keep values
for`; `nats --version` → `0.3.1`). But the cited source supports neither "Mini" nor the `kv add --ttl` reading.
**Fix (optional):** cite this review (`reviews/CHORE-005-r1.md`, check 3) for the Mini/`kv add` help text, or drop
"and Mini".

### F2 — nit — the sample's `[alert.*]` tables are silently ignored by the binary
Command: `grep -rn 'alert' src/` → no output. `grep -n -B2 'pub struct File {' src/config.rs` shows `raw::File`
without `deny_unknown_fields`. The new block's `[alert.nusy-kanban]`, `[alert.yurtle-kanban]` and `[alert.webhook]`
parse as TOML and are accepted, but nothing reads them. This is unchanged by the PR: the alert sinks are a design
feature that is not built yet, and the item asks only for valid TOML. **Fix (optional, not this PR):** a later item
that builds alerts, or a `# not yet read by the binary` comment on the block.

No blocker or should-fix findings.
