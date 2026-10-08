# CHORE-002 (2): which z.ai endpoint answers a plain API key, and what does it report?

**Answer:** with the `NUSY_GLM` API key, **`GET https://api.z.ai/api/monitor/usage/quota/limit`** answers with the
Coding-Plan quota windows (cap, remaining, percent, next reset), and `GET /api/biz/subscription/list` answers with
the plan record (name, status, renewal). Both are undocumented. The Anthropic-protocol messages probe and both model
lists answer too. **None** returns a wallet balance or any rate-limit header. Watch out: on four of the seven
endpoints **a bad key gets HTTP 200** with `{"code": 401, …}` in the body, so a reader that checks only the HTTP status
reads a dead key as `ok`.

- Host: `M5-MBP-2`, macOS 27.0, Python 3.14.2 (`urllib`), doppler v3.76.0. Base: `https://api.z.ai`.
- Date: 2026-10-08, ~18:23 UTC. quotabus sha: `3ca35d1`.
- Key: `NUSY_GLM` from Doppler project `nusy-product-team`, config `dev` (the only GLM key name in the Doppler
  projects this host can read; `santiago` and `fishnet-automation` hold none). Read from the environment only; never
  on argv, never printed.
- Item: CHORE-002; design §4 (GLM / z.ai row) and PRIOR-ART §2 (usagebar's z.ai endpoints).

Authorship: all of the measurement and this write-up were done by an LLM agent session (Claude Opus 5.5,
`M5-MBP-2/s-72a67d16`, the commissioning session, which ran every command and wrote this file). No human reviewed
it. Its review is by a distinct `claude -p` session (a separate process, fresh context, same model family) briefed by
this session with a committed brief; see `reviews/CHORE-002-r1.md`.

## Method

The probe script, `~/.local/state/quotabus-work/CHORE-002/zai_probe.py` (raw bodies go to that dir, not git; stdout
is a redacted shape: strings with `@`, `sk-`, 16+ hex chars or 28+ token chars become `<redacted>`, lists are cut to
3 items):

```python
# CHORE-002 z.ai probe. Key read ONLY from env var named by argv[1] (never on argv); "FAKE" uses sk-test-not-a-key.
# Raw bodies go to the per-item state dir; stdout carries a redacted shape only.
import json, os, sys, time, urllib.request, urllib.error, re
S = os.path.expanduser("~/.local/state/quotabus-work/CHORE-002")
name = sys.argv[1]; label = sys.argv[2]
key = "sk-test-not-a-key" if name == "FAKE" else os.environ.get(name, "")
if not key: sys.exit(f"{name} not set")
B = "https://api.z.ai"
probes = [
  ("POST", B+"/api/anthropic/v1/messages", "x-api-key",
     {"model": "glm-5.2", "max_tokens": 20, "messages": [{"role": "user", "content": "Reply: ok"}]}),
  ("GET", B+"/api/anthropic/v1/models", "x-api-key", None),
  ("GET", B+"/api/paas/v4/models", "bearer", None),
  ("GET", B+"/api/biz/subscription/list", "bearer", None),
  ("GET", B+"/api/biz/subscription/list", "raw", None),
  ("GET", B+"/api/monitor/usage/quota/limit", "bearer", None),
  ("GET", B+"/api/monitor/usage/quota/limit", "raw", None),
]
def shape(o, depth=0):
    if isinstance(o, dict): return {k: shape(v, depth+1) for k, v in o.items()}
    if isinstance(o, list): return [shape(v, depth+1) for v in o[:3]] + (["…%d more" % (len(o)-3)] if len(o) > 3 else [])
    if isinstance(o, (int, float, bool)) or o is None: return o
    s = str(o)
    if re.search(r"@|sk-|[0-9a-f]{16,}|[A-Za-z0-9_\-]{28,}", s): return "<redacted>"
    return s[:60]
for i, (m, url, auth, body) in enumerate(probes):
    h = {"content-type": "application/json", "anthropic-version": "2023-06-01"}
    if auth == "x-api-key": h["x-api-key"] = key
    elif auth == "bearer": h["Authorization"] = "Bearer " + key
    else: h["Authorization"] = key
    req = urllib.request.Request(url, method=m, headers=h, data=json.dumps(body).encode() if body else None)
    t = time.time()
    try:
        r = urllib.request.urlopen(req, timeout=30); code, raw, hd = r.status, r.read(), dict(r.headers)
    except urllib.error.HTTPError as e:
        code, raw, hd = e.code, e.read(), dict(e.headers)
    except Exception as e:
        code, raw, hd = "ERR", type(e).__name__.encode(), {}
    ms = int((time.time()-t)*1000)
    open(f"{S}/zai-{label}-{i}.raw", "wb").write(raw)
    try: sh = shape(json.loads(raw))
    except Exception: sh = "<non-json %d bytes>" % len(raw)
    rl = {k: v for k, v in hd.items() if "ratelimit" in k.lower() or k.lower() == "retry-after"}
    print(json.dumps({"probe": i, "method": m, "path": url[len(B):], "auth": auth, "status": code, "ms": ms,
                      "ratelimit_headers": rl, "body_shape": sh}))
```

The `raw` auth mode sends the key as the bare `Authorization` value (usagebar's form). Cost: one messages call,
15 input + 20 output tokens, per key run; the rest are GETs.

```bash
S=~/.local/state/quotabus-work/CHORE-002
python3 $S/zai_probe.py FAKE fake > $S/zai-fake.jsonl                     # negative control: sk-test-not-a-key
doppler run --project nusy-product-team --config dev -- python3 $S/zai_probe.py NUSY_GLM real > $S/zai-real.jsonl
```

## Output

| # | request | fake key (control) | real key |
|---|---|---|---|
| 0 | `POST /api/anthropic/v1/messages` (`x-api-key`, `glm-5.2`, `max_tokens` 20) | **401** `token expired or incorrect` | **200**, `model: glm-5.2`, `stop_reason: max_tokens`, `usage {input 15, output 20}`, 1608 ms |
| 1 | `GET /api/anthropic/v1/models` (`x-api-key`) | 200, body `code: 401` | 200, 11 models, `glm-4.5` … `glm-5.3-flashx` |
| 2 | `GET /api/paas/v4/models` (Bearer) | **401** | 200, 11 models, `owned_by: z-ai` |
| 3 | `GET /api/biz/subscription/list` (Bearer) | 200, body `code: 401` | 200, body `code: 200`, one subscription (below) |
| 4 | same, raw `Authorization` | 200, body `code: 401` | identical to 3 |
| 5 | `GET /api/monitor/usage/quota/limit` (Bearer) | 200, body `code: 401` | 200, body `code: 200`, quota windows (below) |
| 6 | same, raw `Authorization` | 200, body `code: 401` | identical to 5 |

`ratelimit_headers` (any header containing `ratelimit`, or `retry-after`) was `{}` on all 14 responses.

Control lines (verbatim, `zai-fake.jsonl`):

```
{"probe": 0, … "status": 401, … "body_shape": {"error": {"message": "token expired or incorrect", "type": "401"}}}
{"probe": 1, … "status": 200, … "body_shape": {"code": 401, "msg": "token expired or incorrect", "success": false}}
{"probe": 5, … "status": 200, … "body_shape": {"code": 401, "msg": "token expired or incorrect", "success": false}}
```

**Quota windows, probe 5** (verbatim body shape; nothing here identifies the account):

```json
{"code": 200, "msg": "Operation successful", "success": true,
 "data": {"level": "max", "limits": [
   {"type": "CREDIT_LIMIT", "unit": 3, "number": 5, "usage": 28000,  "currentValue": 0, "remaining": 27999,  "percentage": 1, "nextResetTime": 1791500931454},
   {"type": "CREDIT_LIMIT", "unit": 6, "number": 1, "usage": 140000, "currentValue": 0, "remaining": 139999, "percentage": 1, "nextResetTime": 1791990277960}]}}
```

`nextResetTime` is epoch **milliseconds**: `1791500931454` = 2026-10-08T23:08:51Z (≈ 5 h after the window opened),
`1791990277960` = 2026-10-14T15:04:37Z. Reading [inferred, from these two rows only, not documented]: `unit 3,
number 5` = a 5-hour window and `unit 6, number 1` = a 1-week window; `usage` is the window's **cap** (not the amount
used), `remaining` = cap − used, `percentage` = used %, rounded up (1 used of 28000 reads 1). `currentValue` was 0 in
both rows; its meaning is unknown.

**Subscription, probe 3** — field NAMES only, because the values are a billing record (ids, order and agreement
numbers, purchase date, price, payment state) and this repo is public. Kept values: `productName: "GLM Coding Max"`,
`status: "VALID"`, `billingCycle: "monthly"`, `autoRenew: 1`, `banStatus: 0`, `version: "V3"`. Names of the rest:
`id, customerId, agreementNo, orderNo, productId, description, purchaseTime, valid, initialPrice, standardPrice,
useInitialPrice, actualPrice, renewPrice, currentPeriod, currentRenewTime, nextRenewTime, paymentType,
inCurrentPeriod, paymentChannel, refundable, refundableReason, banExpireTime`.

## What this changes

- Design §4, GLM row: "balance — no documented endpoint" stays true; there is still **no wallet balance** for an API
  key. But the Coding-Plan **quota** is readable: `/api/monitor/usage/quota/limit` gives two windows (5 h, weekly) with
  remaining and reset, which is the GLM equivalent of Claude Max's `rate_limits`. It is undocumented, so under §10 Q4
  it is off in the FOSS config and on, labelled `source = undocumented`, in the fleet's.
- `/api/biz/subscription/list` is the plan check (`status: VALID`, renewal). It returns billing identifiers, so an
  adapter must keep only `productName`, `status` and the renewal date, and never publish the rest.
- **Every z.ai adapter must read the body's `code`/`success`, not only the HTTP status**: a dead key gets HTTP 200 on
  the model list, the subscription list and the quota endpoint (control rows above). The messages probe and
  `/api/paas/v4/models` are the two that return a real HTTP 401.
- No rate-limit headers: headroom for GLM comes from the quota endpoint, or not at all.
- These changes are folded into the same §4 amendment chore as finding (1).
