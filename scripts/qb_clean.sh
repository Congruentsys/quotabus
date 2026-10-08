#!/usr/bin/env bash
# scripts/qb_clean.sh <ID> [--disposable <tree-name>]... — quotabus-loop's clean step, run after an item is LANDED
# and before the next pick (.claude/skills/quotabus-loop/SKILL.md § Cleaning up between items). A script, so the
# step can be tested: tests/test_qb_clean.py.
#
# Operates on the git repository of the CURRENT DIRECTORY (the main checkout, or any worktree of it).
#
# Scope: only worktrees that repository registered whose directory basename is this item's: `<ID>`, `<ID>-merge`,
# `rv-<ID>` or `rv-<ID>-<suffix>`. Another item's tree (`EXP-9010`, `rv-EXP-9010` when ID is EXP-901) is never
# touched, nor the main checkout.
#
# A tree is KEPT (reported, never forced) when:
#   - it holds commits not on origin ("push or report, never remove");
#   - `git worktree remove` refuses it (uncommitted work);
#   - its gitignored `data/` or `runs/` holds a file.
#     `git worktree remove` deletes ignored files silently, so the script checks first. The KEPT line names the
#     path and says what to do: move it to the durable per-item host path `~/.local/state/quotabus-work/<ID>/` and record that
#     path in the item, then re-run; or declare it disposable explicitly with `--disposable <tree-name>`.
# An empty or absent `data/` (and `runs/`) does not keep a tree.
#
# --disposable <tree-name>   (repeatable) the basename of ONE of this item's trees whose ignored data is declared
#                            disposable: that tree is removed with its data/ and runs/. It never applies to
#                            any other tree, and it does not override the commits-not-on-origin or uncommitted rules.
#
# Then `git worktree prune`, and the free space on /tmp is printed. Below ${QB_MIN_FREE_GIB:-25} GiB it prints
# a STOP line and exits 14.
#
# Exit codes: 0 done (kept trees are reported, not an error) | 2 usage | 14 low disk: STOP and report.

set -u

usage() { echo "usage: scripts/qb_clean.sh <ID> [--disposable <tree-name>]..." >&2; exit 2; }

[ $# -ge 1 ] || usage
ID="$1"; shift
case "$ID" in ""|-*|*/*) usage ;; esac
DISPOSABLE=()
while [ $# -gt 0 ]; do
  case "$1" in
    --disposable) [ $# -ge 2 ] && [ -n "$2" ] || usage; DISPOSABLE+=("$2"); shift 2 ;;
    *) usage ;;
  esac
done
MIN_GIB="${QB_MIN_FREE_GIB:-25}"
case "$MIN_GIB" in ''|*[!0-9]*) echo "qb_clean.sh: QB_MIN_FREE_GIB must be a whole number of GiB" >&2; exit 2 ;; esac
git rev-parse --git-dir >/dev/null 2>&1 || { echo "qb_clean.sh: not inside a git repository" >&2; exit 2; }

is_disposable() { local d; for d in "${DISPOSABLE[@]+"${DISPOSABLE[@]}"}"; do [ "$d" = "$1" ] && return 0; done; return 1; }

# The ignored artefact dirs of tree $1 that hold at least one file (empty dirs do not count), space-separated.
artefact_dirs() {
  local w="$1" d found=""
  for d in data runs; do
    [ -d "$w/$d" ] || continue
    if [ -n "$(find "$w/$d" ! -type d -print -quit 2>/dev/null)" ]; then found="$found $d/"; fi
  done
  echo "${found# }"
}

LIST="$(mktemp "${TMPDIR:-/tmp}/qb-clean-$ID.XXXXXX")" || exit 2
trap 'rm -f "$LIST"' EXIT
git worktree list --porcelain | sed -n 's/^worktree //p' > "$LIST"
MAIN="$(head -1 "$LIST")"                     # the first entry is the main checkout: never touched
while IFS= read -r w; do
  [ "$w" = "$MAIN" ] && continue
  b="${w##*/}"
  case "$b" in "$ID"|"$ID-merge"|"rv-$ID"|"rv-$ID-"*) ;; *) continue ;; esac
  [ -d "$w" ] || continue                     # already gone: prune below drops the registration
  if [ -n "$(git -C "$w" log --oneline HEAD --not --remotes=origin -- 2>/dev/null | head -1)" ]; then
    echo "[clean] KEPT $w (commits not on origin) — push or report, never remove"; continue; fi
  if [ -n "$(git -C "$w" status --porcelain 2>/dev/null | head -1)" ]; then
    echo "[clean] KEPT $w (uncommitted work) — report it, never force"; continue; fi
  arts="$(artefact_dirs "$w")"
  if [ -n "$arts" ] && ! is_disposable "$b"; then
    echo "[clean] KEPT $w (ignored artefacts in ${arts// /, }) — move them to ~/.local/state/quotabus-work/$ID/ and record that path in the item, then re-run; or declare them disposable: scripts/qb_clean.sh $ID --disposable $b"
    continue
  fi
  # `git worktree remove` without --force still refuses uncommitted work; ignored files go with the tree.
  if git worktree remove "$w"; then echo "[clean] removed $w${arts:+ (disposable: $arts)}"
  else echo "[clean] KEPT $w (uncommitted work) — report it, never force"; fi
done < "$LIST"
git worktree prune
f=$(df -Pk /tmp | awk 'NR==2 {print int($4/1048576)}'); echo "[clean] ${f} GiB free"
[ "$f" -ge "$MIN_GIB" ] || { echo "[clean] below ${MIN_GIB} GiB — STOP and report the largest dirs (du -sh)"; exit 14; }
exit 0
