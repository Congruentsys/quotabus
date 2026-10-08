# EXP-001 finding — a per-key TTL put expires on Mini's nats-server 2.12.4

**Answer: yes.** A bucket created with `--marker-ttl` reports `Per-Key TTL Supported: true`; a key written with a
5 s TTL was readable at +2 s and absent ("key not found") at +8 s, while a key written without a TTL survived. The
design's two-layer freshness rule (DESIGN.md §2) can rely on the bucket layer on this server, and still keeps
`checked_at` + `ttl_s` in the value for readers on servers without it.

Authorship: measured and written by an LLM agent session (Claude Opus 5.5, M5/s-b1fd4c67, the EXP-001 driver); no human
reviewed it. It is reviewed with EXP-001's PR by a distinct `claude -p` session of the same model family.

## How it was measured
- **When / where:** 2026-10-08T17:31:52Z–17:32:04Z, from M5 (macOS, natscli **0.3.1**) against
  `nats://192.168.8.110:4222` (Mini), **nats-server 2.12.4** (read from the server's INFO line, and `nats account info`
  "Connected Server Version: 2.12.4"). quotabus repo at `origin/main` `dd12fbf`.
- **Shared-bus rule:** the script touched ONLY a bucket it created, `qb_measure_exp001`, and deleted it at the end
  (`nats kv ls --names | grep -c '^qb_measure_exp001$'` → `0`). No other bucket or stream was read for writing.
- **Raw output:** `~/.local/state/quotabus-work/EXP-001/ttl-measure.out` on M5 (sha256 prefix
  `4b80f0000f25fd7e`); the script beside it, `ttl-measure.sh`. The commands, in order:

```bash
export NATS_URL=nats://192.168.8.110:4222; B=qb_measure_exp001
nats kv add $B --history=1 --marker-ttl=2s --description "quotabus EXP-001 per-key TTL measurement (temporary)"
nats kv info $B                                      # Per-Key TTL Supported: true; Limit Marker TTL: 2.00s
nats kv create $B ttl.probe '{"v":"expires in 5s"}' --ttl=5s
nats kv put $B plain.key '{"v":"no ttl"}'
nats kv get $B ttl.probe --raw                       # {"v":"expires in 5s"}  rc=0
nats stream info KV_$B --json                        # config: allow_msg_ttl=True, subject_delete_marker_ttl=2000000000, max_msgs_per_subject=1
sleep 2; nats kv get $B ttl.probe --raw              # {"v":"expires in 5s"}  rc=0 at +2s
sleep 6; nats kv get $B ttl.probe --raw              # nats: error: nats: key not found   rc=1 at +8s
nats kv get $B plain.key --raw                       # {"v":"no ttl"}  rc=0 at +8s
nats kv ls $B                                        # plain.key
sleep 4; nats kv ls $B                               # plain.key   (after the 2 s marker TTL)
nats stream info KV_$B --json                        # state: messages=1, first_seq=2, last_seq=3
nats kv del $B --force
```

## What it changes
1. **natscli 0.3.1 has no `kv put --ttl`.** The design (§3 Outputs) names `nats kv put --ttl=<d>`; in 0.3.1 the
   flag is on `kv create` ("Sets a TTL for the key"), and `kv put --help` lists none. The quotabus binary writes rows
   with the client library, so this only corrects the operator's hand-written recipe: a hand put with a TTL is
   `nats kv create` (which fails when the key exists) — or a delete then create.
2. **The bucket needs `allow_msg_ttl`**, which natscli sets from `--marker-ttl` (stream config above). quotabus
   creates its bucket the same way (`limit_markers` in async-nats's `kv::Config`, server 2.11+); an existing bucket
   created without it (every pre-existing bucket on Mini, per DESIGN.md §3) cannot be relied on to expire keys.
3. **Nothing of the expired key is left in the stream** once the 2 s marker TTL has also passed: the state shows
   `messages=1, first_seq=2` — the plain key (seq 2) alone; seq 1 (the TTL key) and seq 3 are gone. [INFERENCE] seq 3
   is the server's delete marker for the expired key, which a `watch`er would see as a delete entry while it lives;
   a reader treats it as ABSENT (the freshness rule's first line), never as a value. Not measured with a watcher.
