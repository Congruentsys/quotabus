---
yurtle: v2.1
type: quotabus-config
id: fleet/ai-status
---
# AI services the fleet holds

The Yurtle twin of [`quotabus.toml`](quotabus.toml): the same rows, readable in Obsidian and queryable by
`yurtle-rdflib` (docs/DESIGN.md §3 "Config — one model, two front-ends"; §10 Q7). `quotabus --config
quotabus.yurtle.md …` reads **only** the `yurtle-table` blocks below and ignores the rest of this file; the two files
load to the same config (`tests/exp004_config.rs`). The comments of `quotabus.toml` explain each value; they are not
repeated here.

A `secret` is the NAME of an environment variable; the value is injected by the launcher (`doppler run -- …` /
`secretspec run -- …`) and never written here. An empty cell is absence. A comma-separated cell is a list.

## Where rows go, and how often

```yurtle-table
@type Bus

| @id  | url                       | bucket    |
|------|---------------------------|-----------|
| #bus | nats://192.168.8.110:4222 | ai_status |
```

Each query kind on its own schedule (SIG-004); an empty `ttl` is 3 × that kind's interval.

```yurtle-table
@type Schedule

| @id           | interval | ttl |
|---------------|----------|-----|
| #api          | 12h      |     |
| #balance      | 12h      |     |
| #subscription | 1h       |     |
```

```yurtle-table
@type Probe

| @id    | max-tokens |
|--------|------------|
| #probe | 20         |
```

## Services

Model ids marked [unverified] in `quotabus.toml` (together, xai, local-qwen) were not confirmed against the provider.
`xai` is the standing negative control: its key is disabled, so it must read `auth_failed`. The undocumented
subscription sources are off (SIG-003). `local-qwen` is `probe = false` (CHORE-023): a root LaunchDaemon probes the
Sparks and this config only reads their rows (packaging/README.md).

```yurtle-table
@type Service

| @id             | kind         | provider  | family    | account           | base-url                           | protocol  | models                                  | roles        | cost-class | secret                    | sources     | balance-url                                 | balance-path                   | currency | floor | probe |
|-----------------|--------------|-----------|-----------|-------------------|------------------------------------|-----------|-----------------------------------------|--------------|------------|---------------------------|-------------|---------------------------------------------|--------------------------------|----------|-------|-------|
| #glm            | api          | zhipu     | zhipu     | nusy-product-team | https://api.z.ai/api/anthropic     | anthropic | glm-5.3, glm-5.2                        | review, work | metered    | NUSY_GLM                  |             |                                             |                                |          |       |       |
| #deepseek       | api          | deepseek  | deepseek  | nusy-product-team | https://api.deepseek.com/anthropic | anthropic | deepseek-v4-pro, deepseek-v4-flash      | review       | metered    | NUSY_DEEPSEEK             |             | https://api.deepseek.com/user/balance       | balance_infos[0].total_balance | CNY      | 150   |       |
| #kimi           | api          | moonshot  | moonshot  | nusy-product-team | https://api.moonshot.ai/anthropic  | anthropic | kimi-k3                                 | review       | metered    | NUSY_KIMI                 |             | https://api.moonshot.ai/v1/users/me/balance | data.available_balance         | USD      | 100   |       |
| #openai         | api          | openai    | openai    | nusy-product-team | https://api.openai.com/v1          | openai    | gpt-5.6-sol                             | review       | metered    | OPENAI_API_KEY            |             |                                             |                                |          |       |       |
| #together       | api          | together  | meta      | nusy-product-team | https://api.together.xyz/v1        | openai    | meta-llama/Llama-3.3-70B-Instruct-Turbo | review       | metered    | TOGETHER_API_KEY          |             |                                             |                                |          |       |       |
| #xai            | api          | xai       | xai       | nusy-product-team | https://api.x.ai/v1                | openai    | grok-4                                  |              | metered    | XAI_API_KEY               |             |                                             |                                |          |       |       |
| #local-qwen     | local        | qwen      | qwen      | dgx1              | http://192.168.8.180:30000/v1      | openai    | qwen3                                   | work         | local      | NUSY_LOCAL_QWEN           |             |                                             |                                |          |       | false |
| #claude-hankh95 | subscription | anthropic | anthropic | hankh95           | https://api.anthropic.com          |           | claude-haiku-4-5                        |              |            | NUSY_CLAUDE_TOKEN_HANKH95 | stream_json |                                             |                                |          |       |       |
| #copilot        | subscription | github    | openai    | hankh95           | https://api.github.com             |           |                                         |              |            | GITHUB_TOKEN              |             |                                             |                                |          |       |       |
```

## Alerts

`quotabus alert` files one `signal` per crossing (SIG-006), tagged `provider-status`, never auto-closed. A webhook
row (`#webhook`, its `url` cell) would add a JSON POST per crossing.

```yurtle-table
@type AlertSink

| @id            | command       | item-type | tags                   |
|----------------|---------------|-----------|------------------------|
| #nusy-kanban   | nusy-kanban   | signal    | provider-status, infra |
| #yurtle-kanban | yurtle-kanban | signal    | provider-status        |
```
