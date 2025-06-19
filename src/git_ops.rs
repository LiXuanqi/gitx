use git2::{Repository, BranchType};

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