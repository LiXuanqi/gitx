use clap::{Parser, Subcommand};
use inquire::Select;

mod git_ops;
mod branch_naming;
mod metadata;
mod github;
mod status_display;

#[derive(Parser)]
#[command(name = "gitx")]
#[command(about = "A Git extension tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Branch operations
    Branch,
    /// Create/update stacked PRs from commits
    Diff {
        /// Also create/update GitHub PRs
        #[arg(long)]
        github: bool,
    },
    /// Show status of current stacked PRs
    Status,
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Branch => {
            match git_ops::get_all_branches() {
                Ok(branches) => {
                    if branches.is_empty() {
                        println!("No branches found");
                        return;
                    }
                    
                    let selection = Select::new("Select a branch:", branches).prompt();
                    
                    match selection {
                        Ok(chosen_branch) => {
                            match git_ops::switch_branch(&chosen_branch) {
                                Ok(()) => {
                                    println!("Switched to branch: {}", chosen_branch);
                                }
                                Err(e) => {
                                    eprintln!("Error switching to branch '{}': {}", chosen_branch, e);
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Selection cancelled: {}", err);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error getting branches: {}", e);
                }
            }
        }
        Commands::Diff { github } => {
            match git_ops::get_commits_needing_processing() {
                Ok(updates) => {
                    if updates.is_empty() {
                        println!("No new commits or updates to process");
                        return;
                    }
                    
                    let mut new_branches = 0;
                    let mut incremental_updates = 0;
                    
                    for update in &updates {
                        match update {
                            git_ops::CommitUpdateType::NewCommit(commit) => {
                                println!("Creating PR branch for: {}", commit.message.lines().next().unwrap_or(""));
                                
                                if *github {
                                    match git_ops::create_pr_branch_with_github(commit, true).await {
                                        Ok(Some(_pr_info)) => {
                                            new_branches += 1;
                                        }
                                        Ok(None) => {
                                            new_branches += 1;
                                        }
                                        Err(e) => {
                                            eprintln!("Error creating branch/PR '{}': {}", commit.potential_branch_name, e);
                                        }
                                    }
                                } else {
                                    match git_ops::create_pr_branch(commit) {
                                        Ok(()) => {
                                            new_branches += 1;
                                        }
                                        Err(e) => {
                                            eprintln!("Error creating branch '{}': {}", commit.potential_branch_name, e);
                                        }
                                    }
                                }
                            }
                            git_ops::CommitUpdateType::IncrementalUpdate { original_oid, updated_oid, metadata } => {
                                println!("Creating incremental update for: {}", metadata.pr_branch_name);
                                
                                if *github {
                                    match git_ops::create_incremental_commit_with_github(original_oid, updated_oid, metadata, true).await {
                                        Ok(()) => {
                                            incremental_updates += 1;
                                        }
                                        Err(e) => {
                                            eprintln!("Error creating incremental commit/PR update for '{}': {}", metadata.pr_branch_name, e);
                                        }
                                    }
                                } else {
                                    match git_ops::create_incremental_commit(original_oid, updated_oid, metadata) {
                                        Ok(()) => {
                                            incremental_updates += 1;
                                        }
                                        Err(e) => {
                                            eprintln!("Error creating incremental commit for '{}': {}", metadata.pr_branch_name, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if new_branches > 0 || incremental_updates > 0 {
                        println!("\nCompleted: {} new branches, {} incremental updates", new_branches, incremental_updates);
                    }
                }
                Err(e) => {
                    eprintln!("Error analyzing commits: {}", e);
                }
            }
        }
        Commands::Status => {
            match status_display::display_status().await {
                Ok(()) => {
                    // Status displayed successfully
                }
                Err(e) => {
                    eprintln!("Error displaying status: {}", e);
                }
            }
        }
        Commands::Land { all, dry_run } => {
            match git_ops::land_merged_prs(*all, *dry_run).await {
                Ok(()) => {
                    // Landing completed successfully
                }
                Err(e) => {
                    eprintln!("Error during land operation: {}", e);
                }
            }
        }
    }
}
