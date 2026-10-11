//! CHORE-030: `quotabus status` renders a window's "resets in" from `reset_at − now` (the reader's clock, as AGE
//! already is), falling back to `reset_in_s − age` when `reset_at` is absent. A reset already in the past never
//! renders a negative duration (nor a positive one). The record and `status --json` are unchanged: `reset_in_s`
//! stays `reset_at − checked_at` (DESIGN §2, `headroom` row).
//!
//! `status` reads the real clock and has no test seam, so these tests tolerate it: each row is written relative to
//! the real `Utc::now()` a moment before the CLI runs, and each assertion is a band (e.g. 47–50 min for a reset
//! 50 min away), never an exact string. The bands are disjoint from what the old checked_at-relative rendering prints
//! (60m, 110m), so the old rendering fails them; `checker_controls` shows each checker rejects that old text.
//!
//! Fakes only: the file backend in a temp dir, a fake key name with no value needed. No bus, no real key.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use common::{rc, run_cli, text};
use quotabus::{Headroom, Kind, Record, State, Window, record_key};
use serde_json::Value;

const T: Duration = Duration::from_secs(30);
const WIN: &str = "five_hour";

fn rfc3339(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A subscription row for service `id` (its slot is its id), checked `age_s` before now, with one `five_hour` window
/// at 40 % carrying `reset_in_s` and (optionally) `reset_at`.
fn sub_row(id: &str, age_s: i64, reset_in_s: i64, reset_at: Option<DateTime<Utc>>) -> Record {
    let checked_at = Utc::now() - chrono::Duration::seconds(age_s);
    let key = record_key(Kind::Subscription, "github", "acct", id);
    let mut r = common::record(&key, State::Ok, checked_at, 6 * 3600);
    r.family = "openai".into();
    r.latency_ms = None;
    let reset_at = reset_at.map(rfc3339);
    r.headroom = Some(Headroom {
        requests_remaining: None,
        tokens_remaining: None,
        reset_at: reset_at.clone(),
        window_pct: Some(40.0),
        window: Some(WIN.into()),
        windows: vec![Window {
            window: WIN.into(),
            used_pct: 40.0,
            reset_at,
            reset_in_s: Some(reset_in_s),
            pace: None,
        }],
    });
    r
}

struct Fixture {
    _dir: tempfile::TempDir,
    config: PathBuf,
    state: PathBuf,
}

/// The file backend holding `rows`, one `kind = "subscription"` service per row (service id = the row's model).
fn fixture(rows: &[Record]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir(&state).unwrap();
    let mut services = String::new();
    for r in rows {
        std::fs::write(
            state.join(format!("{}.json", r.key)),
            serde_json::to_vec(r).unwrap(),
        )
        .unwrap();
        services.push_str(&format!(
            "[[service]]\nid = {:?}\nkind = \"subscription\"\nprovider = \"github\"\nfamily = \"openai\"\n\
             account = \"acct\"\nsecret = \"QB_UNUSED\"\nsources = [\"copilot_internal\"]\n\n",
            r.model
        ));
    }
    let config = dir.path().join("quotabus.toml");
    std::fs::write(
        &config,
        format!(
            "[file]\ndir = {:?}\n\n{services}",
            state.display().to_string()
        ),
    )
    .unwrap();
    Fixture {
        _dir: dir,
        config,
        state,
    }
}

fn status(config: &Path, args: &[&str]) -> String {
    let mut a = vec!["status"];
    a.extend_from_slice(args);
    let out = run_cli(config, &a, &[("PATH", "/usr/bin:/bin")], T);
    assert!(
        rc(&out) == 0 || rc(&out) == 1,
        "status {args:?} rc {}: {}",
        rc(&out),
        text(&out)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// The text inside the parentheses that follows `five_hour 40%` on service `svc`'s table line, or None when the
/// window renders no parenthesised reset at all.
fn reset_text(table: &str, svc: &str) -> Option<String> {
    let line = table
        .lines()
        .find(|l| l.split_whitespace().next() == Some(svc))
        .unwrap_or_else(|| panic!("no table line for {svc}:\n{table}"));
    let at = line
        .find(&format!("{WIN} 40%"))
        .unwrap_or_else(|| panic!("no `{WIN} 40%` on {svc}'s line: {line:?}"));
    let rest = line[at + WIN.len() + " 40%".len()..].trim_start();
    let inner = rest.strip_prefix('(')?;
    Some(inner[..inner.find(')')?].to_string())
}

/// Seconds a `resets in <N><unit>` text claims, or None when it is not that shape (e.g. `reset due`).
fn claimed_s(reset: &str) -> Option<i64> {
    let d = reset.strip_prefix("resets in ")?.trim();
    let (n, unit) = d.split_at(d.find(|c: char| !c.is_ascii_digit() && c != '-')?);
    let n: i64 = n.parse().ok()?;
    let mult = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        _ => return None,
    };
    Some(n * mult)
}

/// The window claims a reset between `lo_m` and `hi_m` minutes away (inclusive).
fn check_minutes(reset: Option<&str>, lo_m: i64, hi_m: i64) -> Result<(), String> {
    let r = reset.ok_or("no `(resets in …)` rendered")?;
    let s = claimed_s(r).ok_or_else(|| format!("not a `resets in <N><unit>`: {r:?}"))?;
    if (lo_m * 60..=hi_m * 60).contains(&s) {
        Ok(())
    } else {
        Err(format!(
            "rendered ({r}) = {s}s, want {lo_m}–{hi_m} min from NOW"
        ))
    }
}

/// A reset already past: no negative duration (no `-`), and no positive time left either.
fn check_past(reset: Option<&str>) -> Result<(), String> {
    let Some(r) = reset else { return Ok(()) };
    if r.contains('-') {
        return Err(format!("negative duration rendered: ({r})"));
    }
    match claimed_s(r) {
        Some(s) if s > 0 => Err(format!(
            "a reset already past renders ({r}), a positive time left (counted from checked_at?)"
        )),
        _ => Ok(()),
    }
}

// ── the checkers can fail ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn checker_controls() {
    // known answers
    assert_eq!(claimed_s("resets in 50m"), Some(3000));
    assert_eq!(claimed_s("resets in 2h"), Some(7200));
    assert_eq!(claimed_s("resets in 0s"), Some(0));
    assert_eq!(claimed_s("reset due"), None);
    let t = "SERVICE  MODEL  STATE  AGE  SOURCE    DETAIL\nsvc  svc  ok  10m  official  five_hour 40% (resets in 49m)\n";
    assert_eq!(reset_text(t, "svc").as_deref(), Some("resets in 49m"));
    assert_eq!(check_minutes(Some("resets in 49m"), 47, 50), Ok(()));
    assert_eq!(check_past(Some("reset due")), Ok(()));
    assert_eq!(check_past(Some("resets in 0s")), Ok(()));
    // negative controls: the old checked_at-relative rendering and a negative duration are rejected
    assert!(check_minutes(Some("resets in 60m"), 47, 50).is_err());
    assert!(check_minutes(None, 47, 50).is_err());
    assert!(check_minutes(Some("resets in 59m"), 59, 60).is_ok());
    assert!(check_minutes(Some("resets in 49m"), 59, 60).is_err());
    assert!(check_past(Some("resets in 110m")).is_err());
    assert!(check_past(Some("resets in -10m")).is_err());
    assert!(check_past(Some("resets in -600s")).is_err());
}

// ── Done when 1: counted from now, not checked_at ─────────────────────────────────────────────────────────────────

/// A row checked 10 min ago, `reset_in_s` = 3600 and a consistent `reset_at` (= checked_at + 1 h = now + 50 min)
/// shows ≈ 50m. The old rendering shows 60m and fails this.
#[test]
fn checked_ten_min_ago_shows_about_fifty_minutes() {
    let checked = Utc::now() - chrono::Duration::seconds(600);
    let row = sub_row(
        "tenago",
        600,
        3600,
        Some(checked + chrono::Duration::seconds(3600)),
    );
    let f = fixture(&[row]);
    let table = status(&f.config, &[]);
    let r = reset_text(&table, "tenago");
    if let Err(e) = check_minutes(r.as_deref(), 47, 50) {
        panic!("CHORE-030: {e}\n{table}");
    }
}

/// Control: a row checked at now, `reset_in_s` = 3600, `reset_at` = now + 1 h still shows 60m (59m once a second
/// has passed). Green before and after the fix; it goes red if the fix subtracts the age twice or drops the reset.
#[test]
fn control_checked_now_shows_sixty_minutes() {
    let now = Utc::now();
    let row = sub_row(
        "justnow",
        0,
        3600,
        Some(now + chrono::Duration::seconds(3600)),
    );
    let f = fixture(&[row]);
    let table = status(&f.config, &[]);
    let r = reset_text(&table, "justnow");
    if let Err(e) = check_minutes(r.as_deref(), 59, 60) {
        panic!("control: {e}\n{table}");
    }
}

/// `reset_at` is the authority when present (Plan: "from `reset_at − now`"): a row checked 10 min ago whose
/// `reset_at` is now + 20 min renders ≈ 20m, whatever `reset_in_s` says.
#[test]
fn reset_at_is_read_against_now() {
    let now = Utc::now();
    let row = sub_row(
        "byat",
        600,
        3600,
        Some(now + chrono::Duration::seconds(1200)),
    );
    let f = fixture(&[row]);
    let table = status(&f.config, &[]);
    let r = reset_text(&table, "byat");
    if let Err(e) = check_minutes(r.as_deref(), 17, 20) {
        panic!("CHORE-030: {e}\n{table}");
    }
}

/// The Plan's fallback: a window with `reset_in_s` = 3600 and NO `reset_at`, checked 10 min ago, renders
/// `reset_in_s − age` ≈ 50m.
#[test]
fn fallback_without_reset_at_subtracts_age() {
    let row = sub_row("noat", 600, 3600, None);
    let f = fixture(&[row]);
    let table = status(&f.config, &[]);
    let r = reset_text(&table, "noat");
    if let Err(e) = check_minutes(r.as_deref(), 47, 50) {
        panic!("CHORE-030 fallback: {e}\n{table}");
    }
}

// ── Done when 2: a past reset is never a negative duration ────────────────────────────────────────────────────────

/// Checked 2 h ago, `reset_at` = checked_at + 110 min = 10 min BEFORE now, `reset_in_s` = 6600. No `-` and no
/// positive time left (the old rendering says "resets in 110m").
#[test]
fn past_reset_at_shows_no_negative_duration() {
    let checked = Utc::now() - chrono::Duration::seconds(7200);
    let row = sub_row(
        "past",
        7200,
        6600,
        Some(checked + chrono::Duration::seconds(6600)),
    );
    let f = fixture(&[row]);
    let table = status(&f.config, &[]);
    let r = reset_text(&table, "past");
    if let Err(e) = check_past(r.as_deref()) {
        panic!("CHORE-030: {e}\n{table}");
    }
}

/// The same past reset via the fallback (no `reset_at`): `reset_in_s` − age < 0 is never rendered negative.
#[test]
fn past_reset_via_fallback_shows_no_negative_duration() {
    let row = sub_row("pastnoat", 7200, 6600, None);
    let f = fixture(&[row]);
    let table = status(&f.config, &[]);
    let r = reset_text(&table, "pastnoat");
    if let Err(e) = check_past(r.as_deref()) {
        panic!("CHORE-030 fallback: {e}\n{table}");
    }
}

// ── Done when 3: --json and the stored row are byte-for-byte unchanged ───────────────────────────────────────────

/// `status --json` carries the stored row verbatim (`reset_in_s` 3600, `reset_at` as stored), and running `status`
/// (table and json) leaves the stored file's bytes untouched. Green before and after the fix.
#[test]
fn json_and_stored_row_unchanged() {
    let checked = Utc::now() - chrono::Duration::seconds(600);
    let reset_at = checked + chrono::Duration::seconds(3600);
    let row = sub_row("jsonrow", 600, 3600, Some(reset_at));
    let f = fixture(std::slice::from_ref(&row));
    let file = f.state.join(format!("{}.json", row.key));
    let before = std::fs::read(&file).unwrap();

    let _ = status(&f.config, &[]);
    let json: Value = serde_json::from_str(&status(&f.config, &["--json"])).unwrap();
    assert_eq!(
        std::fs::read(&file).unwrap(),
        before,
        "status rewrote the stored row"
    );

    let entry = json
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["service"] == "jsonrow")
        .unwrap_or_else(|| panic!("no jsonrow entry: {json}"));
    let stored: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(
        entry["row"], stored,
        "--json `row` differs from the stored row"
    );
    let w = &entry["row"]["headroom"]["windows"][0];
    assert_eq!(w["reset_in_s"].as_i64(), Some(3600), "{w}");
    assert_eq!(
        w["reset_at"].as_str(),
        Some(rfc3339(reset_at).as_str()),
        "{w}"
    );
    // the json shape is unchanged: no new field for a now-relative reset
    let keys: Vec<&str> = entry
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    let mut keys = keys;
    keys.sort();
    assert_eq!(
        keys,
        ["age_s", "key", "reason", "row", "service", "verdict"]
    );
}
