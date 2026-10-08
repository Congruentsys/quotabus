//! EXP-002 Plan 5 (light checks): `examples/quotabus.toml` carries the subscription services (Claude accounts by
//! their `NUSY_CLAUDE_TOKEN_<ACCOUNT>` names, Copilot), and parses; `docs/DESIGN.md` drops the per-host agent and
//! statusLine and records the measured routes; `packaging/README.md` says nothing runs on the other hosts.

use std::path::Path;

use quotabus::{Config, Kind};

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn the_example_config_has_the_subscription_services() {
    let c = Config::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/quotabus.toml"))
        .expect("examples/quotabus.toml parses");
    let subs: Vec<_> = c
        .services
        .iter()
        .filter(|s| s.kind == Kind::Subscription)
        .collect();
    assert!(
        subs.iter().any(|s| s.provider == "anthropic"
            && s.secret
                .as_deref()
                .is_some_and(|n| n.starts_with("NUSY_CLAUDE_TOKEN_"))),
        "a Claude account named by its NUSY_CLAUDE_TOKEN_<ACCOUNT> secret"
    );
    assert!(
        subs.iter()
            .any(|s| s.provider == "github" && s.secret.is_some()),
        "a Copilot service with its token's NAME"
    );
    for s in &subs {
        assert!(
            !s.account.contains("{host}"),
            "{}: accounts are labels, not hosts",
            s.id
        );
    }
}

#[test]
fn design_drops_the_per_host_agent_and_records_the_measured_routes() {
    let d = read("docs/DESIGN.md");
    for gone in [
        "quotabus agent",
        "quotabus-agent",
        "claude-rate-limits.json",
    ] {
        assert!(!d.contains(gone), "DESIGN still describes {gone:?}");
    }
    for want in [
        "anthropic-ratelimit-unified",
        "oauth-2025-04-20",
        "copilot_internal/user",
        "stream-json",
    ] {
        assert!(d.contains(want), "DESIGN does not record {want:?}");
    }
}

#[test]
fn packaging_readme_says_nothing_runs_on_the_other_hosts() {
    let r = read("packaging/README.md");
    assert!(
        r.to_lowercase().contains("other hosts"),
        "packaging/README.md:\n{r}"
    );
    assert!(!r.contains("quotabus-agent"), "no per-host agent unit");
}
