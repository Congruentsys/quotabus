//! EXP-004 Plan 2 / Definition of Done "the Yurtle and TOML examples load to identical configs"; DESIGN §3 "Config —
//! one model, two front-ends" (the Yurtle twin is the same rows as a `yurtle-table` block, Yurtle v2.1; the binary
//! reads ONLY that block type and ignores the rest of the file) and the `[alert.*]` tables; §10 Q7 ("TOML first,
//! Yurtle in E4") and Q8 ("Signal per crossing", tag `provider-status`).
//!
//! Seams asserted (EXP-004 test partner):
//! - `Config::from_yurtle_str(&str) -> Result<Config, ConfigError>` reads a Yurtle markdown file's `yurtle-table`
//!   block(s); `Config::load(path)` reads a path ending in `.md` as Yurtle, anything else as TOML.
//! - `Config.alert: AlertConfig { nusy_kanban, yurtle_kanban: Option<KanbanSink>, webhook: Option<WebhookSink> }`,
//!   `KanbanSink { command, item_type, tags }`, `WebhookSink { url }`, from `[alert.nusy-kanban]`,
//!   `[alert.yurtle-kanban]`, `[alert.webhook]`.
//! - The Yurtle column names are the ones DESIGN §3 shows (`@id` = `#<service id>`, `base-url`, `balance-url`,
//!   `balance-path`, `currency`, `floor`; a comma-separated cell is a list; an empty cell is absence). Columns the
//!   design does not show are the implementer's choice and are only checked through the examples' equality.

use std::path::{Path, PathBuf};

use quotabus::config::{AlertConfig, BalanceSpec, KanbanSink, WebhookSink};
use quotabus::{Config, Kind};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn toml_example() -> Config {
    Config::load(&examples().join("quotabus.toml")).expect("examples/quotabus.toml loads")
}

fn yurtle_example_text() -> String {
    std::fs::read_to_string(examples().join("quotabus.yurtle.md"))
        .expect("examples/quotabus.yurtle.md exists (EXP-004 Plan 2: examples/ with both)")
}

// ── the Definition of Done: the two examples load to identical configs ──────────────────────────────────────────

#[test]
fn the_yurtle_and_toml_examples_load_to_identical_configs() {
    let toml = toml_example();
    let yurtle = Config::load(&examples().join("quotabus.yurtle.md"))
        .expect("examples/quotabus.yurtle.md loads through Config::load");
    // the comparison is not vacuous: the TOML example is the fleet's whole config
    assert!(
        toml.services.len() >= 9,
        "the TOML example holds every service"
    );
    assert!(
        toml.bus.is_some(),
        "the TOML example names the bus, so the twin must too"
    );
    assert_eq!(
        yurtle, toml,
        "the Yurtle twin must load to the SAME Config as the TOML"
    );
}

#[test]
fn the_yurtle_example_is_a_yurtle_v2_1_file_with_a_yurtle_table_block() {
    let text = yurtle_example_text();
    assert!(text.starts_with("---\n"), "Yurtle frontmatter first");
    assert!(text.contains("yurtle: v2.1"), "declares Yurtle v2.1");
    assert!(
        text.contains("```yurtle-table"),
        "the rows are a yurtle-table block"
    );
    assert!(text.contains("@type"), "the block declares its @type");
}

/// A Yurtle example whose kimi row differs from the TOML in one cell.
fn mutated(text: &str, row: &str, from: &str, to: &str) -> String {
    let mut hit = 0;
    let out: Vec<String> = text
        .lines()
        .map(|l| {
            if l.trim_start().starts_with('|') && l.contains(row) && l.contains(from) {
                hit += 1;
                l.replacen(from, to, 1)
            } else {
                l.to_string()
            }
        })
        .collect();
    assert_eq!(
        hit, 1,
        "the example has exactly one table row for {row} containing {from:?}"
    );
    out.join("\n") + "\n"
}

#[test]
fn control_a_differing_yurtle_row_is_detected() {
    let toml = toml_example();
    let text = yurtle_example_text();
    // one cell of one row: kimi's model id
    let changed = mutated(&text, "#kimi", "kimi-k3", "kimi-k9");
    assert_ne!(changed, text);
    let c = Config::from_yurtle_str(&changed).expect("the mutated twin still loads");
    assert_ne!(
        c, toml,
        "a differing model cell must make the configs differ"
    );
    let kimi = c
        .services
        .iter()
        .find(|s| s.id == "kimi")
        .expect("kimi row");
    assert_eq!(kimi.models, ["kimi-k9"], "the reader took the changed cell");

    // one numeric cell: deepseek's floor
    let changed = mutated(&text, "#deepseek", "| 150", "| 151");
    let c = Config::from_yurtle_str(&changed).expect("the mutated twin still loads");
    assert_ne!(
        c, toml,
        "a differing floor cell must make the configs differ"
    );
}

#[test]
fn control_a_dropped_yurtle_row_is_detected() {
    let toml = toml_example();
    let text = yurtle_example_text();
    let dropped: String = text
        .lines()
        .filter(|l| !(l.trim_start().starts_with('|') && l.contains("#together")))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(dropped, text, "the example has a #together row to drop");
    let c = Config::from_yurtle_str(&dropped).expect("the shortened twin still loads");
    assert_eq!(c.services.len() + 1, toml.services.len());
    assert_ne!(c, toml);
}

// ── the reader itself, against a known answer independent of the examples ──────────────────────────────────────

/// DESIGN §3's own Yurtle example rows (glm, deepseek, claude-hankh95), wrapped in prose, a decoy plain markdown table,
/// a plain `yurtle` block and a `toml` block — all of which the reader must ignore.
const DESIGN_ROWS: &str = r#"---
yurtle: v2.1
type: quotabus-config
id: test/ai-status
---
# AI services the fleet holds

Prose before the block. A plain markdown table is NOT config:

| @id     | kind | provider |
|---------|------|----------|
| #decoy  | api  | decoy    |

```yurtle
@prefix kb: <https://yurtle.dev/kanban/> .
<> kb:note "a plain yurtle block, not a yurtle-table" .
```

```toml
[[service]]
id = "decoy-toml"
```

```yurtle-table
@type Service

| @id             | kind         | provider  | family    | account           | base-url                           | models           | roles        | secret                    | balance-url                           | balance-path                   | currency | floor |
|-----------------|--------------|-----------|-----------|-------------------|------------------------------------|------------------|--------------|---------------------------|---------------------------------------|--------------------------------|----------|-------|
| #glm            | api          | zhipu     | zhipu     | nusy-product-team | https://api.z.ai/api/anthropic     | glm-5.3, glm-5.2 | review, work | NUSY_GLM                  |                                       |                                |          |       |
| #deepseek       | api          | deepseek  | deepseek  | nusy-product-team | https://api.deepseek.com/anthropic | deepseek-v4-pro  | review       | NUSY_DEEPSEEK             | https://api.deepseek.com/user/balance | balance_infos[0].total_balance | CNY      | 150   |
| #claude-hankh95 | subscription | anthropic | anthropic | hankh95           | https://api.anthropic.com          | claude-haiku-4-5 |              | NUSY_CLAUDE_TOKEN_HANKH95 |                                       |                                |          |       |
```

Prose after the block.
"#;

/// The same rows as TOML.
const DESIGN_ROWS_TOML: &str = r#"
[[service]]
id       = "glm"
kind     = "api"
provider = "zhipu"
family   = "zhipu"
account  = "nusy-product-team"
base_url = "https://api.z.ai/api/anthropic"
models   = ["glm-5.3", "glm-5.2"]
roles    = ["review", "work"]
secret   = "NUSY_GLM"

[[service]]
id       = "deepseek"
kind     = "api"
provider = "deepseek"
family   = "deepseek"
account  = "nusy-product-team"
base_url = "https://api.deepseek.com/anthropic"
models   = ["deepseek-v4-pro"]
roles    = ["review"]
secret   = "NUSY_DEEPSEEK"
[service.balance]
url      = "https://api.deepseek.com/user/balance"
path     = "balance_infos[0].total_balance"
currency = "CNY"
floor    = 150

[[service]]
id       = "claude-hankh95"
kind     = "subscription"
provider = "anthropic"
family   = "anthropic"
account  = "hankh95"
base_url = "https://api.anthropic.com"
models   = ["claude-haiku-4-5"]
secret   = "NUSY_CLAUDE_TOKEN_HANKH95"
"#;

#[test]
fn the_design_rows_read_as_yurtle_equal_the_same_rows_as_toml() {
    let toml = Config::from_toml_str(DESIGN_ROWS_TOML).expect("the TOML rows parse");
    let yurtle = Config::from_yurtle_str(DESIGN_ROWS).expect("the Yurtle rows parse");
    assert_eq!(yurtle, toml);
}

#[test]
fn the_reader_maps_ids_lists_empties_and_numbers_as_yurtle_does() {
    let c = Config::from_yurtle_str(DESIGN_ROWS).expect("the Yurtle rows parse");
    let ids: Vec<&str> = c.services.iter().map(|s| s.id.as_str()).collect();
    // `@id` `#glm` is the service id `glm`; the decoy table and the toml block are not rows
    assert_eq!(ids, ["glm", "deepseek", "claude-hankh95"]);
    let glm = &c.services[0];
    assert_eq!(glm.kind, Kind::Api);
    assert_eq!(glm.models, ["glm-5.3", "glm-5.2"], "a comma cell is a list");
    assert_eq!(glm.roles, ["review", "work"]);
    assert_eq!(glm.secret.as_deref(), Some("NUSY_GLM"));
    assert_eq!(
        glm.balance,
        BalanceSpec::None,
        "empty balance cells are absence"
    );
    let ds = &c.services[1];
    match &ds.balance {
        BalanceSpec::Endpoint(b) => {
            assert_eq!(b.url, "https://api.deepseek.com/user/balance");
            assert_eq!(b.path, "balance_infos[0].total_balance");
            assert_eq!(b.currency, "CNY");
            assert_eq!(b.floor, 150.0);
        }
        other => panic!("deepseek's balance cells are an endpoint, got {other:?}"),
    }
    let claude = &c.services[2];
    assert_eq!(claude.kind, Kind::Subscription);
    assert!(claude.roles.is_empty(), "an empty roles cell is no roles");
    // nothing outside the block was read: no bus, no alert sinks
    assert!(c.bus.is_none());
    assert_eq!(c.alert, AlertConfig::default());
}

#[test]
fn control_a_changed_cell_in_the_design_rows_breaks_the_equality() {
    let toml = Config::from_toml_str(DESIGN_ROWS_TOML).unwrap();
    let changed = DESIGN_ROWS.replace("| CNY      |", "| USD      |");
    assert_ne!(changed, DESIGN_ROWS);
    let c = Config::from_yurtle_str(&changed).expect("still parses");
    assert_ne!(c, toml, "a differing currency cell must be detected");
}

#[test]
fn a_yurtle_md_path_loads_through_config_load() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("quotabus.yurtle.md");
    std::fs::write(&p, DESIGN_ROWS).unwrap();
    let loaded = Config::load(&p).expect("Config::load reads a .md file as Yurtle");
    assert_eq!(loaded, Config::from_toml_str(DESIGN_ROWS_TOML).unwrap());
}

// ── `[alert.*]` ──────────────────────────────────────────────────────────────────────────────────────────────────

const ALERT_TABLES: &str = r#"
[alert.nusy-kanban]
command   = "nusy-kanban"
item_type = "signal"
tags      = ["provider-status", "infra"]

[alert.yurtle-kanban]
command   = "yurtle-kanban"
item_type = "signal"
tags      = ["provider-status"]

[alert.webhook]
url = "https://example.invalid/hook"
"#;

#[test]
fn the_alert_tables_load_into_the_config() {
    let c = Config::from_toml_str(ALERT_TABLES).expect("the [alert.*] tables parse");
    assert_eq!(
        c.alert,
        AlertConfig {
            nusy_kanban: Some(KanbanSink {
                command: "nusy-kanban".into(),
                item_type: "signal".into(),
                tags: vec!["provider-status".into(), "infra".into()],
            }),
            yurtle_kanban: Some(KanbanSink {
                command: "yurtle-kanban".into(),
                item_type: "signal".into(),
                tags: vec!["provider-status".into()],
            }),
            webhook: Some(WebhookSink {
                url: "https://example.invalid/hook".into(),
            }),
        }
    );
}

#[test]
fn control_no_alert_tables_means_no_sinks() {
    // the known answer above is not a constant: without the tables there is no sink
    let c = Config::from_toml_str("").unwrap();
    assert_eq!(c.alert, AlertConfig::default());
    assert_ne!(c.alert, Config::from_toml_str(ALERT_TABLES).unwrap().alert);
}

#[test]
fn the_example_files_signals_tagged_provider_status_on_both_boards() {
    // §10 Q8 / SIG-006: "Signal per crossing", tag `provider-status`; EXP-004 Plan 1: yurtle-kanban `create signal`
    let a = toml_example().alert;
    for (name, sink) in [
        ("nusy-kanban", a.nusy_kanban),
        ("yurtle-kanban", a.yurtle_kanban),
    ] {
        let sink = sink.unwrap_or_else(|| panic!("examples/quotabus.toml has [alert.{name}]"));
        assert_eq!(sink.item_type, "signal", "[alert.{name}] item_type");
        assert!(
            sink.tags.iter().any(|t| t == "provider-status"),
            "[alert.{name}] tags carry provider-status: {:?}",
            sink.tags
        );
    }
}
