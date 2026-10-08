//! EXP-003 Plan / DESIGN §9 row E3 (light check): the wiring note for nusy-product-team's review skills (`pairit`)
//! and its external-review doc names the reviewer as `$(quotabus select --role review --exclude-family anthropic)`.
//! The note lives in this repo's docs or README; DESIGN.md (which already quotes the line in §9) and the board do not
//! count, so the check does not pass by construction.

use std::path::{Path, PathBuf};

const LINE: &str = "$(quotabus select --role review --exclude-family anthropic)";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn markdown_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            markdown_under(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

/// README.md and every docs/**/*.md except DESIGN.md.
fn candidates() -> Vec<PathBuf> {
    let mut v = vec![root().join("README.md")];
    markdown_under(&root().join("docs"), &mut v);
    v.retain(|p| p != &root().join("docs/DESIGN.md"));
    v
}

fn notes_naming(line: &str) -> Vec<PathBuf> {
    candidates()
        .into_iter()
        .filter(|p| {
            let t = std::fs::read_to_string(p).unwrap_or_default();
            t.contains(line) && t.contains("external-review") && t.contains("pairit")
        })
        .collect()
}

#[test]
fn a_wiring_note_names_the_selector_line() {
    let found = notes_naming(LINE);
    assert!(
        !found.is_empty(),
        "no wiring note (README.md or docs/**/*.md other than DESIGN.md) names {LINE:?} together with \
         `external-review` and `pairit`; looked in {:?}",
        candidates()
    );
}

#[test]
fn control_a_line_no_note_could_hold_is_not_found() {
    // the scan above can fail: a line that is nowhere is not found, and DESIGN.md's own copy is not counted
    assert!(notes_naming("$(quotabus select --role nowhere-EXP003-control)").is_empty());
    let design = std::fs::read_to_string(root().join("docs/DESIGN.md")).unwrap();
    assert!(design.contains(LINE), "DESIGN §9 quotes the line");
    assert!(!candidates().contains(&root().join("docs/DESIGN.md")));
}
