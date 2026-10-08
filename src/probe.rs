//! The probe runner (DESIGN §4): one row per (service, model), keys from the environment only.

use crate::config::Config;
use crate::record::Record;

type EnvLookup = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Runs one probe cycle over every configured service. A service's `base_url` (and balance `url`) is where it is
/// probed, so a test points it at a local stub.
pub struct Runner {
    config: Config,
    env: EnvLookup,
}

impl Runner {
    /// `env` resolves a secret NAME to its value (`None` or empty ⇒ the service is not probed).
    pub fn new<F>(config: Config, env: F) -> Runner
    where
        F: Fn(&str) -> Option<String> + Send + Sync + 'static,
    {
        Runner {
            config,
            env: Box::new(env),
        }
    }

    /// Resolve secrets from the process environment.
    pub fn from_process_env(config: Config) -> Runner {
        Runner::new(config, |name| std::env::var(name).ok())
    }

    /// One cycle: one redacted row per (service, model). Rows are returned, not published.
    pub async fn run_once(&self) -> Vec<Record> {
        let _ = (&self.config, &self.env);
        todo!("Runner::run_once")
    }
}
