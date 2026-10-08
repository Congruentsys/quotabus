"""CHORE-008: the install step chooses where the central probe runs — `local` or a named host over SSH (Captain Q2,
docs/DESIGN.md §3 and §10 Q2; the fleet's choice is Mini).

Interface fixed by these tests (the implementer meets it; `packaging/install.sh`, run here as `bash install.sh …`):

  packaging/install.sh (--where local | --host <name>) [--user <u>] [--home <dir>] [--os macos|linux]
                       [--render-to <dir>] [launcher options of the implementer's choosing]

  - TARGET: `--where local` (this host, as the current user) or `--host <name>` (a named host over SSH). With neither,
    the script ASKS: it reads one answer line from stdin ("local" or a host name); EOF / an empty answer exits
    non-zero with a message.
  - `local`: user and home default to $USER and $HOME; `--os` defaults to this host's OS (uname) and may be given to
    render for the other one. `--host`: `--user` and `--home` are REQUIRED (nothing connects to discover them) and
    `--os` defaults to macos.
  - `--render-to <dir>`: a dry run. It writes the rendered unit file(s) into <dir> and prints the plan to stdout; it
    installs nothing, runs no ssh/scp/rsync/launchctl/systemctl/sudo and writes nothing under the target home. The plan
    names the absolute install destination of each unit, and for `--host`, the host.
      macos -> <dir>/com.congruentsys.quotabus-probe.plist, destination <home>/Library/LaunchAgents/<same name>;
               the plist's StandardOutPath / StandardErrorPath are under <home>/Library/Logs/quotabus/.
      linux -> <dir>/quotabus-probe.service + <dir>/quotabus-probe.timer (a systemd USER unit and its timer),
               destination <home>/.config/systemd/user/<same names>.
  - An unknown `--where` value, an unknown `--os`, or `--host` without `--user`/`--home` exits non-zero and names the
    offending value or the missing flag on stderr.
  - No secret in any rendered unit, the plan, or argv: keys arrive only through the launcher's environment.
  - `/Users/admin` (today's hard-wired Mini path) appears in no file under packaging/ except packaging/README.md, where
    the fleet's Mini install command is recorded: README names Mini as the fleet's central probe host and carries the
    command, a line containing `install.sh` and `--host <…mini…>`. That command, rendered, yields a unit equivalent to
    today's plist (tests/fixtures/chore008_mini_probe.plist, a verbatim copy of the EXP-001 plist as
    CHORE-007 changed it at 71bebe7: StartInterval is the 300 s tick), field by field.

Fakes only: the environment carries a fake key (`sk-test-not-a-key`) and PATH starts with a directory of fake
ssh/scp/rsync/launchctl/systemctl/sudo that log any call and fail. The controls at the bottom show each checker can fail.
"""
import os
import plistlib
import re
import shlex
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "packaging" / "install.sh"
README = ROOT / "packaging" / "README.md"
PACKAGING = ROOT / "packaging"
MINI_FIXTURE = ROOT / "tests" / "fixtures" / "chore008_mini_probe.plist"
PLIST_NAME = "com.congruentsys.quotabus-probe.plist"
SERVICE_NAME = "quotabus-probe.service"
TIMER_NAME = "quotabus-probe.timer"
FAKE_KEY = "sk-test-not-a-key"
HARD_WIRED = "/Users/admin"
FAKE_TOOLS = ("ssh", "scp", "rsync", "launchctl", "systemctl", "sudo")
# The fields of the plist that decide what runs, when, as whom and where it logs.
PLIST_FIELDS = ("Label", "ProgramArguments", "StartInterval", "RunAtLoad", "StandardOutPath", "StandardErrorPath")


# ---------------------------------------------------------------- helpers

def make_env(tmp_path, user="tester", home=None):
    fakebin = tmp_path / "fakebin"
    fakebin.mkdir(exist_ok=True)
    log = tmp_path / "fake-calls.log"
    for tool in FAKE_TOOLS:
        p = fakebin / tool
        p.write_text(f'#!/bin/sh\necho "{tool} $*" >> "{log}"\nexit 97\n')
        p.chmod(0o755)
    home = Path(home) if home else tmp_path / "home" / user
    home.mkdir(parents=True, exist_ok=True)
    env = {
        "PATH": f"{fakebin}{os.pathsep}{os.environ.get('PATH', '/usr/bin:/bin')}",
        "HOME": str(home),
        "USER": user,
        "LOGNAME": user,
        "LANG": "C",
        # fake keys, by the names the fleet's config uses: none may reach a unit, the plan or argv
        "NUSY_GLM": FAKE_KEY,
        "DEEPSEEK_API_KEY": FAKE_KEY,
        "OPENAI_API_KEY": FAKE_KEY,
    }
    return env, log, home


def run(tmp_path, args, env=None, stdin=subprocess.DEVNULL, user="tester", home=None):
    if env is None:
        env, log, home = make_env(tmp_path, user=user, home=home)
    else:
        log = tmp_path / "fake-calls.log"
    proc = subprocess.run(
        ["bash", str(SCRIPT), *args],
        env=env,
        cwd=str(tmp_path),
        stdin=stdin if not isinstance(stdin, str) else None,
        input=stdin if isinstance(stdin, str) else None,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return proc, log


def rendered_texts(outdir):
    return {p.name: p.read_text() for p in sorted(Path(outdir).iterdir()) if p.is_file()}


def assert_no_secret(texts, key=FAKE_KEY):
    """texts: name -> text. Fails if the key appears anywhere."""
    hits = [name for name, t in texts.items() if key in t]
    assert not hits, f"secret value found in: {hits}"


def assert_no_hard_wired(texts):
    hits = [name for name, t in texts.items() if HARD_WIRED in t]
    assert not hits, f"{HARD_WIRED} found in: {hits}"


def plist_diff(a, b):
    """The PLIST_FIELDS on which two parsed plists differ."""
    return [f for f in PLIST_FIELDS if a.get(f) != b.get(f)]


def readme_mini_command():
    """The install command for Mini recorded in packaging/README.md, as argv after the script path."""
    text = README.read_text()
    for line in text.splitlines():
        line = line.strip().lstrip("$").strip()
        if "install.sh" in line and re.search(r"--host\s+\S*mini\S*", line, re.I):
            argv = shlex.split(line)
            idx = next(i for i, a in enumerate(argv) if a.endswith("install.sh"))
            return argv[idx + 1:]
    return None


def assert_no_installer_side_effects(log, home):
    assert not log.exists(), f"a real-install tool was called during a dry run: {log.read_text()!r}"
    for sub in ("Library/LaunchAgents", ".config/systemd"):
        p = Path(home) / sub
        assert not p.exists() or not any(p.rglob("*")), f"dry run wrote under the target home: {p}"


# ---------------------------------------------------------------- (a) local

def test_local_macos_renders_launchd_plist_for_given_user_and_home(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    home = tmp_path / "Users" / "alice"
    proc, log = run(tmp_path, ["--where", "local", "--os", "macos", "--user", "alice", "--home", str(home),
                               "--render-to", str(out)], home=home, user="alice")
    assert proc.returncode == 0, proc.stderr
    texts = rendered_texts(out)
    assert PLIST_NAME in texts, f"rendered: {sorted(texts)}"
    pl = plistlib.loads((out / PLIST_NAME).read_bytes())
    assert pl["Label"] == "com.congruentsys.quotabus-probe"
    args = pl["ProgramArguments"]
    assert "probe" in args and any(a.endswith("quotabus") for a in args), args
    assert pl["StandardOutPath"].startswith(f"{home}/Library/Logs/quotabus/"), pl["StandardOutPath"]
    assert pl["StandardErrorPath"].startswith(f"{home}/Library/Logs/quotabus/"), pl["StandardErrorPath"]
    assert pl.get("RunAtLoad") is True
    assert isinstance(pl.get("StartInterval"), int) and pl["StartInterval"] > 0
    assert f"{home}/Library/LaunchAgents/{PLIST_NAME}" in proc.stdout, proc.stdout
    assert_no_installer_side_effects(log, home)


def test_local_defaults_user_and_home_from_environment(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    home = tmp_path / "Users" / "bob"
    proc, log = run(tmp_path, ["--where", "local", "--os", "macos", "--render-to", str(out)], user="bob", home=home)
    assert proc.returncode == 0, proc.stderr
    pl = plistlib.loads((out / PLIST_NAME).read_bytes())
    assert pl["StandardOutPath"].startswith(f"{home}/Library/Logs/quotabus/"), pl["StandardOutPath"]
    assert f"{home}/Library/LaunchAgents/{PLIST_NAME}" in proc.stdout, proc.stdout


def test_local_linux_renders_systemd_user_unit_and_timer(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, ["--where", "local", "--os", "linux", "--user", "alice", "--home", "/home/alice",
                               "--render-to", str(out)], user="alice")
    assert proc.returncode == 0, proc.stderr
    texts = rendered_texts(out)
    assert SERVICE_NAME in texts and TIMER_NAME in texts, f"rendered: {sorted(texts)}"
    assert PLIST_NAME not in texts
    svc, tmr = texts[SERVICE_NAME], texts[TIMER_NAME]
    assert "[Service]" in svc
    exec_lines = [l for l in svc.splitlines() if l.startswith("ExecStart=")]
    assert exec_lines and re.search(r"quotabus\S*\s+probe\b", exec_lines[0]), svc
    assert "[Timer]" in tmr and re.search(r"^On(UnitActiveSec|Calendar|BootSec)=", tmr, re.M), tmr
    assert "[Install]" in tmr and "timers.target" in tmr, tmr
    for name in (SERVICE_NAME, TIMER_NAME):
        assert f"/home/alice/.config/systemd/user/{name}" in proc.stdout, proc.stdout
    assert "/Users/" not in svc + tmr, "a Linux unit carries a macOS path"
    assert_no_installer_side_effects(log, tmp_path / "home" / "alice")


# ---------------------------------------------------------------- (b) named host, no connection

def test_named_host_renders_for_remote_user_without_connecting(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, ["--host", "buoy", "--user", "carol", "--home", "/Users/carol", "--os", "macos",
                               "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
    pl = plistlib.loads((out / PLIST_NAME).read_bytes())
    assert pl["StandardOutPath"].startswith("/Users/carol/Library/Logs/quotabus/"), pl["StandardOutPath"]
    assert pl["StandardErrorPath"].startswith("/Users/carol/Library/Logs/quotabus/")
    assert "buoy" in proc.stdout, proc.stdout
    assert f"/Users/carol/Library/LaunchAgents/{PLIST_NAME}" in proc.stdout, proc.stdout
    # the local user's home is not the target's
    assert "/home/tester" not in (out / PLIST_NAME).read_text() and str(tmp_path / "home") not in (out / PLIST_NAME).read_text()
    assert_no_installer_side_effects(log, tmp_path / "home" / "tester")


def test_named_linux_host_renders_systemd_paths_for_remote_home(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, ["--host", "dgx1", "--user", "dave", "--home", "/home/dave", "--os", "linux",
                               "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
    assert {SERVICE_NAME, TIMER_NAME} <= set(rendered_texts(out))
    assert "dgx1" in proc.stdout
    assert f"/home/dave/.config/systemd/user/{SERVICE_NAME}" in proc.stdout, proc.stdout
    assert not log.exists(), log.read_text() if log.exists() else ""


# ---------------------------------------------------------------- asks when no target flag

def test_asks_for_target_on_stdin_when_no_flag(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, ["--os", "linux", "--user", "alice", "--home", "/home/alice", "--render-to", str(out)],
                    stdin="local\n", user="alice")
    assert proc.returncode == 0, proc.stderr
    assert {SERVICE_NAME, TIMER_NAME} <= set(rendered_texts(out))
    assert f"/home/alice/.config/systemd/user/{SERVICE_NAME}" in proc.stdout


def test_no_target_and_no_answer_exits_nonzero_with_message(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    proc, _ = run(tmp_path, ["--os", "macos", "--render-to", str(out)], stdin="")
    assert proc.returncode != 0
    assert re.search(r"local|--host|--where", proc.stderr), proc.stderr
    assert "not implemented" not in proc.stderr
    assert not any(out.iterdir())


# ---------------------------------------------------------------- usage errors

@pytest.mark.parametrize(
    "args, must_name",
    [
        (["--where", "elsewhere", "--os", "macos"], "elsewhere"),
        (["--where", "local", "--os", "windows"], "windows"),
        (["--host", "mini", "--home", "/Users/admin", "--os", "macos"], "--user"),
        (["--host", "mini", "--user", "admin", "--os", "macos"], "--home"),
    ],
    ids=["unknown-where", "unknown-os", "host-missing-user", "host-missing-home"],
)
def test_bad_target_or_missing_flag_exits_nonzero_naming_it(tmp_path, args, must_name):
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, [*args, "--render-to", str(out)])
    assert proc.returncode != 0
    assert must_name in proc.stderr, proc.stderr
    assert "not implemented" not in proc.stderr
    assert not any(out.iterdir()), "a refused run rendered a unit"
    assert not log.exists()


# ---------------------------------------------------------------- (c) Mini == today's plist; (f) recorded in docs

def test_readme_records_mini_as_fleet_central_probe_host_with_install_command():
    text = README.read_text()
    assert re.search(r"\bMini\b", text)
    assert re.search(r"central probe", text, re.I)
    assert readme_mini_command() is not None, "packaging/README.md carries no `install.sh … --host <mini>` line"


def test_installing_for_mini_yields_todays_plist(tmp_path):
    argv = readme_mini_command()
    assert argv is not None, "no Mini install command in packaging/README.md"
    argv = [a for a in argv if a != "--dry-run"]
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, [*argv, "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
    got = plistlib.loads((out / PLIST_NAME).read_bytes())
    want = plistlib.loads(MINI_FIXTURE.read_bytes())
    assert plist_diff(got, want) == [], {f: (got.get(f), want.get(f)) for f in plist_diff(got, want)}
    assert f"/Users/admin/Library/LaunchAgents/{PLIST_NAME}" in proc.stdout, proc.stdout
    assert not log.exists(), "rendering for Mini connected to it"


# ---------------------------------------------------------------- (d) no secret anywhere

@pytest.mark.parametrize(
    "args",
    [
        ["--where", "local", "--os", "macos", "--user", "alice", "--home", "/Users/alice"],
        ["--where", "local", "--os", "linux", "--user", "alice", "--home", "/home/alice"],
        ["--host", "buoy", "--user", "carol", "--home", "/Users/carol", "--os", "macos"],
    ],
    ids=["local-macos", "local-linux", "host-macos"],
)
def test_no_secret_in_units_plan_or_argv(tmp_path, args):
    out = tmp_path / "out"
    out.mkdir()
    proc, log = run(tmp_path, [*args, "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
    texts = rendered_texts(out)
    assert texts, "nothing rendered"
    assert_no_secret({**texts, "<stdout>": proc.stdout, "<stderr>": proc.stderr})
    for name in (PLIST_NAME,):
        if name in texts:
            argv = plistlib.loads((out / name).read_bytes())["ProgramArguments"]
            assert_no_secret({"ProgramArguments": "\n".join(argv)})
            assert not any(re.search(r"(KEY|TOKEN|SECRET|NUSY_GLM)=", a) for a in argv), argv
    if SERVICE_NAME in texts:
        assert not re.search(r"^Environment=.*(KEY|TOKEN|SECRET|NUSY_GLM)=", texts[SERVICE_NAME], re.M)


# ---------------------------------------------------------------- (e) /Users/admin is gone from the sources

def test_local_render_carries_no_hard_wired_mini_path(tmp_path):
    for os_name, home in (("linux", "/home/alice"), ("macos", "/Users/alice")):
        out = tmp_path / f"out-{os_name}"
        out.mkdir()
        proc, _ = run(tmp_path, ["--where", "local", "--os", os_name, "--user", "alice", "--home", home,
                                 "--render-to", str(out)], user="alice")
        assert proc.returncode == 0, proc.stderr
        texts = rendered_texts(out)
        assert texts, "nothing rendered"
        assert_no_hard_wired({**texts, "<stdout>": proc.stdout})


def test_hard_wired_path_absent_from_packaging_sources_except_readme():
    files = [p for p in PACKAGING.rglob("*") if p.is_file() and p != README]
    assert SCRIPT in files
    texts = {str(p.relative_to(ROOT)): p.read_text(errors="replace") for p in files}
    assert_no_hard_wired(texts)


# ---------------------------------------------------------------- controls: each checker can fail

def test_control_secret_checker_catches_a_leak():
    with pytest.raises(AssertionError):
        assert_no_secret({"unit": f"<string>NUSY_GLM={FAKE_KEY}</string>"})
    assert_no_secret({"unit": "<string>doppler</string>"})  # and passes a clean text


def test_control_hard_wired_checker_catches_todays_plist():
    # the known positive: today's plist (the fixture) DOES carry /Users/admin
    with pytest.raises(AssertionError):
        assert_no_hard_wired({"fixture": MINI_FIXTURE.read_text()})
    assert_no_hard_wired({"alice": "/home/alice/.config/systemd/user/quotabus-probe.service"})


def test_control_plist_comparison_catches_each_mutated_field():
    want = plistlib.loads(MINI_FIXTURE.read_bytes())
    assert plist_diff(want, dict(want)) == []
    mutations = {
        "Label": "com.example.other",
        "ProgramArguments": want["ProgramArguments"][:-1],
        "StartInterval": want["StartInterval"] + 1,
        "RunAtLoad": False,
        "StandardOutPath": "/Users/alice/Library/Logs/quotabus/probe.out.log",
        "StandardErrorPath": "/Users/alice/Library/Logs/quotabus/probe.err.log",
    }
    assert set(mutations) == set(PLIST_FIELDS)
    for field, value in mutations.items():
        assert plist_diff({**want, field: value}, want) == [field]


def test_control_fixture_is_the_exp001_mini_plist():
    """The fixture is the EXP-001 plist as of origin/main 71bebe7 (CHORE-007's 300 s tick)."""
    want = plistlib.loads(MINI_FIXTURE.read_bytes())
    assert want["StartInterval"] == 300, "CHORE-007 (71bebe7) made StartInterval the 300 s tick"
    assert "71bebe7" in MINI_FIXTURE.read_text()
    assert want["Label"] == "com.congruentsys.quotabus-probe" and want["RunAtLoad"] is True
    assert want["ProgramArguments"][:7] == ["/opt/homebrew/bin/doppler", "run", "--project", "nusy-product-team",
                                            "--config", "dev", "--"]
    assert want["StandardOutPath"] == "/Users/admin/Library/Logs/quotabus/probe.out.log"
    assert want["ProgramArguments"][-3:] == ["probe", "--config", "/usr/local/etc/quotabus/quotabus.toml"]
    assert FAKE_KEY not in MINI_FIXTURE.read_text()


def test_control_readme_command_finder_needs_a_mini_host_line(tmp_path, monkeypatch):
    fake = tmp_path / "README.md"
    fake.write_text("Run `packaging/install.sh --where local`.\n")
    monkeypatch.setattr(sys.modules[__name__], "README", fake)
    assert readme_mini_command() is None
    fake.write_text("```\npackaging/install.sh --host mini --user admin --home /Users/admin --os macos\n```\n")
    assert readme_mini_command() == ["--host", "mini", "--user", "admin", "--home", "/Users/admin", "--os", "macos"]


def test_control_fake_installer_tools_are_detected(tmp_path):
    env, log, home = make_env(tmp_path)
    subprocess.run([str(tmp_path / "fakebin" / "ssh"), "mini", "true"], env=env, capture_output=True)
    with pytest.raises(AssertionError):
        assert_no_installer_side_effects(log, home)
