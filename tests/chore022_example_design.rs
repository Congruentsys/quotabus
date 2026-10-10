//! CHORE-022 DoD 2: `examples/quotabus.toml` has two local services, DGX1 `http://192.168.8.120:8000/v1` and DGX2
//! `http://192.168.8.121:8000/v1`, with no `secret` and no `models`; DESIGN.md §4's local row matches, citing the
//! spark-model skill and this item. Each check is a function run on the real file AND, as its control, on the stale
//! text the item replaces (which it must reject).

use std::path::Path;

use quotabus::{Config, Kind};

const DGX1: &str = "http://192.168.8.120:8000/v1";
const DGX2: &str = "http://192.168.8.121:8000/v1";
/// What the item says is stale (measured on Mini, 2026-10-09).
const STALE: [&str; 3] = ["192.168.8.180", ":30000", "NUSY_LOCAL_QWEN"];

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Every way `cfg`'s local services miss DoD 2 (empty = it meets it).
fn example_problems(cfg: &Config) -> Vec<String> {
    let mut out = Vec::new();
    let locals: Vec<_> = cfg
        .services
        .iter()
        .filter(|s| s.kind == Kind::Local)
        .collect();
    if locals.len() != 2 {
        out.push(format!("{} local services, want 2", locals.len()));
    }
    let mut urls: Vec<&str> = locals
        .iter()
        .filter_map(|s| s.base_url.as_deref())
        .collect();
    urls.sort();
    if urls != [DGX1, DGX2] {
        out.push(format!("local base_urls {urls:?}, want [{DGX1}, {DGX2}]"));
    }
    for s in &locals {
        if let Some(n) = &s.secret {
            out.push(format!(
                "{}: has secret {n:?}; a local service needs no key",
                s.id
            ));
        }
        if !s.models.is_empty() {
            out.push(format!(
                "{}: has models {:?}; the served model is discovered",
                s.id, s.models
            ));
        }
        if s.protocol.is_none() {
            out.push(format!("{}: no protocol (the probe needs one)", s.id));
        }
    }
    out
}

/// Every stale value left in `text`.
fn stale_in(text: &str) -> Vec<&'static str> {
    STALE.iter().copied().filter(|s| text.contains(s)).collect()
}

/// DESIGN.md §4, from its heading to §5's.
fn section4(design: &str) -> &str {
    let start = design.find("\n## 4.").expect("DESIGN.md has a §4");
    let end = design[start + 1..]
        .find("\n## 5.")
        .map(|e| start + 1 + e)
        .expect("DESIGN.md has a §5");
    &design[start..end]
}

/// The §4 table rows whose first cell names a local service.
fn local_rows(sec: &str) -> Vec<&str> {
    sec.lines()
        .filter(|l| l.starts_with('|'))
        .filter(|l| {
            l.split('|')
                .nth(1)
                .is_some_and(|c| c.to_ascii_lowercase().contains("local"))
        })
        .collect()
}

/// Every way a §4 local row misses DoD 2.
fn row_problems(row: &str) -> Vec<String> {
    let mut out = Vec::new();
    for need in [
        "192.168.8.120:8000",
        "192.168.8.121:8000",
        "spark-model",
        "CHORE-022",
    ] {
        if !row.contains(need) {
            out.push(format!("lacks {need:?}"));
        }
    }
    for s in stale_in(row) {
        out.push(format!("still has stale {s:?}"));
    }
    out
}

#[test]
fn the_example_has_dgx1_and_dgx2_at_8000_with_no_secret_and_no_models() {
    let cfg = Config::load(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/quotabus.toml")
            .as_path(),
    )
    .expect("examples/quotabus.toml parses");
    assert_eq!(example_problems(&cfg), Vec::<String>::new());
}

#[test]
fn the_example_keeps_no_stale_local_value() {
    assert_eq!(
        stale_in(&read("examples/quotabus.toml")),
        Vec::<&str>::new()
    );
}

#[test]
fn control_the_stale_local_service_fails_the_example_check() {
    // the block the item replaces: one local service at :30000 with a secret and a fixed model
    let stale = "[[service]]\nid = \"local-qwen\"\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\n\
                 account = \"dgx1\"\nbase_url = \"http://192.168.8.180:30000/v1\"\nprotocol = \"openai\"\n\
                 models = [\"qwen3\"]\nroles = [\"work\"]\ncost_class = \"local\"\nsecret = \"NUSY_LOCAL_QWEN\"\n";
    let cfg = Config::from_toml_str(stale).expect("the stale block parses");
    let p = example_problems(&cfg);
    assert!(p.iter().any(|x| x.contains("local services")), "{p:?}");
    assert!(p.iter().any(|x| x.contains("has secret")), "{p:?}");
    assert!(p.iter().any(|x| x.contains("has models")), "{p:?}");
    assert_eq!(
        stale_in(stale).len(),
        3,
        "the stale scan sees all three values"
    );

    // and a right-shaped pair that still carries a secret and models is caught per field
    let mut wrong = String::new();
    for (id, url) in [("dgx1", DGX1), ("dgx2", DGX2)] {
        wrong.push_str(&format!(
            "[[service]]\nid = {id:?}\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\naccount = {id:?}\n\
             base_url = {url:?}\nprotocol = \"openai\"\nmodels = [\"qwen3\"]\nsecret = \"X\"\n"
        ));
    }
    let p = example_problems(&Config::from_toml_str(&wrong).unwrap());
    assert_eq!(p.len(), 4, "two secrets and two model lists: {p:?}");
}

#[test]
fn design_section_4_local_row_matches_the_example_and_cites_its_sources() {
    let design = read("docs/DESIGN.md");
    let rows = local_rows(section4(&design));
    assert_eq!(rows.len(), 1, "§4 has one local row: {rows:?}");
    assert_eq!(row_problems(rows[0]), Vec::<String>::new(), "{}", rows[0]);
}

#[test]
fn control_the_stale_design_row_fails_the_row_check() {
    let stale = "| local Qwen, DGX1 `192.168.8.180:30000` (`NUSY_LOCAL_QWEN`) | `GET /v1/models`, then a 20-token \
                 completion | reachability, model list, latency | balance (self-hosted) |";
    assert_eq!(
        local_rows(stale),
        [stale],
        "the row finder finds a local row"
    );
    let p = row_problems(stale);
    assert_eq!(
        p.len(),
        7,
        "four missing citations/hosts and three stale values: {p:?}"
    );
}
