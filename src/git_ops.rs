use git2::{Repository, BranchType, Oid};
use crate::branch_naming;
use crate::metadata;

pub fn get_all_branches() -> Result<Vec<String>, git2::Error> {
    let repo = Repository::open(".")?;
    let mut branches = Vec::new();
    
    let branch_iter = repo.branches(Some(BranchType::Local))?;
    
    for branch in branch_iter {
        let (branch, _) = branch?;
        if let Some(name) = branch.name()? {
            branches.push(name.to_string());
        }
    }
    
    Ok(branches)
}

pub fn switch_branch(branch_name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    
    // Get the branch reference
    let branch_ref = format!("refs/heads/{}", branch_name);
    let obj = repo.revparse_single(&branch_ref)?;
    
    // Checkout the branch
    repo.checkout_tree(&obj, None)?;
    
    // Set HEAD to point to the branch
    repo.set_head(&branch_ref)?;
    
    Ok(())
}

/// Get the current git user name from config
pub fn get_git_username() -> Result<String, git2::Error> {
    let repo = Repository::open(".")?;
    let config = repo.config()?;
    
    config.get_string("user.name")
}

/// Information about updates needed for commits
#[derive(Debug, Clone)]
pub enum CommitUpdateType {
    NewCommit(CommitInfo),
    IncrementalUpdate {
        original_oid: Oid,
        updated_oid: Oid,
        metadata: metadata::CommitMetadata,
    },
}

/// Get commits on main branch that need processing (new commits or incremental updates)
pub fn get_commits_needing_processing() -> Result<Vec<CommitUpdateType>, git2::Error> {
    let repo = Repository::open(".")?;
    let mut updates = Vec::new();
    
    // Get main branch head
    let main_ref = repo.find_reference("refs/heads/main")
        .or_else(|_| repo.find_reference("refs/heads/master"))?;
    let main_commit = main_ref.peel_to_commit()?;
    
    // Walk commits from HEAD
    let mut revwalk = repo.revwalk()?;
    revwalk.push(main_commit.id())?;
    
    let username = get_git_username().unwrap_or_else(|_| "unknown".to_string());
    
    for oid in revwalk.take(10) { // Limit to last 10 commits for now
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("").to_string();
        
        // Check if this position in history has existing metadata stored elsewhere
        // (This handles the case where commits are amended/rebased)
        let current_commit_id = oid.to_string();
        let mut found_metadata_for_position = false;
        
        // Check if we have metadata for this commit
        if let Some(existing_metadata) = metadata::get_commit_metadata(&oid)? {
            // Check if the stored original commit ID matches current commit
            if existing_metadata.is_commit_changed(&current_commit_id) {
                // This means the commit was amended - we need an incremental update
                updates.push(CommitUpdateType::IncrementalUpdate {
                    original_oid: oid,
                    updated_oid: oid,
                    metadata: existing_metadata,
                });
                found_metadata_for_position = true;
            } else {
                // Commit unchanged, skip
                found_metadata_for_position = true;
            }
        }
        
        if !found_metadata_for_position {
            // No metadata found - this is a new commit
            let potential_branch = branch_naming::generate_branch_name(&username, &message);
            
            updates.push(CommitUpdateType::NewCommit(CommitInfo {
                id: oid,
                message: message.clone(),
                potential_branch_name: potential_branch,
            }));
        }
    }
    
    Ok(updates)
}

/// Legacy function for backward compatibility
pub fn get_unpushed_commits() -> Result<Vec<CommitInfo>, git2::Error> {
    let updates = get_commits_needing_processing()?;
    let mut commits = Vec::new();
    
    for update in updates {
        if let CommitUpdateType::NewCommit(commit_info) = update {
            commits.push(commit_info);
        }
    }
    
    Ok(commits)
}

/// Information about a commit that could become a PR
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: Oid,
    pub message: String,
    pub potential_branch_name: String,
}

/// Create a transient PR branch for a specific commit
pub fn create_pr_branch(commit_info: &CommitInfo) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    
    // Get the commit object
    let commit = repo.find_commit(commit_info.id)?;
    
    // Try to create the branch at this commit
    let branch_created = match repo.branch(&commit_info.potential_branch_name, &commit, false) {
        Ok(_) => {
            println!("Created branch: {}", commit_info.potential_branch_name);
            true
        }
        Err(e) if e.code() == git2::ErrorCode::Exists => {
            println!("Branch already exists: {}", commit_info.potential_branch_name);
            true // Branch exists, that's still success for our purposes
        }
        Err(e) => return Err(e), // Real error, propagate it
    };
    
    if branch_created {
        // Store metadata for this commit (only if we don't already have it)
        if !metadata::has_pr_metadata(&commit_info.id) {
            let commit_metadata = metadata::CommitMetadata::new_branch_created(
                commit_info.potential_branch_name.clone(),
                commit_info.id.to_string()
            );
            
            metadata::store_commit_metadata(&commit_info.id, &commit_metadata)
                .map_err(|e| git2::Error::from_str(&format!("Failed to store metadata: {}", e)))?;
        }
    }
    
    Ok(())
}

/// Create an incremental commit on an existing PR branch
pub fn create_incremental_commit(
    original_commit_oid: &Oid,
    updated_commit_oid: &Oid,
    pr_metadata: &metadata::CommitMetadata,
) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    
    // Get the PR branch
    let pr_branch = repo.find_branch(&pr_metadata.pr_branch_name, BranchType::Local)?;
    let pr_branch_commit = pr_branch.get().peel_to_commit()?;
    
    // Get the updated commit
    let updated_commit = repo.find_commit(*updated_commit_oid)?;
    
    // Create a new commit on the PR branch that represents the incremental change
    let signature = repo.signature()?;
    
    // Create commit message for the incremental update
    let incremental_message = format!(
        "Incremental update to: {}\n\nUpdated from commit {}",
        updated_commit.message().unwrap_or("").lines().next().unwrap_or(""),
        &original_commit_oid.to_string()[..8]
    );
    
    // Create the incremental commit on the PR branch
    let tree = updated_commit.tree()?;
    repo.commit(
        Some(&format!("refs/heads/{}", pr_metadata.pr_branch_name)),
        &signature,
        &signature,
        &incremental_message,
        &tree,
        &[&pr_branch_commit],
    )?;
    
    println!("Added incremental commit to: {}", pr_metadata.pr_branch_name);
    
    // Update metadata to track this incremental commit
    let updated_metadata = pr_metadata.clone().add_incremental_commit(
        updated_commit_oid.to_string(),
        updated_commit.message().unwrap_or("").to_string(),
        metadata::IncrementalCommitType::AmendedCommit,
    );
    
    metadata::update_commit_metadata(original_commit_oid, &updated_metadata)
        .map_err(|e| git2::Error::from_str(&format!("Failed to update metadata: {}", e)))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use git2::{Repository, Signature};

    fn create_test_repo() -> Result<(Repository, tempfile::TempDir), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let repo = Repository::init(&temp_dir)?;
        
        // Configure user for commits
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;
        
        // Create initial commit
        let signature = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            // Create a test file
            let test_file_path = temp_dir.path().join("test.txt");
            fs::write(&test_file_path, "test content")?;
            index.add_path(Path::new("test.txt"))?;
            index.write()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;
        drop(tree); // Explicitly drop the tree to release the borrow
        
        Ok((repo, temp_dir))
    }

    #[test]
    fn test_get_all_branches_with_single_branch() {
        let (_repo, _temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to the test repo directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(_temp_dir.path()).unwrap();
        
        let branches = get_all_branches().expect("Failed to get branches");
        
        // Should have at least the main/master branch
        assert!(!branches.is_empty());
        assert!(branches.contains(&"main".to_string()) || branches.contains(&"master".to_string()));
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_get_all_branches_basic() {
        let (_repo, temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to the test repo directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let branches = get_all_branches().expect("Failed to get branches");
        
        // Should have at least one branch (master/main)
        assert!(!branches.is_empty());
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_switch_to_nonexistent_branch() {
        let (_repo, _temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to the test repo directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(_temp_dir.path()).unwrap();
        
        // Try to switch to a branch that doesn't exist
        let result = switch_branch("nonexistent-branch");
        assert!(result.is_err());
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}