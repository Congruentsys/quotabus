//! Redaction is a mechanism, not a rule (DESIGN §6).

use std::sync::LazyLock;

use regex::Regex;

use crate::secret::Secret;

/// Error text is capped at this many chars.
pub const ERROR_CAP: usize = 200;

/// What a scrubbed value becomes.
const MASK: &str = "***";

/// Prefixes whose following token is a credential. `word_start`: the prefix only counts at the start of a word
/// (so `task-1` or `disk-full` is left alone).
const PATTERNS: &[(&str, bool)] = &[("Bearer ", false), ("sk-", true), ("ghp_", true)];

/// An email address. The domain must end in an alphabetic label, so `host/quotabus@0.1.0` is not one.
static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}")
        .expect("email regex")
});
/// An organisation id such as OpenAI's `org-…`.
static ORG_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\borg-[A-Za-z0-9]{6,}").expect("org id regex"));
/// The value of a JSON `organization` / `organization_id` / `org_id` field.
static ORG_FIELD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"("(?:organization|organization_id|org_id)"\s*:\s*")[^"]*(")"#)
        .expect("org field regex")
});

/// Holds every resolved secret value; scrubs them, plus `sk-…`, `ghp_…` and `Bearer …` patterns, email addresses and
/// org ids (DESIGN §6: a record never reveals an email or org id unless the operator typed it as the label).
#[derive(Clone, Default)]
pub struct Redactor {
    secrets: Vec<Secret>,
    /// Labels the operator typed (the configured accounts): an email that IS one of these is kept.
    labels: Vec<String>,
}

impl Redactor {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Redactor {
            secrets,
            labels: Vec::new(),
        }
    }

    pub fn add(&mut self, secret: Secret) {
        self.secrets.push(secret);
    }

    /// Keep `label` (a configured account label) even where it looks like an email.
    pub fn allow_label(&mut self, label: impl Into<String>) {
        self.labels.push(label.into());
    }

    /// Scrub every known secret value and every pattern from `text`.
    pub fn redact(&self, text: &str) -> String {
        let mut values: Vec<&str> = self
            .secrets
            .iter()
            .map(Secret::expose)
            .filter(|v| !v.is_empty())
            .collect();
        // longest first, so a secret that contains another is scrubbed whole
        values.sort_by_key(|v| std::cmp::Reverse(v.len()));
        let mut out = text.to_string();
        for v in values {
            out = out.replace(v, MASK);
        }
        for (prefix, word_start) in PATTERNS {
            out = scrub_pattern(&out, prefix, *word_start);
        }
        out = EMAIL
            .replace_all(&out, |c: &regex::Captures| {
                let m = &c[0];
                if self.labels.iter().any(|l| l == m) {
                    m.to_string()
                } else {
                    MASK.to_string()
                }
            })
            .into_owned();
        out = ORG_ID.replace_all(&out, "org-***").into_owned();
        ORG_FIELD.replace_all(&out, "${1}***${2}").into_owned()
    }

    /// `redact`, then cap at `ERROR_CAP` chars.
    pub fn redact_error(&self, text: &str) -> String {
        self.redact(text).chars().take(ERROR_CAP).collect()
    }
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-_.~+/=".contains(c)
}

/// Replace the token after each `prefix` with `***`.
fn scrub_pattern(text: &str, prefix: &str, word_start: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(prefix) {
        let at_word_start = !word_start
            || rest[..i]
                .chars()
                .next_back()
                .or_else(|| out.chars().next_back())
                .is_none_or(|p| !p.is_ascii_alphanumeric());
        out.push_str(&rest[..i + prefix.len()]);
        rest = &rest[i + prefix.len()..];
        if !at_word_start {
            continue;
        }
        let token_len: usize = rest
            .chars()
            .take_while(|c| is_token_char(*c) && *c != '*')
            .map(char::len_utf8)
            .sum();
        if token_len > 0 {
            out.push_str(MASK);
            rest = &rest[token_len..];
        }
    }
    out.push_str(rest);
    out
}
