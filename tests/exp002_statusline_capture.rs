//! EXP-002 Plan 2 as amended by SIG-008 ("Both + fallback", Captain 2026-10-08): the Claude Code capture.
//! - `quotabus statusline` is the `statusLine` command: stdin JSON → the cache, and one line printed for the bar. The
//!   first render carries no `rate_limits` (CHORE-002 finding) and must never be written as 0 %.
//! - `quotabus tee` sits in a `claude -p --output-format stream-json` pipe: stdout is stdin, unchanged; its
//!   `rate_limit_event` goes into the SAME cache in the SAME normalised shape (0.19 ↔ 19 %).
//! - `quotabus install-statusline` writes the `statusLine` entry into `~/.claude/settings.json` (Q9).
//!
//! HOME is a temp dir in every test; neither command needs a config.

mod exp002;

use exp002::*;
use serde_json::Value;

fn statusline(home: &std::path::Path, stdin: &[u8]) -> std::process::Output {
    let mut cmd = bin(
        home,
        &[("HOME", home.to_str().unwrap()), ("PATH", "/usr/bin:/bin")],
    );
    cmd.arg("statusline");
    run(cmd, stdin)
}

fn tee(home: &std::path::Path, stdin: &[u8]) -> std::process::Output {
    let mut cmd = bin(
        home,
        &[("HOME", home.to_str().unwrap()), ("PATH", "/usr/bin:/bin")],
    );
    cmd.arg("tee");
    run(cmd, stdin)
}

fn json_bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

fn window_of(cache: &Value, name: &str) -> (f64, i64) {
    let w = &cache[name];
    (
        w["used_percentage"]
            .as_f64()
            .unwrap_or_else(|| panic!("{name}.used_percentage missing: {cache}")),
        w["resets_at"]
            .as_i64()
            .unwrap_or_else(|| panic!("{name}.resets_at not epoch seconds: {cache}")),
    )
}

// ── statusline ───────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn statusline_writes_the_measured_windows_to_the_cache() {
    let home = tempfile::tempdir().unwrap();
    let before = now_s();
    let out = statusline(home.path(), &json_bytes(&statusline_json()));
    let after = now_s();
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let c = read_json(&cache_path(home.path()));
    assert_eq!(c["source"], "statusline", "{c}");
    let (p5, r5) = window_of(&c, "five_hour");
    let (p7, r7) = window_of(&c, "seven_day");
    assert!(approx(p5, 19.0, 1e-9), "five_hour {p5}");
    assert_eq!(r5, 1791495000);
    assert!(approx(p7, 9.0, 1e-9), "seven_day {p7}");
    assert_eq!(r7, 1792051200);
    let at = parse_ts(c["captured_at"].as_str().expect("captured_at")).timestamp();
    assert!(
        (before - 1..=after + 1).contains(&at),
        "captured_at {at} is when the hook ran ({before}..{after})"
    );
}

#[test]
fn statusline_prints_one_status_line_with_the_usage() {
    let home = tempfile::tempdir().unwrap();
    let out = statusline(home.path(), &json_bytes(&statusline_json()));
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        s.trim_end().lines().count(),
        1,
        "one line for the bar: {s:?}"
    );
    assert!(s.contains("19%"), "the line shows the 5h usage: {s:?}");
}

#[test]
fn the_first_render_without_rate_limits_writes_no_cache() {
    let home = tempfile::tempdir().unwrap();
    let out = statusline(home.path(), &json_bytes(&first_render_json()));
    assert_eq!(
        rc(&out),
        0,
        "a statusLine hook never fails the bar: {}",
        text(&out)
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "it still prints a line"
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("0%"),
        "an unknown usage is not printed as 0%: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !cache_path(home.path()).exists(),
        "absent rate_limits = not yet known: nothing is written (never 0 %)"
    );
    // control: the same home, the same JSON WITH rate_limits, does write — the absence above is the feature
    let out = statusline(home.path(), &json_bytes(&statusline_json()));
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(
        cache_path(home.path()).exists(),
        "the control render must write the cache"
    );
}

#[test]
fn the_first_render_leaves_an_earlier_capture_untouched() {
    let home = tempfile::tempdir().unwrap();
    let earlier = cache_entry(
        "stream_json",
        60,
        Some((42.0, 1791495000)),
        Some((7.0, 1792051200)),
    );
    write_json(&cache_path(home.path()), &earlier);
    let bytes = std::fs::read(cache_path(home.path())).unwrap();
    let out = statusline(home.path(), &json_bytes(&first_render_json()));
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert_eq!(
        std::fs::read(cache_path(home.path())).unwrap(),
        bytes,
        "a render with no rate_limits must not overwrite a real capture"
    );
}

#[test]
fn a_measured_zero_is_written_as_zero() {
    // the negative control for "never 0 %": a window the JSON REPORTS as 0 is a measurement, and is kept
    let home = tempfile::tempdir().unwrap();
    let mut v = statusline_json();
    v["rate_limits"]["five_hour"]["used_percentage"] = serde_json::json!(0);
    let out = statusline(home.path(), &json_bytes(&v));
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let (p5, _) = window_of(&read_json(&cache_path(home.path())), "five_hour");
    assert_eq!(p5, 0.0);
}

#[test]
fn the_cache_holds_no_path_session_or_prompt_from_the_hook_json() {
    let home = tempfile::tempdir().unwrap();
    let out = statusline(home.path(), &json_bytes(&statusline_json()));
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let raw = std::fs::read_to_string(cache_path(home.path())).unwrap();
    for private in [
        "qb-secret-transcript-path",
        "qb-private-cwd",
        "qb-private-scratch",
        "sess-0f1e2d3c4b5a69788796a5b4c3d2e1f0",
        "prompt-0123456789abcdef",
        "total_cost_usd",
    ] {
        assert!(
            !raw.contains(private),
            "the cache copies {private:?} from the hook JSON:\n{raw}"
        );
    }
    // control: the strings are really in the input, so their absence above is not by construction
    let input = String::from_utf8(json_bytes(&statusline_json())).unwrap();
    assert!(input.contains("qb-private-cwd") && input.contains("total_cost_usd"));
}

#[test]
fn statusline_survives_garbage_on_stdin() {
    let home = tempfile::tempdir().unwrap();
    let out = statusline(home.path(), b"not json {");
    assert_eq!(
        rc(&out),
        0,
        "a statusLine hook must not fail: {}",
        text(&out)
    );
    assert!(!cache_path(home.path()).exists(), "garbage writes nothing");
}

#[test]
fn statusline_needs_no_config() {
    // the hook runs from Claude Code with no QUOTABUS_CONFIG and a cwd with no quotabus.toml
    let home = tempfile::tempdir().unwrap();
    assert!(!home.path().join("quotabus.toml").exists());
    let out = statusline(home.path(), &json_bytes(&statusline_json()));
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(cache_path(home.path()).exists(), "{}", text(&out));
}

// ── tee (claude -p stream-json) ──────────────────────────────────────────────────────────────────────────────────

fn stream_input() -> Vec<u8> {
    let mut lines = stream_lines();
    // a long line and a non-JSON line pass too; the last line has no newline
    lines.insert(
        1,
        format!(r#"{{"type":"assistant","pad":"{}"}}"#, "x".repeat(200_000)),
    );
    lines.push("plain text, not JSON".to_string());
    let mut s = lines.join("\n");
    s.push_str("\n{\"type\":\"tail\"}");
    s.into_bytes()
}

#[test]
fn tee_passes_stdin_through_unchanged() {
    let home = tempfile::tempdir().unwrap();
    let input = stream_input();
    let out = tee(home.path(), &input);
    assert_eq!(rc(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        out.stdout.len(),
        input.len(),
        "stdout length differs from stdin"
    );
    assert!(out.stdout == input, "stdout must be stdin byte for byte");
}

#[test]
fn tee_captures_the_rate_limit_event_in_the_normalised_shape() {
    let home = tempfile::tempdir().unwrap();
    let out = tee(home.path(), &stream_input());
    assert_eq!(rc(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    let c = read_json(&cache_path(home.path()));
    assert_eq!(c["source"], "stream_json", "{c}");
    let (p5, r5) = window_of(&c, "five_hour");
    let (p7, r7) = window_of(&c, "seven_day");
    assert!(approx(p5, 19.0, 1e-6), "utilization 0.19 is 19 %, got {p5}");
    assert!(approx(p7, 9.0, 1e-6), "utilization 0.09 is 9 %, got {p7}");
    assert_eq!((r5, r7), (1791495000, 1792051200));
    assert!(
        c["captured_at"]
            .as_str()
            .is_some_and(|s| !parse_ts(s).to_rfc3339().is_empty())
    );
}

#[test]
fn tee_and_statusline_agree_on_the_same_measurement() {
    // CHORE-002: run B's event and run C's statusLine were the same account in the same minute
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    assert_eq!(
        rc(&statusline(a.path(), &json_bytes(&statusline_json()))),
        0
    );
    assert_eq!(rc(&tee(b.path(), stream_lines().join("\n").as_bytes())), 0);
    let ca = read_json(&cache_path(a.path()));
    let cb = read_json(&cache_path(b.path()));
    for w in ["five_hour", "seven_day"] {
        let (pa, ra) = window_of(&ca, w);
        let (pb, rb) = window_of(&cb, w);
        assert!(approx(pa, pb, 1e-6), "{w}: statusline {pa} vs stream {pb}");
        assert_eq!(ra, rb, "{w} resets_at");
    }
    let keys = |v: &Value| {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(keys(&ca), keys(&cb), "one cache shape for both sources");
}

#[test]
fn a_stream_without_a_rate_limit_event_writes_no_cache() {
    let home = tempfile::tempdir().unwrap();
    let lines: Vec<String> = stream_lines()
        .into_iter()
        .filter(|l| !l.contains("rate_limit_event"))
        .collect();
    let input = lines.join("\n");
    let out = tee(home.path(), input.as_bytes());
    assert_eq!(rc(&out), 0);
    assert_eq!(out.stdout, input.as_bytes(), "still a pass-through");
    assert!(
        !cache_path(home.path()).exists(),
        "no event, no capture (never 0 %)"
    );
}

#[test]
fn an_event_without_unified_windows_writes_no_cache() {
    let home = tempfile::tempdir().unwrap();
    let ev = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1791495000,"rateLimitType":"five_hour"}}"#;
    let out = tee(home.path(), ev.as_bytes());
    assert_eq!(rc(&out), 0);
    assert_eq!(out.stdout, ev.as_bytes());
    assert!(
        !cache_path(home.path()).exists(),
        "an event with no utilization is not a usage reading"
    );
}

#[test]
fn a_later_event_replaces_an_earlier_capture() {
    let home = tempfile::tempdir().unwrap();
    write_json(
        &cache_path(home.path()),
        &cache_entry(
            "statusline",
            120,
            Some((3.0, 1791495000)),
            Some((1.0, 1792051200)),
        ),
    );
    assert_eq!(rc(&tee(home.path(), RATE_LIMIT_EVENT.as_bytes())), 0);
    let (p5, _) = window_of(&read_json(&cache_path(home.path())), "five_hour");
    assert!(approx(p5, 19.0, 1e-6), "the newer reading wins: {p5}");
}

// ── install-statusline ───────────────────────────────────────────────────────────────────────────────────────────

fn install(home: &std::path::Path) -> std::process::Output {
    let mut cmd = bin(
        home,
        &[("HOME", home.to_str().unwrap()), ("PATH", "/usr/bin:/bin")],
    );
    cmd.arg("install-statusline");
    run(cmd, b"")
}

fn settings(home: &std::path::Path) -> std::path::PathBuf {
    home.join(".claude/settings.json")
}

#[test]
fn install_adds_the_statusline_and_keeps_every_other_key() {
    let home = tempfile::tempdir().unwrap();
    let before = serde_json::json!({"theme": "dark", "effortLevel": "high", "env": {"A": "1"},
                                    "permissions": {"allow": ["Bash(ls:*)"]}});
    write_json(&settings(home.path()), &before);
    let out = install(home.path());
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let after = read_json(&settings(home.path()));
    for (k, v) in before.as_object().unwrap() {
        assert_eq!(&after[k], v, "key {k} changed");
    }
    assert_eq!(after["statusLine"]["type"], "command", "{after}");
    let cmd = after["statusLine"]["command"]
        .as_str()
        .expect("statusLine.command");
    assert!(
        cmd.contains("quotabus") && cmd.trim_end().ends_with("statusline"),
        "the command runs `quotabus statusline`: {cmd:?}"
    );
}

#[test]
fn install_is_idempotent() {
    let home = tempfile::tempdir().unwrap();
    write_json(
        &settings(home.path()),
        &serde_json::json!({"theme": "dark"}),
    );
    assert_eq!(rc(&install(home.path())), 0);
    let once = std::fs::read(settings(home.path())).unwrap();
    let out = install(home.path());
    assert_eq!(
        rc(&out),
        0,
        "re-installing our own entry is fine: {}",
        text(&out)
    );
    assert_eq!(std::fs::read(settings(home.path())).unwrap(), once);
}

#[test]
fn install_creates_settings_when_there_are_none() {
    let home = tempfile::tempdir().unwrap();
    let out = install(home.path());
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let s = read_json(&settings(home.path()));
    assert_eq!(s["statusLine"]["type"], "command", "{s}");
}

#[test]
fn install_refuses_to_replace_someone_elses_statusline() {
    let home = tempfile::tempdir().unwrap();
    write_json(
        &settings(home.path()),
        &serde_json::json!({"statusLine": {"type": "command", "command": "~/bin/my-own-bar.sh"}}),
    );
    let bytes = std::fs::read(settings(home.path())).unwrap();
    let out = install(home.path());
    // rc 1, a refusal: a panic (101) or a success (0) is not one
    assert_eq!(
        rc(&out),
        1,
        "a foreign statusLine is refused, not overwritten: {}",
        text(&out)
    );
    assert_eq!(
        std::fs::read(settings(home.path())).unwrap(),
        bytes,
        "file unchanged"
    );
    assert!(
        text(&out).contains("statusLine"),
        "the refusal names what is in the way: {}",
        text(&out)
    );
}
