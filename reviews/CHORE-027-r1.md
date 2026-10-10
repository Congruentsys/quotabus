reviewed-at-sha: bde447adcdea3edd3a8d150b38a04826e4772a3d
verdict: approve
gate: make check rc=0 at bde447adcdea3edd3a8d150b38a04826e4772a3d on M5-MBP-2
brief: reviews/CHORE-027-r1.brief.md @ bde447adcdea3edd3a8d150b38a04826e4772a3d

Authorship: All the work and this review were done by LLM agent sessions (Claude Opus 5.5). No human reviewed it.
"Distinct" means a separate `claude -p` process with a fresh context that wrote none of the work, of the SAME model
family, briefed by the driver (the author) with this committed brief. The commissioning session is
M5-MBP-2/s-72a67d16. Its stake: it ran the redeploy and wrote the finding.

Scope: PR #36 (`measure/CHORE-027-mini-redeploy`), `docs/findings/CHORE-027-mini-redeploy.md` against CHORE-027's
Definition of Done. Every command below was re-run read-only, in the foreground, on 2026-10-10 between 20:37:39Z and
20:38Z, from M5-MBP-2 (bus reads) and over `ssh hankh19@192.168.8.110` (Mini). Nothing was probed, bootstrapped,
written or deleted. No board or GitHub write was made.

## Gate

```
$ make check > /tmp/rv-CHORE-027-make.log 2>&1; echo rc=$?
rc=0
(tail) test result: ok. 6 passed; 0 failed; ... Doc-tests quotabus ... 0 passed; 0 failed
```

## F1 — DoD 1 (built from a recorded sha; previous binary and config kept, dated). Holds. Does not block.

```
$ ~/.local/bin/quotabus --version
quotabus 0.1.0
$ ls -la ~/.local/bin/quotabus* ~/.config/quotabus/
-rwxr-xr-x@ 1 hankh19 staff 12193104 Oct 10 20:28 /Users/hankh19/.local/bin/quotabus
-rwxr-xr-x@ 1 hankh19 staff 12202960 Oct  9 18:14 /Users/hankh19/.local/bin/quotabus.prev-20261010
-rw-r--r--  1 hankh19 staff 6973 Oct 10 20:28 quotabus.toml
-rw-r--r--@ 1 hankh19 staff 6861 Oct  9 18:23 quotabus.toml.prev-20261010
(plus older quotabus.prev-20261009, quotabus-cycle(.prev-20261010), quotabus.toml.bak-20261009)
$ git -C ~/Projects/quotabus rev-parse origin/main
deeb435fbba833cb09604d3783b9739c4ed680b4
```

The installed binary is new (20:28, a different size from the 2026-10-09 18:14 one, which is kept as
`.prev-20261010` as the finding says). Mini's `origin/main` is still `deeb435`, the recorded build sha. The version
string cannot tie the binary to a sha (it is `0.1.0` on both), so provenance rests on the recorded build commands.
That is all the DoD asks for.

```
$ diff ~/.config/quotabus/quotabus.toml.prev-20261010 ~/.config/quotabus/quotabus.toml
121c121
< id         = "local-qwen"   # DGX1 ...
> id         = "dgx1-qwen"    # DGX1 ... no key, no models: probes what /v1/models serves (CHORE-022, CHORE-027)
128d127
< models     = ["nvidia/Qwen3-32B-NVFP4"]  ...
131d129
< # no secret: kind = "local" is probed without a key (src/probe.rs Auth::None)
132a131,141
> [[service]]  id = "dgx2-qwen", kind = "local", provider = "qwen", account = "dgx2",
>              base_url = "http://192.168.8.121:8000/v1", protocol = "openai", roles = ["work"], cost_class = "local"
```
(abridged to one line per field.) Only the local-service block changed, as claimed. Neither service has `secret` or
`models`.

## F2 — DoD 2 (next REGULAR tick exits 0, new alert form, M ≥ 1). Holds. Does not block.

```
$ grep '^alert:' ~/Library/Logs/quotabus/probe.out.log | tail -3
alert: 16 keys checked, 0 crossings filed, 0 re-armed
alert: 16 keys checked, 0 crossings filed, 0 re-armed
alert: 15 keys checked, 2 local skipped, 0 crossings filed, 0 re-armed
$ launchctl print gui/501/com.congruentsys.quotabus-probe | grep -E 'runs|last exit code'
	runs = 496
	last exit code = 0
```

This is identical to the finding. No newer tick had run at 20:37:39Z (the 300 s tick after #496 was not yet due), so
#496 is still the latest. M = 2 ≥ 1.

## F3 — DoD 3 (status: DGX1 keyed by the served id, DGX2's real state, every API/subscription row present and fresh). Holds. Does not block.

```
$ ~/.local/bin/quotabus status --config ~/.config/quotabus/quotabus.toml      (rc=0; DETAIL column abridged)
glm        glm-5.3                                  ok               8m  official
glm        glm-5.2                                  ok               8m  official
deepseek   deepseek-v4-pro                          ok               8m  official
deepseek   deepseek-v4-flash                        ok               8m  official
kimi       kimi-k3                                  quota_exhausted  8m  official
openai     gpt-5.6-sol                              auth_failed      8m  official
together   meta-llama/Llama-3.3-70B-Instruct-Turbo  auth_failed      8m  official
xai        grok-4                                   auth_failed      8m  official
dgx1-qwen  nvidia/Qwen3-32B-NVFP4                   ok               8m  official
dgx2-qwen  dgx2-qwen                                CANNOT-ASSESS    8m  official  cannot_assess:timeout
claude-<6 accounts>                                 ok               8m  undocumented
copilot    copilot                                  ok               8m  undocumented
$ ~/.local/bin/quotabus select --config ~/.config/quotabus/quotabus.toml --role work --prefer cheapest
qwen nvidia/Qwen3-32B-NVFP4
```

Every value matches the finding. Ages are 8m against its 4m, which fits a read four minutes later. DGX1 is keyed by
the served id, not `local-qwen`.

Wording nit (non-blocking): step 4 says "10 api rows" and step 6 says "8 API rows". Both are right. The one-shot's
`once.out.log` (re-read: `cat ~/.local/state/quotabus-work/CHORE-027/once.out.log`, 19 lines + `published 19 rows`)
has 10 `api.*` keys, the 8 model rows plus `api.deepseek…balance` and `api.moonshot…balance`. `status` shows 8 API
rows. One clause saying so would make 8 + 7 + 2 = 17 vs 19 add up for a reader.

## F4 — DoD 4 (old local rows age out by TTL; hours remaining), including the correction. Holds; the correction is right. Does not block.

```
$ nats --server nats://192.168.8.110:4222 kv get ai_status <key> --raw | (python: state, reason, checked_at, ttl_s, checked_at+ttl_s)
local.qwen.dgx1.qwen3                   unknown cannot_assess:secret_unset 2026-10-09T18:19:29Z 129600 expires 2026-10-11T06:19:29Z left_h=9.7
local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4  ok      -                         2026-10-10T20:28:50Z 129600 expires 2026-10-12T08:28:50Z left_h=35.9
local.qwen.dgx2.dgx2-qwen               unknown cannot_assess:timeout      2026-10-10T20:28:50Z 129600 expires 2026-10-12T08:28:50Z left_h=35.9
```

`local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4` was rewritten `ok` at 20:28:50Z, the forced cycle's timestamp, and it is the
row behind `status`'s `dgx1-qwen` line. Discovery keys DGX1 by the id it serves, so this is the live key and not an
old row. The item body was wrong to list it, and the finding's correction stands. `local.qwen.dgx1.qwen3` is the only
stale local row. It had 9.7 h left at 20:37Z (the finding said 9.8 h a few minutes earlier). `kv ls` shows no purge.

## F5 — an un-mentioned leftover alert-state row for a local key. Does not block.

```
$ nats --server nats://192.168.8.110:4222 kv ls ai_status | sort      (local-related lines)
alert.local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4
local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4
local.qwen.dgx1.qwen3
local.qwen.dgx2.dgx2-qwen
$ nats ... kv get ai_status alert.local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 --raw | (python: fields)
{'key': 'local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4', 'state': 'ok', 'at': '2026-10-10T13:26:51.305370Z'}
fields: ['at', 'contract', 'key', 'state']
```

The old binary wrote this alert-arm row before CHORE-021 (13:26Z, before the redeploy). The new binary skips local
keys in `alert`, so nothing will update it again. Unlike the status rows it has no `ttl_s`, so whether it ever leaves
depends on the bucket's own max-age, which I did not read. It is harmless: its state is `ok` (armed), and no local
key is alerted on any more. But the finding's "Old local rows" section says it accounts for what the swap leaves
behind, and this row is not in it. Suggested: one sentence naming the row and saying whether the bucket's TTL removes
it or it stays until a deliberate, item-sanctioned cleanup. Rule 4 still forbids deleting it ad hoc.

## F6 — DoD 5 and the secret rules. Holds. Does not block.

```
$ grep -nEi '@[a-z0-9-]+\.[a-z]|bearer|authorization|x-api-key|sk-[a-z0-9]|api[_-]?key *[=:]|token *[=:]|[A-Za-z0-9_-]{32,}' \
    docs/findings/CHORE-027-mini-redeploy.md <PR #36 title+body>
docs/findings/CHORE-027-mini-redeploy.md:16: ... deeb435fbba833cb09604d3783b9739c4ed680b4 ...
docs/findings/CHORE-027-mini-redeploy.md:28: deeb435fbba833cb09604d3783b9739c4ed680b4
PR body: no hits
```

The only hits are the git sha. There is no key, auth header or email in the finding or the PR. The finding records
its commands, their redacted output, the host (Mini, `gui/501`), the date and the build sha, which is what DoD 5
requires. The doppler command line in step 4 names a project and config, not a secret.

## Verdict

approve. All five DoD lines hold on re-measurement. F3's wording and F5's leftover alert row are optional follow-ups,
and neither blocks.
