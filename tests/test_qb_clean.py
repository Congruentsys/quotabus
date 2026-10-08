"""quotabus-loop's clean step must not delete a worktree's gitignored per-item artefacts.

The clean step is `scripts/qb_clean.sh <ID> [--disposable <tree-name>]...` (.claude/skills/quotabus-loop/SKILL.md
§ Cleaning up between items, a script so it can be tested). It runs on
the git repository of its current directory. Interface, as this file tests it:
  - only this item's trees are touched: basename `<ID>`, `<ID>-merge`, `rv-<ID>` or `rv-<ID>-<suffix>`;
  - a tree whose ignored `data/` (or an ignored path under `runs/`) holds a file is KEPT, with a
    `[clean] KEPT <path>` line naming the path, the durable per-item host path `~/.local/state/quotabus-work/<ID>/` and the
    `--disposable` flag; an empty or absent data/ does not keep it;
  - `--disposable <tree-name>` (repeatable) declares ONE named tree's ignored data disposable: that tree is removed;
  - a tree with commits not on origin, or uncommitted work, is KEPT, `--disposable` or not;
  - removed trees print `[clean] removed <path>`; free space below $QB_MIN_FREE_GIB (default 25) GiB exits 14;
  - exit codes: 0 done (kept trees are not an error), 2 usage, 14 low disk.

Every fixture is a throwaway bare origin, a clone of it (the "main checkout") and real `git worktree add` trees,
all under pytest's tmp dir, with a temp HOME and hermetic git config. Nothing here touches the real repo's
worktrees. The controls at the bottom test these tests: plain `git worktree remove` really deletes an ignored
data/ (the hazard is real), and a clean step that skips the data/ check, is flagged by the same
checker the KEEP tests use.
"""
import os
import re
import stat
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "qb_clean.sh"
LOOP_SKILL = ROOT / ".claude" / "skills" / "quotabus-loop" / "SKILL.md"
PAIRIT_SKILL = ROOT / ".claude" / "skills" / "pairit" / "SKILL.md"
ID = "EXP-901"
LOW_DISK_RC = 14
USAGE_RC = 2

# A clean step with no data/ check: the mutant the controls below must catch.
OLD_SNIPPET = r'''#!/usr/bin/env bash
ID="$1"
MIN_GIB="${QB_MIN_FREE_GIB:-25}"
git worktree list --porcelain | sed -n 's/^worktree //p' > "$TMPDIR/qb-clean-$ID.list"
while IFS= read -r w; do
  b="${w##*/}"
  case "$b" in "$ID"|"$ID-merge"|"rv-$ID"*) ;; *) continue ;; esac
  if [ -n "$(git -C "$w" log --oneline HEAD --not --remotes=origin -- 2>/dev/null | head -1)" ]; then
    echo "[clean] KEPT $w (commits not on origin) — push or report, never remove"; continue; fi
  if git worktree remove "$w"; then echo "[clean] removed $w"
  else echo "[clean] KEPT $w (uncommitted work) — report it, never force"; fi
done < "$TMPDIR/qb-clean-$ID.list"
rm -f "$TMPDIR/qb-clean-$ID.list"; git worktree prune
f=$(df -Pk /tmp | awk 'NR==2 {print int($4/1048576)}'); echo "[clean] ${f} GiB free"
[ "$f" -ge "$MIN_GIB" ] || { echo "[clean] below ${MIN_GIB} GiB — STOP and report the largest dirs (du -sh)"; exit 14; }
'''


# ---------------------------------------------------------------- hermetic git + fixture

@pytest.fixture
def env(tmp_path):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = clean-test\n\temail = clean@test\n[init]\n\tdefaultBranch = main\n"
                   "[advice]\n\tdetachedHead = false\n")
    home = tmp_path / "home"
    home.mkdir()
    tmpdir = tmp_path / "tmpdir"
    tmpdir.mkdir()
    e = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    e.update(GIT_CONFIG_GLOBAL=str(cfg), GIT_CONFIG_NOSYSTEM="1", GIT_TERMINAL_PROMPT="0", HOME=str(home),
             TMPDIR=str(tmpdir), QB_MIN_FREE_GIB="0")
    return e


def _git(env, repo, *args, check=True):
    p = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, env=env)
    if check and p.returncode:
        raise RuntimeError(f"git {args} in {repo}: {p.stderr}")
    return p.stdout.strip()


class Fx:
    """A bare origin with this repo's .gitignore, a main checkout cloned from it, and worktrees under wt/."""

    def __init__(self, tmp, env):
        self.tmp = tmp
        self.env = env
        self.home = Path(env["HOME"])
        seed = tmp / "seed"
        subprocess.run(["git", "init", "-q", str(seed)], check=True, env=env)
        (seed / ".gitignore").write_text((ROOT / ".gitignore").read_text())
        (seed / "README.md").write_text("fixture\n")
        _git(env, seed, "add", "-A")
        _git(env, seed, "commit", "-qm", "fixture")
        self.origin = tmp / "origin.git"
        subprocess.run(["git", "clone", "-q", "--bare", str(seed), str(self.origin)], check=True, env=env)
        self.main = tmp / "main"
        subprocess.run(["git", "clone", "-q", str(self.origin), str(self.main)], check=True, env=env)
        self.wt = tmp / "wt"
        self.wt.mkdir()

    def add(self, name):
        path = self.wt / name
        _git(self.env, self.main, "worktree", "add", "-q", "--detach", str(path), "origin/main")
        return path

    def registered(self):
        out = _git(self.env, self.main, "worktree", "list", "--porcelain")
        return {os.path.realpath(line[len("worktree "):]) for line in out.splitlines() if line.startswith("worktree ")}

    def run(self, *args, script=SCRIPT, cwd=None, **env_over):
        e = dict(self.env)
        e.update(env_over)
        return subprocess.run(["bash", str(script), *args], cwd=str(cwd or self.main), env=e, capture_output=True,
                              text=True, timeout=120)


@pytest.fixture
def fx(tmp_path, env):
    return Fx(tmp_path, env)


def put(tree, rel, content="artefact\n"):
    p = Path(tree) / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content)
    return p


def out(r):
    return r.stdout + r.stderr


def kept_line(r, tree):
    """The `[clean] KEPT <tree>` line(s) of the run's output, joined ('' when there is none)."""
    t = os.path.realpath(str(tree))
    return "\n".join(l for l in out(r).splitlines() if "KEPT" in l and (t in l or str(tree) in l))


def keep_problems(fx, r, tree, artefact):
    """[] when `tree` survived the run with `artefact` byte-intact, still registered, and a KEPT line names it."""
    probs = []
    if not Path(tree).is_dir():
        probs.append(f"{tree} was removed")
    if not Path(artefact).is_file():
        probs.append(f"{artefact} was deleted")
    if os.path.realpath(str(tree)) not in fx.registered():
        probs.append(f"{tree} is no longer a registered worktree")
    if not kept_line(r, tree):
        probs.append(f"no '[clean] KEPT {tree}' line in output:\n{out(r)}")
    return probs


def removed_problems(fx, r, tree):
    probs = []
    if Path(tree).exists():
        probs.append(f"{tree} still exists")
    if os.path.realpath(str(tree)) in fx.registered():
        probs.append(f"{tree} is still registered")
    t = os.path.realpath(str(tree))
    if not any("removed" in l and (t in l or str(tree) in l) for l in out(r).splitlines()):
        probs.append(f"no '[clean] removed {tree}' line in output:\n{out(r)}")
    return probs


def says_what_to_do(fx, line):
    """The KEPT line names the durable per-item path and the explicit disposable flag."""
    durable = (f"~/.local/state/quotabus-work/{ID}/", f"{fx.home}/.local/state/quotabus-work/{ID}/")
    probs = []
    if not any(d in line for d in durable):
        probs.append(f"KEPT line does not name the durable path ~/.local/state/quotabus-work/{ID}/: {line!r}")
    if "--disposable" not in line:
        probs.append(f"KEPT line does not name --disposable: {line!r}")
    return probs


# ---------------------------------------------------------------- 1. the stub/interface

def test_script_exists_and_is_executable_in_git():
    assert SCRIPT.is_file()
    assert SCRIPT.stat().st_mode & stat.S_IXUSR
    mode = subprocess.run(["git", "-C", str(ROOT), "ls-files", "-s", "scripts/qb_clean.sh"],
                          capture_output=True, text=True).stdout.split()
    assert mode and mode[0] == "100755", f"scripts/qb_clean.sh is not committed mode 100755: {mode}"


def test_no_id_is_a_usage_error(fx):
    r = fx.run()
    assert r.returncode == USAGE_RC, out(r)


# ---------------------------------------------------------------- 2. ignored artefacts keep the tree

@pytest.mark.parametrize("name", [ID, f"{ID}-merge", f"rv-{ID}"])
def test_tree_with_nonempty_ignored_data_is_kept_and_says_what_to_do(fx, name):
    tree = fx.add(name)
    art = put(tree, "data/exp901/transcripts/session-1.jsonl", "{\"turn\": 1}\n")
    assert _git(fx.env, tree, "status", "--porcelain") == "", "fixture: data/ must be ignored, the tree clean"
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, art) == []
    line = kept_line(r, tree)
    assert "data/" in line, f"KEPT line does not name data/: {line!r}"
    assert says_what_to_do(fx, line) == []


@pytest.mark.parametrize("rel", ["runs/r1/log.txt", "data/a.bin"])
def test_each_ignored_artefact_dir_keeps_the_tree(fx, rel):
    tree = fx.add(ID)
    art = put(tree, rel)
    assert _git(fx.env, tree, "status", "--porcelain") == ""
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, art) == []
    top = rel.split("/")[0] + "/"
    assert top in kept_line(r, tree), f"KEPT line does not name {top}: {kept_line(r, tree)!r}"
    assert says_what_to_do(fx, kept_line(r, tree)) == []


def test_absent_data_tree_is_removed(fx):
    tree = fx.add(ID)
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, tree) == []


def test_empty_data_dir_tree_is_removed(fx):
    tree = fx.add(ID)
    (tree / "data").mkdir()
    (tree / "runs").mkdir()
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, tree) == []


def test_after_moving_data_to_the_durable_path_the_tree_is_removed_and_the_copy_survives(fx):
    tree = fx.add(ID)
    put(tree, "data/t.jsonl", "keep me\n")
    r1 = fx.run(ID)
    assert r1.returncode == 0, out(r1)
    assert Path(tree).is_dir(), "first run must keep the tree"
    durable = fx.home / ".local" / "state" / "quotabus-work" / ID
    durable.mkdir(parents=True)
    os.rename(tree / "data" / "t.jsonl", durable / "t.jsonl")
    r2 = fx.run(ID)
    assert r2.returncode == 0, out(r2)
    assert removed_problems(fx, r2, tree) == []
    assert (durable / "t.jsonl").read_text() == "keep me\n"


def test_disposable_removes_only_the_named_tree(fx):
    a = fx.add(ID)
    b = fx.add(f"{ID}-merge")
    art_a = put(a, "data/precious.jsonl")
    put(b, "data/scratch.bin")
    r = fx.run(ID, "--disposable", f"{ID}-merge")
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, b) == []
    assert keep_problems(fx, r, a, art_a) == []


def test_disposable_is_repeatable(fx):
    a = fx.add(ID)
    b = fx.add(f"{ID}-merge")
    put(a, "data/x")
    put(b, "runs/y")
    r = fx.run(ID, "--disposable", ID, "--disposable", f"{ID}-merge")
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, a) == []
    assert removed_problems(fx, r, b) == []


def test_disposable_does_not_override_commits_not_on_origin(fx):
    tree = fx.add(ID)
    put(tree, "work.txt")
    _git(fx.env, tree, "add", "work.txt")
    _git(fx.env, tree, "commit", "-qm", "unpushed")
    art = put(tree, "data/x")
    r = fx.run(ID, "--disposable", ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, art) == []


def test_disposable_does_not_override_uncommitted_work(fx):
    tree = fx.add(ID)
    (tree / "README.md").write_text("edited, not committed\n")
    art = put(tree, "data/x")
    r = fx.run(ID, "--disposable", ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, art) == []
    assert (tree / "README.md").read_text() == "edited, not committed\n"


# ---------------------------------------------------------------- 3. the existing rules still hold

def test_commits_not_on_origin_keep_the_tree(fx):
    tree = fx.add(ID)
    f = put(tree, "work.txt")
    _git(fx.env, tree, "add", "work.txt")
    _git(fx.env, tree, "commit", "-qm", "unpushed")
    head = _git(fx.env, tree, "rev-parse", "HEAD")
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, f) == []
    assert "commits not on origin" in kept_line(r, tree)
    assert _git(fx.env, tree, "rev-parse", "HEAD") == head


def test_uncommitted_tracked_change_keeps_the_tree(fx):
    tree = fx.add(ID)
    (tree / "README.md").write_text("edited\n")
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, tree / "README.md") == []
    assert (tree / "README.md").read_text() == "edited\n"


def test_untracked_file_keeps_the_tree(fx):
    tree = fx.add(ID)
    f = put(tree, "notes.txt")
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    assert keep_problems(fx, r, tree, f) == []


def test_all_of_the_items_clean_trees_are_removed(fx):
    trees = [fx.add(n) for n in (ID, f"{ID}-merge", f"rv-{ID}", f"rv-{ID}-r2")]
    r = fx.run(ID)
    assert r.returncode == 0, out(r)
    for t in trees:
        assert removed_problems(fx, r, t) == []


def test_another_items_trees_and_the_main_checkout_are_never_touched(fx):
    mine = fx.add(ID)
    others = [fx.add(n) for n in ("EXP-902", "EXP-9010", f"rv-EXP-9010", "EXP-9010-merge", f"x-{ID}")]
    for t in others:
        put(t, "data/theirs.bin")
    clean_other = fx.add("EXP-903")                       # no data at all: still not ours
    put(fx.main, "data/main.bin")
    r = fx.run(ID, "--disposable", "EXP-902")
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, mine) == []
    reg = fx.registered()
    for t in others + [clean_other]:
        assert Path(t).is_dir() and os.path.realpath(str(t)) in reg, f"{t} was touched"
    for t in others:
        assert (t / "data" / "theirs.bin").is_file(), f"{t}/data was touched"
    assert (fx.main / "data" / "main.bin").is_file()
    assert os.path.realpath(str(fx.main)) in reg


def test_runs_from_a_worktree_of_the_repo(fx):
    other = fx.add("EXP-950")
    tree = fx.add(ID)
    r = fx.run(ID, cwd=other)
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, tree) == []
    assert other.is_dir()


def test_reports_free_space_and_exits_0_above_the_floor(fx):
    r = fx.run(ID, QB_MIN_FREE_GIB="0")
    assert r.returncode == 0, out(r)
    assert re.search(r"\[clean\] \d+ GiB free", out(r)), out(r)


def test_low_disk_exits_14_after_cleaning(fx):
    tree = fx.add(ID)
    kept = fx.add(f"{ID}-merge")
    art = put(kept, "data/x")
    r = fx.run(ID, QB_MIN_FREE_GIB="1000000000")
    assert r.returncode == LOW_DISK_RC, out(r)
    assert "STOP" in out(r)
    assert removed_problems(fx, r, tree) == []
    assert keep_problems(fx, r, kept, art) == []


# ---------------------------------------------------------------- 4. the skills call the script and say where data lives

def _section(text, heading):
    m = re.search(rf"(?ms)^## {re.escape(heading)}.*?(?=^## |\Z)", text)
    assert m, f"no '## {heading}' section"
    return m.group(0)


def test_quotabus_loop_clean_step_calls_the_script():
    sec = _section(LOOP_SKILL.read_text(), "Cleaning up between items")
    assert "scripts/qb_clean.sh" in sec
    assert 'git worktree remove "$w"' not in sec, "the untestable inline loop is still in the skill"
    assert "~/.local/state/quotabus-work/" in sec
    assert "--disposable" in sec
    assert "14" in sec


def test_pairit_measure_lane_puts_artefacts_on_the_durable_path():
    sec = _section(PAIRIT_SKILL.read_text(), "The measure lane")
    assert "~/.local/state/quotabus-work/" in sec, "the measure lane does not name the durable per-item path"


# ---------------------------------------------------------------- 5. controls (tests of these tests)

def test_control_plain_worktree_remove_deletes_ignored_data(fx):
    """The hazard is real: git worktree remove (no --force) removes a tree whose only content is ignored data/."""
    tree = fx.add(ID)
    art = put(tree, "data/transcript.jsonl")
    assert _git(fx.env, tree, "status", "--porcelain") == ""
    p = subprocess.run(["git", "-C", str(fx.main), "worktree", "remove", str(tree)], capture_output=True, text=True,
                       env=fx.env)
    assert p.returncode == 0, p.stderr
    assert not art.exists() and not tree.exists()


def test_control_old_snippet_is_caught_by_the_keep_checker(fx, tmp_path):
    """A clean step that skips the data/ check (the mutant above) fails the KEEP test's checker."""
    mutant = tmp_path / "old_clean.sh"
    mutant.write_text(OLD_SNIPPET)
    tree = fx.add(ID)
    art = put(tree, "data/transcript.jsonl")
    r = fx.run(ID, script=mutant)
    assert r.returncode == 0, out(r)
    probs = keep_problems(fx, r, tree, art)
    assert any("was deleted" in p for p in probs), probs
    assert any("was removed" in p for p in probs), probs


def test_control_old_snippet_still_passes_the_existing_rules(fx, tmp_path):
    """The checkers used for the existing rules accept the mutant, so they are not over-strict."""
    mutant = tmp_path / "old_clean.sh"
    mutant.write_text(OLD_SNIPPET)
    clean = fx.add(ID)
    dirty = fx.add(f"{ID}-merge")
    (dirty / "README.md").write_text("edited\n")
    other = fx.add("EXP-902")
    r = fx.run(ID, script=mutant)
    assert r.returncode == 0, out(r)
    assert removed_problems(fx, r, clean) == []
    assert keep_problems(fx, r, dirty, dirty / "README.md") == []
    assert other.is_dir()
    r14 = fx.run(ID, script=mutant, QB_MIN_FREE_GIB="1000000000")
    assert r14.returncode == LOW_DISK_RC
