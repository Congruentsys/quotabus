---
id: HAZ-003
title: "Direct subscription read gets HTTP 429 without Claude Code's system prompt — every Claude row falls back to stream-json"
type: hazard
status: arrived
priority: high
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: []
---

# Direct subscription read gets HTTP 429 without Claude Code's system prompt — every Claude row falls back to stream-json

Found after EXP-002 merged (PR #12), while publishing its rows to Mini's `ai_status` bucket from M5 on 2026-10-08T20:21Z.

**Measured** (same token, same minute; script `~/.local/state/quotabus-work/EXP-002/oat_headers_probe.py` and a copy with the `system` field removed, run under `doppler run --project nusy-product-team --config dev`):
- `POST /v1/messages` with a setup-token, **with** `"system": "You are Claude Code, Anthropic's official CLI for Claude."` → **200**, with `anthropic-ratelimit-unified-5h-utilization: 0.38`.
- The same call **without** `system` → **429** `rate_limit_error`, with no unified headers.
- `src/subscription.rs` (the direct read, ~line 220) sends no `system`. So `quotabus probe` with `sources = ["unified_headers"]` reads `unknown` (`cannot_assess:no_unified_headers`, "HTTP 429 carried no anthropic-ratelimit-unified headers"). With `stream_json` also listed, every account falls back to `claude -p`: all six rows read `probe=stream_json`. That costs a full Claude Code start per account per hour instead of a ~35-token call, and the probe host needs `claude` installed.

## Definition of Done
1. The direct read sends that `system` string. A stub test asserts the request body carries it, with a control that fails without it.
2. A 429 with no unified headers is classified honestly: not `ok`, and not `quota_exhausted` unless a header says so. Its reason names the cause.
3. Real path, from the probe host under `doppler run`: all six Claude rows read `probe=unified_headers`, `ok`, with both windows. The redacted output goes in the PR.
4. DESIGN §4's Claude row records the requirement (citing this measurement). `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T20:22:18+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T20:22:33+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:done ;
    kb:at "2026-10-08T20:33:40+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
    kb:closedBy <https://github.com/Congruentsys/quotabus/pull/13> ;
  ] .
```


## Comments

### M5-MBP-2/s-72a67d16 (2026-10-08 20:25)

Tests red at 505eef5 (partner): tests/haz003_system_prompt.rs. 5 fail on assertions: no system field in the request body, unknown instead of ok, probe=stream_json reproduces the bug, the reason doesn't name the 429, and DESIGN §4 is missing the requirement. 4 controls pass, each shown able to fail. The stub mimics the measured server (429 without the system prompt, 200 with it).
