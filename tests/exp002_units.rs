//! EXP-002 Plan 4 / DESIGN §5 packaging: the per-host agent's units — `packaging/launchd/
//! com.congruentsys.quotabus-agent.plist` (macOS: M5, Air) and `packaging/systemd/quotabus-agent.service` + `.timer`
//! (Linux user units: DGX1/2). Each runs `quotabus agent` on a tick no longer than the 5-min subscription default
//! (the interval itself lives only in the config), holds NO secret (the agent needs none: `gh` and Claude Code keep
//! their own logins) and NO hard-coded user path (`/Users/<name>`, `/home/<name>`; systemd's `%h` is the way).

use std::path::Path;

const AGENT_PLIST: &str = "packaging/launchd/com.congruentsys.quotabus-agent.plist";
const SERVICE: &str = "packaging/systemd/quotabus-agent.service";
const TIMER: &str = "packaging/systemd/quotabus-agent.timer";

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Every hard-coded user home path in `text`.
fn user_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for pre in ["/Users/", "/home/"] {
        for (i, _) in text.match_indices(pre) {
            let rest: String = text[i + pre.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                .collect();
            if !rest.is_empty() && rest != "Shared" {
                out.push(format!("{pre}{rest}"));
            }
        }
    }
    out
}

/// Every secret-shaped thing in `text`: a key/token pattern or a launcher that injects one.
fn secrets(text: &str) -> Vec<&'static str> {
    [
        "sk-",
        "ghp_",
        "gho_",
        "github_pat_",
        "Bearer ",
        "doppler",
        "secretspec",
        "API_KEY",
        "_TOKEN",
        "PASSWORD",
    ]
    .into_iter()
    .filter(|p| text.contains(p))
    .collect()
}

/// The `<integer>` after `<key>StartInterval</key>`.
fn start_interval(plist: &str) -> Option<u64> {
    let after = plist.split("<key>StartInterval</key>").nth(1)?;
    let open = after.find("<integer>")? + "<integer>".len();
    let close = after.find("</integer>")?;
    after.get(open..close)?.trim().parse().ok()
}

/// The `<string>`s of the `ProgramArguments` array.
fn program_arguments(plist: &str) -> Vec<String> {
    let Some(after) = plist.split("<key>ProgramArguments</key>").nth(1) else {
        return Vec::new();
    };
    let arr = after.split("</array>").next().unwrap_or("");
    arr.split("<string>")
        .skip(1)
        .filter_map(|s| s.split("</string>").next())
        .map(|s| s.trim().to_string())
        .collect()
}

/// `Key=value` lines of an INI-style systemd unit (the first match).
fn unit_value(unit: &str, key: &str) -> Option<String> {
    unit.lines().map(str::trim).find_map(|l| {
        l.strip_prefix(&format!("{key}="))
            .map(|v| v.trim().to_string())
    })
}

/// A systemd time span (`5min`, `300s`, `300`, `2m`, `1h`) in seconds.
fn span_s(v: &str) -> Option<u64> {
    let v = v.trim();
    let digits: String = v.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n: u64 = digits.parse().ok()?;
    match v[digits.len()..].trim() {
        "" | "s" | "sec" | "second" | "seconds" => Some(n),
        "m" | "min" | "minute" | "minutes" => Some(n * 60),
        "h" | "hr" | "hour" | "hours" => Some(n * 3600),
        _ => None,
    }
}

// ── controls: each checker can fail ──────────────────────────────────────────────────────────────────────────────

#[test]
fn control_the_checkers_see_what_they_must() {
    // the central probe's plist carries `/Users/admin` and a doppler launcher: both checkers must flag it
    let probe = read("packaging/launchd/com.congruentsys.quotabus-probe.plist");
    assert!(
        user_paths(&probe).iter().any(|p| p == "/Users/admin"),
        "{:?}",
        user_paths(&probe)
    );
    assert!(secrets(&probe).contains(&"doppler"));
    assert_eq!(
        user_paths("ExecStart=%h/.local/bin/quotabus agent"),
        Vec::<String>::new()
    );
    assert_eq!(
        user_paths("/home/dgx1/x /Users/Shared/y"),
        vec!["/home/dgx1".to_string()]
    );
    assert_eq!(secrets("Environment=NUSY_GLM=sk-test-x"), vec!["sk-"]);
    assert_eq!(
        start_interval("<key>StartInterval</key><integer>300</integer>"),
        Some(300)
    );
    assert_eq!(
        program_arguments(
            "<key>ProgramArguments</key><array><string>/a/q</string><string>agent</string></array>"
        ),
        vec!["/a/q", "agent"]
    );
    assert_eq!(
        unit_value("[Timer]\nOnUnitActiveSec=5min\n", "OnUnitActiveSec").as_deref(),
        Some("5min")
    );
    assert_eq!(span_s("5min"), Some(300));
    assert_eq!(span_s("1h"), Some(3600));
}

// ── launchd (macOS) ──────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_launchd_agent_runs_quotabus_agent_on_a_tick_of_at_most_5_min() {
    let p = read(AGENT_PLIST);
    assert!(
        p.contains("<string>com.congruentsys.quotabus-agent</string>"),
        "Label:\n{p}"
    );
    let args = program_arguments(&p);
    assert!(
        args.first().is_some_and(|a| a.ends_with("quotabus")),
        "the program is the quotabus binary: {args:?}"
    );
    assert!(
        args.iter().any(|a| a == "agent"),
        "it runs `agent`: {args:?}"
    );
    let tick = start_interval(&p).expect("StartInterval");
    assert!(
        (1..=300).contains(&tick),
        "a tick ≤ the 5-min subscription default: {tick}"
    );
    assert!(p.contains("<key>RunAtLoad</key>"), "it reads once at load");
}

#[test]
fn the_launchd_agent_holds_no_secret_and_no_user_path() {
    let p = read(AGENT_PLIST);
    assert_eq!(secrets(&p), Vec::<&str>::new(), "{p}");
    assert_eq!(user_paths(&p), Vec::<String>::new(), "{p}");
}

#[test]
fn the_launchd_agent_is_a_valid_plist() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(AGENT_PLIST);
    assert!(p.exists(), "{} is missing", p.display());
    if Path::new("/usr/bin/plutil").exists() {
        let o = std::process::Command::new("/usr/bin/plutil")
            .arg("-lint")
            .arg(&p)
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "plutil -lint: {}",
            String::from_utf8_lossy(&o.stdout)
        );
    }
}

// ── systemd (Linux user units) ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_systemd_service_runs_quotabus_agent_once_per_activation() {
    let s = read(SERVICE);
    assert!(s.contains("[Service]"), "{s}");
    assert_eq!(unit_value(&s, "Type").as_deref(), Some("oneshot"), "{s}");
    let exec = unit_value(&s, "ExecStart").expect("ExecStart");
    let mut words = exec.split_whitespace();
    assert!(
        words.next().is_some_and(|w| w.ends_with("quotabus")),
        "{exec}"
    );
    assert!(exec.split_whitespace().any(|w| w == "agent"), "{exec}");
}

#[test]
fn the_systemd_timer_ticks_at_most_every_5_min_and_installs_into_timers_target() {
    let t = read(TIMER);
    assert!(t.contains("[Timer]"), "{t}");
    let tick = unit_value(&t, "OnUnitActiveSec")
        .or_else(|| unit_value(&t, "OnUnitInactiveSec"))
        .expect("OnUnitActiveSec (a tick)");
    let s = span_s(&tick).unwrap_or_else(|| panic!("unparsed span {tick:?}"));
    assert!((1..=300).contains(&s), "tick {s} s ≤ the 5-min default");
    assert!(
        unit_value(&t, "OnBootSec").is_some() || unit_value(&t, "OnStartupSec").is_some(),
        "{t}"
    );
    assert_eq!(
        unit_value(&t, "WantedBy").as_deref(),
        Some("timers.target"),
        "{t}"
    );
}

#[test]
fn the_systemd_units_hold_no_secret_and_no_user_path() {
    for rel in [SERVICE, TIMER] {
        let u = read(rel);
        assert_eq!(secrets(&u), Vec::<&str>::new(), "{rel}:\n{u}");
        assert_eq!(user_paths(&u), Vec::<String>::new(), "{rel}:\n{u}");
    }
}

#[test]
fn the_packaging_readme_names_the_agent_units_and_the_statusline_install() {
    let r = read("packaging/README.md");
    for want in [
        "quotabus-agent.plist",
        "quotabus-agent.service",
        "quotabus-agent.timer",
        "install-statusline",
    ] {
        assert!(
            r.contains(want),
            "packaging/README.md does not name {want}:\n{r}"
        );
    }
}
