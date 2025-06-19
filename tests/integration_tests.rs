use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use git2::{Repository, Signature};

/// Helper struct for setting up test repositories
pub struct TestRepo {
    pub temp_dir: TempDir,
    pub repo: Repository,
}

impl TestRepo {
    /// Create a new test repository with initial commit
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
            let test_file_path = temp_dir.path().join("README.md");
            fs::write(&test_file_path, "# Test Repository\n")?;
            index.add_path(Path::new("README.md"))?;
            index.write()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        let _commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;
        drop(tree); // Explicitly drop the tree to release the borrow
        
        Ok(TestRepo { temp_dir, repo })
    }
    
    /// Add a new commit to the repository
    pub fn add_commit(&self, message: &str, content: &str) -> Result<git2::Oid, Box<dyn std::error::Error>> {
        let signature = self.repo.signature()?;
        
        // Create/modify a test file
        let test_file_path = self.temp_dir.path().join("features.txt");
        let existing_content = fs::read_to_string(&test_file_path).unwrap_or_default();
        fs::write(&test_file_path, format!("{}{}\n", existing_content, content))?;
        
        // Stage the file
        let mut index = self.repo.index()?;
        index.add_path(Path::new("features.txt"))?;
        index.write()?;
        
        // Create commit
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let parent = self.repo.head()?.peel_to_commit()?;
        
        let commit_id = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent],
        )?;
        
        Ok(commit_id)
    }
    
    /// Get the path to the repository
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

/// Run a gitx command in the test repository
fn run_gitx_command(repo_path: &Path, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    let gitx_path = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("debug")
        .join("gitx");
    
    Command::new(gitx_path)
        .args(args)
        .current_dir(repo_path)
        .output()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_repo_setup() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Verify repository was created
        assert!(test_repo.repo.head().is_ok());
        
        // Verify initial commit exists
        let head = test_repo.repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Initial commit");
    }
    
    #[test]
    fn test_add_commit() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Add a new commit
        let commit_id = test_repo
            .add_commit("Add feature 1", "feature 1 content")
            .expect("Failed to add commit");
        
        // Verify commit was created
        let commit = test_repo.repo.find_commit(commit_id).unwrap();
        assert_eq!(commit.message().unwrap(), "Add feature 1");
        
        // Verify file content
        let file_path = test_repo.path().join("features.txt");
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("feature 1 content"));
    }
    
    #[test]
    fn test_gitx_status_empty_repo() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Run gitx prs command
        let output = run_gitx_command(test_repo.path(), &["prs"])
            .expect("Failed to run gitx prs");
        
        // Should succeed and show no PRs
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("No stacked PRs found"));
    }
    
    #[test]
    fn test_gitx_diff_local_only() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Add a commit
        test_repo
            .add_commit("Add authentication system", "auth code")
            .expect("Failed to add commit");
        
        // Run gitx diff (local only, no --github flag)
        let output = run_gitx_command(test_repo.path(), &["diff"])
            .expect("Failed to run gitx diff");
        
        // Should succeed
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Creating PR branch for"));
        assert!(stdout.contains("1 new branches"));
    }
    
    #[test] 
    fn test_gitx_diff_latest_vs_all() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Add multiple commits
        test_repo
            .add_commit("Add authentication system", "auth code")
            .expect("Failed to add commit");
        test_repo
            .add_commit("Add validation logic", "validation code")  
            .expect("Failed to add commit");
        test_repo
            .add_commit("Add password reset", "password reset code")
            .expect("Failed to add commit");
        
        // Test latest only (default behavior)
        let output = run_gitx_command(test_repo.path(), &["diff"])
            .expect("Failed to run gitx diff");
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should only process the latest commit
        assert!(stdout.contains("1 new branches"));
        
        // Test all commits
        let output = run_gitx_command(test_repo.path(), &["diff", "--all"])
            .expect("Failed to run gitx diff --all");
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should process multiple commits
        assert!(stdout.contains("new branches"));
    }
    
    #[test]
    fn test_gitx_prs_after_diff() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Add a commit and create PR branch
        test_repo
            .add_commit("Add feature X", "feature X code")
            .expect("Failed to add commit");
        
        let _output = run_gitx_command(test_repo.path(), &["diff"])
            .expect("Failed to run gitx diff");
        
        // Now check PR status
        let output = run_gitx_command(test_repo.path(), &["prs"])
            .expect("Failed to run gitx prs");
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Stacked PR Status"));
        assert!(stdout.contains("Add feature X"));
        assert!(stdout.contains("Branch:"));
    }
    
    #[test]
    fn test_gitx_commit_passthrough() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Create a file to commit
        let test_file = test_repo.path().join("new_file.txt");
        fs::write(&test_file, "test content").expect("Failed to write test file");
        
        // Stage the file
        let output = Command::new("git")
            .args(&["add", "new_file.txt"])
            .current_dir(test_repo.path())
            .output()
            .expect("Failed to run git add");
        assert!(output.status.success());
        
        // Use gitx commit
        let output = run_gitx_command(test_repo.path(), &["commit", "-m", "Add new file"])
            .expect("Failed to run gitx commit");
        
        assert!(output.status.success());
        
        // Verify commit was created
        let head = test_repo.repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Add new file");
    }
    
    #[test]
    fn test_gitx_status_passthrough() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Create an unstaged file
        let test_file = test_repo.path().join("unstaged.txt");
        fs::write(&test_file, "unstaged content").expect("Failed to write test file");
        
        // Use gitx status
        let output = run_gitx_command(test_repo.path(), &["status"])
            .expect("Failed to run gitx status");
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("unstaged.txt"));
    }
    
    #[test]
    fn test_branch_naming_integration() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Add commit with special characters in title
        test_repo
            .add_commit("Add OAuth2.0 authentication & validation!", "oauth code")
            .expect("Failed to add commit");
        
        let _output = run_gitx_command(test_repo.path(), &["diff"])
            .expect("Failed to run gitx diff");
        
        // Check that branch was created with sanitized name
        let output = run_gitx_command(test_repo.path(), &["prs"])
            .expect("Failed to run gitx prs");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should contain sanitized branch name
        assert!(stdout.contains("gitx/"));
        assert!(!stdout.contains("!"));
        assert!(!stdout.contains("&"));
    }
    
    #[test]
    fn test_metadata_persistence() {
        let test_repo = TestRepo::new().expect("Failed to create test repo");
        
        // Add commit and create PR branch
        test_repo
            .add_commit("Add caching layer", "cache code")
            .expect("Failed to add commit");
        
        let _output = run_gitx_command(test_repo.path(), &["diff"])
            .expect("Failed to run gitx diff");
        
        // Check PR status
        let output1 = run_gitx_command(test_repo.path(), &["prs"])
            .expect("Failed to run gitx prs");
        let stdout1 = String::from_utf8_lossy(&output1.stdout);
        
        // Run status again to verify metadata persisted
        let output2 = run_gitx_command(test_repo.path(), &["prs"])
            .expect("Failed to run gitx prs again");
        let stdout2 = String::from_utf8_lossy(&output2.stdout);
        
        // Should show same PR information
        assert_eq!(stdout1, stdout2);
        assert!(stdout2.contains("Add caching layer"));
    }
}