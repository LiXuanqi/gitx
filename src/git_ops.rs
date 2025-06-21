use git2::{Repository, BranchType, Oid};
use crate::branch_naming;
use crate::metadata;
use crate::github::{self, GitHubClientTrait};
use crate::github_utils::generate_pr_body;
use crate::git_utils::GitUtils;

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

/// Determine the appropriate base branch for a commit by looking at its parent
pub fn determine_base_branch_for_commit(commit_oid: &Oid) -> Result<String, git2::Error> {
    let repo = Repository::open(".")?;
    let commit = repo.find_commit(*commit_oid)?;
    
    // If the commit has parents, look at the first parent
    if commit.parent_count() > 0 {
        let parent_commit = commit.parent(0)?;
        let parent_oid = parent_commit.id();
        
        // Check if the parent commit has metadata with a PR branch
        if let Ok(Some(parent_metadata)) = metadata::get_commit_metadata(&parent_oid)
            .map_err(|e| git2::Error::from_str(&e.to_string())) {
            if let Some(_pr_number) = parent_metadata.github_pr_number {
                // If parent has a PR, use its branch name as base
                return Ok(parent_metadata.pr_branch_name);
            }
        }
    }
    
    // Default fallback: use main or master
    let main_ref = repo.find_reference("refs/heads/main")
        .or_else(|_| repo.find_reference("refs/heads/master"));
    
    match main_ref {
        Ok(ref_) => {
            if let Some(name) = ref_.shorthand() {
                Ok(name.to_string())
            } else {
                Ok("main".to_string())
            }
        }
        Err(_) => Ok("main".to_string())
    }
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
    get_commits_needing_processing_impl(false)
}

/// Get only the latest commit that needs processing
pub fn get_latest_commit_needing_processing() -> Result<Vec<CommitUpdateType>, git2::Error> {
    get_commits_needing_processing_impl(true)
}

/// Internal implementation for getting commits needing processing
fn get_commits_needing_processing_impl(latest_only: bool) -> Result<Vec<CommitUpdateType>, git2::Error> {
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
    
    let commit_limit = if latest_only { 1 } else { 10 }; // Only 1 commit if latest_only
    
    for oid in revwalk.take(commit_limit) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("").to_string();
        
        // Check if this position in history has existing metadata stored elsewhere
        // (This handles the case where commits are amended/rebased)
        let current_commit_id = oid.to_string();
        let mut found_metadata_for_position = false;
        
        // Check if we have metadata for this commit
        if let Some(existing_metadata) = metadata::get_commit_metadata(&oid).map_err(|e| git2::Error::from_str(&e.to_string()))? {
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
#[allow(dead_code)]
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

/// Create a PR branch with dependency injection for GitHub client
pub async fn create_pr_branch_with_github_client(
    commit_info: &CommitInfo,
    enable_github: bool,
    github_client: Option<&dyn GitHubClientTrait>,
) -> Result<Option<github::PRInfo>, Box<dyn std::error::Error>> {
    if !enable_github {
        // Local-only mode: create persistent local branch
        create_pr_branch(commit_info).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        return Ok(None);
    }
    
    // GitHub mode: create transient branch, push, create PR, then delete local branch
    if let Some(client) = github_client {
        create_transient_pr_branch_with_github_client(commit_info, client).await
    } else {
        // Create a real GitHub client for production use
        let github_client = github::GitHubClient::new().await?;
        create_transient_pr_branch_with_github_client(commit_info, &github_client).await
    }
}

/// Create a PR branch and optionally create GitHub PR (legacy wrapper)
pub async fn create_pr_branch_with_github(
    commit_info: &CommitInfo,
    enable_github: bool,
) -> Result<Option<github::PRInfo>, Box<dyn std::error::Error>> {
    create_pr_branch_with_github_client(commit_info, enable_github, None).await
}


/// Create a transient PR branch with dependency injection for GitHub client
pub async fn create_transient_pr_branch_with_github_client(
    commit_info: &CommitInfo,
    github_client: &dyn GitHubClientTrait,
) -> Result<Option<github::PRInfo>, Box<dyn std::error::Error>> {
    let repo = Repository::open(".").map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    // 1. Create temporary local branch
    let commit = repo.find_commit(commit_info.id).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let mut temp_branch = repo.branch(&commit_info.potential_branch_name, &commit, false)
        .map_err(|e| e)?;
    
    // 2. Push branch to remote
    GitUtils::push_branch(&commit_info.potential_branch_name).await?;
    
    // 3. Create metadata (before deleting local branch)
    let commit_message = commit.message().unwrap_or("");
    let commit_metadata = metadata::CommitMetadata::new_branch_created(
        commit_info.potential_branch_name.clone(),
        commit_info.id.to_string()
    );
    metadata::store_commit_metadata(&commit_info.id, &commit_metadata)
        .map_err(|e| e)?;
    
    // 4. Create the PR
    let pr_title = commit_message.lines().next().unwrap_or("Untitled commit").to_string();
    let pr_body = generate_pr_body(&commit_metadata, commit_message);
    
    // Determine the appropriate base branch for this commit
    let base_branch = determine_base_branch_for_commit(&commit_info.id)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    let pr_info = github_client.create_pr(
        &commit_info.potential_branch_name,
        &pr_title,
        &pr_body,
        &base_branch,
    ).await?;
    
    // 5. Update metadata with PR number
    let updated_metadata = commit_metadata.with_pr_number(pr_info.number);
    metadata::update_commit_metadata(&commit_info.id, &updated_metadata)
        .map_err(|e| e)?;
    
    // 6. Delete the local branch (keep only on GitHub)
    temp_branch.delete().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    println!("Created GitHub PR #{}: {} (transient branch deleted locally)", pr_info.number, pr_info.url);
    
    Ok(Some(pr_info))
}

/// Create incremental commit with dependency injection for GitHub client
pub async fn create_incremental_commit_with_github_client(
    original_commit_oid: &Oid,
    updated_commit_oid: &Oid,
    pr_metadata: &metadata::CommitMetadata,
    enable_github: bool,
    github_client: Option<&dyn GitHubClientTrait>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !enable_github {
        // Local-only mode: create persistent local incremental commit
        create_incremental_commit(original_commit_oid, updated_commit_oid, pr_metadata)
            .map_err(|e| e)?;
        return Ok(());
    }
    
    // GitHub mode: create transient incremental commit
    if let Some(client) = github_client {
        create_transient_incremental_commit_with_github_client(original_commit_oid, updated_commit_oid, pr_metadata, client).await
    } else {
        // Create a real GitHub client for production use
        let github_client = github::GitHubClient::new().await?;
        create_transient_incremental_commit_with_github_client(original_commit_oid, updated_commit_oid, pr_metadata, &github_client).await
    }
}

/// Create incremental commit and optionally update GitHub PR (legacy wrapper)
pub async fn create_incremental_commit_with_github(
    original_commit_oid: &Oid,
    updated_commit_oid: &Oid,
    pr_metadata: &metadata::CommitMetadata,
    enable_github: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    create_incremental_commit_with_github_client(original_commit_oid, updated_commit_oid, pr_metadata, enable_github, None).await
}

/// Create a transient incremental commit with dependency injection for GitHub client  
pub async fn create_transient_incremental_commit_with_github_client(
    original_commit_oid: &Oid,
    updated_commit_oid: &Oid,
    pr_metadata: &metadata::CommitMetadata,
    github_client: &dyn GitHubClientTrait,
) -> Result<(), Box<dyn std::error::Error>> {
    if pr_metadata.github_pr_number.is_none() {
        println!("Warning: No GitHub PR number found, skipping PR update");
        return Ok(());
    }
    
    let repo = Repository::open(".").map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    // 1. Create temporary local branch with incremental commit
    let updated_commit = repo.find_commit(*updated_commit_oid).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let mut temp_branch = repo.branch(&pr_metadata.pr_branch_name, &updated_commit, false)
        .map_err(|e| e)?;
    
    // 2. Create incremental commit on the temp branch
    let signature = repo.signature().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let incremental_message = format!(
        "Incremental update to: {}\n\nUpdated from commit {}",
        updated_commit.message().unwrap_or("").lines().next().unwrap_or(""),
        &original_commit_oid.to_string()[..8]
    );
    
    let tree = updated_commit.tree().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    repo.commit(
        Some(&format!("refs/heads/{}", pr_metadata.pr_branch_name)),
        &signature,
        &signature,
        &incremental_message,
        &tree,
        &[&updated_commit],
    ).map_err(|e| e)?;
    
    // 3. Push the updated branch to remote
    GitUtils::push_branch(&pr_metadata.pr_branch_name).await?;
    
    // 4. Update metadata to track this incremental commit
    let updated_metadata = pr_metadata.clone().add_incremental_commit(
        updated_commit_oid.to_string(),
        updated_commit.message().unwrap_or("").to_string(),
        metadata::IncrementalCommitType::AmendedCommit,
    );
    metadata::update_commit_metadata(original_commit_oid, &updated_metadata)
        .map_err(|e| e)?;
    
    // 5. Update the GitHub PR
    let commit_message = updated_commit.message().unwrap_or("");
    let pr_body = generate_pr_body(&updated_metadata, commit_message);
    let pr_number = pr_metadata.github_pr_number.unwrap();
    github_client.update_pr(pr_number, None, Some(&pr_body)).await?;
    
    // 6. Delete the local branch (keep only on GitHub)
    temp_branch.delete().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    println!("Updated GitHub PR #{} (transient branch deleted locally)", pr_number);
    
    Ok(())
}

/// Create a transient incremental commit that only exists on GitHub (legacy wrapper)
pub async fn create_transient_incremental_commit_with_github(
    original_commit_oid: &Oid,
    updated_commit_oid: &Oid,
    pr_metadata: &metadata::CommitMetadata,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if GitHub token is available
    if !github::check_github_token() {
        println!("Warning: GITHUB_TOKEN not set, skipping GitHub PR update");
        return Ok(());
    }
    
    // Create a real GitHub client for production use
    let github_client = github::GitHubClient::new().await?;
    create_transient_incremental_commit_with_github_client(original_commit_oid, updated_commit_oid, pr_metadata, &github_client).await
}

/// Land (cleanup) merged PRs by detecting merged status from GitHub and cleaning up local branches
pub async fn land_merged_prs(all: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if GitHub token is available
    if !github::check_github_token() {
        return Err("GITHUB_TOKEN environment variable not set. Required to check PR merge status.".into());
    }
    
    // Get all PR metadata
    let pr_statuses = metadata::get_all_pr_status()
        .map_err(|e| e)?;
    
    if pr_statuses.is_empty() {
        println!("No stacked PRs found.");
        return Ok(());
    }
    
    println!("🔍 Checking PR statuses...");
    
    // Get GitHub client
    let github_client = github::GitHubClient::new().await?;
    
    // Find PRs that have GitHub PR numbers
    let prs_to_check: Vec<_> = pr_statuses.iter()
        .filter_map(|pr| pr.pr_number.map(|num| (num, pr)))
        .collect();
    
    if prs_to_check.is_empty() {
        println!("No PRs with GitHub PR numbers found.");
        return Ok(());
    }
    
    let pr_numbers: Vec<u64> = prs_to_check.iter().map(|(num, _)| *num).collect();
    let github_statuses = github_client.get_multiple_pr_statuses(&pr_numbers).await?;
    
    // Find merged PRs
    let mut merged_prs = Vec::new();
    for status in &github_statuses {
        if status.state == "merged" {
            if let Some((_, pr_info)) = prs_to_check.iter().find(|(num, _)| *num == status.number) {
                merged_prs.push((status, *pr_info));
            }
        }
    }
    
    if merged_prs.is_empty() {
        println!("No merged PRs found ready for cleanup.");
        if !all {
            println!("Use --all to force cleanup of all tracked PRs (requires confirmation).");
        }
        return Ok(());
    }
    
    println!("Found {} merged PR{} ready for cleanup:", 
        merged_prs.len(), 
        if merged_prs.len() == 1 { "" } else { "s" }
    );
    
    for (github_status, _pr_info) in &merged_prs {
        println!("  ✅ PR #{}: {} (merged)", github_status.number, github_status.title);
    }
    
    if dry_run {
        println!("\n🧪 DRY RUN - would perform these actions:");
        println!("🧹 Cleaning up merged PRs:");
        
        for (_, pr_info) in &merged_prs {
            println!("  🗑️  Would delete remote branch: {}", pr_info.branch_name);
            println!("  📝 Would update metadata: mark PR as merged");
        }
        
        println!("  🔄 Would sync with origin/main");
        println!("\nTo actually perform cleanup, run without --dry-run");
        return Ok(());
    }
    
    // Perform actual cleanup
    println!("\n🧹 Cleaning up merged PRs:");
    let mut cleaned_up = 0;
    
    for (github_status, pr_info) in &merged_prs {
        match cleanup_merged_pr(pr_info, github_status.number).await {
            Ok(()) => {
                println!("  🗑️  Deleted remote branch: {}", pr_info.branch_name);
                println!("  📝 Updated metadata: marked PR #{} as merged", github_status.number);
                cleaned_up += 1;
            }
            Err(e) => {
                eprintln!("  ❌ Failed to cleanup PR #{}: {}", github_status.number, e);
            }
        }
    }
    
    // Sync with origin/main
    if cleaned_up > 0 {
        match sync_with_origin_main().await {
            Ok(()) => {
                println!("  🔄 Synced with origin/main");
            }
            Err(e) => {
                eprintln!("  ⚠️  Warning: Failed to sync with origin/main: {}", e);
            }
        }
    }
    
    println!("\n✨ Cleanup complete! {} PR{} cleaned up.", 
        cleaned_up, 
        if cleaned_up == 1 { "" } else { "s" }
    );
    
    Ok(())
}

/// Clean up a single merged PR: delete remote branch and update metadata
async fn cleanup_merged_pr(
    pr_info: &metadata::PRStatusInfo, 
    _pr_number: u64
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")
        .map_err(|e| e)?;
    
    // Delete the local branch if it exists (for backward compatibility with old workflow)
    match repo.find_branch(&pr_info.branch_name, BranchType::Local) {
        Ok(mut branch) => {
            branch.delete()?;
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => {
            // Branch doesn't exist locally, that's expected with transient branches
        }
        Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error>),
    }
    
    // Delete the remote branch on GitHub
    match delete_remote_branch(&pr_info.branch_name).await {
        Ok(()) => {
            // Remote branch deleted successfully
        }
        Err(e) => {
            eprintln!("Warning: Failed to delete remote branch {}: {}", pr_info.branch_name, e);
            // Continue with metadata cleanup even if remote deletion fails
        }
    }
    
    // Update metadata to mark as merged
    let commit_oid = Oid::from_str(&pr_info.commit_id)?;
    if let Some(mut metadata) = metadata::get_commit_metadata(&commit_oid)
        .map_err(|e| e)? 
    {
        metadata.status = metadata::PRStatus::PRMerged;
        
        metadata::update_commit_metadata(&commit_oid, &metadata)
            .map_err(|e| e)?;
    }
    
    Ok(())
}

/// Delete a remote branch from GitHub
async fn delete_remote_branch(branch_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Use git command to delete the remote branch
    let output = tokio::process::Command::new("git")
        .args(&["push", "origin", "--delete", branch_name])
        .output()
        .await?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to delete remote branch: {}", error).into());
    }
    
    Ok(())
}

/// Sync local main branch with origin/main
async fn sync_with_origin_main() -> Result<(), Box<dyn std::error::Error>> {
    // Use git command to pull latest changes
    let output = tokio::process::Command::new("git")
        .args(&["pull", "origin", "main"])
        .output()
        .await?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to sync with origin/main: {}", error).into());
    }
    
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

    #[test]
    fn test_get_commits_needing_processing_latest_only() {
        let (repo, temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to the test repo directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Add multiple commits
        let signature = repo.signature().unwrap();
        for i in 1..=3 {
            let content = format!("Feature {}", i);
            let test_file_path = temp_dir.path().join("features.txt");
            let existing = std::fs::read_to_string(&test_file_path).unwrap_or_default();
            std::fs::write(&test_file_path, format!("{}{}\n", existing, content)).unwrap();
            
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("features.txt")).unwrap();
            index.write().unwrap();
            
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("Add feature {}", i),
                &tree,
                &[&parent],
            ).unwrap();
        }
        
        // Test latest only
        let updates = get_latest_commit_needing_processing().expect("Failed to get latest commits");
        assert_eq!(updates.len(), 1);
        
        // Test all commits
        let updates = get_commits_needing_processing().expect("Failed to get all commits");
        assert!(updates.len() >= 3);
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_commit_info_creation() {
        let commit_info = CommitInfo {
            id: git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap(),
            message: "Add user authentication".to_string(),
            potential_branch_name: "gitx/test/add-user-authentication".to_string(),
        };
        
        assert_eq!(commit_info.message, "Add user authentication");
        assert_eq!(commit_info.potential_branch_name, "gitx/test/add-user-authentication");
    }

    #[test]
    fn test_create_pr_branch() {
        let (repo, temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to the test repo directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Get the HEAD commit
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        
        let commit_info = CommitInfo {
            id: commit.id(),
            message: "Add new feature".to_string(),
            potential_branch_name: "gitx/test/add-new-feature".to_string(),
        };
        
        // Create PR branch
        create_pr_branch(&commit_info).expect("Failed to create PR branch");
        
        // Verify branch was created
        let branches = get_all_branches().expect("Failed to get branches");
        assert!(branches.contains(&"gitx/test/add-new-feature".to_string()));
        
        // Verify metadata was stored
        assert!(crate::metadata::has_pr_metadata(&commit.id()));
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_get_git_username() {
        let (_repo, temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to the test repo directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let username = get_git_username().expect("Failed to get git username");
        assert_eq!(username, "Test User");
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}