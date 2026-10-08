//! Shared helpers for the EXP-002 tests (the per-host `quotabus agent`, the Claude Code statusLine / stream-json
//! capture and the Copilot read). Every test runs the binary with a CLEARED environment: `HOME` is a temp dir (never
//! the real `~/.claude`, `~/.claude.json` or `~/.cache`), the host name comes from `QUOTABUS_HOST`, and `gh` is a fake
//! shell script on a temp `PATH`. No real key, no real bus, no real `gh` login.
//!
//! The seams these tests fix (for the implementer):
//! - `quotabus statusline` (no config needed): reads the statusLine stdin JSON, prints ONE status line, and writes the
//!   cache `$HOME/.cache/quotabus/claude-rate-limits.json` only when the JSON carries `rate_limits`.
//! - `quotabus tee` (no config needed): copies stdin to stdout byte for byte; a `rate_limit_event` line with
//!   `rate_limit_info.unifiedWindows` is normalised into the same cache.
//! - The cache's normalised shape:
//!   `{"source": "statusline"|"stream_json", "captured_at": "<RFC 3339>",
//!     "five_hour": {"used_percentage": <0..100>, "resets_at": <epoch s>} | absent,
//!     "seven_day": {"used_percentage": <0..100>, "resets_at": <epoch s>} | absent}`
//!   (stream-json `utilization` 0..1 × 100 = `used_percentage`; `resetsAt` = `resets_at`).
//! - `quotabus install-statusline` (no config needed): merges `{"statusLine": {"type": "command", "command":
//!   "<…> statusline"}}` into `$HOME/.claude/settings.json`, keeping every other key; idempotent; refuses (rc ≠ 0,
//!   file unchanged) when a different `statusLine` is already there.
//! - `quotabus agent [--force]` (config required): reads only `kind = "subscription"` services, writes one row per
//!   service to the backend, keyed `subscription.<provider>.<account>.<model>` where `account = "{host}"` becomes the
//!   host label and the model slot is the service `id` when `models` is empty.
//! - `QUOTABUS_HOST` overrides the host label (`quotabus::host_label()`, so `observed_by` = `<host>/quotabus@<v>`).
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

pub const T: Duration = Duration::from_secs(30);

/// The measured statusLine JSON of CHORE-002 run C (second call), paths replaced by fixed fake strings.
pub fn statusline_json() -> Value {
    serde_json::json!({
        "session_id": "sess-0f1e2d3c4b5a69788796a5b4c3d2e1f0",
        "transcript_path": "/fake/home/.claude/projects/qb-secret-transcript-path.jsonl",
        "cwd": "/fake/work/qb-private-cwd",
        "scratchpad_dir": "/fake/scratch/qb-private-scratch",
        "prompt_id": "prompt-0123456789abcdef0123456789abcdef",
        "effort": {"level": "medium"},
        "session_name": "Single word reply",
        "model": {"id": "claude-opus-5-5", "display_name": "Opus 5.5"},
        "workspace": {"current_dir": "/fake/work/qb-private-cwd", "project_dir": "/fake/work/qb-private-cwd",
                      "added_dirs": [], "repo": {"host": "github.com", "owner": "Congruentsys", "name": "quotabus"}},
        "version": "2.1.294",
        "output_style": {"name": "default"},
        "cost": {"total_cost_usd": 0.138667, "total_duration_ms": 11898, "total_api_duration_ms": 1817,
                 "total_lines_added": 0, "total_lines_removed": 0},
        "context_window": {"total_input_tokens": 43863, "total_output_tokens": 4, "context_window_size": 1000000,
                           "current_usage": {"input_tokens": 2, "output_tokens": 4,
                                             "cache_creation_input_tokens": 16626, "cache_read_input_tokens": 27235},
                           "used_percentage": 4, "remaining_percentage": 96},
        "exceeds_200k_tokens": false,
        "fast_mode": false,
        "thinking": {"enabled": true},
        "rate_limits": {"five_hour": {"used_percentage": 19, "resets_at": 1791495000_i64},
                        "seven_day": {"used_percentage": 9, "resets_at": 1792051200_i64}}
    })
}

/// The FIRST render's JSON (before any API reply): no `rate_limits`, no `prompt_id`, zero cost.
pub fn first_render_json() -> Value {
    let mut v = statusline_json();
    let o = v.as_object_mut().unwrap();
    o.remove("rate_limits");
    o.remove("prompt_id");
    o.remove("session_name");
    o["cost"] = serde_json::json!({"total_cost_usd": 0.0, "total_duration_ms": 0, "total_api_duration_ms": 0,
                                   "total_lines_added": 0, "total_lines_removed": 0});
    o["context_window"]["current_usage"] = Value::Null;
    v
}

/// The measured stream-json `rate_limit_event` of CHORE-002 run B (`session_id`, `uuid` dropped there too).
pub const RATE_LIMIT_EVENT: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1791495000,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false,"unifiedWindows":{"five_hour":{"utilization":0.19,"resetsAt":1791495000},"seven_day":{"utilization":0.09,"resetsAt":1792051200}}}}"#;

/// A whole `claude -p --verbose --output-format stream-json` transcript in run B's order, the event third.
pub fn stream_lines() -> Vec<String> {
    vec![
        r#"{"type":"system","subtype":"init","model":"claude-opus-5-5","tools":[]}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}"#.to_string(),
        RATE_LIMIT_EVENT.to_string(),
        r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#.to_string(),
    ]
}

/// The quotabus binary with a CLEARED environment plus `envs`, `cwd` as its working directory.
pub fn bin(cwd: &Path, envs: &[(&str, &str)]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_quotabus"));
    cmd.env_clear().env("RUST_BACKTRACE", "0").current_dir(cwd);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd
}

/// Run `cmd` with `stdin` fed in full, failing the test past `T`.
pub fn run(mut cmd: Command, stdin: &[u8]) -> Output {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn quotabus");
    let mut sin = child.stdin.take().unwrap();
    let data = stdin.to_vec();
    let writer = std::thread::spawn(move || {
        // a binary that exits without reading all of stdin closes the pipe: that is its failure, not ours
        let _ = sin.write_all(&data);
    });
    // drain stdout / stderr while it runs, so a large pass-through never blocks on a full pipe
    let drain = |mut r: Box<dyn std::io::Read + Send>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out_t = drain(Box::new(child.stdout.take().unwrap()));
    let err_t = drain(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + T;
    let status = loop {
        if let Some(st) = child.try_wait().unwrap() {
            break st;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            let err = err_t.join().unwrap();
            panic!(
                "quotabus did not finish within {T:?}; stderr: {}",
                String::from_utf8_lossy(&err)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let _ = writer.join();
    Output {
        status,
        stdout: out_t.join().unwrap(),
        stderr: err_t.join().unwrap(),
    }
}

pub fn rc(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

pub fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `$HOME/.cache/quotabus/claude-rate-limits.json` (DESIGN §4, Claude Max row).
pub fn cache_path(home: &Path) -> PathBuf {
    home.join(".cache/quotabus/claude-rate-limits.json")
}

pub fn read_json(p: &Path) -> Value {
    let t = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&t).unwrap_or_else(|e| panic!("{} is not JSON: {e}\n{t}", p.display()))
}

pub fn write_json(p: &Path, v: &Value) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, serde_json::to_vec_pretty(v).unwrap()).unwrap();
}

pub fn now_s() -> i64 {
    Utc::now().timestamp()
}

pub fn rfc3339(epoch_s: i64) -> String {
    Utc.timestamp_opt(epoch_s, 0)
        .unwrap()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

pub fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .unwrap_or_else(|e| panic!("{s:?} is not RFC 3339: {e}"))
        .with_timezone(&Utc)
}

/// A normalised cache entry (the shape `statusline` / `tee` write), captured `age_s` ago.
pub fn cache_entry(
    source: &str,
    age_s: i64,
    five: Option<(f64, i64)>,
    seven: Option<(f64, i64)>,
) -> Value {
    let mut v = serde_json::json!({"source": source, "captured_at": rfc3339(now_s() - age_s)});
    if let Some((p, r)) = five {
        v["five_hour"] = serde_json::json!({"used_percentage": p, "resets_at": r});
    }
    if let Some((p, r)) = seven {
        v["seven_day"] = serde_json::json!({"used_percentage": p, "resets_at": r});
    }
    v
}

pub const LOGIN_UUID: &str = "6f1c0a52-0000-4000-8000-00000000c1a0";
pub const LOGIN_EMAIL: &str = "qb.login.person@example.com";

/// A `~/.claude.json` with `cachedUsageUtilization` in the shape measured on two hosts (nusy-product-team
/// `f8b7f2f531^:scripts/fleet/usage-pacer.sh:46-59`: `utilization` is a PERCENT, `resets_at` an ISO string,
/// `fetchedAtMs` epoch ms). `login`: whether the `oauthAccount` block (an OAuth login) is present.
pub fn claude_json(
    login: Option<&str>,
    cache_uuid: &str,
    fetched_age_s: i64,
    five_pct: f64,
    seven_pct: f64,
) -> Value {
    let now = now_s();
    let mut v = serde_json::json!({
        "numStartups": 42,
        "theme": "dark",
        "cachedUsageUtilization": {
            "fetchedAtMs": (now - fetched_age_s) * 1000,
            "accountUuid": cache_uuid,
            "utilization": {
                "five_hour": {"utilization": five_pct, "resets_at": rfc3339(now + 7200)},
                "seven_day": {"utilization": seven_pct, "resets_at": rfc3339(now + 4 * 86_400)},
                "limits": [
                    {"kind": "session", "percent": five_pct, "resets_at": rfc3339(now + 7200)},
                    {"kind": "weekly_all", "percent": seven_pct, "resets_at": rfc3339(now + 4 * 86_400)}
                ]
            }
        }
    });
    if let Some(uuid) = login {
        v["oauthAccount"] = serde_json::json!({"accountUuid": uuid, "emailAddress": LOGIN_EMAIL});
    }
    v
}

/// One host's sandbox: a fake HOME, a file-backend store, a `bin/` for a fake `gh`, and a config.
pub struct Host {
    pub dir: tempfile::TempDir,
    pub host: String,
}

pub const CLAUDE_SVC: &str = "[[service]]\nid = \"claude-max\"\nkind = \"subscription\"\nprovider = \"anthropic\"\n\
     family = \"anthropic\"\naccount = \"{host}\"\nsources = [\"statusline\", \"claude_json\"]\n";
pub const COPILOT_SVC: &str = "[[service]]\nid = \"copilot\"\nkind = \"subscription\"\nprovider = \"github\"\n\
     family = \"openai\"\naccount = \"{host}\"\nsources = [\"copilot_internal\"]\n";

impl Host {
    pub fn new(host: &str) -> Host {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("home")).unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        Host {
            dir,
            host: host.to_string(),
        }
    }

    pub fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }

    pub fn store(&self) -> PathBuf {
        self.dir.path().join("store")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.dir.path().join("bin")
    }

    pub fn gh_log(&self) -> PathBuf {
        self.dir.path().join("gh.calls")
    }

    /// Write `quotabus.toml`: a file backend in `store/`, then `extra` (tables and services).
    pub fn config(&self, extra: &str) -> PathBuf {
        self.config_with_store(&self.store(), extra)
    }

    pub fn config_with_store(&self, store: &Path, extra: &str) -> PathBuf {
        let p = self.dir.path().join("quotabus.toml");
        std::fs::write(
            &p,
            format!(
                "[file]\ndir = {:?}\n\n{extra}\n",
                store.display().to_string()
            ),
        )
        .unwrap();
        p
    }

    /// A fake `gh` that logs its argv (one line per call) and answers with `stdout` / `stderr` / `code`.
    pub fn fake_gh(&self, stdout: &str, stderr: &str, code: i32) {
        let p = self.bin_dir().join("gh");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\ncat <<'QB_OUT'\n{stdout}\nQB_OUT\ncat >&2 <<'QB_ERR'\n{stderr}\nQB_ERR\nexit {code}\n",
            log = self.gh_log().display()
        );
        std::fs::write(&p, script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// How many times the fake `gh` ran, and with what arguments.
    pub fn gh_calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.gh_log())
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn path_env(&self) -> String {
        format!("{}:/usr/bin:/bin", self.bin_dir().display())
    }

    /// `quotabus --config <cfg> <args>` as this host: HOME, QUOTABUS_HOST and PATH injected.
    pub fn quotabus(&self, cfg: &Path, args: &[&str]) -> Output {
        let home = self.home();
        let path = self.path_env();
        let mut cmd = bin(
            self.dir.path(),
            &[
                ("HOME", home.to_str().unwrap()),
                ("QUOTABUS_HOST", &self.host),
                ("PATH", &path),
            ],
        );
        cmd.arg("--config").arg(cfg).args(args);
        run(cmd, b"")
    }

    pub fn agent(&self, cfg: &Path, args: &[&str]) -> Output {
        let mut a = vec!["agent"];
        a.extend_from_slice(args);
        let out = self.quotabus(cfg, &a);
        assert_eq!(rc(&out), 0, "quotabus agent must exit 0: {}", text(&out));
        out
    }

    pub fn write_cache(&self, v: &Value) {
        write_json(&cache_path(&self.home()), v);
    }

    pub fn write_claude_json(&self, v: &Value) {
        write_json(&self.home().join(".claude.json"), v);
    }
}

/// Every row in a file-backend store, as JSON (so new optional fields need no struct change to be read).
pub fn rows(store: &Path) -> Vec<Value> {
    let Ok(rd) = std::fs::read_dir(store) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.ends_with(".json") && !name.starts_with('.') {
            out.push(read_json(&p));
        }
    }
    out.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
    out
}

pub fn row(store: &Path, key: &str) -> Option<Value> {
    rows(store).into_iter().find(|r| r["key"] == key)
}

pub fn must_row(store: &Path, key: &str) -> Value {
    row(store, key).unwrap_or_else(|| {
        let keys: Vec<String> = rows(store).iter().map(|r| r["key"].to_string()).collect();
        panic!("no row {key} in the store; it holds {keys:?}")
    })
}

pub fn claude_key(host: &str) -> String {
    format!("subscription.anthropic.{host}.claude-max")
}

pub fn copilot_key(host: &str) -> String {
    format!("subscription.github.{host}.copilot")
}

/// The `headroom.windows[]` entry named `name` (`five_hour`, `seven_day`, `monthly`).
pub fn window<'a>(row: &'a Value, name: &str) -> &'a Value {
    row["headroom"]["windows"]
        .as_array()
        .unwrap_or_else(|| panic!("row has no headroom.windows array: {row}"))
        .iter()
        .find(|w| w["window"] == name)
        .unwrap_or_else(|| panic!("no {name} window in {}", row["headroom"]))
}

pub fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

pub fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}
