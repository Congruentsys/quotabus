//! EXP-001 Plan 3 / DESIGN §3-§4: `examples/quotabus.toml` holds the fleet's real services with secret NAMES only.

use std::collections::BTreeSet;
use std::path::Path;

use quotabus::config::BalanceSpec;
use quotabus::{Config, Kind};

fn example() -> (String, Config) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/quotabus.toml");
    let text = std::fs::read_to_string(&p).expect("examples/quotabus.toml exists");
    let cfg = Config::load(&p).expect("examples/quotabus.toml parses");
    (text, cfg)
}

#[test]
fn example_lists_the_six_key_bearing_services_and_the_xai_control() {
    let (_, c) = example();
    let ids: BTreeSet<&str> = c.services.iter().map(|s| s.id.as_str()).collect();
    for id in [
        "glm",
        "deepseek",
        "kimi",
        "openai",
        "together",
        "xai",
        "local-qwen",
    ] {
        assert!(
            ids.contains(id),
            "examples/quotabus.toml lacks service `{id}`; has {ids:?}"
        );
    }
    let names: BTreeSet<&str> = c
        .services
        .iter()
        .filter_map(|s| s.secret.as_deref())
        .collect();
    let want: BTreeSet<&str> = [
        "NUSY_GLM",
        "NUSY_DEEPSEEK",
        "NUSY_KIMI",
        "OPENAI_API_KEY",
        "TOGETHER_API_KEY",
        "XAI_API_KEY",
        "NUSY_LOCAL_QWEN",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, want);
    for s in &c.services {
        if s.secret.is_some() {
            assert!(
                s.base_url.is_some() && s.protocol.is_some(),
                "{} needs base_url and protocol",
                s.id
            );
            assert!(!s.models.is_empty(), "{} needs a model", s.id);
        }
    }
    let qwen = c.services.iter().find(|s| s.id == "local-qwen").unwrap();
    assert_eq!(qwen.kind, Kind::Local);
}

#[test]
fn example_balance_adapters_are_deepseek_and_kimi() {
    let (_, c) = example();
    let path = |id: &str| match &c.services.iter().find(|s| s.id == id).unwrap().balance {
        BalanceSpec::Endpoint(b) => Some(b.path.clone()),
        BalanceSpec::None => None,
    };
    assert_eq!(
        path("deepseek").as_deref(),
        Some("balance_infos[0].total_balance")
    );
    assert_eq!(path("kimi").as_deref(), Some("data.available_balance"));
    assert_eq!(path("glm"), None);
}

#[test]
fn example_carries_names_never_values() {
    let (text, c) = example();
    for bad in ["sk-", "ghp_", "Bearer", "@gmail", "@users.noreply"] {
        assert!(
            !text.contains(bad),
            "examples/quotabus.toml contains {bad:?}"
        );
    }
    for s in &c.services {
        if let Some(n) = &s.secret {
            assert!(
                !n.is_empty()
                    && n.chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'),
                "secret must be an env var NAME, got {n:?}"
            );
        }
    }
    assert_eq!(c.bus.as_ref().map(|b| b.bucket.as_str()), Some("ai_status"));
}
