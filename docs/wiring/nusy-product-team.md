# Wiring note: nusy-product-team's reviewers pick their model with `quotabus select`

**EXP-003, 2026-10-08.** This is a note for nusy-product-team, not a change to it. The change described here lands
in **that** repo, as a separate item on its own board (nusy-product-team's `nk`), written and reviewed there. This repo
never edits nusy-product-team; this file only says what the call looks like and how its exit code is read. The Captain
froze nusy-product-team's loop skills until this app exists (EXP-003's Plan), so the item there is filed once quotabus
is installed on the host that runs the review.

## What it replaces

Two places in nusy-product-team name a reviewer model by hand today:

- **`pairit`** (`.claude/skills/pairit/SKILL.md`, step 4 REVIEW): a distinct `claude -p` reviewer session. When the
  author is a Claude session, a reviewer of a different family runs through `claude -p` pointed at an
  Anthropic-protocol provider (`docs/external-review.md` §4.1).
- **`external-review`** (`docs/external-review.md` §0, §1, §1a): the table of providers and the hand probe of
  §1a ("re-probe before trusting; model names drift"). The rule that does not bend is §0 rule (1): *the reviewer's
  model family is not the author's*.

The line both would use:

```bash
$(quotabus select --role review --exclude-family anthropic)
```

It prints `provider model` on one line: the first candidate in `--prefer` order (default `cheapest`: cost class,
then latency, then context; DESIGN §3, "selector"). Every candidate is equally healthy: a model whose service lists
the `review` role, whose family is not excluded, and whose row on the bus is a fresh `ok` (never `degraded`, never an
expired row, never `unknown`). It reads the bus and never writes it. **A subscription seat is never chosen** (a Claude account, the Copilot seat),
decided on quotabus SIG-009 by an agent session's `steer` pass (bucket 2, 2026-10-08; not a Captain ruling, and open
to the Captain's veto; `kanban-work/signals/SIG-009-*.md:30-38`): the selector names only API and local models, so the
Copilot CLI route of external-review §4.2 is not a pick, and the provider map below needs no `github` arm. Routing
reviews to a seat would be a new feature, a new item for the Captain to file.

## Exit codes, and what the caller does

| rc | meaning | the caller |
|---|---|---|
| 0 | a model was chosen; stdout line 1 is `provider model` | run the review on it |
| 2 | CANNOT-ASSESS: the bus or bucket could not be read (or a bad `--prefer` word) | **do not run the review**; say why on the item, retry later. Never fall back to a hand-picked model: that is the false all-clear §0 rule (2) warns about |
| 3 | nothing qualifies: no configured review model of another family has a fresh `ok` row | **do not run the review**; the item waits (`stranded`/blocked, naming rc 3), and the Captain is told no healthy non-Anthropic reviewer exists |

On rc 2 and rc 3 stdout names no model, so `$(…)` is empty and cannot be mistaken for a pick. Every rc other than 0
means "no review now", never "review with the author's family" (§0 rule (1)).

## The call, for `pairit`'s reviewer step

```bash
pick=$(quotabus select --role review --exclude-family anthropic) ; rc=$?
case $rc in
  0) ;;                                           # chosen
  3) echo "no healthy non-anthropic reviewer (quotabus rc 3): review not run" >&2; exit 3 ;;
  *) echo "quotabus cannot assess the providers (rc $rc): review not run" >&2; exit 2 ;;
esac
read -r provider model <<<"$pick"
# provider -> base URL and the Doppler secret NAME (external-review §1); quotabus's own config holds the same pair
case $provider in
  zhipu)    base=https://api.z.ai/api/anthropic;      secret=NUSY_GLM ;;
  deepseek) base=https://api.deepseek.com/anthropic;  secret=NUSY_DEEPSEEK ;;
  moonshot) base=https://api.moonshot.ai/anthropic;   secret=NUSY_KIMI ;;
  *) echo "quotabus chose $provider, which this reviewer step has no transport for" >&2; exit 2 ;;
esac
doppler run --scope "$PWD" --project nusy-product-team --config dev -- sh -c '
  unset ANTHROPIC_API_KEY CLAUDE_CODE_OAUTH_TOKEN
  ANTHROPIC_AUTH_TOKEN=$(printenv "$1"); export ANTHROPIC_AUTH_TOKEN
  [ -n "$ANTHROPIC_AUTH_TOKEN" ] || { echo "$1 is empty: refusing" >&2; exit 2; }
  ANTHROPIC_BASE_URL=$0; export ANTHROPIC_BASE_URL
  exec claude -p "<the reviewer brief>" --model "$2" --dangerously-skip-permissions' \
  "$base" "$secret" "$model" 2> reviewer.err
```

The secret is passed by NAME and exported by the shell builtin inside the injected environment, so its value is never
on any process's argv (an `env VAR=value …` prefix would put it there). An empty value refuses, as external-review §2
trap 3 requires: an empty `ANTHROPIC_AUTH_TOKEN` would silently fall back to the host's own Anthropic login.
`--n 3 --json` gives the ranked list with each candidate's reason, for the item comment that records which reviewer
ran and why.

## The call, for `external-review`'s runner (§3)

`docs/external-review.md` §0's first row ("a REVIEW by a strong non-Claude model") becomes: run
`$(quotabus select --role review --exclude-family anthropic)`, apply the rc table above, then pass the chosen model to
the streaming runner (`--provider glm|deepseek|kimi` from the same provider map, `--model "$model"`). §1a's hand
probe is replaced by `quotabus status` (DESIGN §9 row C1); the "second opinion from a different family" row becomes
`--exclude-family anthropic --exclude-family <the first reviewer's family>`.

## What the separate item in nusy-product-team must check

- the selector's rc is read, and rc 2 / rc 3 stop the review (a control: a bus that is down gives rc 2 and no review);
- the excluded family is the AUTHOR's family, not a constant, if a non-Claude author ever writes there;
- the provider map covers every provider quotabus's config gives the `review` role, or the step refuses (rc 2) on the
  one it cannot run.
