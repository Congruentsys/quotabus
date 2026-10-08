//! EXP-002 Plan 1-2, 4 and the DoD's "token-login case reads cannot_assess, never ok": `quotabus agent` reads THIS
//! host's Claude subscription state locally and writes one `subscription.anthropic.<host>.<id>` row.
//! - Source order (DESIGN §4, SIG-008): the capture cache (`statusline` or `stream_json`) when fresh; else the
//!   `~/.claude.json` `cachedUsageUtilization` fallback when fresh; else `unknown` with a `cannot_assess:` reason.
//!   "Fresh" is within the subscription TTL (`[ttl] subscription`, default 15 min) of now.
//! - A host with no `oauthAccount` in `~/.claude.json` is a token login: `unknown`,
//!   `reason = cannot_assess:token_login_no_usage_panel`, never `ok` — even with a leftover usage cache.
//! - The row: `observed_by` = `<host>/quotabus@…`, `checked_at` = when the reading it carries was measured (the
//!   cache's `captured_at`, or `fetchedAtMs`), `ttl_s` = the subscription TTL, and `headroom.windows[]` =
//!   `{window, used_pct, reset_at (RFC 3339), reset_in_s (resets_at − checked_at), pace}` where
//!   `pace = (used_pct / 100) / (elapsed / window_len)`, `elapsed = window_len − (resets_at − checked_at)`,
//!   `window_len` 5 h for `five_hour` and 7 d for `seven_day`.
//! - `probe.source`: `official` for the capture cache, `file` for `~/.claude.json`; `probe.name` the source used.

mod exp002;

use exp002::*;

const H1: &str = "qbhost1";

fn claude_only(h: &Host) -> std::path::PathBuf {
    h.config(CLAUDE_SVC)
}

#[test]
fn a_fresh_capture_reads_ok_with_the_hosts_own_key_and_observed_by() {
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        Some((9.0, now + 3 * 86_400)),
    ));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["state"], "ok", "{r}");
    assert_eq!(r["kind"], "subscription");
    assert_eq!(r["provider"], "anthropic");
    assert_eq!(r["account"], H1, "account {{host}} is the host label");
    assert_eq!(r["family"], "anthropic");
    assert_eq!(r["contract"], "ai-status/1");
    assert!(
        r["observed_by"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{H1}/quotabus@")),
        "observed_by is this host: {}",
        r["observed_by"]
    );
    assert_eq!(
        r["probe"]["source"], "official",
        "the capture is the official source"
    );
    assert_eq!(r["probe"]["name"], "statusline");
    assert!(r["reason"].is_null(), "{r}");
}

#[test]
fn the_row_carries_reset_countdown_and_pace_per_window() {
    let h = Host::new(H1);
    let now = now_s();
    let (r5, r7) = (now + 9000, now + 3 * 86_400);
    let entry = cache_entry("stream_json", 10, Some((19.0, r5)), Some((9.0, r7)));
    let captured = parse_ts(entry["captured_at"].as_str().unwrap()).timestamp();
    h.write_cache(&entry);
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["probe"]["name"], "stream_json");
    assert_eq!(
        parse_ts(r["checked_at"].as_str().unwrap()).timestamp(),
        captured,
        "checked_at is when the reading was captured, so the row's age is the reading's age"
    );
    for (name, pct, resets, len) in [
        ("five_hour", 19.0, r5, 18_000.0),
        ("seven_day", 9.0, r7, 604_800.0),
    ] {
        let w = window(&r, name);
        assert!(approx(f(&w["used_pct"]), pct, 1e-9), "{name}: {w}");
        assert_eq!(
            w["reset_in_s"].as_i64(),
            Some(resets - captured),
            "{name} countdown: {w}"
        );
        assert_eq!(
            parse_ts(w["reset_at"].as_str().expect("reset_at")).timestamp(),
            resets,
            "{name} reset_at: {w}"
        );
        let elapsed = len - (resets - captured) as f64;
        let pace = (pct / 100.0) / (elapsed / len);
        assert!(
            approx(f(&w["pace"]), pace, 1e-6),
            "{name}: pace {} vs {pace}",
            w["pace"]
        );
    }
    // known answer, by hand: 19 % used with 8 990 s of the 18 000 s window gone ≈ 0.380
    assert!(approx(f(&window(&r, "five_hour")["pace"]), 0.3804, 1e-3));
}

#[test]
fn an_absent_window_is_left_out_never_written_as_zero() {
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        10,
        None,
        Some((9.0, now + 86_400)),
    ));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    let ws = r["headroom"]["windows"].as_array().expect("windows");
    assert!(
        ws.iter().all(|w| w["window"] != "five_hour"),
        "a five_hour the capture did not carry is unknown, not 0 %: {ws:?}"
    );
    assert!(approx(f(&window(&r, "seven_day")["used_pct"]), 9.0, 1e-9));
}

#[test]
fn the_ttl_is_the_subscription_kinds() {
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        None,
    ));
    h.agent(&claude_only(&h), &[]);
    assert_eq!(
        must_row(&h.store(), &claude_key(H1))["ttl_s"],
        900,
        "default 3 × 5 min"
    );

    let h = Host::new(H1);
    h.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        None,
    ));
    h.agent(
        &h.config(&format!("[ttl]\nsubscription = \"7m\"\n\n{CLAUDE_SVC}")),
        &[],
    );
    assert_eq!(
        must_row(&h.store(), &claude_key(H1))["ttl_s"],
        420,
        "[ttl] subscription = 7m"
    );
}

#[test]
fn a_stale_capture_falls_back_to_claude_json() {
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        3600,
        Some((19.0, now + 9000)),
        None,
    ));
    h.write_claude_json(&claude_json(Some(LOGIN_UUID), LOGIN_UUID, 30, 88.0, 18.0));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["state"], "ok", "{r}");
    assert_eq!(r["probe"]["name"], "claude_json");
    assert_eq!(r["probe"]["source"], "file");
    assert!(
        approx(f(&window(&r, "five_hour")["used_pct"]), 88.0, 1e-9),
        "the fallback's 88 %, not the stale 19 %"
    );
    assert!(approx(f(&window(&r, "seven_day")["used_pct"]), 18.0, 1e-9));
    let checked = parse_ts(r["checked_at"].as_str().unwrap()).timestamp();
    assert!(
        (now - 32..=now - 28).contains(&checked),
        "checked_at = fetchedAtMs ({checked} vs {})",
        now - 30
    );
}

#[test]
fn control_a_fresh_capture_is_preferred_over_claude_json() {
    // the same files as above with the capture made fresh: the capture wins, so the fallback above is staleness
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        None,
    ));
    h.write_claude_json(&claude_json(Some(LOGIN_UUID), LOGIN_UUID, 30, 88.0, 18.0));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["probe"]["name"], "statusline");
    assert!(approx(f(&window(&r, "five_hour")["used_pct"]), 19.0, 1e-9));
}

#[test]
fn a_missing_capture_falls_back_to_claude_json() {
    let h = Host::new(H1);
    h.write_claude_json(&claude_json(Some(LOGIN_UUID), LOGIN_UUID, 30, 88.0, 18.0));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["state"], "ok", "{r}");
    assert_eq!(r["probe"]["name"], "claude_json");
}

#[test]
fn a_token_login_reads_cannot_assess_never_ok() {
    let h = Host::new(H1);
    // no oauthAccount block: token auth, which never populates the usage panel (account-util-publish.sh:38-46)
    let mut j = claude_json(None, LOGIN_UUID, 30, 88.0, 18.0);
    j.as_object_mut().unwrap().remove("cachedUsageUtilization");
    h.write_claude_json(&j);
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["state"], "unknown", "{r}");
    assert_eq!(r["reason"], "cannot_assess:token_login_no_usage_panel");
}

#[test]
fn a_token_login_with_a_leftover_usage_cache_still_reads_cannot_assess() {
    // a fresh-looking cachedUsageUtilization left by an EARLIER interactive login describes another session's account
    let h = Host::new(H1);
    h.write_claude_json(&claude_json(None, LOGIN_UUID, 30, 88.0, 18.0));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_ne!(r["state"], "ok", "a token login is never ok: {r}");
    assert_eq!(r["reason"], "cannot_assess:token_login_no_usage_panel");
}

#[test]
fn control_the_same_file_with_an_oauth_login_reads_ok() {
    // only the oauthAccount block differs from the case above: it is what makes that case cannot_assess
    let h = Host::new(H1);
    h.write_claude_json(&claude_json(Some(LOGIN_UUID), LOGIN_UUID, 30, 88.0, 18.0));
    h.agent(&claude_only(&h), &[]);
    assert_eq!(must_row(&h.store(), &claude_key(H1))["state"], "ok");
}

#[test]
fn a_usage_cache_from_another_account_is_not_attributed() {
    let h = Host::new(H1);
    h.write_claude_json(&claude_json(
        Some(LOGIN_UUID),
        "00000000-0000-4000-8000-0000000000ff",
        30,
        88.0,
        18.0,
    ));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(
        r["state"], "unknown",
        "the cache's accountUuid is not the login's: {r}"
    );
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{r}"
    );
}

#[test]
fn a_stale_claude_json_and_no_capture_read_unknown() {
    let h = Host::new(H1);
    h.write_claude_json(&claude_json(
        Some(LOGIN_UUID),
        LOGIN_UUID,
        4 * 86_400,
        2.0,
        1.0,
    ));
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(
        r["state"], "unknown",
        "a 4-day-old 2 % is not a reading (the DGX2 case): {r}"
    );
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{r}"
    );
}

#[test]
fn nothing_to_read_still_writes_an_unknown_row() {
    // absence-as-health is the failure this project exists to end: a host with no source says so in a row
    let h = Host::new(H1);
    h.agent(&claude_only(&h), &[]);
    let r = must_row(&h.store(), &claude_key(H1));
    assert_eq!(r["state"], "unknown", "{r}");
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{r}"
    );
    assert!(
        r["observed_by"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{H1}/"))
    );
}

#[test]
fn the_login_email_never_reaches_a_row_or_the_output() {
    let h = Host::new(H1);
    h.write_claude_json(&claude_json(Some(LOGIN_UUID), LOGIN_UUID, 30, 88.0, 18.0));
    let out = h.agent(&claude_only(&h), &[]);
    let all: String = rows(&h.store())
        .iter()
        .map(|r| r.to_string())
        .collect::<String>()
        + &text(&out);
    assert!(!all.contains(LOGIN_EMAIL), "the login email leaked:\n{all}");
    assert!(!all.contains("qb.login.person"), "{all}");
}

#[test]
fn default_sources_read_the_official_capture() {
    // a claude-max service with no `sources` uses the official ones (capture, then ~/.claude.json)
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        None,
    ));
    let svc = "[[service]]\nid = \"claude-max\"\nkind = \"subscription\"\nprovider = \"anthropic\"\n\
               family = \"anthropic\"\naccount = \"{host}\"\n";
    h.agent(&h.config(svc), &[]);
    assert_eq!(must_row(&h.store(), &claude_key(H1))["state"], "ok");
}

#[test]
fn each_host_writes_its_own_row() {
    // two hosts, one store (the bus): two rows, each keyed and observed_by its own host
    let a = Host::new("qbhosta");
    let b = Host::new("qbhostb");
    let now = now_s();
    a.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        None,
    ));
    b.write_cache(&cache_entry(
        "statusline",
        10,
        Some((55.0, now + 9000)),
        None,
    ));
    let store = a.store();
    a.agent(&a.config_with_store(&store, CLAUDE_SVC), &[]);
    b.agent(&b.config_with_store(&store, CLAUDE_SVC), &[]);
    let ra = must_row(&store, &claude_key("qbhosta"));
    let rb = must_row(&store, &claude_key("qbhostb"));
    assert!(ra["observed_by"].as_str().unwrap().starts_with("qbhosta/"));
    assert!(rb["observed_by"].as_str().unwrap().starts_with("qbhostb/"));
    assert!(approx(f(&window(&ra, "five_hour")["used_pct"]), 19.0, 1e-9));
    assert!(
        approx(f(&window(&rb, "five_hour")["used_pct"]), 55.0, 1e-9),
        "host b's own reading"
    );
    let subs = rows(&store)
        .into_iter()
        .filter(|r| r["kind"] == "subscription")
        .count();
    assert_eq!(subs, 2, "one subscription row per host");
}

#[test]
fn the_agent_reads_only_subscription_services() {
    // an api service in the same config is the central probe's: the agent never probes it or writes its row
    let h = Host::new(H1);
    let now = now_s();
    h.write_cache(&cache_entry(
        "statusline",
        10,
        Some((19.0, now + 9000)),
        None,
    ));
    let api = "[[service]]\nid = \"ds\"\nkind = \"api\"\nprovider = \"deepseek\"\nfamily = \"deepseek\"\n\
               account = \"acct\"\nbase_url = \"http://127.0.0.1:1/ds\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\n\
               secret = \"QB_DS\"\n";
    h.agent(&h.config(&format!("{api}\n{CLAUDE_SVC}")), &[]);
    let keys: Vec<String> = rows(&h.store())
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_string())
        .collect();
    assert!(
        keys.iter().all(|k| k.starts_with("subscription.")),
        "{keys:?}"
    );
    assert!(keys.contains(&claude_key(H1)), "{keys:?}");
}
