"""HAZ-004: the fleet install command for Mini names the measured user, home and user-owned paths.

From the item (Done when 1-2; decided G3, no sudo, user-owned, reversible):
  - `packaging/README.md`'s fleet command uses `--user hankh19 --home /Users/hankh19`; the binary goes to
    `/Users/hankh19/.local/bin/` (quotabus, and the `quotabus-cycle` wrapper), the config to
    `/Users/hankh19/.config/quotabus/quotabus.toml`.
  - Any other doc that repeats them (packaging/, DESIGN §5, the wiring note) is brought in line: none names the old
    user `admin`, `/Users/admin` or a `/usr/local/{bin,etc}/quotabus` path.
  - `install.sh --render-to <dir>` run with the README's command (extracted from the README, never retyped here)
    renders a plist whose ProgramArguments and log paths are all under `/Users/hankh19`. The launcher's own argv
    (`/opt/homebrew/bin/doppler run … --`, measured present on Mini) is the one part of ProgramArguments that is not:
    every absolute path AFTER the launcher's `--`, and both log paths, are under `/Users/hankh19/`.

Fakes only: ssh/scp/rsync/launchctl/systemctl/loginctl/sudo on PATH log and fail; nothing connects to Mini.
Each checker carries a control showing it can fail on the old command.
"""
import os
import plistlib
import re
import shlex
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "packaging" / "install.sh"
README = ROOT / "packaging" / "README.md"
PLIST_NAME = "com.congruentsys.quotabus-probe.plist"
HOME = "/Users/hankh19"
BIN_DIR = f"{HOME}/.local/bin/"
CONFIG = f"{HOME}/.config/quotabus/quotabus.toml"
FAKE_TOOLS = ("ssh", "scp", "rsync", "launchctl", "systemctl", "loginctl", "sudo")
FAKE_KEY = "sk-test-not-a-key"
# The old, unmeasured fleet values (HAZ-004): no doc may carry them.
OLD = re.compile(r"/Users/admin\b|--user\s+admin\b|/usr/local/(bin|etc)/quotabus")
# The old fleet command as the README carried it before HAZ-004 (a known-bad input for the controls).
OLD_COMMAND = ("packaging/install.sh --host mini --user admin --home /Users/admin --os macos --launcher "
               "\"/opt/homebrew/bin/doppler run --project nusy-product-team --config dev --\" "
               "--quotabus /usr/local/bin/quotabus --probe-config /usr/local/etc/quotabus/quotabus.toml --interval 300")


# ---------------------------------------------------------------- helpers

def fleet_command(text):
    """argv after the script path, from the first README line that runs install.sh with --host <…mini…>."""
    for line in text.splitlines():
        line = line.strip().lstrip("$").strip()
        if "install.sh" in line and re.search(r"--host\s+\S*mini\S*", line, re.I):
            argv = shlex.split(line)
            idx = next(i for i, a in enumerate(argv) if a.endswith("install.sh"))
            return argv[idx + 1:]
    return None


def flag(argv, name):
    """The value given for --name in argv (the last one, as a shell parser takes it), or None."""
    vals = [argv[i + 1] for i, a in enumerate(argv[:-1]) if a == name]
    return vals[-1] if vals else None


def check_fleet_argv(argv):
    """The fleet command's target and paths are the decided, user-owned ones."""
    assert argv is not None, "packaging/README.md carries no `install.sh … --host <mini>` line"
    assert flag(argv, "--user") == "hankh19", argv
    assert flag(argv, "--home") == HOME, argv
    qb = flag(argv, "--quotabus")
    assert qb is not None and qb.startswith(BIN_DIR) and Path(qb).name in ("quotabus", "quotabus-cycle"), qb
    assert flag(argv, "--probe-config") == CONFIG, argv


def doc_hits(texts):
    """name -> list of old-value matches."""
    return {name: [m.group(0) for m in OLD.finditer(t)] for name, t in texts.items() if OLD.search(t)}


def docs_in_scope():
    files = [p for p in (ROOT / "packaging").rglob("*.md")]
    files.append(ROOT / "docs" / "DESIGN.md")
    files += [p for p in (ROOT / "docs" / "wiring").rglob("*") if p.is_file()]
    return {str(p.relative_to(ROOT)): p.read_text(errors="replace") for p in files}


def alert_wrapper_block(text):
    """The first fenced ```sh block that mentions quotabus-cycle (the README's alert wrapper), or None."""
    for m in re.finditer(r"```sh\n(.*?)```", text, re.S):
        if "quotabus-cycle" in m.group(1):
            return m.group(1)
    return None


def wrapper_paths(block):
    """Every absolute path naming a quotabus binary in the wrapper block."""
    return re.findall(r"(/\S*?/quotabus(?:-cycle)?)\b", block)


def render(tmp_path, argv):
    fakebin = tmp_path / "fakebin"
    fakebin.mkdir()
    log = tmp_path / "fake-calls.log"
    for tool in FAKE_TOOLS:
        p = fakebin / tool
        p.write_text(f'#!/bin/sh\necho "{tool} $*" >> "{log}"\nexit 97\n')
        p.chmod(0o755)
    home = tmp_path / "home" / "tester"
    home.mkdir(parents=True)
    out = tmp_path / "out"
    out.mkdir()
    env = {"PATH": f"{fakebin}{os.pathsep}/usr/bin:/bin", "HOME": str(home), "USER": "tester", "LANG": "C",
           "NUSY_GLM": FAKE_KEY}
    proc = subprocess.run(["bash", str(SCRIPT), *argv, "--render-to", str(out)], env=env, cwd=str(tmp_path),
                          capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=60)
    return proc, out, log, home


def check_plist_under_home(pl):
    """Every absolute path after the launcher's `--` in ProgramArguments, and both log paths, are under HOME."""
    args = pl["ProgramArguments"]
    assert "--" in args, f"no launcher separator in {args}"
    tail = args[args.index("--") + 1:]
    abs_paths = [a for a in tail if a.startswith("/")]
    assert abs_paths, f"no absolute path after the launcher: {args}"
    bad = [a for a in abs_paths if not a.startswith(HOME + "/")]
    assert not bad, f"ProgramArguments paths not under {HOME}: {bad}"
    assert tail[0].startswith(BIN_DIR), tail[0]
    assert CONFIG in tail, tail
    for key in ("StandardOutPath", "StandardErrorPath"):
        assert pl[key].startswith(HOME + "/"), (key, pl[key])
    assert not any(OLD.search(a) for a in args), args


# ---------------------------------------------------------------- Done when 1: the README's fleet command

def test_readme_fleet_command_uses_measured_user_home_and_user_owned_paths():
    check_fleet_argv(fleet_command(README.read_text()))


def test_readme_alert_wrapper_uses_user_owned_bin_dir():
    text = README.read_text()
    block = alert_wrapper_block(text)
    assert block is not None, "packaging/README.md carries no ```sh quotabus-cycle wrapper"
    paths = wrapper_paths(block)
    assert paths, block
    bad = [p for p in paths if not p.startswith(BIN_DIR)]
    assert not bad, f"wrapper paths not under {BIN_DIR}: {bad}"
    # the install line for the wrapper names the same user-owned path
    assert f"--quotabus {BIN_DIR}quotabus-cycle" in text, "the wrapper's install flag is not under ~/.local/bin"


def test_no_doc_names_admin_or_usr_local_quotabus():
    texts = docs_in_scope()
    assert "packaging/README.md" in texts and "docs/DESIGN.md" in texts
    assert any(k.startswith("docs/wiring/") for k in texts), sorted(texts)
    hits = doc_hits(texts)
    assert not hits, f"old fleet values in docs: {hits}"


# ---------------------------------------------------------------- Done when 2: the rendered plist

def test_rendering_readme_fleet_command_puts_every_path_under_hankh19(tmp_path):
    argv = fleet_command(README.read_text())
    assert argv is not None, "packaging/README.md carries no `install.sh … --host <mini>` line"
    proc, out, log, home = render(tmp_path, argv)
    assert proc.returncode == 0, proc.stderr
    pl = plistlib.loads((out / PLIST_NAME).read_bytes())
    check_plist_under_home(pl)
    # the plan names the install destination under the target home, and nothing remote ran
    assert f"{HOME}/Library/LaunchAgents/{PLIST_NAME}" in proc.stdout, proc.stdout
    assert not log.exists(), f"a dry run called an installer tool: {log.read_text()}"
    assert not (home / "Library").exists(), "a dry run wrote under the local home"
    assert FAKE_KEY not in proc.stdout + (out / PLIST_NAME).read_text()


# ---------------------------------------------------------------- controls: each checker can fail

def test_control_fleet_argv_checker_rejects_the_old_command():
    with pytest.raises(AssertionError):
        check_fleet_argv(fleet_command(OLD_COMMAND))
    # mutation: right user and home, binary still in /usr/local
    good = (f"packaging/install.sh --host mini --user hankh19 --home {HOME} --os macos "
            f"--quotabus {BIN_DIR}quotabus-cycle --probe-config {CONFIG} --interval 300")
    check_fleet_argv(fleet_command(good))  # known answer
    with pytest.raises(AssertionError):
        check_fleet_argv(fleet_command(good.replace(f"{BIN_DIR}quotabus-cycle", "/usr/local/bin/quotabus-cycle")))
    with pytest.raises(AssertionError):
        check_fleet_argv(fleet_command(good.replace(CONFIG, "/usr/local/etc/quotabus/quotabus.toml")))
    with pytest.raises(AssertionError):
        check_fleet_argv(fleet_command(good.replace("--user hankh19", "--user admin")))
    with pytest.raises(AssertionError):
        check_fleet_argv(None)


def test_control_doc_scan_catches_each_old_value():
    for bad in ("--home /Users/admin", "--user admin --home x", "/usr/local/bin/quotabus-cycle",
                "/usr/local/etc/quotabus/quotabus.toml"):
        assert doc_hits({"d": bad}), bad
    assert not doc_hits({"d": f"--user hankh19 --home {HOME} {BIN_DIR}quotabus {CONFIG} /usr/local/bin/nats-server"})


def test_control_wrapper_paths_finds_old_paths():
    old = ('#!/bin/sh\n# /usr/local/bin/quotabus-cycle\nshift\n/usr/local/bin/quotabus probe "$@"; rc=$?\n'
           '/usr/local/bin/quotabus alert "$@" || rc=$?\nexit $rc\n')
    paths = wrapper_paths(old)
    assert paths and not all(p.startswith(BIN_DIR) for p in paths), paths
    new = old.replace("/usr/local/bin/", BIN_DIR)
    assert all(p.startswith(BIN_DIR) for p in wrapper_paths(new)), wrapper_paths(new)
    assert alert_wrapper_block(f"x\n```sh\n{new}```\n") == new


def test_control_plist_checker_rejects_the_old_command_render(tmp_path):
    proc, out, log, _ = render(tmp_path, fleet_command(OLD_COMMAND))
    assert proc.returncode == 0, proc.stderr
    pl = plistlib.loads((out / PLIST_NAME).read_bytes())
    with pytest.raises(AssertionError):
        check_plist_under_home(pl)
    # mutations of a good plist: each field moved off HOME turns the checker red
    good = {
        "ProgramArguments": ["/opt/homebrew/bin/doppler", "run", "--", f"{BIN_DIR}quotabus", "probe", "--config",
                             CONFIG],
        "StandardOutPath": f"{HOME}/Library/Logs/quotabus/probe.out.log",
        "StandardErrorPath": f"{HOME}/Library/Logs/quotabus/probe.err.log",
    }
    check_plist_under_home(good)  # known answer
    for key, value in (
        ("StandardOutPath", "/Users/admin/Library/Logs/quotabus/probe.out.log"),
        ("StandardErrorPath", "/var/log/quotabus/probe.err.log"),
        ("ProgramArguments", [*good["ProgramArguments"][:3], "/usr/local/bin/quotabus", "probe", "--config", CONFIG]),
        ("ProgramArguments", [*good["ProgramArguments"][:6], "/usr/local/etc/quotabus/quotabus.toml"]),
    ):
        with pytest.raises(AssertionError):
            check_plist_under_home({**good, key: value})
