reviewed-at-sha: 11ca437fbae51709365403011c889556113d91f4
verdict: changes
gate: make check rc=0 at 11ca437fbae51709365403011c889556113d91f4 on M5-MBP-2
brief: reviews/CHORE-002-r1.brief.md @ 11ca437fbae51709365403011c889556113d91f4

Authorship: all of the work under review and this review were done by LLM agent sessions (Claude Opus 5.5). No human
reviewed either. This reviewer is "distinct" only in this sense: a separate `claude -p` process with a fresh context
that wrote none of the work. It is the same model family, and the author briefed it with the committed brief
`reviews/CHORE-002-r1.brief.md`. The commissioning session is `M5-MBP-2/s-72a67d16`. Its stake: it ran every
measurement, wrote both findings files and gains from `approve`.

## Re-run results (host M5-MBP-2, 2026-10-08, ~18:25–18:27 UTC, Claude Code 2.1.294)

Artefacts: `~/.local/state/quotabus-work/CHORE-002/rv1/`. The copies of `hook.sh`, `settings.json`,
`interactive.py` and `zai_probe.py` differ from the author's only in the path (`diff` shows one changed line in
each). The code blocks in both findings files match the author's script files byte-for-byte, after stripping
whitespace.

**`~/.claude/settings.json` keys**: `jq -r 'keys|join(",")'` returned the same list before and after. `diff` printed
`keys-unchanged`:
`agentPushNotifEnabled,effortLevel,enabledPlugins,env,model,permissions,skipDangerousModePermissionPrompt,switchModelsOnFlag,theme,tui`.

**(1) statusLine.** I ran the finding's commands with `$R` set to `rv1/`:
```
A rc=0
B rc=0
2026-10-08T18:25:48Z
ls: /Users/hankh95/.local/state/quotabus-work/CHORE-002/rv1/statusline-calls.tsv: No such file or directory
ok
system init / assistant None / rate_limit_event None / result success
{"status": "allowed", "resetsAt": 1791495000, "rateLimitType": "five_hour", "overageStatus": "rejected", "overageDisabledReason": "org_level_disabled", "isUsingOverage": false, "unifiedWindows": {"five_hour": {"utilization": 0.19, "resetsAt": 1791495000}, "seven_day": {"utilization": 0.09, "resetsAt": 1792051200}}}
```
I then ran the control, `python3 $R/interactive.py; cut -f1,2 $R/statusline-calls.tsv | uniq -c; grep -c qb-statusline-probe $R/run-interactive.tty`:
```
rc=0
   1 2026-10-08T18:25:54Z	interactive
   1 2026-10-08T18:26:06Z	interactive
1
2026-10-08T18:25:54Z rate_limits-present=False None current_usage-null=True cost=0
2026-10-08T18:26:06Z rate_limits-present=True {'five_hour': {'used_percentage': 19, 'resets_at': 1791495000}, 'seven_day': {'used_percentage': 9, 'resets_at': 1792051200}} current_usage-null=False cost=0.1428…
```
This reproduces the finding. The hook stays silent under `-p`. The control fires twice, and `rate_limits` is absent
on the first render and present on the second. `rate_limit_event` agrees with the statusLine: 0.19 ↔ 19 and
0.09 ↔ 9, with identical reset epochs. `date -u -r 1791495000` printed `2026-10-08T21:30:00Z` and
`date -u -r 1792051200` printed `2026-10-15T08:00:00Z`. Both match the file.

**(2) z.ai.** I ran `python3 -I rv1/zai_probe.py FAKE fake`, then
`cd ~/Projects/nusy-product-team && doppler run --project nusy-product-team --config dev -- python3 -I rv1/zai_probe.py NUSY_GLM real`.
Both rc=0, and `zai-real.err` was 0 bytes. Status, body `code` and rate-limit headers, per probe:
```
fake 0 messages 401 · 1 anthropic/models 200 body 401 · 2 paas/models 401 · 3,4 subscription 200 body 401 · 5,6 quota 200 body 401
real 0 200 · 1 200 · 2 200 · 3,4 200 body 200 · 5,6 200 body 200        ratelimit_headers {} on all 14
```
- Real probe 0: `usage {input 15, output 20}` and `stop_reason max_tokens`.
- Probes 1 and 2: 11 models each.
- Subscription: 1 record with 28 fields. The kept values match the file. Probe 3's body equals probe 4's.
- Probe 5's `limits` equal probe 6's.
- My quota body is identical to the author's: `remaining 27999 / 139999`, `percentage 1`, and the same
  `nextResetTime` values. `date -u -r 1791500931` printed `2026-10-08T23:08:51Z` and `date -u -r 1791990277`
  printed `2026-10-14T15:04:37Z`. Both match the file.

**Artefacts vs the files.**
- The author's `statusline-calls.tsv` has 2 lines, and the keys of the first call are exactly the second call's
  keys minus `prompt_id`, `session_name`, `prompt_cache` and `rate_limits`.
- `grep -c qb-statusline-probe run-interactive.tty` returns 1.
- The `.err` files are 0 bytes, and `run-print-stream.out` has 4 lines in the stated order.
- The 22 names of withheld fields plus the 6 kept fields equal the 28 keys in the raw record.

**Secrets.** I grepped the diff and the PR body for emails, `sk-` keys, long hex or token strings, `bearer`, and the
words price, order, agreement, customer and purchase. The hits are only the field names, the brief's own wording,
the noreply address and `total_cost_usd`, which is Claude Code's session cost, not a subscription price. A script
then checked whether any non-kept value of the subscription record appears in the diff or the PR body. The one hit,
`currentPeriod`, is a 1-digit int that matches digits anywhere in the text: a false positive. Neither the PR body
(13 lines) nor the diff contains a key.

**Definition of Done.** Two findings files exist under `docs/findings/`. Each gives the exact command and its output.
(1) says whether the hook fires and shows the JSON it receives. (2) names the endpoint that answers and what it
reports. All of this is met.

## Findings

**F1 (should-fix): the files say a §4 amendment chore "is filed", but no such item exists on origin.**
`docs/findings/CHORE-002-statusline-under-claude-p.md:154` says "a chore to amend `docs/DESIGN.md` §4 is filed
alongside this finding". `CHORE-002-zai-endpoint.md:141` says "folded into the same §4 amendment chore".
Command: `git fetch -q origin && git ls-tree -r --name-only origin/main kanban-work/ | grep -v TEMPLATE`.
Output: `CHORE-001`, `CHORE-002`, `CHORE-003`, `EXP-001…007`, `SIG-001…007` and `VOY-001`. No amendment chore is
listed. Fix: file the chore with `create … --push` and cite its ID in both files, or reword both lines to "to be
filed" until it exists.

**F2 (should-fix): the files treat the Captain's Q9 "yes" as still standing, but that yes depended on this
measurement and the measurement came out negative.**
Command: `git show origin/main:kanban-work/expeditions/EXP-002-*.md | grep -n "claude -p"`.
Output, line 19: "installed by this expedition's unit, **only after CHORE-002 shows it fires under `claude -p`**".
The measurement shows the opposite. Finding (1) says the statusLine is "still worth installing for interactive
hosts" and that E2 "should add a second official source". That is a design decision on a Captain-conditioned
question, and no session may make it. Fix: say plainly in "What this changes" that EXP-002's stated precondition is
now false. Present the stream-json `rate_limit_event` route as a *recommendation* for the Captain, for example as a
comment on EXP-002 or a new signal. Do not present it as settled. Note it in the §4 amendment chore from F1.

**F3 (should-fix): "four of the seven endpoints" is the wrong count.**
`CHORE-002-zai-endpoint.md:6`. Command: re-run of the fake control (above). Output: a fake key gets HTTP 200 on
probes 1, 3, 4, 5 and 6. That is **five of seven probes**, or **three of the five distinct endpoints**
(`/api/anthropic/v1/models`, `/api/biz/subscription/list` and `/api/monitor/usage/quota/limit`). "What this changes"
on line ~137 names those three correctly. Fix: "on three of the five endpoints (five of the seven probes)".

**F4 (should-fix): the [inferred] reading of the quota fields must record that the counter did not move.**
`CHORE-002-zai-endpoint.md:117-119` reads `remaining` as cap minus used, "1 used of 28000". My re-run made another
35-token messages call against the same key at 18:26. Probes 5 and 6 then still read `remaining 27999`,
`percentage 1`, with identical `nextResetTime`. That is the same body as the author's ~18:23 run, which was itself
taken right after a messages call. Command: the real-key run above, then
`python3 -I -c "…json.load(open('zai-real-5.raw'))…"`. Output:
`"usage": 28000, "currentValue": 0, "remaining": 27999, "percentage": 1, "nextResetTime": 1791500931454`.
So either the counter lags, or a 35-token call is below its unit. Either way an adapter cannot treat `remaining` as a
live per-call meter. Fix: add one sentence to the [inferred] paragraph: a further messages call (reviewer re-run
18:26Z) left `remaining`/`percentage` unchanged, so the reading is unconfirmed and the counter's granularity or lag
is unknown.

**F5 (nit): §10 Q4 is cited as if it were decided.** `CHORE-002-zai-endpoint.md:133` says "under §10 Q4 it is off
in the FOSS config and on … in the fleet's". Command:
`git show origin/main:kanban-work/signals/SIG-003-*.md | head -6`. Output: `id: SIG-003`,
`title: "Q4: undocumented usage endpoints — …?"`, `status: backlog`. The question is open. Fix: "per §10 Q4's
recommendation (SIG-003, open)". Repo rule: say so in a comment on SIG-003.

**F6 (nit): the "verbatim" quota body has its keys reordered, and one timing claim is not inferred-labelled.**
`CHORE-002-zai-endpoint.md:107-113` calls the body "verbatim", but the raw body orders its keys
`code, msg, data{limits…, level}, success`, and the file shows `level` first and `success` before `data`. Line 116
says "≈ 5 h after the window opened", but nothing measured when the window opened. It is a back-calculation
(23:08:51 − 5 h = 18:08:51, before any probe). Fix: change "verbatim" to "reformatted", and mark the 5 h remark
[inferred].

**F7 (nit): one shown output has no command, and one block is not the file it is labelled as.**
- Finding (1) shows the two-line `1 … interactive` block without the command that produced it (it is
  `cut -f1,2 statusline-calls.tsv | uniq -c`). That goes against repo rule 1.
- It shows `settings.json` with `$HOME/…`, but the artefact holds the absolute path
  (`/Users/hankh95/.local/…`; the `diff` output above shows it). This is fine as redaction of the username, but
  say so.
- "The control would have failed (no file) had `--settings` not installed the hook" is a counterfactual that was
  never run. Either run it or cut it.
