reviewed-at-sha: 71e367611e1e96d148e824a9e654527c2579c3cf
verdict: approve
gate: make check rc=0 at 71e367611e1e96d148e824a9e654527c2579c3cf on M5-MBP-2
brief: reviews/CHORE-016-r1.brief.md @ 71e367611e1e96d148e824a9e654527c2579c3cf

Authorship: All of the work under review was done by LLM agent sessions (Claude Opus 5.5): the deployment, the
measurements and the finding. This review was done by an LLM agent session too, and no human reviewed either. The
Captain authorised the deployment ("go ahead, ssh from here") and was GUI-logged-in on Mini. This reviewer is
"distinct" in a narrow sense. It is a separate `claude -p` process with a fresh context, of the SAME model family
(Claude Opus 5.5), and it wrote none of the work. The driver briefed it with the committed brief
`reviews/CHORE-016-r1.brief.md`. The commissioning session is `M5-MBP-2/s-72a67d16`: it did the deployment, ran the
measurements and wrote the finding. This reviewer ran only read-only commands on Mini, from M5 over `ssh mini`, on
2026-10-09 between 03:20 and 03:23 UTC. It made no board writes and no GitHub writes.

## Checks

**Gate.** `PATH=/Users/hankh95/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-016 make check` → rc=0. The
last suite printed `test result: ok. 6 passed; 0 failed`.

**DoD 1–3 (install).** The DoD's paths are `/usr/local/...` and its user is `admin`. The item's 02:59 comment says to
read them as HAZ-004's corrected paths: `~/.local/bin` and `~/.config/quotabus` under `/Users/hankh19`. The checks
below use those paths.
- I rendered the README's fleet command locally with `--quotabus /Users/hankh19/.local/bin/quotabus-cycle
  --render-to /tmp/rv016-render`, then compared it with Mini's installed plist:
  `diff <(plutil -convert xml1 -o - /tmp/rv016-render/*.plist) <(plutil -convert xml1 -o - /tmp/rv016-installed.plist)`
  → no difference (`PLIST-SAME-AS-RENDER`).
- I compared the README's wrapper block with `ssh mini 'cat ~/.local/bin/quotabus-cycle'` → identical
  (`WRAPPER-SAME`).
- `plutil -extract ProgramArguments json` → `[".../doppler","run","--project","nusy-product-team","--config","dev",
  "--","/Users/hankh19/.local/bin/quotabus-cycle","probe","--config","/Users/hankh19/.config/quotabus/quotabus.toml"]`.
  `StartInterval` → `300`.
- `grep -c "^\[\[service\]\]" ~/.config/quotabus/quotabus.toml` → `14`, which matches the finding's 14 tables.
  `examples/quotabus.toml` has 7 API/local services (zhipu, deepseek, moonshot, openai, meta, xai, qwen), which
  matches the finding's list.
- `launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E "runs|last exit"` → `runs = 3`,
  `last exit code = 0` (at 03:22:15 UTC). That is one more tick than the two the DoD requires.

**DoD 4, re-run on Mini (03:20:40 UTC).**
- `quotabus status` → rc=0, with all 16 rows and the same states and details as §3. The ages have moved on: `8m` on
  the 03:12 rows, and `3m` on `local-qwen`, which tick 2 rewrote. No row is missing.
- `select --role review --exclude-family anthropic` → `deepseek deepseek-v4-flash`, rc=0.
- `select` with all six families excluded → `quotabus: nothing qualifies for role "review" excluding anthropic, zhipu,
  deepseek, moonshot, openai, meta: no configured model with the role has a fresh ok row`, rc=3. The control excludes
  every family with a review role: `xai` has `roles = []` in the example config, so leaving it out is correct.
- `nats ... kv ls ai_status | sort` → 22 keys, the same as the list in §5 (`diff` → `KV-SAME`).
- Alerts after tick 3: `grep -c "^ALERT"` → `4`. `grep -E "^(alert:|published)"` →
  ```
  published 18 rows to bucket ai_status
  alert: 16 keys checked, 4 crossings filed, 0 re-armed
  published 1 rows to bucket ai_status
  alert: 16 keys checked, 0 crossings filed, 0 re-armed
  published 1 rows to bucket ai_status
  alert: 16 keys checked, 0 crossings filed, 0 re-armed
  ```
  There is still one alert per crossing over three ticks.

**DoD 5 / secrets.** I re-ran the §7 grep → `0` in all five files. On Mini, `grep -cE "@[A-Za-z0-9.-]+\.(com|net|org)"`
over both logs → `0` and `0`. I grepped `git diff origin/main...HEAD` for `@|sk-|Bearer|ghp_|github_pat_|{"error`. The
only hits were the finding's own grep pattern text, the writer string `quotabus@0.1.0`, and the noreply commit
identity in the brief. The diff has no secret, no raw provider error body and no personal email.

**Over-claims.** None material. The finding says plainly that a stale row was not produced live and that the
stream-json fallback was not exercised. I checked two claims that the finding makes without quoting their output:
- The "first source" claim. I ran `grep -oE "probe=[a-z_]+" probe.err.log | sort | uniq -c` →
  `1 probe=copilot_internal`, `6 probe=unified_headers`. This supports it.
- The kimi floor of 100. I ran `grep -n floor ~/.config/quotabus/quotabus.toml` → line 78 `floor = 100` in the
  moonshot block. This supports it.

## Findings

F1 (minor, wording; not blocking). Two claims in the finding are not backed by the output it quotes.
- "every subscription read used its first source ... per `probe.err.log`" (§2). The finding quotes no output for this.
- "counted the 7 subscription token names" (§1). The finding quotes no command or output for this.

My re-run supports the first; my commands are under Over-claims above. I did not re-run the second, because it needs
Doppler in `gui/501`. Under Rule 1, the finding should quote its command and output for both claims, or soften them.

F2 (minor, wording; not blocking). "Mini's launchd unit published 18 rows over two ticks" (Answer) can be read as 18
writes in total. The log shows 18 rows on tick 1 and 1 row on tick 2, which is 19 writes to 18 distinct keys. §6
already says this; the summary line could say "18 rows on tick 1".
