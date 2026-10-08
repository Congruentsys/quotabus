//! Balance adapters (DESIGN §3 `[service.balance]`, §4 DeepSeek / Kimi).

use crate::record::State;

/// Read a number (or a numeric string) at a path such as `balance_infos[0].total_balance`.
pub fn extract_amount(json: &serde_json::Value, path: &str) -> Option<f64> {
    let mut cur = json;
    for part in path.split('.') {
        let (name, indices) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !name.is_empty() {
            cur = cur.get(name)?;
        }
        let mut rest = indices;
        while let Some(stripped) = rest.strip_prefix('[') {
            let end = stripped.find(']')?;
            cur = cur.get(stripped[..end].parse::<usize>().ok()?)?;
            rest = &stripped[end + 1..];
        }
        if !rest.is_empty() {
            return None;
        }
    }
    match cur {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok().filter(|f| f.is_finite()),
        _ => None,
    }
}

/// A balance at or below `floor` (a negative one included) turns a WORKING model (ok, degraded, rate_limited) into
/// `quota_exhausted`. It never replaces a measured failure (auth_failed, model_missing, unknown): that state stands,
/// as it does above the floor (DESIGN §2, the EXP-001 real-run ruling).
pub fn apply_floor(probe_state: State, amount: f64, floor: f64) -> State {
    if amount <= floor {
        exhaust(probe_state)
    } else {
        probe_state
    }
}

/// What a balance at or below its floor does to a model's state: a working one reads `quota_exhausted`, any other
/// stands.
pub fn exhaust(probe_state: State) -> State {
    let working = matches!(
        probe_state,
        State::Ok | State::Degraded | State::RateLimited
    );
    if working {
        State::QuotaExhausted
    } else {
        probe_state
    }
}
