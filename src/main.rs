use clap::Parser;

mod git_ops;
mod branch_naming;
mod metadata;
mod github;
mod status_display;
mod config;
mod cli;
mod commands;

use cli::{Cli, Commands};


#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Branch => commands::branch::handle_branch(),
        Commands::Commit { args } => commands::commit::handle_commit(args),
        Commands::Diff { all, dry_run } => commands::diff::handle_diff(*all, *dry_run).await,
        Commands::Init => commands::init::handle_init(),
        Commands::Land { all, dry_run } => commands::land::handle_land(*all, *dry_run).await,
        Commands::Prs => commands::prs::handle_prs().await,
        Commands::Status { args } => commands::status::handle_status(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
