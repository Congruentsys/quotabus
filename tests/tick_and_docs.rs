//! CHORE-007 DoD 4-5: the launchd `StartInterval` is a TICK (300 s) documented as such in `packaging/README.md`, with
//! no interval duplicated outside the config; `examples/quotabus.toml` uses `[intervals]`; `docs/DESIGN.md` no longer
//! carries the 15-min cadence or its "≤ 96 calls" cost line.

use std::path::Path;
use std::time::Duration;

use quotabus::{Config, QueryKind};

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

const PLIST: &str = "packaging/launchd/com.congruentsys.quotabus-probe.plist";

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
fn the_plist_start_interval_is_a_300s_tick() {
    let p = read(PLIST);
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
