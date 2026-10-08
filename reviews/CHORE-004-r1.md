reviewed-at-sha: e641c2131b0e7bae53ab0f3c7bfcef486625d87a
verdict: approve
gate: make check rc=0 at e641c2131b0e7bae53ab0f3c7bfcef486625d87a on M5-MBP-2
brief: reviews/CHORE-004-r1.brief.md @ e641c2131b0e7bae53ab0f3c7bfcef486625d87a

Authorship: all of the work under review and this review were done by LLM agent sessions (Claude Opus 5.5). No human
reviewed it. The diff was written by a docs-author sub-agent of the resident session M5-MBP-2/s-72a67d16, which
commissioned it, checked it, ran the CHORE-002 measurements it cites, and gains from `approve`. This reviewer is
"distinct" only in the sense that it is a separate `claude -p` process with a fresh context that wrote none of the
work; it is the same model family, and it was briefed by the author with the committed brief named above.

## What was checked (host M5-MBP-2, 2026-10-08, PR #4 at the sha above)

`git ls-remote origin docs/CHORE-004-design-s4-measured` → `e641c2131b0e7bae53ab0f3c7bfcef486625d87a`.
`git diff --stat origin/main...HEAD` → `docs/DESIGN.md | 11 ++++-----`, `reviews/CHORE-004-r1.brief.md | 52 +++`.

1. **Definition of Done** (`git diff origin/main...HEAD -- docs/DESIGN.md`):
   - DoD 1 (Claude Max row): "measure first" is gone; the row now says the hook does **not** fire under `claude -p`
     (text or stream-json), only interactively; `rate_limits` absent on the first render ⇒ "not yet known, never 0 %";
     the stream-json `rate_limit_event` is named as a "measured alternative, pending SIG-008 (open)". Met.
   - DoD 2 (GLM row): `GET /api/monitor/usage/quota/limit` (5 h and weekly windows, undocumented, "field meanings
     [inferred]", 35-token call left `remaining` unchanged) and `GET /api/biz/subscription/list` (plan status and
     renewal, billing fields never published), "pending SIG-003 (open)"; "no wallet balance" kept; HTTP 200 + body
     `code: 401` on three of five endpoints ⇒ read body `code`/`success`; no rate-limit headers. Met.
   - DoD 3: every changed cell cites `docs/findings/CHORE-002-*.md`; gate below. Met.
2. **Recounts against the findings files** (`docs/findings/CHORE-002-zai-endpoint.md` = Z,
   `docs/findings/CHORE-002-statusline-under-claude-p.md` = S):
   - "three of the five endpoints" — Z table: fake key gives 200 + body `code: 401` on rows 1 (`/api/anthropic/v1/models`),
     3–4 (`/api/biz/subscription/list`), 5–6 (`/api/monitor/usage/quota/limit`); real 401 on rows 0 (messages) and 2
     (`/api/paas/v4/models`). 3 of 5 endpoints; the diff names the same five. ✓
   - "35-token" — Z:121 "a further 35-token messages call (the reviewer's re-run, 18:26Z) left `remaining` and
     `percentage` unchanged". ✓ (wording strength: F1)
   - "epoch ms" — Z: "`nextResetTime` is epoch **milliseconds**". ✓
   - "5 h and weekly" — Z: `unit 3, number 5` = 5-hour, `unit 6, number 1` = 1-week, marked [inferred]; the diff
     covers it with "field meanings [inferred]". ✓
   - "first render" — S: "The FIRST call (18:21:27Z, before any API reply) had the same keys minus … **`rate_limits`**". ✓
   - "0.19/0.09 ↔ 19/9 %", "same minute" — S: "It agrees with run C's statusLine to the unit (0.19 ↔ 19 %, 0.09 ↔ 9 %,
     identical `resetsAt`): same account, same minute." ✓
   - "No rate-limit headers on any response" — Z: "`ratelimit_headers` … was `{}` on all 14 responses." ✓
   - "Bearer or raw `Authorization`" — Z rows 5/6 "identical to 5". ✓
3. **Records, decides nothing.** SIG-003 and SIG-008 are each written "(open)" where cited
   (`grep -n "SIG-00[38]" docs/DESIGN.md` → lines 1, 186, 193, 283; both signal files have `status: backlog`).
   Q9: VOY-001 "Captain's rulings" line 21 reads "Yes to a `statusLine` entry … installed by the per-host agent
   expedition after the measurement chore confirms it fires under `claude -p`"; EXP-002 line 19 "only after CHORE-002
   shows it fires under `claude -p`". The diff calls the yes "on file only as paraphrase, conditional on C2 confirming
   the hook fires under `claude -p`" and quotes no Captain words. Faithful; no invented quote. The stream-json route is
   labelled "an agent's recommendation there, not a decision".
4. **Stale sentences.** `grep -n -i "measure first\|measured first\|Q9\|statusLine\|claude -p\|wallet\|z\.ai\|quota/limit\|C2" docs/DESIGN.md`:
   no "measure first" remains in §4; line 122's `sources = ["statusline", "claude_json"]`, §9's C2/E2 rows (263–264)
   and Q9's "*Recommend yes* … measured first by C2" are the design as filed and are not contradicted while SIG-008 is
   open. One near-miss: F2.
5. **Secrets.** `(git diff origin/main...HEAD -- docs/DESIGN.md | grep '^+'; gh pr view 4 … body) | grep -i -E
   "sk-|@[a-z]|[0-9a-f]{16,}|[A-Za-z0-9_\-]{32,}|agreementNo|orderNo|price|purchase|customerId|\$[0-9]|USD|CNY"` → 4
   hits, all false positives (a design path `IDEA-13333-PROVIDER-STATUS-MONITOR.md`, the reason slug
   `cannot_assess:token_login_no_usage_panel`, and the PR's head sha). No key, token, email, account, order or
   agreement number, price or purchase date.
6. **Gate.** `make check` in /tmp/rv-CHORE-004 → `35 passed in 6.17s`, `All work items valid. Checked 20 items`, rc=0.

## Findings

**F1 (nit) — the GLM row drops the source's "Unconfirmed" and states a conclusion.**
Command: `grep -n "35-token\|Unconfirmed" docs/DESIGN.md docs/findings/CHORE-002-zai-endpoint.md`
Output: Z:121 `**Unconfirmed:** a further 35-token messages call (the reviewer's re-run, …` — continuing "So the counter
either lags or does not register a call that small, and an adapter must not treat `remaining` as a live per-call
meter." DESIGN.md:186 "a further 35-token call left `remaining` unchanged, so it is no live per-call meter".
The source states one observation with two explanations and an adapter rule; the diff states a property of the
endpoint. Fix: "a further 35-token call left `remaining` unchanged (unconfirmed: it lags, or ignores a call that
small), so an adapter must not read it as a live per-call meter".

**F2 (nit) — §10 Q4 still names only "z.ai biz endpoints", while the newly recorded quota endpoint is `/api/monitor/…`.**
Command: `sed -n 278p docs/DESIGN.md`
Output: `4. **Undocumented sources** (Anthropic `oauth/usage`, Copilot `copilot_internal/user`, z.ai biz endpoints). …`
§4 line 186 now places `GET /api/monitor/usage/quota/limit` under "pending SIG-003", which is Q4. Not a
contradiction (pre-existing wording, and SIG-003's body also says "z.ai biz endpoints"), but a reader of Q4 alone
would not see the quota endpoint is covered. Fix: "z.ai `biz/subscription/list` and `monitor/usage/quota/limit`", or
leave it and note it on SIG-003.

No blocker or should-fix findings: verdict approve.
