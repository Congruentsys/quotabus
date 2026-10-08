//! EXP-002 Plan 4: every key comes from the environment only and never appears on argv, in a row, a log or an error.
//! The stubs echo the fake tokens back in their error bodies; `quotabus probe` runs at debug log level; stdout,
//! stderr, every stored row and the fake `claude`'s argv are scanned. A control shows the scan finds a planted token.

mod common;
mod exp002;

use std::time::Duration;

use common::leaks;
use exp002::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);

#[test]
fn control_the_scan_finds_a_planted_token() {
    assert!(
        !leaks(
            &format!("error: bad token {FAKE_OAT}"),
            &[FAKE_OAT, FAKE_GH]
        )
        .is_empty()
    );
    assert!(
        !leaks(
            &format!("Authorization: token {FAKE_GH}"),
            &[FAKE_OAT, FAKE_GH]
        )
        .is_empty()
    );
    assert!(
        leaks(
            "auth_failed (cannot_assess:secret_unset) ***",
            &[FAKE_OAT, FAKE_GH]
        )
        .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_token_reaches_output_rows_logs_or_argv() {
    let s = MockServer::start().await;
    // the Claude account answers 401 quoting the token; Copilot answers 401 quoting its token
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string(format!(
            "{{\"type\":\"error\",\"error\":{{\"type\":\"authentication_error\",\"message\":\"invalid bearer {FAKE_OAT}\"}}}}"
        )))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/user"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(format!("{{\"message\":\"Bad credentials {FAKE_GH}\"}}")),
        )
        .mount(&s)
        .await;
    // a second Claude account whose 200 has no headers, so the stream-json fallback runs with the token in its env
    let s2 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(200, None, None, None))
        .mount(&s2)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{}\n{}\n{}",
            dir.path().join("store").display().to_string(),
            claude_both(&s.uri()),
            claude_svc(
                "claude-two",
                "two",
                &s2.uri(),
                &["unified_headers", "stream_json"]
            ),
            copilot_svc(&s.uri(), &["copilot_internal"]),
        ),
    )
    .unwrap();
    let c = cfg.clone();
    let b = bin.display().to_string();
    let out = tokio::task::spawn_blocking(move || {
        common::run_cli(
            &c,
            &["probe"],
            &[
                (CLAUDE_SECRET, FAKE_OAT),
                (GH_SECRET, FAKE_GH),
                ("QUOTABUS_CLAUDE_BIN", b.as_str()),
                ("PATH", "/usr/bin:/bin"),
                ("RUST_LOG", "debug"),
            ],
            T,
        )
    })
    .await
    .unwrap();
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));

    let mut all = common::text(&out);
    let store = dir.path().join("store");
    let mut n = 0;
    for e in std::fs::read_dir(&store).unwrap_or_else(|e| panic!("{}: {e}\n{all}", store.display()))
    {
        all.push_str(&std::fs::read_to_string(e.unwrap().path()).unwrap());
        n += 1;
    }
    assert!(n >= 3, "three subscription rows were written ({n}):\n{all}");
    all.push_str(&claude_log(dir.path()).join("\n"));
    assert!(
        claude_runs(dir.path()) >= 1,
        "the fallback ran, so its argv was scanned"
    );
    assert_eq!(
        leaks(&all, &[FAKE_OAT, FAKE_GH]),
        Vec::<String>::new(),
        "a token leaked:\n{all}"
    );
    assert!(
        all.contains("auth_failed"),
        "the 401s are reported (as auth_failed), just without the token"
    );
}
