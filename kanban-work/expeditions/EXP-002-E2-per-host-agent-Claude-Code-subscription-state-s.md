---
id: EXP-002
title: "E2: central subscription reads — every Claude account (Doppler setup-token) and Copilot, from the probe host; no per-host install"
type: expedition
status: underway
priority: high
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-08
tags: [v1.0, VOY-001]
depends_on: [EXP-001, CHORE-002, CHORE-007]
---

# E2: central subscription reads — every Claude account (Doppler setup-token) and Copilot, from the probe host; no per-host install

Part of VOY-001; after E1, CHORE-002 and CHORE-007. Design §3 (probe), §4 (Claude Max and Copilot rows), §9 row E2.

**Rescoped by the Captain, 2026-10-08:** "Doppler has the API keys for all agents (includeing all of the claude agents that run on each machine. We should not need to run anything else on the other machines (just Mini or M5 depending on where we host this)". The per-host agent, the statusLine install and the five-host units of the earlier plan are dropped. Everything runs in `quotabus probe` on the probe host (Mini for the fleet, §10 Q2 / CHORE-008).

**Measured 2026-10-08 on M5** (raw artefacts in `~/.local/state/quotabus-work/EXP-002/`, never in git):
- Doppler `nusy-product-team` holds `NUSY_CLAUDE_TOKEN_<ACCOUNT>` for six accounts. All six are setup-tokens (`oat` class).
- `GET api.anthropic.com/api/oauth/usage` with such a token → **403 permission_error**, "OAuth token does not meet scope requirement user:profile" (fake-token control: 401). So this route cannot be used.
- `POST /v1/messages` with the token (Bearer, `anthropic-beta: oauth-2025-04-20`, Haiku, `max_tokens` 1, ~35 input tokens) → 200, with headers `anthropic-ratelimit-unified-{5h,7d}-{utilization,reset,status}`, `-status` and `-overage-status`. All six accounts answered 200 with both windows (fake-token control: 401).
- `claude -p --output-format stream-json` with `CLAUDE_CODE_OAUTH_TOKEN` set to the same token → `rate_limit_event` with the same numbers as the headers (0.3 / 0.12, the same resets).
- Copilot: `GET api.github.com/copilot_internal/user` with Doppler `GITHUB_TOKEN` → 200, `quota_snapshots.premium_interactions.{entitlement, remaining, percent_remaining, unlimited}`, `quota_reset_date`.

**Captain's rulings, 2026-10-08:**
- Claude read: "Direct, stream-json fallback". The direct 1-token call comes first; if its unified headers are missing, `claude -p` stream-json runs once for that account.
- Subscription cadence: "Hourly". `[intervals] subscription` defaults to 1 h, with its own key and its own TTL (§10 Q5).
- Undocumented sources (SIG-003): the unified headers and `copilot_internal` are undocumented. They are on in the fleet config, off in the FOSS example, and every row is labelled `source = undocumented`. The stream-json fallback is Claude Code's own output (`source = official`).

## Plan
1. Config: `kind = "subscription"` services, each naming its `secret` (e.g. `NUSY_CLAUDE_TOKEN_HANKH95`) and its `account` slug, plus `sources`. `[intervals] subscription` defaults to `1h`, and `[ttl] subscription` to 3 × that.
2. Claude: a direct read per account → one `subscription.anthropic.<account>.<service>` row: the 5 h and 7 d windows as `used_pct`, `reset_at`, `reset_in_s` and `pace`. Statuses: `allowed` → ok; `rejected` / exhausted → `quota_exhausted`; 401/403 → `auth_failed`. Missing headers → the stream-json fallback when `claude` is on PATH, else `unknown` with a `cannot_assess:` reason. A window that is absent is never written as 0 %.
3. Copilot: `copilot_internal/user` with the configured token → a `monthly` window from `percent_remaining` and `quota_reset_date`, and `requests_remaining`. Remaining 0 or "exceeded your monthly quota" → `quota_exhausted`.
4. Each kind runs on its own due-ness (CHORE-007's scheduler); `--force` reads now. Every key comes from the environment only and never appears on argv, in a row, a log or an error; output goes through the redactor.
5. Docs: DESIGN §3/§4/§9 drop the per-host agent and statusLine, and record the measured routes above. `examples/quotabus.toml` gets the subscription services. `packaging/README.md` says nothing runs on the other hosts.

## Definition of Done
Six `subscription.anthropic.*` rows and one `subscription.github.*` (Copilot) row on the bus, all `observed_by` the probe host, each with its windows, resets and age. A bad-token control reads `auth_failed`, never `ok`. Tests use a local HTTP stub and fake tokens. The real path runs once under `doppler run` from the probe host, and its redacted output goes in the PR.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T17:22:10+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:backlog ;
    kb:at "2026-10-08T18:29:05+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:ready ;
    kb:at "2026-10-08T18:53:15+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T18:57:09+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 17:22)

Released with depends_on EXP-001, CHORE-002 (statusLine install waits on CHORE-002's measurement, Captain Q9). SIG-003 is built to the recommendation (undocumented sources off by default in the FOSS config, on in the fleet config, every row labelled source=undocumented); SIG-004's 5-min subscription cadence is config.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:29)

CHORE-002 (PR #3): the statusLine hook does NOT fire under claude -p. Step 2's precondition ('only after CHORE-002 shows it fires under claude -p') is therefore false, and the Captain's conditional Q9 yes goes back as SIG-008 (open). That signal also names a measured alternative: stream-json rate_limit_event. Back to harbor until SIG-008 is answered; the Copilot and ~/.claude.json fallback steps do not depend on it.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:53)

SIG-008 ruled (Captain 2026-10-08, 'Both + fallback'). Step 2 becomes: install the statusLine for interactive hosts, ALSO tee the stream-json rate_limit_event from claude -p runs into the same cache, and keep the ~/.claude.json fallback. A missing rate_limits reads as unknown, never 0 % (docs/findings/CHORE-002-statusline-under-claude-p.md). SIG-004/Q5: subscription reads stay at 5 min. SIG-003/Q4: Copilot copilot_internal is on in the fleet, off in FOSS, labelled. Released again to provisioning.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:56)

Captain 2026-10-08, adding to the Q5 ruling: "for how often to query, make sure there are separate times for the different types of queries." So each query kind has its OWN config interval, set independently: API (messages) probe, balance read, and subscription read (and any later kind, e.g. a quota-window read). There is no shared interval. Defaults per the SIG-004 ruling: API 12 h, balance 12 h, subscription 5 min.

### M5-MBP-2/s-72a67d16 (2026-10-08 18:58)

Now depends on CHORE-007 (per-kind intervals, Captain Q5): it lands first, and this item then adds its subscription interval (5 min) as a third key of the same [intervals] table. The claim is kept.

### M5-MBP-2/s-72a67d16 (2026-10-08 19:40)

tests red at bf1c9d6 (partner sub-agent): 75 new tests in tests/exp002_*.rs (subscription config, statusline capture and tee and install-statusline, agent for claude and copilot, schedule, units). 67 fail, every one on a todo!() stub or an assertion, none on a compile error. 8 controls pass, each shown able to fail.
Measured 2026-10-08, key names and value shapes only: ~/.claude.json cachedUsageUtilization has {fetchedAtMs, accountUuid, utilization.{five_hour,seven_day}.{utilization %, resets_at ISO}}, which matches the test fixture. It is present on Mini (fetched ~2026-09-23, stale) and the Spark (~2026-10-06), and ABSENT on M5 and Air (Claude Code 2.1.294). So the fallback alone often reads unknown, which is why the statusLine plus stream-json sources matter (SIG-008).
