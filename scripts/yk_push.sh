#!/usr/bin/env bash
# scripts/yk_push.sh <yurtle-kanban args…> — ONE yurtle-kanban board write, run on a fresh origin/main in this
# session's own board worktree and landed on origin at once (CHORE-014).
#
# The local fix for yurtle-kanban #1251 (https://github.com/Congruentsys/yurtle-kanban/issues/1251): `move`,
# `comment`, `rank` and `voyage add` commit locally only. Retire this script when #1251 ships and this repo pins
# that release.
#
# Runnable from the main checkout, any worktree of it, or a subdirectory of either. The caller's HEAD, branch,
# index and working tree are never touched; nothing of the caller's is pushed.
#
# Board tree: /tmp/qb-board-<first 8 of CLAUDE_CODE_SESSION_ID>, or $YK_PUSH_TREE (for tests). A path there that
# is not a live worktree of THIS repo is discarded and recreated. After every run the tree is clean and holds no
# commit that is not on origin.
#
# A lost race (push rejected because origin moved) drops the commit and re-runs the SAME verb on the new
# origin/main — never a rebase (two comments on one item conflict when one is rebased onto the other).
#
# Exit codes: 0 landed | 1 refused / usage / no session / missing venv | 4 origin unreachable | 5 five lost races
# | 6 the verb made no commit: nothing was pushed (HAZ-010).
#   0 is given only when the verb moved the board tree's HEAD past the origin/main it started on and that commit was
#     pushed. 6 covers a harmless no-op and a verb that failed to commit alike: nothing yurtle-kanban documents
#     tells them apart, and no output text is read. A redo after a lost race that commits nothing is 6 too.
#   EVERY non-zero exit of the verb is mapped to 1, so yurtle-kanban's own exit codes 4, 5 and 6 never collide with
#     this script's 4, 5 and 6.
#   A verb given --push is refused (1, nothing run): it writes origin itself and is run directly, not through here.
#
# What the verb left uncommitted (on 6, and on 1 when a refusing verb left an edit) is listed as `git status --short`
# lines and saved, untracked files included, as ONE patch: ${TMPDIR:-/tmp}/yk-push-uncommitted.XXXXXX, a new file
# per run, its absolute path printed. `git apply <patch>` on a checkout of the origin/main the verb ran on (named in
# the message) gives the edit back. The patch is the caller's to delete; this script never removes one.
# Portable: bash 3.2 and BSD userland (no timeout, no readlink -f, no GNU-only flags).

set -u
me="yk_push.sh"
say() { echo "$me: $*" >&2; }

[ $# -gt 0 ] || { say "usage: scripts/yk_push.sh <yurtle-kanban args…>"; exit 1; }

# 7. No stdin bodies: a retry re-runs the verb and cannot re-read stdin. Relative --body-file paths are made
# absolute, because the verb runs in the board tree, not in the caller's directory.
# 10. No --push verbs (HAZ-010): such a verb writes origin itself, from a tree of its own, and commits nothing here.
args=()
prev=""
for a in "$@"; do
  case "$prev" in
    --body|--title|-m|--message|-s|--summary) ;;   # the text of an option, not a flag
    *) [ "$a" = "--push" ] && { say "usage: refused: a verb given --push writes origin itself; run it directly (yurtle-kanban $1 … --push), not through scripts/yk_push.sh; nothing run"; exit 1; } ;;
  esac
  case "$a" in
    --body-file=-) say "refused: --body-file=- reads stdin, which a retried write cannot re-read; pass --body or --body-file <file>"; exit 1 ;;
  esac
  if [ "$prev" = "--body-file" ]; then
    [ "$a" = "-" ] && { say "refused: --body-file - reads stdin, which a retried write cannot re-read; pass --body or --body-file <file>"; exit 1; }
    case "$a" in /*) ;; *) a="$PWD/$a" ;; esac
  else
    case "$a" in
      --body-file=/*) ;;
      --body-file=*) a="--body-file=$PWD/${a#--body-file=}" ;;
    esac
  fi
  args+=("$a")
  prev="$a"
done

# 3. The session's board tree.
if [ -n "${YK_PUSH_TREE:-}" ]; then
  W="$YK_PUSH_TREE"
elif [ -n "${CLAUDE_CODE_SESSION_ID:-}" ]; then
  W="/tmp/qb-board-${CLAUDE_CODE_SESSION_ID:0:8}"
else
  say "CLAUDE_CODE_SESSION_ID is unset (and no YK_PUSH_TREE): no board tree to write in; nothing written"
  exit 1
fi
case "$W" in /*) ;; *) W="$PWD/$W" ;; esac
W="${W%/}"
[ -n "$W" ] && [ "$W" != "/" ] || { say "refused: board tree path '$W'"; exit 1; }

# The main checkout, from wherever we were called.
common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || { say "not inside a git repo"; exit 1; }
main="${common%/.git}"
[ "$main" != "$common" ] || { say "cannot find the main checkout (git common dir: $common)"; exit 1; }

# 8. The repo's own yurtle-kanban, with PYTHONPATH emptied.
YK="$main/.venv/bin/yurtle-kanban"
[ -x "$YK" ] || { say "no yurtle-kanban at $main/.venv (missing $YK): create the venv (CLAUDE.md § Session start)"; exit 1; }

# 6. Origin reachable before anything exists or is written.
git -C "$main" fetch -q origin || { say "origin unreachable, nothing written"; exit 4; }

phys() { (cd "$1" 2>/dev/null && pwd -P); }

live_tree() {   # true when $W is the top of a live worktree of THIS repo
  [ -d "$W" ] || return 1
  local wc top
  wc=$(git -C "$W" rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || return 1
  top=$(git -C "$W" rev-parse --show-toplevel 2>/dev/null) || return 1
  [ "$(phys "$wc")" = "$(phys "$common")" ] && [ "$(phys "$top")" = "$(phys "$W")" ]
}

ready=0
cleanup() {    # the tree ends clean and on origin, success or failure
  [ "$ready" = 1 ] || return 0
  git -C "$W" checkout -q -f --detach refs/remotes/origin/main 2>/dev/null
  git -C "$W" clean -qfd -e .venv 2>/dev/null
}
trap cleanup EXIT

if ! live_tree; then
  if [ -e "$W" ] || [ -L "$W" ]; then
    say "discarding stale board tree path $W (not a live worktree of $main)"
    rm -rf "$W" || { say "cannot remove $W"; exit 1; }
  fi
  git -C "$main" worktree prune 2>/dev/null
  git -C "$main" worktree add -q --detach "$W" refs/remotes/origin/main || { say "cannot create board tree $W"; exit 1; }
fi
[ -e "$W/.venv" ] || ln -sfn "$main/.venv" "$W/.venv" || { say "cannot link $main/.venv into $W"; exit 1; }
ready=1

# The verb as the messages name it: `comment`, `voyage add`, `control halt`.
verb="$1"
case "${2:-}" in ""|*[!a-z-]*|-*) ;; *) verb="$verb $2" ;; esac

# 9. What the verb left uncommitted in the board tree (HAZ-010): listed, and saved as one patch outside the board
# tree and the caller's checkout, before the clean-up discards it. Prints nothing when the tree is clean.
left_behind() {
  local st t tp d p
  st=$(git -C "$W" status --short -uall -- . ':(exclude).venv' 2>/dev/null)
  [ -n "$st" ] || return 0
  say "'$verb' left this uncommitted in the board tree (git status --short):"
  printf '%s\n' "$st" >&2
  t="${TMPDIR:-/tmp}"
  [ "$t" = "/" ] || t="${t%/}"
  case "$t" in /*) ;; *) t="$PWD/$t" ;; esac
  tp=$(phys "$t") || tp=""
  for d in "$W" "$main" "$(git rev-parse --show-toplevel 2>/dev/null)"; do
    [ -n "$d" ] && d=$(phys "$d") || continue
    case "$tp/" in "$d"/*) tp="" ;; esac     # never inside the board tree or a checkout
  done
  [ -n "$tp" ] || t="/tmp"
  if p=$(mktemp "$t/yk-push-uncommitted.XXXXXX" 2>/dev/null) \
     && git -C "$W" add -A 2>/dev/null \
     && git -C "$W" diff --cached --binary --no-color --no-ext-diff --no-textconv --src-prefix=a/ --dst-prefix=b/ \
          HEAD -- . ':(exclude).venv' > "$p" 2>/dev/null && [ -s "$p" ]; then
    say "saved as a patch (yours to delete): $p"
    say "to get it back: git apply $p   on a checkout of $start"
  else
    [ -n "${p:-}" ] && rm -f "$p"
    say "COULD NOT save it as a patch under $t: the edit listed above is lost — report it"
  fi
}

# 4. Write on a fresh origin/main; on a lost race drop the commit and redo the verb.
i=1
while [ $i -le 5 ]; do
  if [ $i -gt 1 ]; then
    git -C "$W" fetch -q origin || { say "origin unreachable, nothing written"; exit 4; }
  fi
  git -C "$W" checkout -q -f --detach refs/remotes/origin/main && git -C "$W" clean -qfd -e .venv \
    || { say "cannot reset board tree $W"; exit 1; }
  start=$(git -C "$W" rev-parse --verify -q HEAD) || { say "cannot read HEAD of board tree $W"; exit 1; }
  (cd "$W" && PYTHONPATH= "$YK" "${args[@]}")
  rc=$?
  if [ $rc -ne 0 ]; then
    say "refused by yurtle-kanban ('$verb' exited $rc), nothing written"
    left_behind
    exit 1
  fi
  # 9. "Landed" needs a commit: the verb must have moved HEAD past the origin/main it started on (HAZ-010).
  if [ "$(git -C "$W" rev-parse --verify -q HEAD)" = "$start" ]; then
    if [ $i -gt 1 ]; then
      say "'$verb' made no commit when redone after $((i - 1)) lost race(s) (its earlier commit was dropped): nothing was pushed"
    else
      say "'$verb' made no commit: nothing was pushed"
    fi
    left_behind
    exit 6
  fi
  if git -C "$W" push -q origin HEAD:refs/heads/main 2>/dev/null; then
    git -C "$main" fetch -q origin 2>/dev/null
    exit 0
  fi
  [ $i -lt 5 ] && sleep "$((i - 1)).$((RANDOM % 10))"   # lost the race: redo on the new origin/main
  i=$((i + 1))
done
say "5 lost races, nothing written — report it"
exit 5
