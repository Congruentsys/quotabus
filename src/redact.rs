//! Redaction is a mechanism, not a rule (DESIGN §6).

use crate::secret::Secret;

/// Error text is capped at this many chars.
pub const ERROR_CAP: usize = 200;

/// Holds every resolved secret value; scrubs them, plus `sk-…`, `ghp_…` and `Bearer …` patterns.
#[derive(Clone, Default)]
pub struct Redactor {
    secrets: Vec<Secret>,
}

impl Redactor {
    pub fn new(secrets: Vec<Secret>) -> Self {
        Redactor { secrets }
    }

    pub fn add(&mut self, secret: Secret) {
        self.secrets.push(secret);
    }

    /// Scrub every known secret value and every pattern from `text`.
    pub fn redact(&self, text: &str) -> String {
        let _ = (&self.secrets, text);
        todo!("Redactor::redact")
    }

    /// `redact`, then cap at `ERROR_CAP` chars.
    pub fn redact_error(&self, text: &str) -> String {
        let _ = text;
        todo!("Redactor::redact_error")
    }
}
