---
id: HAZ-001
title: "nats:// URL credentials are dropped, and could leak into a connection error (found in EXP-001)"
type: hazard
status: underway
priority: medium
assignee: M5/s-b1fd4c67
created: 2026-10-08
depends_on: []
---

# nats:// URL credentials are dropped, and could leak into a connection error (found in EXP-001)

Found while landing EXP-001 (PR #2, noted by the test partner and the implementer). A bus URL with credentials, `nats://user:pass@host:4222` (or `QUOTABUS_NATS_URL` with them), is passed to async-nats without the user/password, so a bus that needs auth reads CANNOT-ASSESS (bus unreachable) — and the connection-error text could carry the URL, password included, which the Redactor does not know as a secret.

Mini's bus needs no credentials today, so nothing is broken in the fleet; an outsider with an authenticated bus is.

## Done when
Credentials in a nats:// URL reach the connection (or are refused with a clear config error telling the user to use `--creds` / env), and no error, log or row ever contains the password: a test against a throwaway nats-server with a user/password that goes red without the fix.

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

<> kb:statusChange [
    kb:status kb:ready ;
    kb:at "2026-10-08T18:40:20+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ],
  [
    kb:status kb:in_progress ;
    kb:at "2026-10-08T18:40:47+00:00"^^xsd:dateTime ;
    kb:by "M5/s-b1fd4c67" ;
  ] .
```


## Comments

### M5/s-b1fd4c67 (2026-10-08 18:44)

tests red at 77629d0 (partner): 4 of 7 red on assertions — creds in [bus] url and in QUOTABUS_NATS_URL never reach the connection (authorization violation); and async-nats at debug/trace level logs the server address WITH the password, so a wrong-password or down-bus run leaks it to stderr. 3 controls pass (server accepts qbuser and refuses anonymous; no creds => CANNOT-ASSESS rc 2; leak checker catches the password).
