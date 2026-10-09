---
id: HAZ-004
title: "packaging README's fleet command names user 'admin' and /usr/local paths that do not exist on Mini"
type: hazard
status: underway
priority: medium
assignee: M5-MBP-2/s-72a67d16
created: 2026-10-09
depends_on: []
---

# packaging README's fleet command names user 'admin' and /usr/local paths that do not exist on Mini

Found while preparing CHORE-016. The fleet command in `packaging/README.md` (§ The fleet: Mini) is `install.sh --host mini --user admin --home /Users/admin … --quotabus /usr/local/bin/quotabus --probe-config /usr/local/etc/quotabus/quotabus.toml`. It names a user, a home and two directories that do not exist on Mini.

Measured from M5 on 2026-10-09, quotabus fed0654, read-only over `ssh mini` (the ~/.ssh/config Host mini has User hankh19):
- `ssh admin@192.168.8.110` → `Permission denied (publickey,password,keyboard-interactive)`
- `ls -d /Users/*` → `/Users/hankh19 /Users/Shared`; `id` → `uid=501(hankh19)`, which is in group 80(admin)
- `ls -ld /usr/local/bin /usr/local/etc` → `No such file or directory` for both. Creating them needs sudo.
- `/opt/homebrew/bin/doppler` → present (3.75.2). `~/.cargo/bin/cargo` is present, and a checkout is at `~/Projects/quotabus`.

Run as written, the command fails at SSH. Run with the right user, it would install a unit pointing at a binary path that cannot exist without sudo.

**Decided (G3, no sudo, user-owned paths; reversible):** the fleet install uses `--user hankh19 --home /Users/hankh19`, the binary goes to `/Users/hankh19/.local/bin/`, and the config goes to `/Users/hankh19/.config/quotabus/quotabus.toml`.

## Done when
1. `packaging/README.md`'s fleet command (and the `quotabus-cycle` wrapper's paths) uses the measured user, home and user-owned paths above. Any other doc that repeats them (DESIGN §5, the wiring note, CHORE-008's tests or fixtures) is brought in line. A test that pins the old command is changed only as a ruled test edit, by the test partner.
2. `install.sh --render-to <dir>` run with the corrected command renders a plist whose ProgramArguments and log paths are all under `/Users/hankh19`; the output is quoted in the PR.
3. `make check` is green.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-09T02:59:27+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-09T02:59:37+00:00"^^xsd:dateTime ;
    kb:by "M5-MBP-2/s-72a67d16" ;
  ] .
```
