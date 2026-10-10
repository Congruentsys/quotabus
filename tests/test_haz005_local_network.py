"""HAZ-005: macOS Local Network privacy blocks the launchd probe from LAN hosts ("No route to host").

From the item (Definition of Done 1) and the driver's measurement on Mini (2026-10-10, a board comment on HAZ-005):
  - The grant attaches to the launchd job's PROGRAM: the first ProgramArguments entry. That is the launcher
    (e.g. /opt/homebrew/bin/doppler) when `--launcher` is given, else the quotabus binary itself. Replacing the
    quotabus binary under a granted launcher did NOT reset the grant. (Whether a launcher upgrade resets it is
    unmeasured, so nothing here asserts it.)
  (a) `install.sh`'s plan on macOS (exercised with the dry run `--render-to <dir>`) states the Local Network step:
      allow it in System Settings -> Privacy & Security -> Local Network, NAMES the program to allow (the launcher's
      program path with --launcher, the quotabus path without), and says how to confirm it (a LAN row reads a real
      state, not "No route to host").
  (b) On `--os linux` the plan does not print the step.
  (c) `packaging/README.md` states the step, that the grant goes to the unit's first program (the launcher), that
      replacing the quotabus binary does not reset it under a launcher, and how to confirm.

Assertions are on stable substrings ("Local Network", "Privacy & Security", "No route to host", the program path),
never on exact prose. Fakes only: ssh/scp/rsync/launchctl/systemctl/loginctl/sudo on PATH log and fail; the
environment carries a fake key. Each checker carries a known answer and mutations that turn it red.
"""
import os
import re
import shlex
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "packaging" / "install.sh"
README = ROOT / "packaging" / "README.md"
FAKE_TOOLS = ("ssh", "scp", "rsync", "launchctl", "systemctl", "loginctl", "sudo")
FAKE_KEY = "sk-test-not-a-key"

HOME = "/Users/tester-target"
QB = f"{HOME}/.local/bin/quotabus"
CONFIG = f"{HOME}/.config/quotabus/quotabus.toml"
DOPPLER = "/opt/homebrew/bin/doppler"
SECRETSPEC = "/usr/local/bin/secretspec"
BASE = ["--host", "probehost", "--user", "tester", "--home", HOME, "--quotabus", QB, "--probe-config", CONFIG,
        "--interval", "300"]


# ---------------------------------------------------------------- helpers

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
    assert proc.returncode == 0, proc.stderr
    assert not log.exists(), f"a dry run called an installer tool: {log.read_text()}"
    return proc.stdout


def local_network_step(text):
    """The plan's Local Network step: from the first line naming "Local Network" to the next blank line (or the end),
    at most 12 lines. None if no line names it."""
    lines = text.splitlines()
    start = next((i for i, l in enumerate(lines) if "Local Network" in l), None)
    if start is None:
        return None
    step = []
    for l in lines[start:start + 12]:
        if step and not l.strip():
            break
        step.append(l)
    return "\n".join(step)


def check_plan_step(plan, program):
    """The plan states the Local Network step, names `program` as what to allow, and says how to confirm."""
    step = local_network_step(plan)
    assert step is not None, f"the plan states no Local Network step:\n{plan}"
    assert "Privacy & Security" in step, step
    assert re.search(r"\ballow", step, re.I), step
    assert re.search(re.escape(program) + r"(?![\w/.-])", step), f"the step does not name {program}:\n{step}"
    assert "No route to host" in step, f"the step does not say how to confirm (not 'No route to host'):\n{step}"


def local_network_text(text):
    """Every README paragraph (blank-line separated) that mentions Local Network, joined; "" if none."""
    paras = re.split(r"\n\s*\n", text)
    return "\n\n".join(p for p in paras if "Local Network" in p)


FIRST_PROGRAM = re.compile(r"\bfirst\b[^.]{0,120}\b(program|ProgramArguments)\b|ProgramArguments\s*\[\s*0\s*\]"
                           r"|\b(program|ProgramArguments)\b[^.]{0,120}\bfirst\b", re.I)
NO_RESET = re.compile(r"\b(replac\w*|redeploy\w*|upgrad\w*\s+(of\s+)?(the\s+)?quotabus)\b[^.]{0,200}?"
                      r"\b(does\s+not|doesn't|did\s+not|didn't|not)\b[^.]{0,40}\breset"
                      r"|\b(does\s+not|doesn't|did\s+not|didn't|not)\b[^.]{0,40}\breset\w*\b[^.]{0,200}?"
                      r"\b(replac\w*|redeploy\w*)", re.I | re.S)


def check_readme(text):
    ln = local_network_text(text)
    assert ln, "packaging/README.md does not mention Local Network"
    assert "Privacy & Security" in ln, ln
    assert "No route to host" in ln, "README does not say how to confirm (not 'No route to host')"
    assert "launcher" in ln, "README does not say the grant goes to the launcher"
    assert FIRST_PROGRAM.search(ln), "README does not say the grant attaches to the unit's first program"
    assert NO_RESET.search(ln), "README does not say replacing the quotabus binary does not reset the grant"
    assert "quotabus" in ln


# ---------------------------------------------------------------- (a) the macOS plan

def test_macos_plan_with_launcher_names_the_launcher_program(tmp_path):
    plan = render(tmp_path, [*BASE, "--os", "macos", "--launcher",
                             f"{DOPPLER} run --project p --config dev --"])
    check_plan_step(plan, DOPPLER)
    assert FAKE_KEY not in plan


def test_macos_plan_names_whatever_launcher_is_given_not_a_hard_wired_one(tmp_path):
    plan = render(tmp_path, [*BASE, "--os", "macos", "--launcher", f"{SECRETSPEC} run --"])
    check_plan_step(plan, SECRETSPEC)
    assert DOPPLER not in (local_network_step(plan) or ""), "the step names doppler though the launcher is secretspec"


def test_macos_plan_without_launcher_names_the_quotabus_binary(tmp_path):
    plan = render(tmp_path, [*BASE, "--os", "macos"])
    check_plan_step(plan, QB)


def test_readme_fleet_command_plan_names_doppler(tmp_path):
    for line in README.read_text().splitlines():
        if "install.sh" in line and re.search(r"--host\s+\S*mini\S*", line, re.I):
            argv = shlex.split(line.strip().lstrip("$").strip())
            argv = argv[next(i for i, a in enumerate(argv) if a.endswith("install.sh")) + 1:]
            break
    else:
        pytest.fail("packaging/README.md carries no `install.sh … --host <mini>` line")
    plan = render(tmp_path, argv)
    check_plan_step(plan, DOPPLER)


# ---------------------------------------------------------------- (b) linux: no step

def test_linux_plan_has_no_local_network_step(tmp_path):
    plan = render(tmp_path, [*BASE, "--os", "linux", "--launcher", f"{DOPPLER} run --"])
    assert "Local Network" not in plan, plan
    assert "Privacy & Security" not in plan, plan


# ---------------------------------------------------------------- (c) the README

def test_readme_states_the_local_network_step():
    check_readme(README.read_text())


# ---------------------------------------------------------------- controls: each checker can fail

GOOD_PLAN = (
    "os: macos\n"
    f"launcher: {DOPPLER} run --\n"
    "Local Network: after install, allow it in System Settings > Privacy & Security > Local Network:\n"
    f"  allow {DOPPLER} (the unit's first program)\n"
    "  confirm: a LAN row reads a real state, not \"No route to host\"\n"
)


def test_control_plan_checker():
    check_plan_step(GOOD_PLAN, DOPPLER)  # known answer
    for bad, program in (
        ("os: macos\nlauncher: x\n", DOPPLER),                                   # no step at all (today's plan)
        (GOOD_PLAN, QB),                                                         # names the wrong program
        (GOOD_PLAN, "/opt/homebrew/bin/dop"),                                    # a prefix is not the program
        (GOOD_PLAN.replace("No route to host", "unreachable"), DOPPLER),         # no confirm
        (GOOD_PLAN.replace("Privacy & Security", "Settings"), DOPPLER),          # no where-to-allow
        (GOOD_PLAN.replace(f"  allow {DOPPLER}", f"\n  allow {DOPPLER}"), DOPPLER),  # program outside the step
    ):
        with pytest.raises(AssertionError):
            check_plan_step(bad, program)
    # the launcher line alone (outside the step) does not count as naming the program
    with pytest.raises(AssertionError):
        check_plan_step(GOOD_PLAN.replace(f"  allow {DOPPLER} (the unit's first program)\n", ""), DOPPLER)


GOOD_README = (
    "# Packaging\n\nintro\n\n"
    "## macOS: Local Network\n\n"
    "After install, allow the probe in System Settings → Privacy & Security → Local Network. The grant attaches to "
    "the unit's first program (the first ProgramArguments entry): the launcher, e.g. /opt/homebrew/bin/doppler, or "
    "quotabus itself when there is no launcher. Replacing the quotabus binary under a granted launcher does not reset "
    "it. Confirm: a LAN row reads a real state, not \"No route to host\" (Local Network).\n"
)


def test_control_readme_checker():
    check_readme(GOOD_README)  # known answer
    for bad in (
        "# Packaging\n\nnothing about it\n",
        GOOD_README.replace("No route to host", "unreachable"),
        GOOD_README.replace("Privacy & Security", "Settings"),
        GOOD_README.replace("first program (the first ProgramArguments entry)", "program"),
        GOOD_README.replace("does not reset it", "resets it"),
        GOOD_README.replace("launcher", "wrapper"),
    ):
        with pytest.raises(AssertionError):
            check_readme(bad)
    # the facts in a paragraph that never names Local Network do not count
    split = GOOD_README.replace(" (Local Network)", "").replace("Local Network. The grant", "Local Network.\n\nThe grant")
    with pytest.raises(AssertionError):
        check_readme(split)
