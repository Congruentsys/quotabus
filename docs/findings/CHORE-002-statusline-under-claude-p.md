# CHORE-002 (1): does the Claude Code `statusLine` hook fire under `claude -p`?

**Answer: no.** Under `claude -p` (text and stream-json output) the `statusLine` command is never run. In an
interactive session it is, and its stdin JSON carries `rate_limits.{five_hour,seven_day}.{used_percentage,resets_at}`
from the first render after an API reply. **But** `claude -p --output-format stream-json` emits a
`rate_limit_event` carrying the same two windows, so a `-p` session is readable without a statusLine.

- Host: `M5-MBP-2` (MacBook Pro, Apple M5), macOS 27.0, Claude Code `2.1.294`, Claude Max login (OAuth).
- Date: 2026-10-08, 18:21–18:22 UTC. quotabus sha: `3ca35d1` (origin/main when measured).
- Item: CHORE-002; design §4 (Claude Max row) and §10 Q9.

Authorship: all of the measurement and this write-up were done by an LLM agent session (Claude Opus 5.5,
`M5-MBP-2/s-72a67d16`, the commissioning session, which ran every command and wrote this file). No human reviewed
it. Its review is by a distinct `claude -p` session (a separate process, fresh context, same model family) briefed by
this session with a committed brief; see `reviews/CHORE-002-r1.md`.

## Method

The user's `~/.claude/settings.json` was **not** touched (its keys before and after: `agentPushNotifEnabled,
effortLevel, enabledPlugins, env, model, permissions, skipDangerousModePermissionPrompt, switchModelsOnFlag, theme,
tui` — no `statusLine`). The hook is installed per run with `--settings <file>`, which adds to the user settings.

Artefacts: `~/.local/state/quotabus-work/CHORE-002/` on the host (not in git).

`hook.sh` — appends one line per call (UTC time, the `QB_MODE` label inherited from the launching shell, the stdin
JSON), then prints a line for the status bar:

```sh
#!/bin/sh
# statusLine probe: append one record per invocation (label from QB_MODE), then print a line
d="$HOME/.local/state/quotabus-work/CHORE-002"
{ printf '%s\t%s\t' "$(date -u +%FT%TZ)" "${QB_MODE:-unset}"; tr -d '\n'; echo; } >> "$d/statusline-calls.tsv"
echo "qb-statusline-probe"
```

`settings.json` (the artefact holds the absolute path; the home directory is shown here as `$HOME`):

```json
{"statusLine":{"type":"command","command":"$HOME/.local/state/quotabus-work/CHORE-002/hook.sh"}}
```

Run A, `-p` text, and run B, `-p` stream-json (from the main checkout; `S=~/.local/state/quotabus-work/CHORE-002`):

```bash
QB_MODE=print-text env -u ANTHROPIC_BASE_URL -u ANTHROPIC_AUTH_TOKEN claude --model opus --settings $S/settings.json \
  -p "Reply with the single word: ok" < /dev/null > $S/run-print-text.out 2> $S/run-print-text.err
QB_MODE=print-stream-json env -u ANTHROPIC_BASE_URL -u ANTHROPIC_AUTH_TOKEN claude --model opus --settings $S/settings.json \
  -p --verbose --output-format stream-json "Reply with the single word: ok" < /dev/null > $S/run-print-stream.out 2> $S/run-print-stream.err
sleep 3; ls $S/statusline-calls.tsv
```

Run C, the **positive control** (interactive, same settings, in a pty; it waits 10 s, types the prompt, waits 20 s,
then SIGTERMs claude): `python3 $S/interactive.py`

```python
# positive control: run interactive claude in a pty with the probe settings for ~25s, type a prompt, then exit
import os, pty, sys, time, select, signal
S = os.path.expanduser("~/.local/state/quotabus-work/CHORE-002")
env = dict(os.environ, QB_MODE="interactive")
for k in ("ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"): env.pop(k, None)
pid, fd = pty.fork()
if pid == 0:
    os.execvpe("claude", ["claude", "--model", "opus", "--settings", f"{S}/settings.json"], env)
log = open(f"{S}/run-interactive.tty", "wb")
def pump(t):
    end = time.time() + t
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.5)
        if r:
            try: log.write(os.read(fd, 65536))
            except OSError: return
pump(10); os.write(fd, b"Reply with the single word: ok\r"); pump(20)
os.kill(pid, signal.SIGTERM); time.sleep(1)
try: os.kill(pid, signal.SIGKILL)
except ProcessLookupError: pass
```

## Output

Runs A and B: both `rc=0`, A printed `ok`, B wrote 4 stream-json lines. After both, the hook had never been called:

```
ls: ~/.local/state/quotabus-work/CHORE-002/statusline-calls.tsv: No such file or directory
```

Run C (control): the hook was called twice and the status bar showed its line (`grep -c qb-statusline-probe
$S/run-interactive.tty` → `1`). `cut -f1,2 $S/statusline-calls.tsv | sort | uniq -c`:

```
   1 2026-10-08T18:21:27Z	interactive
   1 2026-10-08T18:21:39Z	interactive
```

So the probe fires when a statusLine is due, and `-p` never makes one due.

The JSON the hook received on its second call, strings of 30+ chars or containing `/` replaced by `<str>`
(`session_id`, paths, `prompt_id`), the `prompt_cache` block cut to its point:

```json
{
 "session_id": "<str>", "transcript_path": "<str>", "cwd": "<str>", "scratchpad_dir": "<str>", "prompt_id": "<str>",
 "effort": {"level": "medium"},
 "session_name": "Single word reply",
 "model": {"id": "claude-opus-5-5", "display_name": "Opus 5.5"},
 "workspace": {"current_dir": "<str>", "project_dir": "<str>", "added_dirs": [],
               "repo": {"host": "github.com", "owner": "Congruentsys", "name": "quotabus"}},
 "version": "2.1.294",
 "output_style": {"name": "default"},
 "cost": {"total_cost_usd": 0.138667, "total_duration_ms": 11898, "total_api_duration_ms": 1817,
          "total_lines_added": 0, "total_lines_removed": 0},
 "context_window": {"total_input_tokens": 43863, "total_output_tokens": 4, "context_window_size": 1000000,
                    "current_usage": {"input_tokens": 2, "output_tokens": 4, "cache_creation_input_tokens": 16626,
                                      "cache_read_input_tokens": 27235},
                    "used_percentage": 4, "remaining_percentage": 96},
 "exceeds_200k_tokens": false,
 "prompt_cache": {"warm": true, "ttl": "1h", "…": "…"},
 "fast_mode": false,
 "thinking": {"enabled": true},
 "rate_limits": {"five_hour": {"used_percentage": 19, "resets_at": 1791495000},
                 "seven_day": {"used_percentage": 9, "resets_at": 1792051200}}
}
```

The FIRST call (18:21:27Z, before any API reply) had the same keys minus `prompt_id`, `session_name`,
`prompt_cache` and **`rate_limits`**, with zero cost and `context_window.current_usage: null`. A hook must therefore
treat a missing `rate_limits` as "not yet known", never as 0 %.

`resets_at` is epoch seconds: `1791495000` = 2026-10-08T21:30:00Z, `1792051200` = 2026-10-15T08:00:00Z.

### The `-p` path: stream-json's `rate_limit_event` (run B)

Run B's line types, in order: `system/init`, `assistant`, **`rate_limit_event`**, `result/success`. The event:

```json
{"type": "rate_limit_event", "rate_limit_info": {"status": "allowed", "resetsAt": 1791495000, "rateLimitType": "five_hour",
 "overageStatus": "rejected", "overageDisabledReason": "org_level_disabled", "isUsingOverage": false,
 "unifiedWindows": {"five_hour": {"utilization": 0.19, "resetsAt": 1791495000},
                    "seven_day": {"utilization": 0.09, "resetsAt": 1792051200}}}}
```

It agrees with run C's statusLine to the unit (0.19 ↔ 19 %, 0.09 ↔ 9 %, identical `resetsAt`): same account, same
minute. (`session_id`, `uuid` dropped.) Command to see it: `grep rate_limit_event $S/run-print-stream.out`.

## What this changes

- Design §4, Claude Max row: "whether the hook fires under `claude -p` is **measure first**" → measured: **it does
  not**. The statusLine capture only sees hosts where someone runs an interactive session; a host that only runs
  `claude -p` reviewers never writes the cache.
- §10 Q9: the Captain's "yes" (2026-10-08) was **conditional**. EXP-002 step 2 installs the statusLine "only after
  CHORE-002 shows it fires under `claude -p`", and **that precondition is now false**. This finding does not decide
  what follows. The question is back with the Captain as **SIG-008 (open)**, and EXP-002 is back in `harbor` until it
  is answered. The measured alternative is offered there as a *recommendation*, not a decision: the
  `rate_limit_event` of a `claude -p --output-format stream-json` run (e.g. a reviewer launcher that tees that event
  to the same cache file), in the same normalised shape (`utilization` × 100 = `used_percentage`, `resetsAt` =
  `resets_at`), next to the statusLine for interactive hosts.
- The measured facts are recorded in `docs/DESIGN.md` §4 by **CHORE-004** (filed, released behind CHORE-002), which
  records them and decides nothing.
- The hook must treat an absent `rate_limits` (the first render) as unknown, not zero.
