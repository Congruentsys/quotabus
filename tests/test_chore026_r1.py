"""CHORE-026 review r1 (reviews/CHORE-026-r1.md): F1 and F2, against the README's own quotabus-cycle wrapper.

F1: with no `--config`, `alert` gets no arguments, so its own fallback ($QUOTABUS_CONFIG, else ./quotabus.toml) applies;
    an empty `--config ""` would make the real alert exit 2.
F2: the `--config=<file>` form reaches `alert` as `--config <file>`.
Same method as tests/test_chore026_cycle_wrapper.py (wrapper extracted from the README, quotabus path pointed at a
clap-like stub); its helpers are reused, not retyped. Fakes only.
"""
from test_chore026_cycle_wrapper import readme_wrapper, run_wrapper, PRE_FIX_WRAPPER


def test_f1_no_config_gives_alert_no_arguments(tmp_path):
    proc, calls = run_wrapper(tmp_path, readme_wrapper(), ["probe", "--force"])
    assert [c[0] for c in calls] == ["probe", "alert"], calls
    assert calls[0] == ["probe", "--force"], calls[0]
    assert calls[1] == ["alert"], f"alert argv {calls[1]}: with no --config, alert must use its own fallback"


def test_f1_control_old_wrapper_also_gave_bare_alert(tmp_path):
    """Control: the pre-fix wrapper called a bare alert here (the behaviour F1 restores), so the check can pass."""
    _, calls = run_wrapper(tmp_path, PRE_FIX_WRAPPER.replace('alert "$@"', 'alert'), ["probe", "--force"])
    assert calls[1] == ["alert"], calls


def test_f2_config_equals_form_reaches_alert(tmp_path):
    cfg = str(tmp_path / "quotabus.toml")
    _, calls = run_wrapper(tmp_path, readme_wrapper(), ["probe", f"--config={cfg}", "--force"])
    subs = [c[0] for c in calls]
    assert subs == ["probe", "alert"], calls
    assert calls[0] == ["probe", f"--config={cfg}", "--force"], calls[0]
    assert calls[1] == ["alert", "--config", cfg], f"alert argv {calls[1]}"
    # no rc check: the shared stub's probe rejects `--config=<file>` (real clap accepts it); argv is the property here
