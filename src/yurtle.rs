//! The Yurtle config front-end (DESIGN §3 "Config — one model, two front-ends"; §10 Q7; EXP-004): a small, strict
//! reader for exactly the `yurtle-table` block of Yurtle v2.1 (`yurtle-spec.md` "Tabular Blocks", lines 137-157 at
//! the time DESIGN cites it): a fenced block whose info string is `yurtle-table`, holding one `@type <Type>` line and
//! one markdown table whose header row names the predicates and whose `@id` column names the subject.
//!
//! This is NOT a Yurtle parser. No Rust Yurtle reader exists (`docs/PRIOR-ART.md` names only the Python
//! `yurtle-rdflib`), so this module reads that one block type and nothing else: plain `yurtle` blocks, other fenced
//! blocks, frontmatter, prose and plain markdown tables are ignored. Of the spec's table rules it applies: the header
//! row is the predicates (rule 1), `@id` is required and is `#<id>` (rule 2), an `@type` line before the table (rule 3;
//! the per-row `@type` column is refused), an empty cell is absence (rule 4), a comma-separated cell is a list
//! (rule 5). Literal typing (rule 6) is by column, from the quotabus schema below, not by inference: a cell in a
//! numeric column must be a number, and every other cell is a string. Prefixes, CURIE headers and `@base` are not
//! supported and are refused.
//!
//! The rows become the same tables the TOML front-end reads (`[bus]`, `[[service]]`, `[alert.*]` …), so both
//! front-ends share one validation path ([`crate::Config::from_yurtle_str`]).
//!
//! | `@type`     | `@id` rows                                  | columns |
//! |-------------|---------------------------------------------|---------|
//! | `Service`   | `#<service id>`, one per `[[service]]`      | `kind provider family account base-url protocol models roles cost-class secret sources publish-balance probe balance-url balance-path currency floor context` |
//! | `Bus`       | `#bus` (at most one)                        | `url bucket` |
//! | `File`      | `#file` (at most one)                       | `dir` |
//! | `Probe`     | `#probe` (at most one)                      | `max-tokens degraded-latency-ms warn-pct` |
//! | `Schedule`  | `#api`, `#balance`, `#subscription`         | `interval ttl` (`[intervals]` / `[ttl]`) |
//! | `AlertSink` | `#nusy-kanban`, `#yurtle-kanban`, `#webhook` | `command item-type tags url` |
//!
//! List columns: `models roles sources tags context`; `context` entries are `<model>=<tokens>`. Numeric: `floor`,
//! `max-tokens`, `degraded-latency-ms`, `warn-pct`. Boolean: `publish-balance`, `probe` (`true` / `false`).

use toml::{Table, Value};

use crate::config::ConfigError;

fn err(msg: impl Into<String>) -> ConfigError {
    ConfigError(format!("yurtle-table: {}", msg.into()))
}

/// One `yurtle-table` block: its `@type`, header and rows (each cell trimmed), with the line each row is on.
struct Block {
    ty: String,
    header: Vec<String>,
    rows: Vec<(usize, Vec<String>)>,
}

/// An opening code fence: its char, its length and its info string (CommonMark: up to 3 spaces of indent).
fn fence(line: &str) -> Option<(char, usize, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let t = &line[indent..];
    let c = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = t.chars().take_while(|x| *x == c).count();
    (n >= 3).then(|| (c, n, t[n..].trim()))
}

/// Whether `line` closes a fence opened with `n` of `c`.
fn closes(line: &str, c: char, n: usize) -> bool {
    match fence(line) {
        Some((c2, n2, info)) => c2 == c && n2 >= n && info.is_empty(),
        None => false,
    }
}

/// The cells of a `| a | b |` row, trimmed. `None` when the line is not a table row.
fn cells(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    let inner = t.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|c| c.trim().to_string()).collect())
}

fn is_separator(cells: &[String]) -> bool {
    cells.iter().all(|c| {
        let c = c.trim_matches(':');
        !c.is_empty() && c.chars().all(|x| x == '-')
    })
}

/// Every `yurtle-table` block in `text`, in order. Other fenced blocks (of any length, so a `yurtle-table` shown
/// inside a ````` ````markdown ````` example is not read) are skipped whole.
fn blocks(text: &str) -> Result<Vec<Block>, ConfigError> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some((c, n, info)) = fence(lines[i]) else {
            i += 1;
            continue;
        };
        let start = i + 1;
        let mut end = start;
        while end < lines.len() && !closes(lines[end], c, n) {
            end += 1;
        }
        if info == "yurtle-table" {
            if end == lines.len() {
                return Err(err(format!(
                    "the block opened on line {} is never closed",
                    i + 1
                )));
            }
            out.push(block(&lines[start..end], start)?);
        }
        i = end + 1;
    }
    Ok(out)
}

/// One block's body. `first` is the 0-based line index of its first body line (for messages).
fn block(body: &[&str], first: usize) -> Result<Block, ConfigError> {
    let mut ty: Option<String> = None;
    let mut header: Option<Vec<String>> = None;
    let mut separated = false;
    let mut rows = Vec::new();
    for (j, line) in body.iter().enumerate() {
        let at = first + j + 1;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix("@type") {
            if header.is_some() || ty.is_some() {
                return Err(err(format!(
                    "line {at}: one `@type` line, before the table"
                )));
            }
            let name = rest.trim();
            if name.is_empty() || name.contains(char::is_whitespace) {
                return Err(err(format!("line {at}: `@type <Type>`")));
            }
            ty = Some(name.to_string());
            continue;
        }
        let Some(row) = cells(t) else {
            return Err(err(format!(
                "line {at}: only an `@type` line and one markdown table may be in a block (`@prefix`, `@base` and \
                 prose are not supported here): {t:?}"
            )));
        };
        match (&header, separated) {
            (None, _) => header = Some(row),
            (Some(_), false) => {
                if !is_separator(&row) {
                    return Err(err(format!(
                        "line {at}: the header row is followed by a |---| separator"
                    )));
                }
                separated = true;
            }
            (Some(h), true) => {
                if row.len() != h.len() {
                    return Err(err(format!(
                        "line {at}: {} cells, the header has {}",
                        row.len(),
                        h.len()
                    )));
                }
                rows.push((at, row));
            }
        }
    }
    let ty = ty.ok_or_else(|| err(format!("the block at line {first} has no `@type` line")))?;
    let header = header.ok_or_else(|| err(format!("the {ty} block has no table")))?;
    if !separated {
        return Err(err(format!("the {ty} block's table has no separator row")));
    }
    if header.first().map(String::as_str) != Some("@id") {
        return Err(err(format!("the {ty} table's first column is `@id`")));
    }
    for (k, h) in header.iter().enumerate() {
        if h.is_empty() || header[..k].contains(h) {
            return Err(err(format!(
                "the {ty} table has an empty or repeated column {h:?}"
            )));
        }
    }
    Ok(Block { ty, header, rows })
}

/// How a column's cells are typed.
#[derive(Clone, Copy)]
enum Col {
    Str,
    List,
    Int,
    Float,
    Bool,
    /// `<model>=<tokens>, …` → a table of integers.
    Context,
}

/// `(yurtle column, TOML key, type)` for each `@type`; `None` for an unknown type.
fn schema(ty: &str) -> Option<&'static [(&'static str, &'static str, Col)]> {
    use Col::*;
    Some(match ty {
        "Service" => &[
            ("kind", "kind", Str),
            ("provider", "provider", Str),
            ("family", "family", Str),
            ("account", "account", Str),
            ("base-url", "base_url", Str),
            ("protocol", "protocol", Str),
            ("models", "models", List),
            ("roles", "roles", List),
            ("cost-class", "cost_class", Str),
            ("secret", "secret", Str),
            ("sources", "sources", List),
            ("publish-balance", "publish_balance", Bool),
            ("probe", "probe", Bool),
            // the [service.balance] table, flattened
            ("balance-url", "url", Str),
            ("balance-path", "path", Str),
            ("currency", "currency", Str),
            ("floor", "floor", Float),
            ("context", "context", Context),
        ],
        "Bus" => &[("url", "url", Str), ("bucket", "bucket", Str)],
        "File" => &[("dir", "dir", Str)],
        "Probe" => &[
            ("max-tokens", "max_tokens", Int),
            ("degraded-latency-ms", "degraded_latency_ms", Int),
            ("warn-pct", "warn_pct", Float),
        ],
        "Schedule" => &[("interval", "interval", Str), ("ttl", "ttl", Str)],
        "AlertSink" => &[
            ("command", "command", Str),
            ("item-type", "item_type", Str),
            ("tags", "tags", List),
            ("url", "url", Str),
        ],
        _ => return None,
    })
}

fn list(cell: &str, at: usize, col: &str) -> Result<Vec<String>, ConfigError> {
    let items: Vec<String> = cell.split(',').map(|v| v.trim().to_string()).collect();
    if items.iter().any(String::is_empty) {
        return Err(err(format!("line {at}: {col} has an empty list item")));
    }
    Ok(items)
}

fn typed(cell: &str, col: Col, name: &str, at: usize) -> Result<Value, ConfigError> {
    let bad = |what: &str| err(format!("line {at}: {name} {cell:?} is not {what}"));
    Ok(match col {
        Col::Str => Value::String(cell.to_string()),
        Col::List => Value::Array(
            list(cell, at, name)?
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
        Col::Int => Value::Integer(cell.parse().map_err(|_| bad("an integer"))?),
        Col::Float => match cell.parse::<i64>() {
            Ok(i) => Value::Integer(i),
            Err(_) => Value::Float(
                cell.parse::<f64>()
                    .ok()
                    .filter(|f| f.is_finite())
                    .ok_or_else(|| bad("a number"))?,
            ),
        },
        Col::Bool => match cell {
            "true" => Value::Boolean(true),
            "false" => Value::Boolean(false),
            _ => return Err(bad("true or false")),
        },
        Col::Context => {
            let mut t = Table::new();
            for item in list(cell, at, name)? {
                let (model, tokens) = item
                    .split_once('=')
                    .ok_or_else(|| bad("<model>=<tokens>, …"))?;
                let tokens: i64 = tokens
                    .trim()
                    .parse()
                    .map_err(|_| bad("<model>=<tokens>, …"))?;
                if t.insert(model.trim().to_string(), Value::Integer(tokens))
                    .is_some()
                {
                    return Err(err(format!("line {at}: {name} names {model:?} twice")));
                }
            }
            Value::Table(t)
        }
    })
}

/// A row as `(id, TOML key → value)`; empty cells are left out.
fn row_table(b: &Block, at: usize, row: &[String]) -> Result<(String, Table), ConfigError> {
    let cols = schema(&b.ty).ok_or_else(|| {
        err(format!(
            "unknown @type {:?} (Service, Bus, File, Probe, Schedule, AlertSink)",
            b.ty
        ))
    })?;
    let id = row[0]
        .strip_prefix('#')
        .filter(|id| !id.is_empty() && !id.contains(char::is_whitespace))
        .ok_or_else(|| err(format!("line {at}: @id {:?} is `#<id>`", row[0])))?
        .to_string();
    let mut t = Table::new();
    for (h, cell) in b.header.iter().zip(row).skip(1) {
        let (_, key, col) = cols.iter().find(|(c, _, _)| c == h).ok_or_else(|| {
            err(format!(
                "the {} table has a column {h:?} quotabus does not read",
                b.ty
            ))
        })?;
        if !cell.is_empty() {
            t.insert((*key).to_string(), typed(cell, *col, h, at)?);
        }
    }
    Ok((id, t))
}

/// The yurtle-table blocks of `text` as the TOML config's tables.
pub fn config_table(text: &str) -> Result<Table, ConfigError> {
    let mut out = Table::new();
    let mut services = Vec::new();
    let mut alert = Table::new();
    let mut intervals = Table::new();
    let mut ttl = Table::new();
    for b in blocks(text)? {
        for (at, row) in &b.rows {
            let (id, mut t) = row_table(&b, *at, row)?;
            match b.ty.as_str() {
                "Service" => {
                    let mut balance = Table::new();
                    for k in ["url", "path", "currency", "floor"] {
                        if let Some(v) = t.remove(k) {
                            balance.insert(k.to_string(), v);
                        }
                    }
                    if !balance.is_empty() {
                        t.insert("balance".into(), Value::Table(balance));
                    }
                    t.insert("id".into(), Value::String(id));
                    services.push(Value::Table(t));
                }
                "Bus" | "File" | "Probe" => {
                    let want = b.ty.to_lowercase();
                    if id != want {
                        return Err(err(format!("line {at}: the {} row's @id is #{want}", b.ty)));
                    }
                    if out.insert(want.clone(), Value::Table(t)).is_some() {
                        return Err(err(format!("line {at}: a second #{want} row")));
                    }
                }
                "Schedule" => {
                    if !["api", "balance", "subscription"].contains(&id.as_str()) {
                        return Err(err(format!(
                            "line {at}: a Schedule row is #api, #balance or #subscription, not #{id}"
                        )));
                    }
                    for (key, dest) in [("interval", &mut intervals), ("ttl", &mut ttl)] {
                        if let Some(v) = t.remove(key)
                            && dest.insert(id.clone(), v).is_some()
                        {
                            return Err(err(format!("line {at}: a second #{id} {key}")));
                        }
                    }
                }
                "AlertSink" => {
                    if !["nusy-kanban", "yurtle-kanban", "webhook"].contains(&id.as_str()) {
                        return Err(err(format!(
                            "line {at}: an AlertSink row is #nusy-kanban, #yurtle-kanban or #webhook, not #{id}"
                        )));
                    }
                    if alert.insert(id.clone(), Value::Table(t)).is_some() {
                        return Err(err(format!("line {at}: a second #{id} row")));
                    }
                }
                _ => unreachable!("row_table refused the unknown type"),
            }
        }
    }
    if !services.is_empty() {
        out.insert("service".into(), Value::Array(services));
    }
    for (name, t) in [("intervals", intervals), ("ttl", ttl), ("alert", alert)] {
        if !t.is_empty() {
            out.insert(name.into(), Value::Table(t));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_of(ty: &str, table: &str) -> String {
        format!("```yurtle-table\n@type {ty}\n\n{table}\n```\n")
    }

    #[test]
    fn a_nested_fence_is_not_read_and_an_unclosed_block_is_refused() {
        let nested = format!(
            "````markdown\n{}````\n",
            block_of("Bus", "| @id | url |\n|-|-|\n| #bus | x |")
        );
        assert!(config_table(&nested).unwrap().is_empty());
        assert!(config_table("```yurtle-table\n@type Bus\n").is_err());
    }

    #[test]
    fn strictness_unknown_type_column_bad_id_and_ragged_rows_are_refused() {
        for bad in [
            block_of("Nope", "| @id | x |\n|-|-|\n| #a | 1 |"),
            block_of("Bus", "| @id | colour |\n|-|-|\n| #bus | red |"),
            block_of("Bus", "| @id | url |\n|-|-|\n| bus | x |"),
            block_of("Bus", "| @id | url |\n|-|-|\n| #bus | x | y |"),
            block_of("Probe", "| @id | max-tokens |\n|-|-|\n| #probe | twenty |"),
            "```yurtle-table\n| @id | url |\n|-|-|\n| #bus | x |\n```\n".to_string(),
            "```yurtle-table\n@prefix x: <y> .\n@type Bus\n| @id | url |\n|-|-|\n| #bus | x |\n```\n".to_string(),
        ] {
            assert!(config_table(&bad).is_err(), "refused: {bad}");
        }
    }

    #[test]
    fn schedule_rows_split_into_intervals_and_ttl() {
        let t = config_table(&block_of(
            "Schedule",
            "| @id | interval | ttl |\n|---|---|---|\n| #api | 12h | 36h |\n| #subscription | 1h | |",
        ))
        .unwrap();
        assert_eq!(t["intervals"]["api"].as_str(), Some("12h"));
        assert_eq!(t["intervals"]["subscription"].as_str(), Some("1h"));
        assert_eq!(t["ttl"]["api"].as_str(), Some("36h"));
        assert!(
            t["ttl"].get("subscription").is_none(),
            "an empty cell is absence"
        );
    }
}
