# Flowback packet: CHORE-020 → nusy-product-team: tell every machine that quotabus exists

**From** quotabus CHORE-020 (`kanban-work/chores/CHORE-020-*.md`). **To** nusy-product-team, where quotabus is tracked
by nusy-product-team IDEA-13333. **Written** 2026-10-09 in the quotabus repo only. No quotabus session edits
nusy-product-team. This packet is the whole of the ask. A session of that repo lands it under that repo's own rules
(its `CLAUDE.md`, `nk`, its review). Each machine then re-syncs its `~/.claude/CLAUDE.md` with that repo's own *Re-sync
recipe* (`docs/AGENT-MACHINE-CANON.md:72-235` at the base below). A quotabus session never does that re-sync.

**Authorship:** an LLM agent sub-agent (Claude Opus 5.5) wrote this packet for the driver session M5-MBP-2/s-72a67d16.
No human has reviewed it.

**Base:** nusy-product-team `origin/main` at `aa270c1612470f5e5e9e57dc63e3806a54541dd9`, read on 2026-10-09 with
`git show` only. Each target file's last commit at that base (`git log -1 --format=%H origin/main -- <file>`):

| file | last changed at | change asked |
|---|---|---|
| `docs/AGENT-MACHINE-CANON.md` | `42c51bab04c90e02c307373a3d59431037183464` | insert ONE bullet inside the fleet-managed block (§1) |
| `CLAUDE.md` | `d1b7c970293633626551668673f490cfdf265255` | insert ONE new `##` section (§2) |

Line numbers below are at those commits. If either file has moved on, place the text by its anchor (the quoted
neighbouring line), not by its number.

## 0. Why: the Captain's words

The item's own record (`kanban-work/chores/CHORE-020-*.md:16-27`):

- Captain 2026-10-09: "Once working we should update the CLAUDE.md on every machine to know this tool exists".
  (`:16`)
- Two rulings for this item, Captain 2026-10-09, given by AskUserQuestion (`:20-27`):
  - **How other machines read:** "Raw nats kv". The Captain did **not** take the default the agent offered, which
    was to install the `quotabus` reader binary on each machine. So nothing is installed on other machines, and
    agents read the bus with natscli.
  - **Where the line goes:** "Both (Recommended)". The Captain **chose the default the agent offered**: ONE short
    pointer line in the fleet-managed block, with the detail in project `CLAUDE.md`.

"Both" follows the canon doc's own rule. The block states the highest-cost rules and points at their source, "so the
block stays deliberately short" (`42c51bab04:docs/AGENT-MACHINE-CANON.md:44-47`). Its discriminator puts fleet canon
"in project `CLAUDE.md`, with only a pointer here" (`:39-41`).

**"Once working" is met.** VOY-001's Definition of Done was measured MET on Mini's live bus
(`docs/findings/CHORE-016-mini-deploy.md`). The probe job runs on Mini under launchd, with a 300 s tick
(`packaging/README.md:4-7`, `:38`; `packaging/install.sh:19-20`).

**Why the CLAUDE.md text has to state the freshness rule (G1).** quotabus's binary applies the freshness rule for
every reader (`docs/DESIGN.md:62-70`). A raw `nats kv get` does not apply it. Each reader therefore has to apply it,
so the text says what the rule is. Without it, a raw reader can take an `ok` past its `ttl_s` as current. The design names
the failure this rule prevents: a 44-day-old `EXHAUSTED` that a raw read of another bucket still returns
(`docs/DESIGN.md:72-76`).

## 1. `docs/AGENT-MACHINE-CANON.md`: one bullet in the fleet-managed block

- **Where:** inside the markers (`<!-- BEGIN fleet-managed` at `:239`, `<!-- END fleet-managed -->` at `:265`). Put it
  directly after the **Kanban** bullet (`:255-258`, which ends "Arrow store on Mini is the single source of truth — no
  markdown files for work items."). Put it before the **Session start** bullet (`:259`). Both bullets are about Mini's
  bus.
- **Shape:** one bullet, wrapped over three lines with the two-space continuation indent. This is the same style as
  every other bullet in the block (`:242-264`). It is ONE item, but not one physical line. Every other bullet wraps,
  so a single 250-column line would break the block's style. Join the three lines if the receiving session reads
  "one line" literally. The words do not change.
- **Copy-safe:** the bullet is plain ASCII, with no backtick, no `$` and no `{`. The hazards at `:57-62` (an unquoted
  heredoc runs backtick spans and expands `${…}`) therefore cannot touch it, even if someone pastes it badly. Still
  copy the bytes as the doc says.

The bullet, exactly (insert after line 258):

```markdown
- **AI provider status:** quotabus publishes live provider and account status to the NATS KV
  bucket ai_status on Mini's bus; read it with natscli, nothing to install. An absent or expired
  key is UNKNOWN, never ok. Commands, keys and the freshness rule: project CLAUDE.md, quotabus.
```

The last word, "quotabus", names the §2 heading below. If the receiving session renames that heading, change the
pointer to match.

## 2. `CLAUDE.md`: one new section

- **Where:** a new `##` section after **`## One session per machine`** (`d1b7c97029:CLAUDE.md:503-526`) and before
  **`## Merge subject and the transport`** (`:528`). That section introduces Mini as "the bus host: NATS
  `192.168.8.110:4222`" (`:505-506`), and this section is about one more thing on that bus. None of the 14 existing
  `##` headings (`git show origin/main:CLAUDE.md | grep -n '^## '`) covers provider status. The nearest is the
  FOSS-repo line in **Still true** (`:679`), but it is about how FOSS repos land code, not about reading them.
- **Leave the rest alone.**

### 2.1 The text, in full (insert between lines 526 and 528, with one blank line each side)

````markdown
## AI provider status — quotabus (read the bus with natscli)

**quotabus** (https://github.com/Congruentsys/quotabus, a standalone FOSS repo tracked here by IDEA-13333) publishes
live AI provider and account status to the NATS KV bucket **`ai_status` on Mini's bus**. That covers each API key's
models, each subscription seat (a Claude login, the Copilot seat) and the self-hosted endpoints. **Probing runs only
on Mini**: a launchd job ticks every 300 s and probes whatever is due. Nothing is installed on any other machine
(Captain 2026-10-09: "Raw nats kv"). Read the bucket with natscli:

```bash
nats --server nats://192.168.8.110:4222 kv ls ai_status                 # every key
nats --server nats://192.168.8.110:4222 kv get ai_status <key> --raw    # one ai-status/1 JSON row
```

Keys (slugs, never an email):
- `api.<provider>.<account>.<model>`: an API key's model.
- `subscription.<provider>.<account>.<service>`: a subscription seat.
- `local.<provider>.<host>.<model>`: a self-hosted endpoint.
- `api.<provider>.<account>.balance`: that key's balance read, not a model.
- `alert.*`: the alerter's dedup state, **not status**. Never read one as a provider's state.

**The freshness rule. A raw read skips it, so you apply it yourself:**
- **Key absent** (`nats: error: nats: key not found`, rc 1) → **UNKNOWN**, never ok. The bucket has per-key TTL, so an
  expired row is physically gone. Absent is how most stale rows look.
- **`checked_at + ttl_s < now`** → **UNKNOWN**, even when the key is present.
- **Only `"state":"ok"` is usable.** No other state is usable: `auth_failed`, `model_missing`, `quota_exhausted`,
  `rate_limited`, `degraded`, and `unknown` (a probe that could not measure, `"reason":"cannot_assess:<why>"`).
- **Bus unreachable** → **CANNOT-ASSESS**. Never read it as "no problems".

The source of the rule and of every field is quotabus `docs/DESIGN.md` §2.
````

Every claim in that text, against its source:

| claim | source |
|---|---|
| repo URL | `git -C <quotabus checkout> remote get-url origin` → `https://github.com/Congruentsys/quotabus.git` (2026-10-09) |
| tracked by IDEA-13333 | quotabus `CLAUDE.md:13`; nusy-product-team `aa270c1:docs/external-review.md:38` |
| bucket `ai_status` on Mini's bus, `nats://192.168.8.110:4222` | §3 `kv info` (below); `d1b7c97029:CLAUDE.md:505-506` |
| probing only on Mini, launchd, 300 s tick, "probes whatever is due" | `packaging/README.md:4-7` (the tick; each run "makes only" the due calls), `:38` (the Mini install line, `--interval 300`); `packaging/install.sh:19-20`; `docs/findings/CHORE-016-mini-deploy.md:11` (Mini's launchd unit published 18 rows) |
| nothing installed elsewhere, "Raw nats kv" | Captain 2026-10-09, `kanban-work/chores/CHORE-020-*.md:22-24` |
| key scheme (`api`, `subscription`, `local`; the slugs; no email) | `docs/DESIGN.md:31`, `:40-41`, `:47-48`; §3 `kv ls` |
| `.balance` is not a model | `docs/DESIGN.md:33-38` |
| `alert.*` is dedup state | `docs/DESIGN.md:260`, `:266-268` (crossing-dedup at `alert.<key>` in the same bucket); `kanban-work/chores/CHORE-020-*.md:45`; §3 `kv ls` (4 `alert.api.*` keys sit beside the 18 status keys) |
| absent → UNKNOWN; expired → UNKNOWN even if present; unreachable → CANNOT-ASSESS | `docs/DESIGN.md:64-68` |
| per-key TTL means an expired key is physically absent | `docs/DESIGN.md:72-73`; §3 `kv info`: "Per-Key TTL Supported: true" |
| the state words; `unknown` carries `cannot_assess:<why>` | `docs/DESIGN.md:51-52` |
| only `ok` is usable (`degraded` is not) | `kanban-work/chores/CHORE-020-*.md:43`; `docs/DESIGN.md:325-327` (the selector chooses "Only a fresh measured `ok`", "`degraded` included" among the refused) |
| `key not found`, rc 1 | §3 absent-key read |

DESIGN maps a written `unknown` to CANNOT-ASSESS (`docs/DESIGN.md:67`), not to UNKNOWN. The section lists it under
"not usable", and both labels mean the same for a reader choosing a model. The section does not merge the two labels.

## 3. Measurements (read-only)

Host M5 (`hostname -s` → `M5-MBP-2`). Date 2026-10-09, first command at 2026-10-09T18:15:11Z. quotabus `origin/main`
`a640fcc9290275eadc4595ebea78b836af154be4`. natscli against Mini's bus, read verbs only: no put, del, purge or create.

```
$ nats --timeout 5s --server nats://192.168.8.110:4222 kv ls ai_status        # rc=0, 22 keys; grouped, accounts masked:
   4 alert.api.<provider>.nusy-product-team.<model>
  10 api.<provider>.nusy-product-team.<model|balance>     (deepseek ×3 incl. balance, moonshot ×2 incl. balance,
                                                          openai, together, xai, zhipu ×2)
   1 local.qwen.dgx1.qwen3
   6 subscription.anthropic.<acct>.<svc>
   1 subscription.github.<acct>.<svc>
```

That is 18 status keys plus 4 `alert.*` keys. It matches the count the item recorded (`kanban-work/chores/CHORE-020-*.md:47`).

```
$ nats --timeout 5s --server nats://192.168.8.110:4222 kv get ai_status api.deepseek.nusy-product-team.deepseek-v4-flash --raw   # rc=0
{"contract":"ai-status/1","key":"api.deepseek.nusy-product-team.deepseek-v4-flash","kind":"api","provider":"deepseek",
 "account":"nusy-product-team","model":"deepseek-v4-flash","family":"deepseek","state":"ok","reason":null,
 "balance":{"amount":<masked>,"currency":"CNY","source":"https://api.deepseek.com/user/balance"},"headroom":null,
 "latency_ms":776,"probe":{"name":"messages","source":"official"},"error":null,
 "checked_at":"2026-10-09T15:13:50.575143Z","ttl_s":129600,"observed_by":"Mac-mini/quotabus@0.1.0"}
```

The row is wrapped here for width, and the balance amount is masked. `checked_at` + 129600 s (36 h) is after the read
time, so the row is fresh and its `ok` is usable.

```
$ nats --timeout 5s --server nats://192.168.8.110:4222 kv get ai_status api.nope.x.y --raw; echo rc=$?
nats: error: nats: key not found
rc=1
```

```
$ nats --timeout 5s --server nats://192.168.8.110:4222 kv info ai_status      # rc=0; excerpt
Information for Key-Value Store Bucket ai_status created 2026-10-08 18:08:54
            History Kept: 1
           Values Stored: 22
   Per-Key TTL Supported: true
             Description: quotabus: AI provider & account status (ai-status/1)
```

`which quotabus` on M5 printed "quotabus not found" (rc 1). M5 reads the bus only with natscli, which is how the
Captain's ruling has every non-Mini machine read it.

## 4. One thing the receiving session should know (not part of the ask)

nusy-product-team's `docs/external-review.md` §1a, landed from quotabus CHORE-001
(`aa270c1:docs/external-review.md:35-60`, last changed at `31f62c5dd7`), tells a reviewer to run `quotabus status` and
`quotabus select` (`:45-48`). Those commands need the binary. Under the "Raw nats kv" ruling, nothing installs that
binary off Mini. It is absent on M5, measured above. The other three machines have not been checked. On those
machines §1a's commands fail with "command not found". That is not a false `ok`. This packet does not ask for any
change to §1a. Whether §1a should gain a raw-read fallback, or a pointer to the new section, is the nusy-product-team
session's call. It may need the Captain, because §1a's `select` is the reviewer pick.

## 5. What comes back

Once both changes have landed on nusy-product-team `main`, reply on the chore with the landing sha (one sha, or one
per file). A quotabus session then cites that sha on CHORE-020 and closes it. Each machine's re-sync of
`~/.claude/CLAUDE.md` follows `docs/AGENT-MACHINE-CANON.md`'s *Re-sync recipe*. Until a machine re-syncs, its block
lacks the pointer. Nothing reports that drift, as the canon doc itself warns (`42c51bab04:docs/AGENT-MACHINE-CANON.md:17-25`).
