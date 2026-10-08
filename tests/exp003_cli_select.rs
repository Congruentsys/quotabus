//! EXP-003 / DESIGN §3 "selector": `quotabus select --role review --exclude-family anthropic [--prefer
//! cheapest|fastest|largest-context] [--n 1]` prints `provider model` on line 1 (for `$(…)`), the ranked list with
//! reasons under `--json`; rc 3 when nothing qualifies. §2: a store that cannot be read is CANNOT-ASSESS rc 2, never a
//! pick. §10 Q6: it never acts — reading leaves the store as it was. Run against the FILE backend (no bus).
//!
//! CLI contract asserted (EXP-003 test partner): without `--json`, stdout is exactly `--n` lines (default 1), each
//! `<provider> <model>`, best first; with `--json`, a JSON array of the first `--n` candidates, each an object with at
//! least `provider`, `model` and a non-empty `reason`; exit 0 when something qualifies. When nothing qualifies (or the
//! store is unreadable) stdout names no configured model, so `$(…)` never yields a usable pick.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use common::{rc, run_cli, text};
use quotabus::{Config, Kind, Prefer, Query, Record, State, record_key, select};
use serde_json::Value;

const T: Duration = Duration::from_secs(30);

/// (id, kind, provider/family, model, roles, cost_class, context, latency)
type Svc = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    u64,
    u64,
);
const FLEET: &[Svc] = &[
    (
        "glm",
        "api",
        "zhipu",
        "glm-5.3",
        "\"review\", \"work\"",
        "metered",
        200_000,
        400,
    ),
    (
        "ds-pro",
        "api",
        "deepseek",
        "deepseek-v4-pro",
        "\"review\"",
        "metered",
        128_000,
        300,
    ),
    (
        "ds-flash",
        "api",
        "deepseek",
        "deepseek-v4-flash",
        "\"review\"",
        "metered",
        64_000,
        200,
    ),
    (
        "claude-api",
        "api",
        "anthropic",
        "claude-opus",
        "\"review\"",
        "metered",
        1_000_000,
        50,
    ),
    (
        "qwen",
        "local",
        "qwen",
        "qwen3",
        "\"review\"",
        "local",
        32_000,
        900,
    ),
];

const MODELS: &[&str] = &[
    "glm-5.3",
    "deepseek-v4-pro",
    "deepseek-v4-flash",
    "claude-opus",
    "qwen3",
];

fn services_toml() -> String {
    FLEET
        .iter()
        .map(|(id, kind, p, m, roles, cost, ctx, _)| {
            format!(
                "[[service]]\nid = {id:?}\nkind = {kind:?}\nprovider = {p:?}\nfamily = {p:?}\naccount = \"acct\"\n\
                 models = [{m:?}]\nroles = [{roles}]\ncost_class = {cost:?}\nsecret = \"QB_UNUSED\"\n\
                 [service.context]\n{m:?} = {ctx}\n"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn kind_of(k: &str) -> Kind {
    match k {
        "local" => Kind::Local,
        _ => Kind::Api,
    }
}

/// Every model's row, fresh and `ok`, then `edit` applied.
fn rows(edit: impl Fn(&mut Record)) -> Vec<Record> {
    FLEET
        .iter()
        .map(|(_, kind, p, m, _, _, _, lat)| {
            let mut r = common::record(
                &record_key(kind_of(kind), p, "acct", m),
                State::Ok,
                Utc::now() - chrono::Duration::seconds(30),
                3600,
            );
            r.model = m.to_string();
            r.latency_ms = Some(*lat);
            edit(&mut r);
            r
        })
        .collect()
}

struct Fixture {
    dir: tempfile::TempDir,
    config: PathBuf,
}

impl Fixture {
    fn state(&self) -> PathBuf {
        self.dir.path().join("state")
    }
}

fn fixture(rows: &[Record]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir(&state).unwrap();
    for r in rows {
        std::fs::write(
            state.join(format!("{}.json", r.key)),
            serde_json::to_vec(r).unwrap(),
        )
        .unwrap();
    }
    let config = dir.path().join("quotabus.toml");
    std::fs::write(
        &config,
        format!("[file]\ndir = {:?}\n\n{}", state.display(), services_toml()),
    )
    .unwrap();
    Fixture { dir, config }
}

fn select_cli(f: &Fixture, extra: &[&str]) -> std::process::Output {
    let mut args = vec![
        "select",
        "--role",
        "review",
        "--exclude-family",
        "anthropic",
    ];
    args.extend_from_slice(extra);
    run_cli(&f.config, &args, &[], T)
}

fn stdout(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn names_no_model(out: &str) {
    for m in MODELS {
        assert!(
            !out.contains(m),
            "stdout names {m} although nothing qualified:\n{out}"
        );
    }
}

// ---- line 1 is exactly `provider model`; --n ----

#[test]
fn line_one_is_exactly_provider_space_model() {
    let f = fixture(&rows(|_| {}));
    let o = select_cli(&f, &["--prefer", "fastest"]);
    assert_eq!(rc(&o), 0, "{}", text(&o));
    // known answer: claude-opus (50 ms) is excluded, deepseek-v4-flash (200 ms) is next
    assert_eq!(
        stdout(&o),
        "deepseek deepseek-v4-flash\n",
        "exactly one line for `$(…)`; stderr: {}",
        text(&o)
    );
}

#[test]
fn control_without_the_exclusion_line_one_is_the_anthropic_model() {
    let f = fixture(&rows(|_| {}));
    let o = run_cli(
        &f.config,
        &["select", "--role", "review", "--prefer", "fastest"],
        &[],
        T,
    );
    assert_eq!(rc(&o), 0, "{}", text(&o));
    assert_eq!(stdout(&o).lines().next(), Some("anthropic claude-opus"));
}

#[test]
fn n_prints_that_many_lines_best_first() {
    let f = fixture(&rows(|_| {}));
    let o = select_cli(&f, &["--prefer", "largest-context", "--n", "3"]);
    assert_eq!(rc(&o), 0, "{}", text(&o));
    assert_eq!(
        stdout(&o),
        "zhipu glm-5.3\ndeepseek deepseek-v4-pro\ndeepseek deepseek-v4-flash\n"
    );
}

#[test]
fn json_is_the_ranked_list_with_reasons() {
    let f = fixture(&rows(|_| {}));
    let o = select_cli(&f, &["--prefer", "fastest", "--n", "10", "--json"]);
    assert_eq!(rc(&o), 0, "{}", text(&o));
    let v: Value =
        serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("--json: {e}\n{}", text(&o)));
    let arr = v.as_array().expect("a JSON array");
    let got: Vec<(&str, &str)> = arr
        .iter()
        .map(|c| {
            (
                c["provider"].as_str().unwrap(),
                c["model"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("deepseek", "deepseek-v4-flash"),
            ("deepseek", "deepseek-v4-pro"),
            ("zhipu", "glm-5.3"),
            ("qwen", "qwen3")
        ]
    );
    for c in arr {
        assert!(
            c["reason"].as_str().is_some_and(|r| !r.trim().is_empty()),
            "no reason: {c}"
        );
    }
}

// ---- the library call equals the CLI ----

#[test]
fn the_library_call_equals_the_cli() {
    let r = rows(|r| {
        if r.model == "glm-5.3" {
            r.state = State::RateLimited;
        }
    });
    let f = fixture(&r);
    let config = Config::load(&f.config).expect("the fixture config parses");
    for (word, prefer) in [
        ("cheapest", Prefer::Cheapest),
        ("fastest", Prefer::Fastest),
        ("largest-context", Prefer::LargestContext),
    ] {
        let q = Query {
            role: "review".into(),
            exclude_families: vec!["anthropic".into()],
            prefer: Some(prefer),
            now: Utc::now(),
        };
        let lib: Vec<(String, String)> = select(&r, &q, &config)
            .into_iter()
            .map(|c| (c.provider, c.model))
            .collect();
        assert!(!lib.is_empty(), "{word}: the library chose nothing");
        let o = select_cli(&f, &["--prefer", word, "--n", "10", "--json"]);
        assert_eq!(rc(&o), 0, "{word}: {}", text(&o));
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        let cli: Vec<(String, String)> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["provider"].as_str().unwrap().into(),
                    c["model"].as_str().unwrap().into(),
                )
            })
            .collect();
        assert_eq!(cli, lib, "--prefer {word}");
        let plain = select_cli(&f, &["--prefer", word]);
        assert_eq!(
            stdout(&plain),
            format!("{} {}\n", lib[0].0, lib[0].1),
            "--prefer {word}, line 1"
        );
    }
}

// ---- exit codes ----

#[test]
fn rc_3_when_every_candidate_is_unknown() {
    let f = fixture(&rows(|r| {
        r.state = State::Unknown;
        r.reason = Some("cannot_assess:unreachable".into());
    }));
    let o = select_cli(&f, &[]);
    assert_eq!(rc(&o), 3, "{}", text(&o));
    names_no_model(&stdout(&o));
}

#[test]
fn rc_3_when_every_row_is_absent_or_expired() {
    let f = fixture(&[]);
    let o = select_cli(&f, &[]);
    assert_eq!(rc(&o), 3, "no rows: {}", text(&o));
    names_no_model(&stdout(&o));
    let f = fixture(&rows(|r| {
        r.checked_at = Utc::now() - chrono::Duration::seconds(7200);
        r.ttl_s = 60;
    }));
    let o = select_cli(&f, &[]);
    assert_eq!(rc(&o), 3, "every row expired: {}", text(&o));
    names_no_model(&stdout(&o));
}

#[test]
fn rc_3_when_only_the_excluded_family_is_healthy() {
    // DoD: the selector refuses a family the author uses, even as the last healthy one
    let f = fixture(&rows(|r| {
        if r.family != "anthropic" {
            r.state = State::QuotaExhausted;
        }
    }));
    let o = select_cli(&f, &[]);
    assert_eq!(rc(&o), 3, "{}", text(&o));
    names_no_model(&stdout(&o));
}

#[test]
fn control_rc_0_when_one_candidate_is_healthy() {
    let f = fixture(&rows(|r| {
        if r.model != "qwen3" {
            r.state = State::Unknown;
            r.reason = Some("cannot_assess:unreachable".into());
        }
    }));
    let o = select_cli(&f, &[]);
    assert_eq!(rc(&o), 0, "{}", text(&o));
    assert_eq!(stdout(&o), "qwen qwen3\n");
}

#[test]
fn rc_2_when_the_bus_is_unreachable() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("quotabus.toml");
    let port = common::closed_port();
    std::fs::write(
        &config,
        format!(
            "[bus]\nurl = \"nats://127.0.0.1:{port}\"\nbucket = \"ai_status\"\n\n{}",
            services_toml()
        ),
    )
    .unwrap();
    let o = run_cli(
        &config,
        &[
            "select",
            "--role",
            "review",
            "--exclude-family",
            "anthropic",
        ],
        &[],
        T,
    );
    assert_eq!(rc(&o), 2, "CANNOT-ASSESS, never a pick: {}", text(&o));
    names_no_model(&stdout(&o));
}

#[test]
fn rc_2_when_the_row_directory_is_missing() {
    let f = fixture(&rows(|_| {}));
    std::fs::remove_dir_all(f.state()).unwrap();
    let o = select_cli(&f, &[]);
    assert_eq!(rc(&o), 2, "{}", text(&o));
    names_no_model(&stdout(&o));
}

// ---- §10 Q6: it never acts ----

fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn select_leaves_the_store_as_it_was() {
    let f = fixture(&rows(|r| {
        if r.model == "glm-5.3" {
            r.state = State::QuotaExhausted;
        }
    }));
    let before = snapshot(&f.state());
    let o = select_cli(&f, &["--n", "5", "--json"]);
    assert_eq!(rc(&o), 0, "{}", text(&o));
    let none = fixture(&[]);
    let o3 = select_cli(&none, &[]);
    assert_eq!(rc(&o3), 3, "{}", text(&o3));
    assert_eq!(snapshot(&f.state()), before, "select wrote to the store");
    assert!(
        snapshot(&none.state()).is_empty(),
        "select wrote to an empty store"
    );
    // control: the snapshot sees a write
    std::fs::write(f.state().join("x.json"), b"{}").unwrap();
    assert_ne!(snapshot(&f.state()), before);
}
