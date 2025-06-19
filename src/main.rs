use clap::{Parser, Subcommand};
use inquire::Select;

mod git_ops;

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
    }
}
