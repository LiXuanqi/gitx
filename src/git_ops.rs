use git2::{Repository, BranchType, Oid};
use crate::branch_naming;

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

/// Get commits on main branch that don't have corresponding PR branches
pub fn get_unpushed_commits() -> Result<Vec<CommitInfo>, git2::Error> {
    let repo = Repository::open(".")?;
    let mut commits = Vec::new();
    
    // Get main branch head
    let main_ref = repo.find_reference("refs/heads/main")
        .or_else(|_| repo.find_reference("refs/heads/master"))?;
    let main_commit = main_ref.peel_to_commit()?;
    
    // Walk commits from HEAD
    let mut revwalk = repo.revwalk()?;
    revwalk.push(main_commit.id())?;
    
    // Get existing transient branches to avoid duplicates
    let existing_branches = get_all_branches()?;
    let transient_branches: Vec<String> = existing_branches
        .into_iter()
        .filter(|b| branch_naming::is_transient_pr_branch(b))
        .collect();
    
    let username = get_git_username().unwrap_or_else(|_| "unknown".to_string());
    
    for oid in revwalk.take(10) { // Limit to last 10 commits for now
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("").to_string();
        
        // Generate what the branch name would be
        let potential_branch = branch_naming::generate_branch_name(&username, &message);
        
        // Skip if we already have a branch for this commit
        if !transient_branches.contains(&potential_branch) {
            commits.push(CommitInfo {
                id: oid,
                message: message.clone(),
                potential_branch_name: potential_branch,
            });
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
    
    // Create the branch at this commit
    let _branch = repo.branch(&commit_info.potential_branch_name, &commit, false)?;
    
    println!("Created branch: {}", commit_info.potential_branch_name);
    
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