mod cli;
mod config;
mod daemon;
mod capture;
mod reader;
mod redact;
mod segment;
mod writer;
mod prune;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ambient-context")]
#[command(about = "Keeps a written record of what you worked on")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the capture daemon
    Start {
        /// Override the capture folder
        #[arg(long)]
        folder: Option<String>,
    },
    /// Stop the capture daemon
    Stop,
    /// Show daemon status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print the path to today's file (creates it if missing)
    Today,
    /// Take a single accessibility snapshot
    Snapshot {
        /// Show unredacted output
        #[arg(long)]
        unredacted: bool,
    },
    /// View daemon logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Start { folder } => cli::start(folder),
        Commands::Stop => cli::stop(),
        Commands::Status { json } => cli::status(json),
        Commands::Today => cli::today(),
        Commands::Snapshot { unredacted } => cli::snapshot(unredacted),
        Commands::Logs { follow } => cli::logs(follow),
    };

    if let Err(e) = result {
        eprintln!("ambient-context: {e}");
        std::process::exit(1);
    }
}
