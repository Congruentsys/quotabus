//! CHORE-025 (CHORE-024 r1 F6): the alert summary's `M` ("local skipped") for a local service that DISCOVERS its
//! models (kind `local`, no `models`; CHORE-022 `slots_in`).
//!
//! Summary format (pinned by tests/chore024_subscription_alert_summary.rs), the last line of `alert`'s stdout:
//!
//! ```text
//! alert: <N> keys checked, <M> local skipped, <F> crossings filed, <C> re-armed
//! ```
//!
//! DoD 1: `quotabus alert` over a discovering local service reads M=1 when the store holds none of its rows (the
//! service is still one slot, its id), and M = the served-id count when the newest cycle wrote several. Control: the
//! ids of an OLDER cycle are not counted (older cycle larger than the newest, so a count of either all rows or the
//! older cycle reads a different M; the two cycles share no id, so each wrong count is reachable).
//!
//! DoD 2: counting with `svc.slots()` (empty for a discovering service, so M=0) turns these red.
//!
//! Fakes only: the file backend in a temp dir, a fake `nusy-kanban`, no key, no bus.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use chrono::{DateTime, Utc};
use common::{rc, run_cli, text};
use quotabus::{Backend, FileBackend, Kind, State, record_key};

const T: Duration = Duration::from_secs(60);

fn local_key(model: &str) -> String {
    record_key(Kind::Local, "qwen", "dgx1", model)
}

struct Rig {
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    store: FileBackend,
}

/// One discovering local service `dgx1-qwen` (no `models`), and a fake `nusy-kanban` alert sink.
fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let bin = dir.path().join("nusy-kanban");
    std::fs::write(
        &bin,
        "#!/bin/sh\ncat > /dev/null\necho 'Created SIG-999'\nexit 0\n",
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cfg = dir.path().join("quotabus.toml");
    let s = format!(
        "[file]\ndir = {:?}\n\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\nsubscription = \"2m\"\n\n\
         [[service]]\nid = \"dgx1-qwen\"\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\n\
         account = \"dgx1\"\nbase_url = \"http://127.0.0.1:1/v1\"\nprotocol = \"openai\"\n\
         roles = [\"work\"]\ncost_class = \"local\"\n\n\
         [alert.nusy-kanban]\ncommand = {:?}\nitem_type = \"signal\"\ntags = [\"provider-status\"]\n",
        state_dir.display().to_string(),
        bin.display().to_string()
    );
    std::fs::write(&cfg, &s).unwrap();
    let parsed =
        quotabus::Config::from_toml_str(&s).unwrap_or_else(|e| panic!("config parses: {e}\n{s}"));
    assert!(
        parsed.services[0].discovers(),
        "fixture: the service must be a DISCOVERING local service"
    );
    Rig {
        _dir: dir,
        cfg,
        store: FileBackend::new(&state_dir),
    }
}

/// One probe cycle: every row stamped with the same `checked_at`, as a real cycle writes them.
async fn cycle(rig: &Rig, at: DateTime<Utc>, models: &[&str]) {
    for m in models {
        rig.store
            .put(&common::record(&local_key(m), State::Ok, at, 600))
            .await
            .unwrap();
    }
}

async fn alert(rig: &Rig) -> Output {
    let c = rig.cfg.clone();
    let out = tokio::task::spawn_blocking(move || {
        run_cli(&c, &["alert"], &[("PATH", "/usr/bin:/bin")], T)
    })
    .await
    .unwrap();
    assert_eq!(rc(&out), 0, "alert: {}", text(&out));
    out
}

/// (N, M, F, C) from the summary line, the last line of stdout; Err when it is not the pinned format.
fn summary(stdout: &str) -> Result<(usize, usize, usize, usize), String> {
    let line = stdout.lines().last().ok_or("no stdout")?.trim();
    let rest = line
        .strip_prefix("alert: ")
        .ok_or(format!("not a summary: {line:?}"))?;
    let parts: Vec<&str> = rest.split(", ").collect();
    let suffixes = [
        "keys checked",
        "local skipped",
        "crossings filed",
        "re-armed",
    ];
    if parts.len() != 4 {
        return Err(format!("not a summary: {line:?}"));
    }
    let mut n = [0usize; 4];
    for (i, (p, suf)) in parts.iter().zip(suffixes).enumerate() {
        let num = p
            .strip_suffix(suf)
            .map(str::trim)
            .ok_or(format!("field {i} not `<n> {suf}`: {line:?}"))?;
        n[i] = num
            .parse()
            .map_err(|_| format!("field {i} not a number: {line:?}"))?;
    }
    Ok((n[0], n[1], n[2], n[3]))
}

fn m_of(o: &Output) -> usize {
    let s = String::from_utf8_lossy(&o.stdout).into_owned();
    summary(&s).unwrap_or_else(|e| panic!("{e}\n{}", text(o))).1
}

// ── control on the parser ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn control_the_summary_parser_has_known_answers_and_can_fail() {
    assert_eq!(
        summary("noise\nalert: 3 keys checked, 2 local skipped, 1 crossings filed, 0 re-armed\n"),
        Ok((3, 2, 1, 0))
    );
    assert!(summary("alert: 3 keys checked, 1 crossings filed, 0 re-armed").is_err());
    assert!(
        summary("alert: 3 keys checked, x local skipped, 1 crossings filed, 0 re-armed").is_err()
    );
    assert!(summary("").is_err());
}

// ── DoD 1 ────────────────────────────────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovering_local_service_with_no_rows_skips_one_slot() {
    let rig = rig();
    let o = alert(&rig).await;
    assert_eq!(
        m_of(&o),
        1,
        "no rows: the service is one slot (its id)\n{}",
        text(&o)
    );
    let s = String::from_utf8_lossy(&o.stdout).into_owned();
    assert_eq!(
        summary(&s).unwrap().0,
        0,
        "nothing is checked: the only service is local"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovering_local_service_skips_every_id_of_its_newest_cycle() {
    let rig = rig();
    cycle(&rig, Utc::now(), &["model-a", "model-b", "model-c"]).await;
    let o = alert(&rig).await;
    assert_eq!(
        m_of(&o),
        3,
        "the newest cycle served three ids\n{}",
        text(&o)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn control_an_older_cycles_ids_are_not_counted() {
    let rig = rig();
    let now = Utc::now();
    // older cycle: four ids; newest cycle: two, disjoint from the older (a key shared by both cycles would carry
    // the newer stamp, so the older cycle would yield fewer ids than it wrote; r1 F5)
    cycle(
        &rig,
        now - chrono::Duration::seconds(60),
        &["old-1", "old-2", "old-3", "old-4"],
    )
    .await;
    cycle(&rig, now, &["model-a", "model-b"]).await;
    let o = alert(&rig).await;
    let m = m_of(&o);
    assert_ne!(
        m,
        6,
        "every distinct id in the store (older cycle counted)\n{}",
        text(&o)
    );
    assert_ne!(m, 4, "the older cycle's four ids\n{}", text(&o));
    assert_eq!(m, 2, "only the newest cycle's ids\n{}", text(&o));
}
