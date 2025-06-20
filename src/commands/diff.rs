use inquire::MultiSelect;
use crate::git_ops;

/// Display commits and let user interactively select which ones to process
fn select_commits_to_process(updates: &[git_ops::CommitUpdateType]) -> Result<Vec<git_ops::CommitUpdateType>, Box<dyn std::error::Error>> {
    // Create display options for the user with indices
    let options: Vec<(usize, String)> = updates.iter().enumerate().map(|(i, update)| {
        let display = match update {
            git_ops::CommitUpdateType::NewCommit(commit) => {
                let short_id = &commit.id.to_string()[..8];
                let title = commit.message.lines().next().unwrap_or("Untitled");
                format!("🆕 {} {} (new commit)", short_id, title)
            }
            git_ops::CommitUpdateType::IncrementalUpdate { updated_oid, metadata, .. } => {
                let short_id = &updated_oid.to_string()[..8];
                let title = metadata.pr_branch_name.split('/').last().unwrap_or("unknown");
                format!("🔄 {} {} (incremental update)", short_id, title)
            }
        };
        (i, display)
    }).collect();
    
    // Extract just the display strings for the menu
    let option_strings: Vec<String> = options.iter().map(|(_, display)| display.clone()).collect();
    
    // Show multi-select menu
    let selected_options = MultiSelect::new("Select commits to process:", option_strings)
        .with_help_message("Use space to select/deselect, arrow keys to navigate, enter to confirm")
        .prompt()?;
    
    if selected_options.is_empty() {
        return Err("No commits selected".into());
    }
    
    // Map selected options back to indices, then to commits
    let selected_updates: Vec<git_ops::CommitUpdateType> = selected_options
        .into_iter()
        .filter_map(|selected_option| {
            // Find the index for this selected option
            options.iter()
                .find(|(_, display)| *display == selected_option)
                .map(|(index, _)| updates[*index].clone())
        })
        .collect();
    
    Ok(selected_updates)
}

pub async fn handle_diff(github: bool, all: bool) -> Result<(), Box<dyn std::error::Error>> {
    let updates = if all {
        git_ops::get_commits_needing_processing()
    } else {
        git_ops::get_latest_commit_needing_processing()
    };
    
    match updates {
        Ok(updates) => {
            if updates.is_empty() {
                println!("No new commits or updates to process");
                return Ok(());
            }
            
            // If --all flag is used, show interactive selection (if multiple commits)
            let selected_updates = if all {
                if updates.len() > 1 {
                    match select_commits_to_process(&updates) {
                        Ok(selected) => selected,
                        Err(e) => {
                            eprintln!("Selection cancelled: {}", e);
                            return Ok(());
                        }
                    }
                } else {
                    // Only one commit, process it directly
                    println!("Only one commit available, processing it:");
                    updates
                }
            } else {
                updates
            };
            
            let mut new_branches = 0;
            let mut incremental_updates = 0;
            
            for update in &selected_updates {
                match update {
                    git_ops::CommitUpdateType::NewCommit(commit) => {
                        println!("Creating PR branch for: {}", commit.message.lines().next().unwrap_or(""));
                        
                        if github {
                            match git_ops::create_pr_branch_with_github(commit, true).await {
                                Ok(Some(_pr_info)) => {
                                    new_branches += 1;
                                }
                                Ok(None) => {
                                    new_branches += 1;
                                }
                                Err(e) => {
                                    eprintln!("Error creating branch/PR '{}': {:#}", commit.potential_branch_name, e);
                                    
                                    // Print the full error chain for debugging
                                    let mut source = e.source();
                                    while let Some(err) = source {
                                        eprintln!("  Caused by: {}", err);
                                        source = err.source();
                                    }
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
                        
                        if github {
                            match git_ops::create_incremental_commit_with_github(original_oid, updated_oid, metadata, true).await {
                                Ok(()) => {
                                    incremental_updates += 1;
                                }
                                Err(e) => {
                                    eprintln!("Error creating incremental commit/PR update for '{}': {:#}", metadata.pr_branch_name, e);
                                    
                                    // Print the full error chain for debugging
                                    let mut source = e.source();
                                    while let Some(err) = source {
                                        eprintln!("  Caused by: {}", err);
                                        source = err.source();
                                    }
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
    Ok(())
}