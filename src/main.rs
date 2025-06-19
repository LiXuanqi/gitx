use clap::{Parser, Subcommand};

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
            println!("Branch command executed");
        }
    }
}
