reviewed-at-sha: 48f74c53799ff763354c6f00d6d9e43757db849d
verdict: changes
gate: make check rc=0 at 48f74c53799ff763354c6f00d6d9e43757db849d on M5-MBP-2
brief: reviews/CHORE-022-r1.brief.md @ 48f74c53799ff763354c6f00d6d9e43757db849d

Authorship: LLM agent sessions (Claude Opus 5.5) did all of the work and this review. No human reviewed it.
"Distinct" means this review ran in a separate `claude -p` process with a fresh context that wrote none of the
work. It is the SAME model family as the author. The driver (the author) briefed it with the committed brief named
above. The commissioning session is M5-MBP-2/s-72a67d16. Its stake: it drove the work and wrote no tests or code.
Sub-agents wrote the tests and the code.

Host M5-MBP-2, 2026-10-10 (UTC), scratch worktree `/tmp/rv-CHORE-022` detached at the sha above,
`CARGO_TARGET_DIR=/tmp/qb-target-rv-CHORE-022`. Scratch files (a throwaway test file and a `[file]`-backend config)
were deleted before this commit. No bus was touched. No board or GitHub writes were made.

## Summary

The tests match the Definition of Done. The tests were not changed after the ruled edit. The mutations go red. The
per-cycle paths are honest, and the gate is green. One G1 defect blocks: a discovering service's old model row is
never superseded. When the box goes down or switches model, `select` keeps picking the old model as `ok` for up to
the 36 h TTL (F1). With a configured `models` list, the same scenario reads `cannot_assess:unreachable` and `select`
picks nothing. That makes F1 a regression that discovery introduced, and no test covers status or select on a
discovering service (F2).

## Findings

### F1 — BLOCKS. After a box goes down or switches model, its last served model still reads `ok` in `status` and `select`

`discover()` writes a failure row under the SERVICE id (`local.qwen.dgx1.dgx1-qwen`). The last served model's row
(`local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4`) is left as it was. `ServiceConfig::slots_in` (src/config.rs) reports every
model that has a row in the store. Each row then passes `freshness()` on its own, so the old `ok` row is still fresh
until `checked_at + ttl_s`. That is 129600 s (36 h) at the defaults. Freshness is applied to every row, but it does
not cover this case: no newer row ever lands on that key. Under a fixed `models` list, the down cycle overwrites
the same key with `unknown`.

Live, keyless, against the real DGX1, with a scratch config holding only a `[file]` backend in
`/tmp/rv-CHORE-022/scratch/store` and the two example services. Then the same config with dgx1's `base_url` pointed
at a closed port (`127.0.0.1:9`), to simulate the box going down. The account and keys stay the same:

```
$ env -i PATH=/usr/bin:/bin $B --config scratch/live.toml probe
local.qwen.dgx1.nvidia-Qwen3-32B-NVFP4 ok
local.qwen.dgx2.dgx2-qwen unknown (cannot_assess:unreachable)
published 2 rows to /tmp/rv-CHORE-022/scratch/store
probe rc=0
$ env -i PATH=/usr/bin:/bin $B --config scratch/down.toml probe --force
local.qwen.dgx1.dgx1-qwen unknown (cannot_assess:unreachable)
local.qwen.dgx2.dgx2-qwen unknown (cannot_assess:timeout)
published 2 rows to /tmp/rv-CHORE-022/scratch/store
probe rc=0
$ env -i PATH=/usr/bin:/bin $B --config scratch/down.toml status
SERVICE    MODEL                   STATE          AGE  SOURCE    DETAIL
dgx1-qwen  dgx1-qwen               CANNOT-ASSESS  5s   official  cannot_assess:unreachable
dgx1-qwen  nvidia/Qwen3-32B-NVFP4  ok             13s  official
dgx2-qwen  dgx2-qwen               CANNOT-ASSESS  5s   official  cannot_assess:timeout
status rc=0
$ env -i PATH=/usr/bin:/bin $B --config scratch/down.toml select --role work
qwen nvidia/Qwen3-32B-NVFP4
select rc=0
$ grep -ho '"ttl_s":[0-9]*' scratch/store/* | sort | uniq -c
   3 "ttl_s":129600
```

The box is down, yet `select` hands out `qwen nvidia/Qwen3-32B-NVFP4` with rc 0. That is a false `ok` to the
consumer, which DESIGN §2 forbids. (`status --check dgx1-qwen` does exit 2, because the worst slot wins. `select`
has no such guard.)

The same result in-process, set against the configured-list control. A scratch test (`tests/zz_rv_scratch.rs`,
since deleted) ran `Runner::run_once` against a wiremock stub serving `A`, then against a closed port. It merged
the two cycles' rows newest-per-key, as the store holds them, and called `quotabus::select`:

```
$ cargo test -q --test zz_rv_scratch -- --nocapture --test-threads=1
models=""
  cycle1: [("local.qwen.dgx1.A", "ok", None)]
  cycle2 (box down): [("local.qwen.dgx1.dgx1", "unknown", Some("cannot_assess:unreachable"))]
  select after: ["A local.qwen.dgx1.A"]
models="models = [\"A\"]\n"
  cycle1: [("local.qwen.dgx1.A", "ok", None)]
  cycle2 (box down): [("local.qwen.dgx1.A", "unknown", Some("cannot_assess:unreachable"))]
  select after: []
.switch A->B, select after: ["A local.qwen.dgx1.A", "B local.qwen.dgx1.B"]
```

The switch case has the same cause. After `spark-model switch` A→B, the probe has just listed B only, yet A stays
selectable for 36 h. A request routed to A would then 404.

A fix is the implementer's call. Two directions:
(a) In `probe_service`, when the list fails, write the failure row under every model slot the store holds for the
service, not only the service id. After a successful list, write a non-`ok` row (for example `unknown`,
`cannot_assess:not_served`; not `model_missing`, per DoD 1) for each previously seen id that is no longer listed.
(b) In `slots_in`, or in a shared helper that status and select both use, report only the slots from the
service's NEWEST cycle: the rows whose `checked_at` is not older than the service's newest list or failure row.
Option (a) keeps the readers unchanged and keeps the store's own picture honest for `serve`/`alert`. Whichever is
chosen needs a red-first test: down-after-up and switch, read through `select` (and `status`).

### F2 — BLOCKS (it goes with F1). No test covers the `status`/`select` change (`slots_in`)

```
$ grep -ln "slots_in\|quotabus::select\|\"select\"\|\"status\"" tests/chore022_*.rs; echo "grep rc=$?"
grep rc=1
```

src/main.rs and src/select.rs each change one line (`slots()` → `slots_in(rows)`). That change is minimal, and it is
correct as far as it goes: without it, a discovering service has no slots and is never reported. But the item's
tests never exercise it, so F1 went unseen. The DoD does not name status/select explicitly, so this is not a gap
in the tests against the body. It is a gap in the coverage of the code the PR changed. The F1 fix should come with
these tests.

### F3 — does not block. The tests in T and in the ruled edit match the Definition of Done and DESIGN §4

DoD 1 is covered by `a_local_service_without_models_probes_the_model_v1_models_serves`,
`every_served_model_gets_its_own_row`, `a_configured_models_list_is_probed_as_configured_not_discovered` and
`an_unreachable_box_without_models_is_one_unreachable_row` (plus two-box and mixed variants). DoD 2 is covered by
`tests/chore022_example_design.rs` (example and §4 row, each with a stale-value control) and by 4d18e54's
`examples_config.rs` (both URLs, no secret, no models, exactly two local services, a secret control). DoD 3 is covered
by `a_switched_served_model_reads_the_new_id_and_no_model_missing`, `no_request_to_a_keyless_local_service_carries_authorization`
(it asserts that both the GET and the POST happened before it checks the header) and the unreachable tests. Each
check has a named control: `control_a_fixed_models_list_after_a_switch_does_read_model_missing`,
`control_a_local_service_with_a_key_does_send_authorization` and
`control_an_unreachable_box_with_two_configured_models_is_two_rows`. Tests ran green in the gate (below):

```
     Running tests/chore022_example_design.rs (...)
test result: ok. 5 passed; 0 failed; ...
     Running tests/chore022_local_discovery.rs (...)
test result: ok. 12 passed; 0 failed; ...
```

### F4 — does not block. The test files are unchanged since the ruled edit

```
$ git diff 4d18e54..HEAD -- tests/chore022_local_discovery.rs tests/chore022_example_design.rs tests/examples_config.rs | wc -l
       0
```

### F5 — does not block. Mutations go red, and the code does not special-case the tests

Each mutation was applied, run, and then restored with `git checkout`. The final `git status --short` shows no
changes under src/.

```
== M1: discovers() always false
 src/config.rs | 2 +-
test result: FAILED. 5 passed; 7 failed; ...
== M2: Authorization on the list request whatever the key
 src/probe.rs | 2 +-
test result: FAILED. 11 passed; 1 failed; ...
test no_request_to_a_keyless_local_service_carries_authorization ... FAILED
== M3: unreachable discovery writes nothing (no row under the service id)
 src/probe.rs | 2 +-
test result: FAILED. 9 passed; 3 failed; ...
```

Reading `git show c66572d -- src/` turned up no test names, stub ports, fixture model ids or other test-only
branches. Discovery is keyed only on `kind == Local && models.is_empty()`.

### F6 — does not block. The per-cycle G1 paths are honest

The scratch test against a wiremock stub, one discovering service, prints `(key, state, reason, error)`:

```
list 404 "not found" chat 200: [("local.qwen.dgx1.dgx1", "unknown", Some("cannot_assess:not_found"), Some("not found"))]
list 200 "{\"data\":[]}" chat 200: [("local.qwen.dgx1.dgx1", "unknown", Some("cannot_assess:no_model_served"), None)]
list 200 "<html>nope</html>" chat 200: [("local.qwen.dgx1.dgx1", "unknown", Some("cannot_assess:bad_model_list"), Some("<html>nope</html>"))]
list 200 "{\"data\":[{\"name\":\"x\"}]}" chat 200: [("local.qwen.dgx1.dgx1", "unknown", Some("cannot_assess:no_model_served"), None)]
list 200 "{\"data\":[{\"id\":\"A\"}]}" chat 500: [("local.qwen.dgx1.A", "unknown", Some("cannot_assess:server_error"), Some("{\"error\":\"boom\"}"))]
list 200 "{\"data\":[{\"id\":\"A\"}]}" chat 404: [("local.qwen.dgx1.A", "unknown", Some("cannot_assess:not_found"), Some("{\"error\":\"boom\"}"))]
list 401 "unauthorized" chat 200: [("local.qwen.dgx1.dgx1", "auth_failed", None, Some("unauthorized"))]
```

No path within one cycle yields `ok`. A 404 on the list is `cannot_assess:not_found`, never `model_missing`. An
unreachable box writes one row and invents no model row. The false `ok` arises only across cycles (F1).

### F7 — does not block. The real DGX1 is discovered and probed. A down DGX2 can read `timeout` rather than `unreachable`

```
$ curl -s --max-time 5 http://192.168.8.120:8000/v1/models
{"object":"list","data":[{"id":"nvidia/Qwen3-32B-NVFP4","object":"model",...,"owned_by":"vllm",...}]} curl rc=0
$ curl -s --max-time 5 http://192.168.8.121:8000/v1/models
 curl rc=28
```

In F1's live run, DGX2 read `cannot_assess:unreachable` on the first cycle and `cannot_assess:timeout` on the
second (`--force`). A box with no route answers by timeout rather than with a refused connection. Both readings are
honest, non-`ok` and one row per service. But "unreachable" in DoD 1 and DESIGN §4 describes what a down box usually
reads, not what it always reads. Worth a word in §4. It is not a defect.

### F8 — does not block. The secret rules hold

```
$ git diff origin/main...HEAD | grep -nE "sk-[A-Za-z0-9]{10,}|Bearer [A-Za-z0-9]{12,}|ghp_|xai-[A-Za-z0-9]{10,}"; echo "grep rc=$?"
grep rc=1
$ grep -n "FAKE_SK" tests/common/mod.rs
14:pub const FAKE_SK: &str = "sk-test-not-a-key-0123456789abcdef";
```

The only key in the tests is the shared fake. `discover()` sends a key only when one is configured, and wraps it in
`sensitive_str`. Error bodies go through `redactor.redact_error`, and reasons through `redactor.redact`. The example
drops `NUSY_LOCAL_QWEN`. No key is read from argv.

### F9 — does not block. Gate

```
$ make check > /tmp/rv-CHORE-022-check.log 2>&1; echo rc=$?
rc=0
$ grep -c "test result: ok" /tmp/rv-CHORE-022-check.log
48
```

## Verdict

**changes.** Fix F1 and add the F2 tests (red first, written by the test partner from this finding). Everything
else is ready.
