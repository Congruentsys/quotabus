"""CHORE-003: the work skills and scripts are quotabus's own, not a copy of the repo they were ported from.

Definition of Done: no skill or script names the source repo, its research programme or its prefix, and nothing
still calls the retired board-write wrapper (yurtle-kanban 3.4.0 pushes `move`, `comment`, `rank` and `voyage add`
to origin by default, #1279). The controls at the bottom show the checker can fail.
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKILLS = ROOT / ".claude" / "skills"

# Assembled from parts so this file does not trip its own check.
FORBIDDEN = [
    re.compile("nusy-" + "replicant"), re.compile(r"\brep" + r"licant-"),
    re.compile(r"\bL" + r"UM\b"), re.compile("research/" + "LUM"),
    re.compile(r"\br" + r"24\b|\br" + r"24[-_]"), re.compile("yk_" + r"push\.sh"),
]


def files():
    out = sorted(SKILLS.rglob("*.md")) + sorted((ROOT / "scripts").glob("*"))
    assert out, "no skills or scripts found"
    return [p for p in out if p.is_file()]


def hits(text):
    return [m.group(0) for rx in FORBIDDEN for m in rx.finditer(text)]


def test_no_skill_or_script_names_the_source_repo():
    bad = {str(p.relative_to(ROOT)): hits(p.read_text()) for p in files()}
    bad = {k: v for k, v in bad.items() if v}
    assert bad == {}, bad


def test_the_retired_wrapper_is_gone():
    assert not (ROOT / "scripts" / ("yk_" + "push.sh")).exists()


def test_the_picker_and_loop_exist_under_quotabus_names():
    for name in ("quotabus-next", "quotabus-loop", "pairit"):
        text = (SKILLS / name / "SKILL.md").read_text()
        assert re.search(rf"(?m)^name: {name}$", text), name


def test_control_the_checker_catches_each_forbidden_name():
    samples = ["nusy-" + "replicant-24", "rep" + "licant-loop", "the L" + "UM programme", "research/" + "LUM/x",
               "scripts/r" + "24_clean.sh", "/tmp/r" + "24-board-x", "scripts/yk_" + "push.sh comment"]
    for s in samples:
        assert hits(s), f"checker misses {s!r}"


def test_control_the_checker_passes_clean_text():
    assert hits("quotabus-loop picks EXP-001; yurtle-kanban move pushes; plum and r245 are fine") == []
