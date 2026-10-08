"""CHORE-003 r1 F1: the repo's venv is never tracked.

Every worktree recipe links the main checkout's venv into the tree (`ln -s "$PWD/.venv" /tmp/<ID>/.venv`). An ignore
line of `.venv/` matches only a directory, so that link was committed once (a mode-120000 entry pointing at one
host's absolute path); merged, it would replace a checkout's real venv with a link on the next pull.
"""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def tracked(*paths):
    return subprocess.run(["git", "-C", str(ROOT), "ls-files", "-s", "--", *paths], capture_output=True, text=True,
                          check=True).stdout.split("\n")


def absolute_symlinks():
    out = []
    for line in tracked():
        if line.startswith("120000 "):
            path = line.split("\t", 1)[1]
            sha = line.split()[1]
            target = subprocess.run(["git", "-C", str(ROOT), "cat-file", "-p", sha], capture_output=True,
                                    text=True, check=True).stdout
            if target.startswith("/"):
                out.append((path, target))
    return out


def test_venv_is_not_tracked():
    assert [l for l in tracked(".venv") if l] == []


def test_no_tracked_symlink_points_at_an_absolute_path():
    assert absolute_symlinks() == []


def test_gitignore_ignores_a_venv_link_as_well_as_a_dir():
    r = subprocess.run(["git", "-C", str(ROOT), "check-ignore", "--no-index", "-q", ".venv"])
    assert r.returncode == 0, "`.venv` (a link, not only a directory) is not ignored"
