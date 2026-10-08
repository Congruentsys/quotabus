# make check — the merge gate for the board and the scripts: their tests, then `yurtle-kanban validate`.
# Runs from the main checkout or any worktree of it (a worktree links the main checkout's .venv).
# EXP-001 adds the Rust half: cargo fmt --check, clippy -D warnings, cargo test (--locked). CARGO and
# CARGO_TARGET_DIR are overridable; the default target dir is outside the tree, one per checkout.
MAIN := $(shell git rev-parse --path-format=absolute --git-common-dir 2>/dev/null | sed "s|/\.git$$||")
PY ?= $(firstword $(wildcard .venv/bin/python) $(wildcard $(MAIN)/.venv/bin/python) python3)
YK ?= $(firstword $(wildcard .venv/bin/yurtle-kanban) $(wildcard $(MAIN)/.venv/bin/yurtle-kanban) yurtle-kanban)
export PYTHONDONTWRITEBYTECODE := 1
CARGO ?= $(firstword $(shell command -v cargo 2>/dev/null) $(wildcard $(HOME)/.cargo/bin/cargo) cargo)
CARGO_TARGET_DIR ?= /tmp/qb-target-$(notdir $(CURDIR))
export CARGO_TARGET_DIR

.PHONY: check
check:
	PYTHONPATH= $(PY) -m pytest -q tests
	PYTHONPATH= $(YK) validate
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets --locked -- -D warnings
	$(CARGO) test --locked
