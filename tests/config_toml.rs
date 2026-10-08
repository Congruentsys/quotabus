//! EXP-001 Plan 2 / DESIGN §3: `quotabus.toml` loads; `secret` is a NAME. (DESIGN §3 writes several keys on one line
//! with `;`, which is not TOML; this is the same content one key per line.)

use std::time::Duration;

use quotabus::config::{BalanceSpec, Protocol};
use quotabus::{Config, Kind};

pub const DESIGN_S3: &str = r#"
[bus]
url     = "nats://192.168.8.110:4222"
bucket  = "ai_status"

[probe]
interval = "15m"
ttl      = "45m"
max_tokens = 20

[[service]]
id = "glm"
kind = "api"
provider = "zhipu"
family = "zhipu"
account = "nusy-product-team"
base_url = "https://api.z.ai/api/anthropic"
protocol = "anthropic"
models = ["glm-5.3", "glm-5.2", "glm-4.6"]
roles = ["review", "work"]
cost_class = "metered"
secret = "NUSY_GLM"
balance = "none"

[[service]]
id = "deepseek"
kind = "api"
provider = "deepseek"
family = "deepseek"
account = "nusy-product-team"
base_url = "https://api.deepseek.com/anthropic"
protocol = "anthropic"
models = ["deepseek-v4-pro", "deepseek-v4-flash"]
roles = ["review"]
cost_class = "metered"
secret = "NUSY_DEEPSEEK"
[service.balance]
url = "https://api.deepseek.com/user/balance"
path = "balance_infos[0].total_balance"
currency = "CNY"
floor = 150

[[service]]
id = "claude-max"
kind = "subscription"
provider = "anthropic"
family = "anthropic"
account = "{host}"
sources = ["statusline", "claude_json"]

[[service]]
id = "copilot"
kind = "subscription"
provider = "github"
family = "openai"
account = "{host}"
sources = ["copilot_internal"]

[alert.nusy-kanban]
command = "nusy-kanban"
item_type = "signal"
tags = ["provider-status", "infra"]
[alert.yurtle-kanban]
command = "yurtle-kanban"
item_type = "issue"
[alert.webhook]
url = "https://example.invalid/hook"
"#;

#[test]
fn the_design_s3_config_loads() {
    let c = Config::from_toml_str(DESIGN_S3).expect("DESIGN §3 config parses");
    let bus = c.bus.as_ref().expect("[bus]");
    assert_eq!(bus.url, "nats://192.168.8.110:4222");
    assert_eq!(bus.bucket, "ai_status");
    assert_eq!(c.probe.interval, Duration::from_secs(15 * 60));
    assert_eq!(c.probe.ttl, Duration::from_secs(45 * 60));
    assert_eq!(c.probe.max_tokens, 20);
    let ids: Vec<&str> = c.services.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["glm", "deepseek", "claude-max", "copilot"]);

    let glm = &c.services[0];
    assert_eq!(glm.kind, Kind::Api);
    assert_eq!(glm.protocol, Some(Protocol::Anthropic));
    assert_eq!(
        glm.base_url.as_deref(),
        Some("https://api.z.ai/api/anthropic")
    );
    assert_eq!(glm.models, ["glm-5.3", "glm-5.2", "glm-4.6"]);
    assert_eq!(glm.secret.as_deref(), Some("NUSY_GLM"));
    assert_eq!(glm.balance, BalanceSpec::None);

    let ds = &c.services[1];
    match &ds.balance {
        BalanceSpec::Endpoint(b) => {
            assert_eq!(b.url, "https://api.deepseek.com/user/balance");
            assert_eq!(b.path, "balance_infos[0].total_balance");
            assert_eq!(b.currency, "CNY");
            assert_eq!(b.floor, 150.0);
        }
        other => panic!("deepseek balance should be an endpoint, got {other:?}"),
    }
    assert_eq!(c.services[2].kind, Kind::Subscription);
    assert_eq!(c.services[2].sources, ["statusline", "claude_json"]);
    assert_eq!(c.services[2].secret, None);
}

#[test]
fn omitting_bus_selects_the_file_backend_and_bucket_defaults_to_ai_status() {
    let no_bus = Config::from_toml_str(
        "[[service]]\nid=\"x\"\nkind=\"api\"\nprovider=\"p\"\nfamily=\"f\"\naccount=\"a\"\nmodels=[\"m\"]\n",
    )
    .unwrap();
    assert!(
        no_bus.bus.is_none(),
        "[bus] omitted ⇒ no bus (the file backend)"
    );

    let default_bucket = Config::from_toml_str("[bus]\nurl = \"nats://127.0.0.1:4222\"\n").unwrap();
    assert_eq!(default_bucket.bus.unwrap().bucket, "ai_status");

    let named =
        Config::from_toml_str("[bus]\nurl = \"nats://127.0.0.1:4222\"\nbucket = \"qb_other\"\n")
            .unwrap();
    assert_eq!(named.bus.unwrap().bucket, "qb_other");
}

#[test]
fn probe_defaults_are_the_design_values() {
    let c = Config::from_toml_str("").unwrap();
    assert_eq!(c.probe.interval, Duration::from_secs(15 * 60));
    assert_eq!(c.probe.ttl, Duration::from_secs(45 * 60));
    assert_eq!(c.probe.max_tokens, 20);
    assert!(c.services.is_empty());
}

#[test]
fn durations_parse_in_seconds_minutes_and_hours() {
    let c = Config::from_toml_str("[probe]\ninterval = \"90s\"\nttl = \"2h\"\n").unwrap();
    assert_eq!(c.probe.interval, Duration::from_secs(90));
    assert_eq!(c.probe.ttl, Duration::from_secs(7200));
}

#[test]
fn control_bad_configs_are_refused() {
    // negative controls: a broken file, an unknown kind, a service with no id, a bad duration
    assert!(Config::from_toml_str("[[service]\nid = ").is_err());
    assert!(
        Config::from_toml_str(
            "[[service]]\nid=\"x\"\nkind=\"apii\"\nprovider=\"p\"\nfamily=\"f\"\naccount=\"a\"\n"
        )
        .is_err()
    );
    assert!(
        Config::from_toml_str(
            "[[service]]\nkind=\"api\"\nprovider=\"p\"\nfamily=\"f\"\naccount=\"a\"\n"
        )
        .is_err()
    );
    assert!(Config::from_toml_str("[probe]\nttl = \"forever\"\n").is_err());
}

#[test]
fn the_config_holds_secret_names_not_values() {
    let c = Config::from_toml_str(DESIGN_S3).unwrap();
    let dump = format!("{c:?}");
    assert!(dump.contains("NUSY_GLM"), "the NAME is the config's");
    // no value of any kind is in a parsed config: nothing that looks like a key
    assert!(!dump.contains("sk-") && !dump.contains("Bearer"), "{dump}");
}

#[test]
fn load_reads_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("quotabus.toml");
    std::fs::write(&p, DESIGN_S3).unwrap();
    let c = Config::load(&p).unwrap();
    assert_eq!(c.services.len(), 4);
    assert!(Config::load(&dir.path().join("missing.toml")).is_err());
}
