"""CHORE-008 review r1 (reviews/CHORE-008-r1.md): F1 launchd domain, F2 linger, F3 systemd escaping, F4 --help,
F5 launcher refusal. These drive the REAL-install path of packaging/install.sh (no --render-to) with fake
ssh/scp/launchctl/systemctl/loginctl/id first on PATH that log their argv and succeed, and a scratch fake home.
Nothing here touches the real ~/Library/LaunchAgents or any host.
"""
import os
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "packaging" / "install.sh"
LABEL = "com.congruentsys.quotabus-probe"
FAKE_UID = "4242"
FAKES = ("ssh", "scp", "rsync", "launchctl", "systemctl", "loginctl", "sudo")


def make_env(tmp_path):
    fakebin = tmp_path / "fakebin"
    fakebin.mkdir()
    log = tmp_path / "calls.log"
    for tool in FAKES:
        p = fakebin / tool
        # one line per call: the tool, then each argv word on its own tab-separated field
        p.write_text(f'#!/bin/sh\n{{ printf "%s" "{tool}"; for a in "$@"; do printf "\\t%s" "$a"; done; echo; }} >> "{log}"\nexit 0\n')
        p.chmod(0o755)
    idp = fakebin / "id"
    idp.write_text(f'#!/bin/sh\ncase "$1" in -u) echo {FAKE_UID} ;; -un) echo tester ;; *) echo "uid={FAKE_UID}" ;; esac\n')
    idp.chmod(0o755)
    home = tmp_path / "home" / "tester"
    home.mkdir(parents=True)
    env = {
        "PATH": f"{fakebin}{os.pathsep}/usr/bin:/bin",
        "HOME": str(home),
        "USER": "tester",
        "LANG": "C",
        "TMPDIR": str(tmp_path),
    }
    return env, log, home


def run(tmp_path, args):
    env, log, home = make_env(tmp_path)
    proc = subprocess.run(["bash", str(SCRIPT), *args], env=env, cwd=str(tmp_path), capture_output=True, text=True,
                          stdin=subprocess.DEVNULL, timeout=60)
    calls = [line.split("\t") for line in log.read_text().splitlines()] if log.exists() else []
    return proc, calls, home


def remote_commands(calls):
    """The command strings handed to ssh (its last argv word)."""
    return [c[-1] for c in calls if c[0] == "ssh"]


def local_launchctl(calls):
    return [c[1:] for c in calls if c[0] == "launchctl"]


# ---------------------------------------------------------------- F1: explicit gui/<uid> domain

def test_f1_local_macos_install_bootstraps_into_gui_uid_domain(tmp_path):
    proc, calls, home = run(tmp_path, ["--where", "local", "--os", "macos"])
    assert proc.returncode == 0, proc.stderr
    plist = home / "Library" / "LaunchAgents" / f"{LABEL}.plist"
    assert plist.is_file()
    lc = local_launchctl(calls)
    assert ["bootstrap", f"gui/{FAKE_UID}", str(plist)] in lc, lc
    assert ["bootout", f"gui/{FAKE_UID}/{LABEL}"] in lc, lc
    assert lc.index(["bootout", f"gui/{FAKE_UID}/{LABEL}"]) < lc.index(["bootstrap", f"gui/{FAKE_UID}", str(plist)])
    assert not any(c and c[0] in ("load", "unload") for c in lc), f"legacy launchctl verb used: {lc}"


def test_f1_host_macos_install_computes_uid_on_target_and_bootstraps(tmp_path):
    proc, calls, _ = run(tmp_path, ["--host", "mini", "--user", "admin", "--home", "/Users/admin", "--os", "macos"])
    assert proc.returncode == 0, proc.stderr
    remote = " ; ".join(remote_commands(calls))
    assert "launchctl bootstrap gui/$(id -u)" in remote, remote
    assert f"launchctl bootout gui/$(id -u)/{LABEL}" in remote, remote
    assert FAKE_UID not in remote, "the uid was computed on the installing host, not the target"
    assert "launchctl load" not in remote and "launchctl unload" not in remote, remote
    assert not local_launchctl(calls), "launchctl ran locally for a --host install"


def test_f1_control_fake_launchctl_is_logged(tmp_path):
    env, log, _ = make_env(tmp_path)
    subprocess.run([str(tmp_path / "fakebin" / "launchctl"), "load", "x"], env=env)
    assert log.read_text().split("\n")[0].split("\t") == ["launchctl", "load", "x"]


# ---------------------------------------------------------------- F2: linger on Linux

def test_f2_host_linux_install_enables_linger_for_the_user(tmp_path):
    proc, calls, _ = run(tmp_path, ["--host", "dgx1", "--user", "dave", "--home", "/home/dave", "--os", "linux"])
    assert proc.returncode == 0, proc.stderr
    remote = " ; ".join(remote_commands(calls))
    assert "loginctl enable-linger dave" in remote, remote
    assert "systemctl --user enable --now quotabus-probe.timer" in remote, remote


def test_f2_local_linux_install_enables_linger_for_the_user(tmp_path):
    proc, calls, home = run(tmp_path, ["--where", "local", "--os", "linux"])
    assert proc.returncode == 0, proc.stderr
    assert ["loginctl", "enable-linger", "tester"] in calls, calls
    assert (home / ".config" / "systemd" / "user" / "quotabus-probe.timer").is_file()


# ---------------------------------------------------------------- F3: systemd escaping

def test_f3_percent_and_dollar_are_escaped_in_execstart(tmp_path):
    out = tmp_path / "out"
    proc, calls, _ = run(tmp_path, ["--where", "local", "--os", "linux", "--home", "/home/a",
                                    "--quotabus", "/opt/100%/$HOME/qb", "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
    exec_line = [l for l in (out / "quotabus-probe.service").read_text().splitlines() if l.startswith("ExecStart=")][0]
    assert "/opt/100%%/$$HOME/qb" in exec_line, exec_line
    assert not calls


def test_f3_control_plain_path_is_unchanged(tmp_path):
    out = tmp_path / "out"
    proc, _, _ = run(tmp_path, ["--where", "local", "--os", "linux", "--home", "/home/a", "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
    assert "ExecStart=/usr/local/bin/quotabus probe --config /usr/local/etc/quotabus/quotabus.toml" in \
        (out / "quotabus-probe.service").read_text()


# ---------------------------------------------------------------- F4: --help

def test_f4_help_prints_only_the_comment_header(tmp_path):
    proc, _, _ = run(tmp_path, ["--help"])
    assert proc.returncode == 0
    assert "--where local" in proc.stdout
    assert "set -eu" not in proc.stdout
    assert all(l.startswith("#") for l in proc.stdout.splitlines() if l), proc.stdout


# ---------------------------------------------------------------- F5: refuse a credential-looking launcher

@pytest.mark.parametrize("launcher", [
    "env OPENAI_API_KEY=sk-test-not-a-key quotabus-wrap",
    "doppler run --token dp.st.fake --",
    "doppler run --token=dp.st.fake --",
    "env GH_TOKEN=abc --",
    "env MY_SECRET=abc --",
    "wrap sk-test-not-a-key",
])
def test_f5_launcher_that_looks_like_a_credential_is_refused(tmp_path, launcher):
    out = tmp_path / "out"
    out.mkdir()
    proc, calls, _ = run(tmp_path, ["--where", "local", "--os", "macos", "--launcher", launcher,
                                    "--render-to", str(out)])
    assert proc.returncode != 0
    assert "--launcher" in proc.stderr
    assert "sk-test-not-a-key" not in proc.stderr + proc.stdout and "dp.st.fake" not in proc.stderr + proc.stdout
    assert not any(out.iterdir())
    assert not calls


@pytest.mark.parametrize("launcher", [
    "/opt/homebrew/bin/doppler run --project nusy-product-team --config dev --",
    "secretspec run --",
])
def test_f5_control_ordinary_launchers_are_accepted(tmp_path, launcher):
    out = tmp_path / "out"
    proc, _, _ = run(tmp_path, ["--where", "local", "--os", "macos", "--launcher", launcher, "--render-to", str(out)])
    assert proc.returncode == 0, proc.stderr
