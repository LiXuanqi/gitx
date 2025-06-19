use clap::{Parser, Subcommand};
use inquire::Select;

mod git_ops;
mod branch_naming;

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
    Diff,
}

fn main() {
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
        Commands::Diff => {
            match git_ops::get_unpushed_commits() {
                Ok(commits) => {
                    if commits.is_empty() {
                        println!("No new commits to create PRs for");
                        return;
                    }
                    
                    println!("Found {} commits that could become PRs:", commits.len());
                    
                    // Create branches for each commit
                    let mut created_count = 0;
                    for commit in &commits {
                        println!("Creating PR branch for: {}", commit.message.lines().next().unwrap_or(""));
                        
                        match git_ops::create_pr_branch(commit) {
                            Ok(()) => {
                                created_count += 1;
                            }
                            Err(e) => {
                                eprintln!("Error creating branch '{}': {}", commit.potential_branch_name, e);
                            }
                        }
                    }
                    
                    println!("\nCreated {} PR branches", created_count);
                }
                Err(e) => {
                    eprintln!("Error analyzing commits: {}", e);
                }
            }
        }
    }
}
