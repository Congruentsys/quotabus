---
id: HAZ-002
title: "Throwaway nats-server test fixture races under the parallel suite (connection reset at startup)"
type: hazard
status: backlog
priority: medium
assignee: null
created: 2026-10-08
depends_on: []
---

# Throwaway nats-server test fixture races under the parallel suite (connection reset at startup)

Seen on M5 while landing HAZ-001 (2026-10-08): one full `make check` run failed in `kv_publisher::a_short_ttl_row_is_absent_after_expiry_and_a_long_one_survives` at `tests/kv_publisher.rs:70` with `Unreachable("IO error: Connection reset by peer (os error 54)")` while connecting to the test's throwaway nats-server. The same test passed 10/10 alone on the branch and 10/10 alone on `main`, and the full gate passed on 3 re-runs, so it is a race under the parallel suite. The likely cause is that the fixture in `tests/common/mod.rs` hands back the server before it accepts connections, or two servers get the same free port. It is not proved.

A flaky gate trains sessions to re-run until green, and a real failure then gets re-run away.

## Done when
The throwaway-server helper waits until the server really accepts a connection (or retries a refused or reset first connect within a bound), and keeps a port it hands out. The full `cargo test --locked` passes 10 runs in a row on M5. This is a test-only change, made by a test partner and reviewed like code.