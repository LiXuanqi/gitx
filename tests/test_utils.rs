use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command as StdCommand;

/// A test repository wrapper that provides convenient methods for testing gitx functionality
/// 
/// # Builder-Style API Examples
/// 
/// ```rust
/// // For non-git scenarios (rare)
/// let repo = TestRepo::empty();
/// 
/// // Most common: basic git repository
/// let repo = TestRepo::with_git();
/// 
/// // Git repository with gitx configuration
/// let repo = TestRepo::with_gitx();
/// 
/// // Fully configured with sample commits (great for testing)
/// let repo = TestRepo::with_commits();
/// ```
pub struct TestRepo {
    pub temp_dir: assert_fs::TempDir,
}

impl TestRepo {
    /// Create an empty temporary directory (not a git repository)
    /// Use this when you need to test non-git scenarios
    pub fn empty() -> Self {
        Self {
            temp_dir: assert_fs::TempDir::new().unwrap(),
        }
    }

    /// Create a git repository with basic configuration
    pub fn with_git() -> Self {
        let repo = Self::empty();
        repo.init_git_internal();
        repo
    }

    /// Create a git repository with gitx configuration
    pub fn with_gitx() -> Self {
        let repo = Self::with_git();
        repo.setup_gitx_config();
        repo
    }

    /// Create a git repository with gitx configuration and sample commits
    pub fn with_commits() -> Self {
        let repo = Self::with_gitx();
        repo.add_sample_commits();
        repo
    }

    /// Initialize this directory as a git repository (internal method)
    fn init_git_internal(&self) {
        let output = StdCommand::new("git")
            .args(&["init"])
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to initialize git repo");
        
        assert!(output.status.success(), "Git init failed: {}", String::from_utf8_lossy(&output.stderr));
        
        // Configure basic git settings for the test repo
        self.set_git_config("user.name", "TestUser")
            .expect("Failed to set git user.name");
        self.set_git_config("user.email", "test@example.com")
            .expect("Failed to set git user.email");
    }

    /// Add sample commits to the repository (internal method)
    fn add_sample_commits(&self) {
        self.add_and_commit("initial.txt", "initial content", "Initial commit")
            .add_and_commit("feature.txt", "feature content", "Add feature")
            .add_and_commit("bugfix.txt", "bugfix content", "Fix bug");
    }

    /// Add a file with content to the repository
    pub fn add_file(&self, filename: &str, content: &str) -> &Self {
        self.temp_dir.child(filename).write_str(content).unwrap();
        self
    }

    /// Stage files for commit
    pub fn git_add(&self, files: &[&str]) -> &Self {
        let mut args = vec!["add"];
        args.extend(files);
        
        let output = StdCommand::new("git")
            .args(&args)
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to git add");
        
        assert!(output.status.success(), "Git add failed: {}", String::from_utf8_lossy(&output.stderr));
        self
    }

    /// Create a commit with the given message
    pub fn git_commit(&self, message: &str) -> &Self {
        let output = StdCommand::new("git")
            .args(&["commit", "-m", message])
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to git commit");
        
        assert!(output.status.success(), "Git commit failed: {}", String::from_utf8_lossy(&output.stderr));
        self
    }

    /// Add a file and commit it in one step
    pub fn add_and_commit(&self, filename: &str, content: &str, commit_message: &str) -> &Self {
        self.add_file(filename, content)
            .git_add(&[filename])
            .git_commit(commit_message)
    }

    /// Set a git config value
    pub fn set_git_config(&self, key: &str, value: &str) -> Result<&Self, String> {
        let output = StdCommand::new("git")
            .args(&["config", key, value])
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to run git config");
        
        if output.status.success() {
            Ok(self)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Get a git config value
    pub fn get_git_config(&self, key: &str) -> Option<String> {
        let output = StdCommand::new("git")
            .args(&["config", key])
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to get git config");
        
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        } else {
            None
        }
    }

    /// Set up complete gitx configuration
    pub fn setup_gitx_config(&self) -> &Self {
        let configs = [
            ("gitx.github.token", "ghp_test_token_123"),
            ("gitx.github.enabled", "true"),
            ("gitx.github.baseBranch", "main"),
            ("gitx.branch.autoCleanup", "true"),
        ];
        
        for (key, value) in &configs {
            self.set_git_config(key, value)
                .expect(&format!("Failed to set config {}", key));
        }
        
        self
    }

    /// Check if gitx is properly configured
    pub fn is_gitx_configured(&self) -> bool {
        let required_configs = [
            "gitx.github.token",
            "gitx.github.enabled",
            "gitx.github.baseBranch",
            "gitx.branch.autoCleanup",
        ];
        
        required_configs.iter().all(|key| self.get_git_config(key).is_some())
    }

    /// Get all git config as a string (useful for debugging)
    pub fn get_all_git_config(&self) -> String {
        let output = StdCommand::new("git")
            .args(&["config", "--list"])
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to list git config");
        
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Get the path to the temporary directory
    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Assert that a file exists
    pub fn assert_file_exists(&self, filename: &str) -> &Self {
        self.temp_dir.child(filename).assert(predicate::path::exists());
        self
    }

    /// Assert that a file has specific content
    pub fn assert_file_content(&self, filename: &str, expected_content: &str) -> &Self {
        self.temp_dir.child(filename).assert(expected_content);
        self
    }

    /// Assert that the git repository structure exists
    pub fn assert_git_repo(&self) -> &Self {
        self.temp_dir.child(".git").assert(predicate::path::is_dir());
        self.temp_dir.child(".git/config").assert(predicate::path::is_file());
        self.temp_dir.child(".git/HEAD").assert(predicate::path::is_file());
        self
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty_directory() {
        let repo = TestRepo::empty();
        
        // Should be a temp directory but not a git repo
        assert!(repo.temp_dir.path().exists());
        assert!(!repo.temp_dir.child(".git").path().exists());
    }
    
    #[test]
    fn test_with_git() {
        let repo = TestRepo::with_git();
        
        // Test basic git functionality
        repo.assert_git_repo();
        assert_eq!(repo.get_git_config("user.name"), Some("Test User".to_string()));
        assert_eq!(repo.get_git_config("user.email"), Some("test@example.com".to_string()));
    }
    
    #[test]
    fn test_repo_file_operations() {
        let repo = TestRepo::with_git();
        
        repo.add_file("test.txt", "test content")
            .assert_file_exists("test.txt")
            .assert_file_content("test.txt", "test content");
    }
    
    #[test]
    fn test_repo_commit_workflow() {
        let repo = TestRepo::with_git();
        
        repo.add_and_commit("README.md", "# Test Project", "Initial commit");
        
        // Verify the commit was created (basic check)
        repo.assert_file_exists("README.md")
            .assert_file_content("README.md", "# Test Project");
    }
    
    #[test]
    fn test_with_gitx() {
        let repo = TestRepo::with_gitx();
        
        assert!(repo.is_gitx_configured());
        repo.assert_git_repo();
        
        // Test specific config values
        assert_eq!(repo.get_git_config("gitx.github.enabled"), Some("true".to_string()));
        assert_eq!(repo.get_git_config("gitx.github.baseBranch"), Some("main".to_string()));
    }
    
    #[test]
    fn test_with_commits() {
        let repo = TestRepo::with_commits();
        
        assert!(repo.is_gitx_configured());
        repo.assert_git_repo()
            .assert_file_exists("initial.txt")
            .assert_file_exists("feature.txt")
            .assert_file_exists("bugfix.txt");
    }
    
    #[test]
    fn test_builder_style_workflow() {
        let repo = TestRepo::with_git();
        
        repo.add_and_commit("feature.txt", "awesome feature", "Add awesome feature");
        
        repo.assert_file_exists("feature.txt")
            .assert_file_content("feature.txt", "awesome feature");
    }
}