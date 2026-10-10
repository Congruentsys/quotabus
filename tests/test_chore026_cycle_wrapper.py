"""CHORE-026: the quotabus-cycle wrapper passes only `--config <file>` to `alert`.

From the item (Done when): "The wrapper in `packaging/README.md` passes only `--config <file>` to `alert` [...] with a
test that runs the wrapper with `probe --force --config <file>` against stub `quotabus` binaries [...] and shows both
steps get valid argv".

Method: the wrapper is extracted from the README (never retyped here); its absolute `.../quotabus` path is rewritten
to a stub in a tmp dir. The stub logs its argv and mimics clap for the two subcommands the wrapper calls:
  - `probe` takes `--force` (optional) and `--config <file>`, in any order;
  - `alert` takes `--config <file>` and nothing else (it has no `--force`): any other argument -> exit 2.
Controls: the stub really rejects `alert --force`; the argv checker fails on the pre-fix wrapper and on a wrapper
that skips the alert or drops `--force` from the probe; the wrapper's exit code still reports a failed step.
Fakes only: no real quotabus, key or bus.
"""
import os
import re
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "packaging" / "README.md"
REAL_QB = re.compile(r"/\S*?/quotabus(?=[\s;|&]|$)")  # an absolute path to the quotabus binary (not quotabus-cycle)

# The wrapper as the README carried it before CHORE-026 (the known-bad input for the controls).
PRE_FIX_WRAPPER = (
    "#!/bin/sh\n"
    "# /Users/hankh19/.local/bin/quotabus-cycle — one probe cycle, then the alert over its rows\n"
    "shift\n"
    "export QUOTABUS_CLAUDE_BIN=/opt/homebrew/bin/claude\n"
    '/Users/hankh19/.local/bin/quotabus probe "$@"; rc=$?\n'
    '/Users/hankh19/.local/bin/quotabus alert "$@" || rc=$?\n'
    "exit $rc\n"
)

# A clap-like stub: logs one tab-separated line of argv per call, validates it, exits 0 (or $STUB_FAIL_<SUB>).
STUB = r'''#!/bin/sh
{ printf "%s" "quotabus"; for a in "$@"; do printf "\t%s" "$a"; done; echo; } >> "$STUB_LOG"
sub="$1"; shift
cfg=""
case "$sub" in
  probe)
    while [ $# -gt 0 ]; do
      case "$1" in
        --force) shift ;;
        --config) [ $# -ge 2 ] || { echo "error: a value is required for '--config <CONFIG>'" >&2; exit 2; }; cfg="$2"; shift 2 ;;
        *) echo "error: unexpected argument '$1' found" >&2; exit 2 ;;
      esac
    done
    [ -n "$STUB_FAIL_PROBE" ] && exit "$STUB_FAIL_PROBE"
    ;;
  alert)
    while [ $# -gt 0 ]; do
      case "$1" in
        --config) [ $# -ge 2 ] || { echo "error: a value is required for '--config <CONFIG>'" >&2; exit 2; }; cfg="$2"; shift 2 ;;
        *) echo "error: unexpected argument '$1' found" >&2; exit 2 ;;
      esac
    done
    [ -n "$STUB_FAIL_ALERT" ] && exit "$STUB_FAIL_ALERT"
    ;;
  *) echo "error: unrecognized subcommand '$sub'" >&2; exit 2 ;;
esac
[ -n "$cfg" ] || { echo "stub: no --config given" >&2; exit 2; }
exit 0
'''


# ---------------------------------------------------------------- helpers

def readme_wrapper():
    """The first fenced ```sh block in packaging/README.md that mentions quotabus-cycle (the alert wrapper)."""
    for m in re.finditer(r"```sh\n(.*?)```", README.read_text(), re.S):
        if "quotabus-cycle" in m.group(1):
            return m.group(1)
    return None


def install(tmp_path, wrapper_text):
    """Write the stub and the wrapper (its quotabus paths pointed at the stub); return (wrapper path, log path)."""
    stub = tmp_path / "bin" / "quotabus"
    stub.parent.mkdir()
    stub.write_text(STUB)
    stub.chmod(0o755)
    body = REAL_QB.sub(str(stub), wrapper_text)
    assert str(stub) in body, "the wrapper names no absolute quotabus path to stub"
    wrapper = tmp_path / "quotabus-cycle"
    wrapper.write_text(body)
    wrapper.chmod(0o755)
    return wrapper, tmp_path / "calls.log"


def run_wrapper(tmp_path, wrapper_text, args, **env_extra):
    wrapper, log = install(tmp_path, wrapper_text)
    env = {"PATH": "/usr/bin:/bin", "HOME": str(tmp_path), "STUB_LOG": str(log), **env_extra}
    proc = subprocess.run(["/bin/sh", str(wrapper), *args], capture_output=True, text=True, env=env, timeout=30)
    calls = [line.split("\t")[1:] for line in log.read_text().splitlines()] if log.exists() else []
    return proc, calls


def run_stub(tmp_path, args):
    _, log = install(tmp_path, PRE_FIX_WRAPPER)
    env = {"PATH": "/usr/bin:/bin", "STUB_LOG": str(log)}
    return subprocess.run(["/bin/sh", str(tmp_path / "bin" / "quotabus"), *args],
                          capture_output=True, text=True, env=env, timeout=30)


def check_cycle(proc, calls, cfg, forced):
    """Both steps ran once, in order, each with valid argv; the probe kept --force iff given; the wrapper exited 0."""
    subs = [c[0] for c in calls]
    assert subs == ["probe", "alert"], f"steps run: {calls}"
    probe, alert = calls
    assert probe.count("--config") == 1 and probe[probe.index("--config") + 1] == cfg, probe
    assert ("--force" in probe) == forced, f"probe argv {probe} (forced={forced})"
    assert alert == ["alert", "--config", cfg], f"alert argv {alert}: alert takes only --config <file>"
    assert proc.returncode == 0, f"wrapper rc={proc.returncode} stderr={proc.stderr!r}"


# ---------------------------------------------------------------- the property

def test_readme_carries_the_wrapper():
    block = readme_wrapper()
    assert block is not None, "packaging/README.md carries no ```sh quotabus-cycle wrapper"
    assert REAL_QB.search(block), block


@pytest.mark.parametrize("args", [
    ["probe", "--force", "--config", "CFG"],
    ["probe", "--config", "CFG", "--force"],
], ids=["force-first", "force-last"])
def test_forced_one_shot_gives_both_steps_valid_argv(tmp_path, args):
    cfg = str(tmp_path / "quotabus.toml")
    args = [cfg if a == "CFG" else a for a in args]
    proc, calls = run_wrapper(tmp_path, readme_wrapper(), args)
    check_cycle(proc, calls, cfg, forced=True)


def test_scheduled_unit_argv_still_works(tmp_path):
    """Control: what install.sh renders (`probe --config <file>`), which works before and after the fix."""
    cfg = str(tmp_path / "quotabus.toml")
    proc, calls = run_wrapper(tmp_path, readme_wrapper(), ["probe", "--config", cfg])
    check_cycle(proc, calls, cfg, forced=False)


def test_wrapper_still_reports_a_failed_step(tmp_path):
    """The README's contract: the wrapper exits non-zero if either step failed (a fix must not lose that)."""
    cfg = str(tmp_path / "quotabus.toml")
    for fail in ("STUB_FAIL_PROBE", "STUB_FAIL_ALERT"):
        d = tmp_path / fail
        d.mkdir()
        proc, calls = run_wrapper(d, readme_wrapper(), ["probe", "--force", "--config", cfg], **{fail: "1"})
        assert [c[0] for c in calls] == ["probe", "alert"], (fail, calls)
        assert proc.returncode != 0, f"{fail}: wrapper rc=0 although a step failed"


# ---------------------------------------------------------------- controls

def test_control_stub_rejects_alert_force(tmp_path):
    proc = run_stub(tmp_path, ["alert", "--force", "--config", "x.toml"])
    assert proc.returncode == 2 and "unexpected argument '--force'" in proc.stderr, (proc.returncode, proc.stderr)


def test_control_stub_accepts_valid_argv(tmp_path):
    for args in (["alert", "--config", "x.toml"], ["probe", "--force", "--config", "x.toml"],
                 ["probe", "--config", "x.toml"]):
        d = tmp_path / "-".join(a.strip("-") for a in args)
        d.mkdir()
        proc = run_stub(d, args)
        assert proc.returncode == 0, (args, proc.stderr)


def test_control_checker_fails_on_pre_fix_wrapper(tmp_path):
    """The defect as found in HAZ-005 case 1: the old wrapper hands --force to alert, alert exits 2."""
    cfg = str(tmp_path / "quotabus.toml")
    proc, calls = run_wrapper(tmp_path, PRE_FIX_WRAPPER, ["probe", "--force", "--config", cfg])
    assert proc.returncode == 2 and calls[1] == ["alert", "--force", "--config", cfg], (proc.returncode, calls)
    with pytest.raises(AssertionError):
        check_cycle(proc, calls, cfg, forced=True)


@pytest.mark.parametrize("mutant", [
    # skips the alert entirely
    lambda w: re.sub(r"^.*quotabus alert.*$", "", w, flags=re.M),
    # "fixes" alert by dropping every argument from both steps (the probe loses --force)
    lambda w: w.replace('probe "$@"', 'probe --config "$3"').replace('alert "$@"', 'alert --config "$3"'),
], ids=["no-alert", "probe-loses-force"])
def test_control_checker_fails_on_mutants(tmp_path, mutant):
    cfg = str(tmp_path / "quotabus.toml")
    bad = mutant(PRE_FIX_WRAPPER)
    assert bad != PRE_FIX_WRAPPER
    proc, calls = run_wrapper(tmp_path, bad, ["probe", "--force", "--config", cfg])
    with pytest.raises(AssertionError):
        check_cycle(proc, calls, cfg, forced=True)
