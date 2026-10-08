# Flowback packet: CHORE-001 → nusy-product-team `docs/external-review.md` §1a

**From** quotabus CHORE-001 (VOY-001, DESIGN §9 row C1, `docs/DESIGN.md:427`). **To** nusy-product-team, tracked
there by nusy-product-team IDEA-13333. **Written** 2026-10-08 by M5-MBP-2/s-72a67d16, in the quotabus repo only.
No quotabus session edits nusy-product-team: this packet is the whole of the ask, and a session of that repo lands
it under that repo's own rules (its `CLAUDE.md`, `nk`, its review). DESIGN §9 calls the change "doc-only, straight
to main" (`docs/DESIGN.md:427`). That is how quotabus describes the change. It does not override nusy-product-team's
own landing rule.

**Depends on quotabus** `origin/main` at `6f5a5fa8ae6730360e347b6f8adde0d1a83bfe27` (the `status` and `select`
subcommands, their flags and exit codes, as cited below).

## 1. The exact change

- **Repo:** nusy-product-team. **File:** `docs/external-review.md`.
- **Base:** commit `f03397e352`, the last commit that touched the file. `git log -3 origin/main -- docs/external-review.md`
  in that repo, run 2026-10-08 with origin/main at `428bf3561f`, lists `f03397e352` first. The file is the same at both
  commits: `git diff --stat f03397e352 origin/main -- docs/external-review.md` printed nothing.
- **Replace lines 35–52** (`git show f03397e352:docs/external-review.md | sed -n 35,52p`). That range runs from the
  heading `## 1a. ⚠ Which models work — PROBED 2026-09-24 on M5 (re-probe before trusting; model names drift)`, through
  the probe method (37–38) and the 11-row table (40–50), to the line `**Two measured pitfalls from the same session:**`
  (52).
- **Keep lines 53–59 verbatim**: the two pitfall bullets, the `glm-5.3` context-window warning and the no-tools
  reviewer that invented findings. They are measurements, not model availability. Only their lead-in line (52) is
  reworded, because "the same session" would have no antecedent once the table is gone.
- **Leave the rest alone.** That includes §1 line 33 ("Probe before you trust a row: … the smoke in §3"), §1's
  provider table and §3's Smoke block. The new §1a points at that Smoke as the by-hand fallback.

### 1.1 The new text, in full (paste over lines 35–52)

````markdown
## 1a. Which models work right now — read `quotabus status`, don't trust a table

The hand-probed table that stood here (M5, 2026-09-24) is retired; it is kept in git at `f03397e352` (lines 35–50).
Live availability is now published by **quotabus** (https://github.com/Congruentsys/quotabus, tracked by IDEA-13333) to the
NATS KV bucket **`ai_status` on Mini's bus** (`nats://192.168.8.110:4222`). The bucket holds one row per
(service, model) for API keys and one per subscription account (each Claude account, the Copilot seat). Every row
carries its age and expires on its own: an absent or expired row reads **UNKNOWN**, and a bus that cannot be read
reads **CANNOT-ASSESS**. A stale "ok" is never shown as current.

```bash
quotabus --config <quotabus.toml> status                    # one table: SERVICE MODEL STATE AGE SOURCE DETAIL
quotabus --config <quotabus.toml> status --kind api         # only API keys (also: local, subscription); --stale: only UNKNOWN; --json
quotabus --config <quotabus.toml> status --check glm        # rc 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN (worst of its models)
pick=$(quotabus --config <quotabus.toml> select --role review --exclude-family anthropic); rc=$?   # "provider model", or nothing
```

`--config` may be left out when `$QUOTABUS_CONFIG` or `./quotabus.toml` names the config (quotabus's
`examples/quotabus.toml` is the fleet's services and bucket). `status` lists only the services that config names.
Without `--check` it exits 0, or 2 when the bus cannot be read. A state is `ok`, `auth_failed`, `model_missing`,
`quota_exhausted`, `rate_limited` or `degraded`. **The reviewer pick is `select`**, wired as in quotabus's
`docs/wiring/nusy-product-team.md`. It chooses only a fresh `ok` model whose family is not excluded. rc 0 means
chosen. rc 2 means CANNOT-ASSESS (bus unreadable). rc 3 means nothing qualifies. On rc 2 or rc 3, **do not run the
review, and never fall back to a hand-picked model**; that is §0 rule (2)'s false all-clear. A subscription seat
(a Claude account, the Copilot seat) appears in `status` but is never a `select` candidate. This is the interim rule
while quotabus SIG-009 is open ("Can a subscription seat (a Claude account, Copilot) be a 'select' candidate — and as
which model?").

**Only if the bus is down** (`status` rc 2 / CANNOT-ASSESS): probe by hand with §3's Smoke, one tiny call per model
(a 404 names the model as missing, a 401 names the key). Treat the result as that moment's reading, not a table to
keep.

**Two measured pitfalls (M5, 2026-09-24, from the hand-probe session this section replaced):**
````

Every command, flag and exit code above exists on quotabus `6f5a5fa`:

| claim in the new text | source on quotabus main |
|---|---|
| bucket `ai_status`, Mini's url | `docs/DESIGN.md:119-120`, `:255`; `examples/quotabus.toml:8-10`; `src/config.rs:211` (`DEFAULT_BUCKET`) |
| `--config`, else `$QUOTABUS_CONFIG`, else `./quotabus.toml` | `src/main.rs:23-25` |
| `status --json`, `--kind api\|local\|subscription`, `--stale`, `--check <id>` | `src/main.rs:37-51`; `README.md:17-18`; `docs/DESIGN.md:257` |
| `--check` rc 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN | `src/main.rs:50`; `src/freshness.rs:25-33`; `docs/DESIGN.md:257` |
| plain `status` rc 0, or 2 when the store cannot be read | `src/main.rs:748-752`, `:78` (`CANNOT_ASSESS = 2`) |
| table columns SERVICE MODEL STATE AGE SOURCE DETAIL | `src/main.rs:853` |
| the state words | `src/record.rs:12-20` (snake_case); UNKNOWN / CANNOT-ASSESS labels `src/freshness.rs:37-43` |
| only configured services are listed | `src/main.rs:686-691` |
| `select --role review --exclude-family anthropic`, rc 0/2/3, empty stdout on 2 and 3 | `src/main.rs:54-70`; `docs/DESIGN.md:258`, `:333-337`; `docs/wiring/nusy-product-team.md:23`, `:37-39` |
| only a fresh `ok` is chosen, `degraded` refused | `docs/DESIGN.md:318-320` |
| a seat is never a candidate, pending SIG-009 (open) | `docs/DESIGN.md:321-325`; `kanban-work/signals/SIG-009-*.md:3-5` (`status: backlog`) |
| UNKNOWN = absent or expired, CANNOT-ASSESS = unreachable bus | `README.md:8-9`; `src/main.rs:47` |
| §3's Smoke and "a 404 names the model as missing, a 401 names the key" | nusy-product-team `f03397e352:docs/external-review.md:33`, `:137-145` |

## 2. Why

1. **The table is a snapshot, and it says so.** Its own heading reads "re-probe before trusting; model names drift"
   (`f03397e352:docs/external-review.md:35`). It was measured once, from one host (M5, 2026-09-24). Every reader
   after that reads a date, not a state. quotabus publishes the same facts with their age, and each row expires on its
   own.
2. **It is what nusy-product-team's own design asked for.** IDEA-13333's design, row C1, says "`docs/external-review.md`
   §1a replaced by 'run `quotabus status`'; the hand probe retired" (nusy-product-team
   `origin/main:docs/design/IDEA-13333-PROVIDER-STATUS-MONITOR.md:259`, the same row as quotabus `docs/DESIGN.md:427`).
3. **An honest "no" is better than a hand pick.** `select` refuses rather than guesses. That is the property §0
   rule (2) wants.

## 3. Evidence

- **EXP-002, live rows on Mini's `ai_status`.** The EXP-002 comment of 2026-10-08 20:22
  (`kanban-work/expeditions/EXP-002-*.md:111`): "DoD met on the bus: 2026-10-08T20:21Z, from M5, 'doppler run
  --project nusy-product-team --config dev -- quotabus --config <fleet-subscription.toml> probe --force' gave
  'published 7 rows to bucket ai_status' (Mini). All 7 ok: six subscription.anthropic.* rows and
  subscription.github.hankh95.copilot, observed_by M5."
- **EXP-003, rc 3 when nothing is healthy.** From `reviews/EXP-003-r1.md:69-94` (C6, M5-MBP-2, 2026-10-08T20:50:25Z,
  read-only against Mini's `ai_status`):
  ```
  $ quotabus --config /tmp/rv-EXP-003-cfg/quotabus.toml select --role review --exclude-family anthropic --json ; echo rc=$?
  []
  quotabus: nothing qualifies for role "review" excluding anthropic: no configured model with the role has a fresh ok row
  select rc=3
  ```
  Every API model read `UNKNOWN … absent` in `status --kind api` (rc 0, lines 77–87). The review's conclusion
  (lines 92–93): "rc 3 with `[]` on stdout is the honest answer: there was no false pick and no fall-through to a
  hand-picked model."
- **Re-read for this packet** (M5-MBP-2, 2026-10-08T21:42:17Z, read-only; account labels masked here):
  ```
  $ nats --timeout 5s --server nats://192.168.8.110:4222 kv ls ai_status
  subscription.anthropic.<acct>.claude-<acct>      (6 keys)
  subscription.github.hankh95.copilot
  ```

### 3.1 Precondition: be honest about what the bucket holds today

On 2026-10-08 at 21:42Z, **Mini's `ai_status` held no API row**, only the 7 subscription rows above. The central
`probe` writes API rows, and it is not yet running on Mini. CHORE-008 (`arrived`) built `packaging/install.sh` and
records Mini as the probe host (`packaging/README.md:30-40`). That follows Captain 2026-10-08, on SIG-001: "If this is FOSS, the config will have to ask to run locally or on another host. In this case, run on mini" (`kanban-work/signals/SIG-001-*.md:26`). Mini is the host that was recommended. Making the host a config choice was not the recommended default (`docs/DESIGN.md:446`). The re-read above shows nothing has run it there.
`quotabus` was also not on M5's `PATH` (`which quotabus` printed "quotabus not found"). If nusy-product-team lands
this text before the probe runs on Mini, `status --kind api` reads UNKNOWN for every API model and `select` exits 3.
That reading is true, and the new text sends the reader to §3's Smoke for exactly that case. But it does remove
today's only list of working models. **Recommended order:** install the probe on Mini (quotabus `packaging/install.sh
--host mini …`, `packaging/README.md:37`), confirm that `quotabus status --kind api` shows fresh rows, then land §1a.
The nusy-product-team session decides the order. This packet does not.

## 4. Lines to cite on nusy-product-team IDEA-13333

Paste as an `nk` comment on IDEA-13333 once the change has landed there. Fill in `<sha>`.

```
quotabus CHORE-001 (C1): docs/external-review.md §1a now points at `quotabus status` (Mini's NATS KV bucket
ai_status) and `quotabus select --role review --exclude-family anthropic`; the 2026-09-24 hand-probed table is
retired (kept at f03397e352:docs/external-review.md:35-50) and one line keeps the by-hand probe (§3's Smoke) for
when the bus is down. Landed here at <sha>. Depends on quotabus 6f5a5fa (status/select CLI and exit codes).
Evidence: quotabus EXP-002 (7 live rows on ai_status, 2026-10-08T20:21Z) and EXP-003 review C6 (select rc 3, `[]`,
when nothing qualifies). Packet: quotabus docs/flowback/CHORE-001-to-nusy-product-team.md.
```

On the quotabus side, a quotabus session then comments on CHORE-001 with the same `<sha>` and moves it to done. That
meets the item's last clause: "the commit is cited on this item and on IDEA-13333"
(`kanban-work/chores/CHORE-001-*.md:18`).
