# quotabus — instructions for agents

- **The board is `kanban-work/`, run with yurtle-kanban** (`pip install yurtle-kanban`; 3.4.0 at creation). Create
  items with `yurtle-kanban create <type> "<title>" --body-file - --push` (atomic: it allocates the id, commits and
  pushes). Never hand-number an item.
- **Design first:** `docs/DESIGN.md` is the agreed design; a change to it is a PR, not a silent edit.
- **Never commit, log or publish a secret.** Keys arrive by environment only; every output passes the redactor.
- **Code lands by a branch and a GitHub PR** reviewed by a different session than the one that wrote it. Tests are
  written before the code.
- **No human time estimates.** Size work by agent context: a chore is a fraction of a context, an expedition is what
  one session can land.
- This repo is tracked from nusy-product-team by IDEA-13333 (dual-tracking for FOSS repos).
