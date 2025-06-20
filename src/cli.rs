use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitx")]
#[command(about = "A Git extension tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Branch operations
    Branch,
    /// Create a commit (passthrough to git commit)
    Commit {
        /// Arguments to pass to git commit
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Create/update stacked PRs from commits
    Diff {
        /// Show all commits and let user choose interactively
        #[arg(long)]
        all: bool,
        /// Show what would be done without creating PRs
        #[arg(long)]
        dry_run: bool,
    },
    /// Show status of current stacked PRs
    Prs,
    /// Show git status (passthrough to git status)
    Status {
        /// Arguments to pass to git status
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Initialize gitx configuration for this repository
    Init,
    /// Clean up merged PRs and sync with remote
    Land {
        /// Clean up all merged PRs
        #[arg(long)]
        all: bool,
        /// Show what would be cleaned up without making changes
        #[arg(long)]
        dry_run: bool,
    },
}