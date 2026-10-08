use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "quotabus",
    version,
    about = "AI provider & account status on a bus"
)]
struct Cli {
    /// Path to quotabus.toml.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run one probe cycle and write one row per (service, model) to the backend.
    Probe,
    /// Read rows and apply the freshness rule.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        stale: bool,
        /// Exit 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN for this service id.
        #[arg(long)]
        check: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let _ = (cli.config, cli.command);
    todo!("quotabus main")
}
