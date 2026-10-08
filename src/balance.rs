//! Balance adapters (DESIGN §3 `[service.balance]`, §4 DeepSeek / Kimi).

use crate::record::State;

/// Read a number (or a numeric string) at a path such as `balance_infos[0].total_balance`.
pub fn extract_amount(json: &serde_json::Value, path: &str) -> Option<f64> {
    let _ = (json, path);
    todo!("extract_amount")
}

/// A balance at or below `floor` (a negative one included) reads `quota_exhausted`; above it the probe's state stands.
pub fn apply_floor(probe_state: State, amount: f64, floor: f64) -> State {
    let _ = (probe_state, amount, floor);
    todo!("apply_floor")
}
