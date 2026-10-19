reviewed-at-sha: e8dc29b0e4916fd928980ae363b4ef91b40bfceb
verdict: approve
gate: make check rc=0 at e8dc29b0e4916fd928980ae363b4ef91b40bfceb on `<mini>`
brief: reviews/CHORE-031-r1.brief.md @ e8dc29b0e4916fd928980ae363b4ef91b40bfceb

Authorship: all the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
"Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model
family, briefed by the driver (the author) with this committed brief. The commissioning session is
`<mini>/s-e7976c42`. Its stake: it ran the redeploy and wrote the finding.

Host: `<mini>` (user `<user>`, gui/501), run on Mini itself. Reads taken 2026-10-11T17:56:06Z – 17:58:43Z.
Reviewed: `docs/findings/CHORE-031-mini-v1.0.1.md` against CHORE-031's Definition of Done, in scratch worktree
`/tmp/rv-CHORE-031` (detached at the SHA above). No probe was run, no launchd job was bootstrapped, no bus key was
written, doppler was not run, and `forced.err.log` was not read.

## Verdict summary

Every DoD line is met, and every read-only command re-ran to the same values, or to values that moved only by the
clock. The installed binary is byte-identical to my own rebuild of the tag. Two non-blocking notes (F7, F8) are about
how the record is kept, not about the result.

## Findings

### F1. Definition of Done, line by line: met (not blocking)

| DoD line | Evidence | Met |
|---|---|---|
| PR merged with the 1.0.1 bump and the CHANGELOG entry | PR #40 merge `cdc656f…` = `v1.0.1^{commit}` (F2). The release body carries the `### Fixed` entry for #39 | yes |
| `git ls-remote origin refs/tags/v1.0.1` non-empty; `gh release view v1.0.1` exists | F2 | yes |
| finding landed by a PR with a distinct reviewer who re-runs the read-only commands | PR #41 is open; this review is that re-run | pending merge |
| `quotabus --version` prints `quotabus 1.0.1` | F3 | yes |
| forced cycle and one regular tick exit 0, each with its alert line | step 4 (`04-forced-exit.txt`, `forced.out.log`), step 6 (tick #751); I saw #752 exit 0 too (F4) | yes |
| a fresh row's `observed_by` ends `quotabus@1.0.1` | F5 | yes |
| `status` with every configured row present, and a subscription row counting "(resets in …)" from now | F5, F6 | yes |
| `select --role review --exclude-family anthropic` rc 0 or rc 3 with the reason | rc 0, `deepseek deepseek-v4-flash` (F5) | yes |
| the rollback | recorded, and checked in F10 | yes |
| no secret anywhere | F9 | yes |
| only bus writes are quotabus's own rows | The rows I read carry `observed_by <mini>/quotabus@1.0.1`. A reviewer cannot prove an absence of other writes from reads; nothing in the artefacts suggests any other write | yes, as far as reads can show |

### F2. Tag and release reproduce (not blocking)

```
$ git -C /Users/<user>/Projects/quotabus rev-parse v1.0.1 'v1.0.1^{commit}'
bb9a529771fbab40cd8f8814d17789e861e4fd18
cdc656f994ae8cdb6985856b792184d0c4c45b74
$ git -C /Users/<user>/Projects/quotabus ls-remote origin refs/tags/v1.0.1
bb9a529771fbab40cd8f8814d17789e861e4fd18	refs/tags/v1.0.1
$ gh pr view 40 --json mergeCommit
{"mergeCommit":{"oid":"cdc656f994ae8cdb6985856b792184d0c4c45b74"}}
$ gh release view v1.0.1
title:	v1.0.1
tag:	v1.0.1
draft:	false
prerelease:	false
...
published:	2026-10-11T17:47:43Z
url:	https://github.com/Congruentsys/quotabus/releases/tag/v1.0.1
...
### Fixed
- `quotabus status` counted "(resets in …)" from the row's `checked_at`, ... (#39).
```

These match step 1 and the finding's Release block exactly. The tag is annotated (its object differs from its
commit), and its commit is PR #40's merge.

### F3. The installed binary is the tag's build, byte for byte (not blocking)

```
$ ~/.local/bin/quotabus --version
quotabus 1.0.1
$ ~/.local/bin/quotabus.prev-v1.0.0 --version
quotabus 1.0.0
$ git -C /Users/<user>/Projects/quotabus archive v1.0.1 | tar -x -C /tmp/rv-CHORE-031-build/src
$ cd /tmp/rv-CHORE-031-build/src && CARGO_TARGET_DIR=/tmp/rv-CHORE-031-build/target ~/.cargo/bin/cargo build --release --locked -q
build rc=0
$ /tmp/rv-CHORE-031-build/target/release/quotabus --version
quotabus 1.0.1
$ shasum -a 256 /tmp/rv-CHORE-031-build/target/release/quotabus ~/.local/bin/quotabus
035bb9899f5937d2b5a14c71c4a8505e1fe75bd4d7c9e215ea664663af0b5905  /tmp/rv-CHORE-031-build/target/release/quotabus
035bb9899f5937d2b5a14c71c4a8505e1fe75bd4d7c9e215ea664663af0b5905  /Users/<user>/.local/bin/quotabus
```

The hashes match (12190240 bytes each). A mismatch would have been plausible for a build in a different directory,
but on this host and toolchain the build reproduced, which is stronger evidence than `--version` alone. The build
dir was removed afterwards.

```
$ ls -la ~/.local/bin/quotabus*
-rwxr-xr-x@ 1 <user>  staff  12190240 Oct 11 17:48 /Users/<user>/.local/bin/quotabus
-rwxr-xr-x@ 1 <user>  staff       816 Oct 10 15:26 /Users/<user>/.local/bin/quotabus-cycle
-rwxr-xr-x@ 1 <user>  staff       444 Oct  9 03:12 /Users/<user>/.local/bin/quotabus-cycle.prev-20261010
-rwxr-xr-x@ 1 <user>  staff  12184272 Oct  9 03:12 /Users/<user>/.local/bin/quotabus.prev-20261009
-rwxr-xr-x@ 1 <user>  staff  12202960 Oct  9 18:14 /Users/<user>/.local/bin/quotabus.prev-20261010
-rwxr-xr-x@ 1 <user>  staff  12193104 Oct 10 20:28 /Users/<user>/.local/bin/quotabus.prev-20261011
-rwxr-xr-x@ 1 <user>  staff  12193376 Oct 11 14:52 /Users/<user>/.local/bin/quotabus.prev-v1.0.0
```

The binary mtime (17:48) matches step 3's install time (17:48:35Z). `prev-v1.0.0` keeps the v1.0.0 mtime because of
`cp -p`, and CHORE-029's `prev-20261011` is still there, as claimed.

### F4. Regular ticks run the new binary and exit 0 (not blocking)

```
$ grep '^alert:' ~/Library/Logs/quotabus/probe.out.log | tail -3
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
$ launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E 'runs =|last exit code|program'     # 17:56:06Z
	program = /opt/homebrew/bin/doppler
	runs = 751
	last exit code = 0
$ launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E 'runs =|last exit code'             # 17:58:43Z
	runs = 752
	last exit code = 0
$ tail -3 ~/Library/Logs/quotabus/probe.out.log
nothing due
published 0 rows to bucket ai_status
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
```

Tick #751 matches step 6. Tick #752, newer than the finding, also exited 0 with the same alert line, and the program
is still the doppler launcher (HAZ-005). The one-shot job is gone:
`launchctl print gui/501/com.congruentsys.quotabus-chore031` returned rc 113 (no such service), so "booted out
afterwards" holds.

### F5. Bus rows, status and select reproduce; no row changed state (not blocking)

```
$ nats --server nats://<bus-host>:4222 kv get ai_status api.deepseek.nusy-product-team.deepseek-v4-flash --raw | jq -c '{key,state,observed_by,checked_at,ttl_s}'
{"key":"api.deepseek.nusy-product-team.deepseek-v4-flash","state":"ok","observed_by":"<mini>/quotabus@1.0.1","checked_at":"2026-10-11T17:48:36.351136Z","ttl_s":129600}
$ nats --server nats://<bus-host>:4222 kv get ai_status local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 --raw | jq -c '{key,state,observed_by,checked_at,ttl_s}'
{"key":"local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4","state":"ok","observed_by":"<mini>/quotabus@1.0.1","checked_at":"2026-10-11T17:48:36.351136Z","ttl_s":129600}
```

Both carry `checked_at` from the forced cycle (17:48:36Z, same as step 5), which is consistent with "nothing due" on
the later ticks.

`~/.local/bin/quotabus status --config ~/.config/quotabus/quotabus.toml` at 17:56:12Z returned rc 0 with 17 rows:
8 API, 2 local, 7 subscription. The STATE and DETAIL columns are the same as step 7's. Only AGE (3m → 7m) and the
five-hour "(resets in …)" values moved (107m → 103m, 87m → 83m), and they moved by the elapsed clock:
```
glm ... ok | glm ... ok | deepseek-v4-pro ok balance 245.17 CNY | deepseek-v4-flash ok balance 245.17 CNY
kimi-k3 quota_exhausted balance 17.41715 USD | gpt-5.6-sol auth_failed | Llama-3.3-70B auth_failed | grok-4 auth_failed
dgx1-qwen ok | dgx2-qwen CANNOT-ASSESS cannot_assess:timeout
claude-* (6) ok | copilot ok monthly 0% (resets in 20d)
rc=0
$ ~/.local/bin/quotabus select --role review --exclude-family anthropic --config ~/.config/quotabus/quotabus.toml
deepseek deepseek-v4-flash
rc=0
```
(The table is condensed here; the full output matches step 7 row for row apart from AGE and the time-left values.)

Step 4's 19 key/state lines are identical to CHORE-029's:
```
$ diff <(grep -E '^(api|local|subscription)\.[^ ]+ ' docs/findings/CHORE-029-mini-v1.0.0.md | sort -u) <(grep -E '^(api|local|subscription)\.[^ ]+ ' docs/findings/CHORE-031-mini-v1.0.1.md | sort -u) && echo "step-4 key/state lines identical"
step-4 key/state lines identical
```
`forced.out.log` (state lines only, via grep) holds exactly those 19 lines, plus `published 19 rows to bucket
ai_status` and the alert line, as step 4 quotes.

### F6. The CHORE-030 fix is live: status counts from now (not blocking)

```
now=2026-10-11T17:56:13Z
$ nats ... kv get ai_status subscription.anthropic.<acct-1>.claude-<acct-1> --raw | jq -c --arg now "$NOW" '{checked_at, five_hour: (.headroom.windows[]|select(.window=="five_hour")|{reset_at,reset_in_s, from_now_min: (((.reset_at|fromdateiso8601) - ($now|fromdateiso8601))/60|floor), stored_min: (.reset_in_s/60|floor)})}'
{"checked_at":"2026-10-11T17:48:36.351136Z","five_hour":{"reset_at":"2026-10-11T19:40:00Z","reset_in_s":6683,"from_now_min":103,"stored_min":111}}
$ (the same, for subscription.anthropic.<acct-3>.claude-<acct-3>)
{"checked_at":"2026-10-11T17:48:36.351136Z","five_hour":{"reset_at":"2026-10-11T19:20:00Z","reset_in_s":5483,"from_now_min":83,"stored_min":91}}
$ ~/.local/bin/quotabus status --config ~/.config/quotabus/quotabus.toml | grep -E "claude-<acct-1>|claude-<acct-3>"
claude-<acct-1>         claude-<acct-1>                         ok               7m   undocumented  five_hour 1% (resets in 103m) seven_day 1% (resets in 5d)
claude-<acct-3>         claude-<acct-3>                         ok               7m   undocumented  five_hour 18% (resets in 83m) seven_day 66% (resets in 2d)
```

`status` prints 103m and 83m, the from-now values. The stored values stay at 111m and 91m. This is a stronger
check than the finding's own: with the row aged 7 minutes rather than 4, the gap between the two counts is wider,
and `status` still tracks the clock. The stored `reset_in_s` is unchanged, as DESIGN §2 requires.

### F7. Raw artefacts are thinner than the finding in two places (not blocking)

```
$ cat ~/.local/state/quotabus-work/CHORE-031/08-resets-from-now.txt
now=2026-10-11T17:52:30Z
$ nats ... | jq -c '{checked_at, five_hour: (.headroom.windows[]|select(.window=="five_hour")|{reset_at,reset_in_s})}'
{"checked_at":"...","five_hour":{"reset_at":"2026-10-11T19:40:00Z","reset_in_s":6683,"from_now_min":107,"stored_min":111}}
...
$ cat ~/.local/state/quotabus-work/CHORE-031/02-build.txt
$ /tmp/qb-build-CHORE-031/target/release/quotabus --version
quotabus 1.0.1
```

- In `08-resets-from-now.txt` the recorded filter cannot produce the `from_now_min`/`stored_min` fields it shows.
  The finding discloses this openly ("echoes a shorter filter than the one that ran"). My F6 re-run with the
  finding's quoted filter reproduces the same values shifted by the clock, so the result stands.
- `02-build.txt` holds only `--version`, not the `cargo build` command or its `build rc=0` that the finding quotes.
  F3's rebuild covers this.

Next time, record the artefact with `tee` from the command that actually ran.

### F8. The brief names a backup that does not exist (not blocking; a brief typo)

```
$ ~/.local/bin/quotabus.prev-v1.0.1 --version
zsh: no such file or directory: /Users/<user>/.local/bin/quotabus.prev-v1.0.1
rc=127
```

The brief's allowed-command list says `quotabus.prev-v1.0.1`. The item's plan and the finding both name the backup
`quotabus.prev-v1.0.0`, which is right: the backup holds the v1.0.0 that was replaced, and it exists (F3). The
error is in the brief only, not in the finding.

### F9. Secret rules: hold (not blocking)

```
$ grep -cEi 'bearer|sk-[a-z0-9]|api[_-]?key=|authorization:|x-api-key|@[a-z0-9-]+\.(com|net|org)|ghp_|xox' docs/findings/CHORE-031-mini-v1.0.1.md reviews/CHORE-031-r1.brief.md <PR #41 body>
<PR #41 body>:0
reviews/CHORE-031-r1.brief.md:0
docs/findings/CHORE-031-mini-v1.0.1.md:1
$ grep -nEi '...' docs/findings/CHORE-031-mini-v1.0.1.md
104:These are the same 19 keys and states as CHORE-029's step 4. A grep of both logs for `bearer|sk-|api[_-]?key=` counted
```

The only hit is the finding quoting its own scan pattern, which is not a secret. The only address in any of the
three is the GitHub noreply commit identity in the brief (`…@users.noreply.github.com`), which is not a personal
email. The account names in the subscription keys and rows are all already published in
`docs/findings/CHORE-016-mini-deploy.md`:
```
$ for a in <user> <user> <acct-3> hankh95 <acct-4> <acct-5>; do grep -c "subscription.anthropic.$a\|claude-$a\|subscription.github.$a" docs/findings/CHORE-016-mini-deploy.md; done
<user> 5 | <user> 8 | <acct-3> 4 | hankh95 7 | <acct-4> 4 | <acct-5> 4
```
The DETAIL balances (CNY/USD) are account figures, not credentials, and CHORE-029 published them too.

### F10. The config is untouched, and the rollback restores the backup (not blocking)

```
$ stat -f '%Sm' -t '%FT%T%z' ~/.config/quotabus/quotabus.toml
2026-10-10T20:28:35+0000
```

The mtime predates the 17:48Z install by about 21 hours (it lines up with the 2026-10-10 20:28 deploy that left
`quotabus.prev-20261011`), so the config was not written by this redeploy. The rollback
`mv ~/.local/bin/quotabus.prev-v1.0.0 ~/.local/bin/quotabus` puts back the file that answers `quotabus 1.0.0` (F3).
Since the config is the same, that is the whole of v1.0.0's deployment. One nit: `mv` consumes the backup.
`cp -p` would keep it, but this is a matter of preference and not a defect. I did not run the rollback.

### F11. Gate (not blocking)

```
$ cd /tmp/rv-CHORE-031 && CARGO=/Users/<user>/.cargo/bin/cargo CARGO_TARGET_DIR=/tmp/rv-CHORE-031-target make check
make check rc=0
...
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

This covered the script tests, `yurtle-kanban validate`, `cargo clippy --all-targets --locked -- -D warnings` and
the Rust tests, at e8dc29b on `<mini>`. The target dir was removed afterwards.
