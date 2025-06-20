use crate::git_ops;

pub async fn handle_land(all: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    match git_ops::land_merged_prs(all, dry_run).await {
        Ok(()) => {
            // Landing completed successfully
        }
        Err(e) => {
            eprintln!("Error during land operation: {}", e);
        }
    }
    Ok(())
}