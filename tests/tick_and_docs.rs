//! CHORE-007 DoD 4-5: the launchd `StartInterval` is a TICK (300 s) documented as such in `packaging/README.md`, with
//! no interval duplicated outside the config; `examples/quotabus.toml` uses `[intervals]`; `docs/DESIGN.md` no longer
//! carries the 15-min cadence or its "≤ 96 calls" cost line.
//!
//! CHORE-008 (ruled test edit): the fleet plist is no longer a file in `packaging/launchd/`; `packaging/install.sh`
//! renders it. The fleet unit is therefore the README's Mini install command rendered with `--render-to <dir>` (a dry
//! run: nothing connects; ssh/scp/launchctl/… on PATH are fakes that log and fail). The assertion is unchanged: the
//! fleet unit's StartInterval is the 300 s tick.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use quotabus::{Config, QueryKind};

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

const PLIST_NAME: &str = "com.congruentsys.quotabus-probe.plist";

/// The fleet's Mini install command recorded in `packaging/README.md`: the first line that runs `install.sh` with
/// `--host <…mini…>`, without a leading `$ ` prompt. None if the README records none.
fn readme_mini_command(readme: &str) -> Option<String> {
    readme.lines().find_map(|l| {
        let l = l.trim().trim_start_matches('$').trim();
        let mut words = l.split_whitespace();
        let runs_install = l.contains("install.sh");
        let host_is_mini = std::iter::from_fn(|| words.next())
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| w[0] == "--host" && w[1].to_lowercase().contains("mini"));
        (runs_install && host_is_mini).then(|| l.to_string())
    })
}

/// The fleet plist: the README's Mini command rendered with `--render-to` into a temp dir, read back as text.
fn render_fleet_plist() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cmd = readme_mini_command(&read("packaging/README.md")).expect(
        "packaging/README.md records the Mini install command (`install.sh … --host mini …`)",
    );
    let tmp = tempfile::tempdir().unwrap();
    let (bin, out, home) = (
        tmp.path().join("bin"),
        tmp.path().join("out"),
        tmp.path().join("home"),
    );
    for d in [&bin, &out, &home] {
        std::fs::create_dir_all(d).unwrap();
    }
    let log = tmp.path().join("fake-calls.log");
    for tool in [
        "ssh",
        "scp",
        "rsync",
        "launchctl",
        "systemctl",
        "loginctl",
        "sudo",
    ] {
        let p = bin.join(tool);
        std::fs::write(
            &p,
            format!(
                "#!/bin/sh\necho \"{tool} $*\" >> \"{}\"\nexit 97\n",
                log.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let o = Command::new("bash")
        .arg("-c")
        .arg(format!("{cmd} --render-to \"$1\""))
        .arg("bash")
        .arg(&out)
        .current_dir(root)
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("HOME", &home)
        .env("USER", "tester")
        .output()
        .expect("run bash");
    assert!(
        o.status.success(),
        "rendering the README's Mini command failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!log.exists(), "a dry run called an installer tool");
    let p = out.join(PLIST_NAME);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The `<integer>` after `<key>StartInterval</key>`, or None.
fn start_interval(plist: &str) -> Option<u64> {
    let after = plist.split("<key>StartInterval</key>").nth(1)?;
    let open = after.find("<integer>")? + "<integer>".len();
    let close = after.find("</integer>")?;
    after.get(open..close)?.trim().parse().ok()
}

#[test]
fn control_start_interval_reads_a_known_plist() {
    assert_eq!(
        start_interval("<key>StartInterval</key>\n    <integer>900</integer>"),
        Some(900)
    );
    assert_eq!(start_interval("<key>RunAtLoad</key><true/>"), None);
}

#[test]
fn control_readme_mini_command_needs_a_mini_host_line() {
    assert_eq!(
        readme_mini_command("Run `packaging/install.sh --where local`."),
        None
    );
    assert_eq!(
        readme_mini_command("packaging/install.sh --host buoy --user a --home /h"),
        None
    );
    assert_eq!(
        readme_mini_command(
            "```\n$ packaging/install.sh --host mini --user hankh19 --interval 300\n```"
        )
        .as_deref(),
        Some("packaging/install.sh --host mini --user hankh19 --interval 300")
    );
}

#[test]
fn the_plist_start_interval_is_a_300s_tick() {
    let p = render_fleet_plist();
    assert_eq!(
        start_interval(&p),
        Some(300),
        "StartInterval is the 300 s tick"
    );
}

#[test]
fn the_packaging_readme_describes_the_tick_and_no_probe_interval() {
    let r = read("packaging/README.md");
    assert!(
        r.contains("tick"),
        "README calls StartInterval a tick:\n{r}"
    );
    assert!(r.contains("300"), "README names the 300 s tick:\n{r}");
    for stale in ["900", "15 min", "[probe] interval"] {
        assert!(
            !r.contains(stale),
            "README still carries {stale:?}: the interval lives only in the config\n{r}"
        );
    }
}

#[test]
fn the_example_config_uses_intervals_and_parses() {
    let text = read("examples/quotabus.toml");
    assert!(
        text.contains("[intervals]"),
        "examples/quotabus.toml has an [intervals] table"
    );
    let c = Config::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/quotabus.toml"))
        .expect("examples/quotabus.toml parses");
    assert_eq!(
        c.interval_for(QueryKind::Api),
        Duration::from_secs(12 * 3600)
    );
    assert_eq!(
        c.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * 3600)
    );
}

#[test]
fn design_no_longer_carries_the_15_minute_cadence() {
    let d = read("docs/DESIGN.md");
    assert!(
        !d.contains("≤ 96 calls"),
        "DESIGN §4 still has the 15-min cost line"
    );
    assert!(
        !d.contains("interval = \"15m\""),
        "DESIGN §3 still has interval = \"15m\""
    );
    assert!(
        d.contains("[intervals]"),
        "DESIGN §3 shows the [intervals] table"
    );
}
