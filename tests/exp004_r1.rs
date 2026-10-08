//! EXP-004 review r1 (`reviews/EXP-004-r1.md`), the driver's rulings:
//! - **F1:** one item per crossing, and only a measured `ok` clears (DESIGN §3 alert; the Plan's "a service going
//!   bad"). Once a key is alerted, a move to a DIFFERENT bad state files nothing; only a measured `ok` re-arms it.
//! - **F3:** a kanban sink's environment holds no variable outside an allowlist: a variable quotabus was given but
//!   does not name (what `doppler run` injects beyond the config's `secret`s) never reaches the sink. PATH and HOME
//!   do.
//!
//! Fakes only: the kanban sinks are scripts that record their argv, stdin and environment NAMES (never values) and
//! touch no board; the store is the file backend in a temp dir.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_PLAIN, rc, run_cli, text};
use quotabus::{Backend, FileBackend, Record, State};

const T: Duration = Duration::from_secs(60);
const KEY: &str = "api.moonshot.nusy-product-team.kimi-k2-5";
const PLANTED: &str = "QB_UNCONFIGURED_SECRET";

/// A fake kanban executable that records each call: `<log>/<name>.XXXXXX.argv` (NUL-separated) and `.env` (one
/// variable NAME per line, as the process received them, before the script touches PATH).
struct Fake {
    _dir: tempfile::TempDir,
    bin: PathBuf,
    log: PathBuf,
}

impl Fake {
    fn new() -> Fake {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        let log = dir.path().join("log");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&log).unwrap();
        let name = "nusy-kanban";
        let script = format!(
            "#!/bin/sh\n\
             f=$(/usr/bin/mktemp '{log}/{name}.XXXXXX') || exit 99\n\
             /usr/bin/env | /usr/bin/sed -n 's/^\\([A-Za-z_][A-Za-z0-9_]*\\)=.*/\\1/p' > \"$f.env\"\n\
             PATH=/usr/bin:/bin; export PATH\n\
             printf '%s\\0' \"$@\" > \"$f.argv\"\n\
             cat > /dev/null\n\
             echo 'Created SIG-999'\n\
             exit 0\n",
            log = log.display(),
        );
        let p = bin.join(name);
        std::fs::write(&p, script).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        Fake {
            _dir: dir,
            bin,
            log,
        }
    }

    fn path(&self) -> PathBuf {
        self.bin.join("nusy-kanban")
    }

    fn records(&self, ext: &str) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&self.log)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == ext))
            .collect();
        v.sort();
        v
    }

    /// How many creates were filed.
    fn creates(&self) -> usize {
        self.records("argv").len()
    }

    /// The env NAMES of every call.
    fn env_names(&self) -> Vec<Vec<String>> {
        self.records("env")
            .iter()
            .map(|p| {
                std::fs::read_to_string(p)
                    .unwrap()
                    .lines()
                    .map(str::to_string)
                    .collect()
            })
            .collect()
    }
}

struct Rig {
    fake: Fake,
    dir: tempfile::TempDir,
    cfg: PathBuf,
    store: FileBackend,
}

fn rig() -> Rig {
    let fake = Fake::new();
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n\
             [[service]]\nid = \"kimi\"\nkind = \"api\"\nprovider = \"moonshot\"\nfamily = \"moonshot\"\n\
             account = \"nusy-product-team\"\nbase_url = \"http://127.0.0.1:1/kimi\"\nprotocol = \"anthropic\"\n\
             models = [\"kimi-k2.5\"]\nroles = [\"review\"]\nsecret = \"QB_KIMI\"\n\n\
             [alert.nusy-kanban]\ncommand = {:?}\nitem_type = \"signal\"\ntags = [\"provider-status\"]\n",
            state.display().to_string(),
            fake.path().display().to_string(),
        ),
    )
    .unwrap();
    Rig {
        fake,
        store: FileBackend::new(&state),
        dir,
        cfg,
    }
}

async fn alert(rig: &Rig, extra: &[(&'static str, &'static str)]) -> Output {
    let c = rig.cfg.clone();
    let home = rig.dir.path().display().to_string();
    let extra = extra.to_vec();
    tokio::task::spawn_blocking(move || {
        let mut envs: Vec<(&str, &str)> = vec![
            ("PATH", "/usr/bin:/bin"),
            ("HOME", home.as_str()),
            ("QB_KIMI", FAKE_PLAIN),
        ];
        envs.extend(extra);
        run_cli(&c, &["alert"], &envs, T)
    })
    .await
    .unwrap()
}

fn row(state: State) -> Record {
    common::record(KEY, state, Utc::now(), 600)
}

/// Put each state as a fresh row and run `alert` after each; return the creates filed.
async fn run_sequence(states: &[State]) -> usize {
    let rig = rig();
    for st in states {
        rig.store.put(&row(*st)).await.unwrap();
        let out = alert(&rig, &[]).await;
        assert_eq!(rc(&out), 0, "alert after {st:?}: {}", text(&out));
    }
    rig.fake.creates()
}

/// `a, b, a, b, …`, `n` long.
fn alternating(a: State, b: State, n: usize) -> Vec<State> {
    (0..n).map(|i| if i % 2 == 0 { a } else { b }).collect()
}

/// The same with a measured `ok` after each bad reading.
fn with_ok_between(bad: &[State]) -> Vec<State> {
    bad.iter().flat_map(|s| [*s, State::Ok]).collect()
}

// ── F1 ───────────────────────────────────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn r1_f1_rate_limited_and_quota_exhausted_alternating_ten_cycles_files_one() {
    let seq = alternating(State::RateLimited, State::QuotaExhausted, 10);
    assert_eq!(
        run_sequence(&seq).await,
        1,
        "one outage that never cleared is ONE item, whatever bad state it reads"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn r1_f1_degraded_and_rate_limited_alternating_ten_cycles_files_one() {
    let seq = alternating(State::Degraded, State::RateLimited, 10);
    assert_eq!(run_sequence(&seq).await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn r1_f1_control_an_ok_between_each_bad_reading_files_again_after_each_ok() {
    // the count above can be more than one: a measured ok between readings re-arms the key every time
    for (a, b) in [
        (State::RateLimited, State::QuotaExhausted),
        (State::Degraded, State::RateLimited),
    ] {
        let bad = alternating(a, b, 10);
        assert_eq!(
            run_sequence(&with_ok_between(&bad)).await,
            10,
            "{a:?}/{b:?} with ok between: one item per return to bad"
        );
    }
}

// ── F3 ───────────────────────────────────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn r1_f3_a_kanban_sinks_environment_holds_only_allowlisted_variables() {
    let rig = rig();
    rig.store.put(&row(State::ModelMissing)).await.unwrap();
    let out = alert(&rig, &[(PLANTED, "x")]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let envs = rig.fake.env_names();
    assert_eq!(envs.len(), 1, "the sink ran once");
    let names = &envs[0];
    assert!(
        !names.iter().any(|n| n == PLANTED),
        "{PLANTED} (given to quotabus, named by no config) reached the sink: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "QB_KIMI"),
        "the configured secret reached the sink: {names:?}"
    );
    for want in ["PATH", "HOME"] {
        assert!(
            names.iter().any(|n| n == want),
            "{want} is passed to the sink: {names:?}"
        );
    }
}

#[test]
fn r1_f3_control_the_name_check_sees_a_passed_variable() {
    // the fake records what it is given: a variable passed to it shows up by NAME (and its value does not)
    let fake = Fake::new();
    let st = std::process::Command::new(fake.path())
        .args(["create", "signal", "t"])
        .env_clear()
        .env(PLANTED, "x-value-not-recorded")
        .env("HOME", "/tmp")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(st.success());
    let envs = fake.env_names();
    assert_eq!(envs.len(), 1);
    assert!(envs[0].iter().any(|n| n == PLANTED), "{:?}", envs[0]);
    assert!(envs[0].iter().any(|n| n == "HOME"), "{:?}", envs[0]);
    let raw = std::fs::read_to_string(&fake.records("env")[0]).unwrap();
    assert!(!raw.contains("x-value-not-recorded"), "names only: {raw}");
}
