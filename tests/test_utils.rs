use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command as StdCommand;

/// A test repository wrapper that provides convenient methods for testing gitx functionality
pub struct TestRepo {
    pub temp_dir: assert_fs::TempDir,
}

impl TestRepo {
    /// Create a new empty temporary directory (not yet a git repo)
    pub fn new() -> Self {
        Self {
            temp_dir: assert_fs::TempDir::new().unwrap(),
        }
    }

    /// Initialize this directory as a git repository
    pub fn init_git(&self) -> &Self {
        let output = StdCommand::new("git")
            .args(&["init"])
            .current_dir(&self.temp_dir)
            .output()
            .expect("Failed to initialize git repo");
        
        assert!(output.status.success(), "Git init failed: {}", String::from_utf8_lossy(&output.stderr));
        
        // Configure basic git settings for the test repo
        self.set_git_config("user.name", "Test User")
            .expect("Failed to set git user.name");
        self.set_git_config("user.email", "test@example.com")
            .expect("Failed to set git user.email");
        
        self
    }

    /// Create a new git repository (combines new() + init_git())
    pub fn new_git() -> Self {
        let repo = Self::new();
        repo.init_git();
        repo
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

/// Builder pattern for creating test repositories with different configurations
pub struct TestRepoBuilder {
    repo: TestRepo,
}

impl TestRepoBuilder {
    pub fn new() -> Self {
        Self {
            repo: TestRepo::new(),
        }
    }

    pub fn with_git(self) -> Self {
        self.repo.init_git();
        self
    }

    pub fn with_gitx_config(self) -> Self {
        self.repo.setup_gitx_config();
        self
    }

    pub fn with_file(self, filename: &str, content: &str) -> Self {
        self.repo.add_file(filename, content);
        self
    }

    pub fn with_commit(self, filename: &str, content: &str, commit_message: &str) -> Self {
        self.repo.add_and_commit(filename, content, commit_message);
        self
    }

    pub fn with_multiple_commits(self) -> Self {
        self.repo
            .add_and_commit("initial.txt", "initial content", "Initial commit")
            .add_and_commit("feature.txt", "feature content", "Add feature")
            .add_and_commit("bugfix.txt", "bugfix content", "Fix bug");
        self
    }

    pub fn build(self) -> TestRepo {
        self.repo
    }
}

impl Default for TestRepoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience constructors
impl TestRepo {
    /// Create a fully configured test repository (git + gitx config)
    pub fn new_configured() -> Self {
        TestRepoBuilder::new()
            .with_git()
            .with_gitx_config()
            .build()
    }

    /// Create a test repository with git, gitx config, and sample commits
    pub fn new_with_commits() -> Self {
        TestRepoBuilder::new()
            .with_git()
            .with_gitx_config()
            .with_multiple_commits()
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_repo_basic_functionality() {
        let repo = TestRepo::new_git();
        
        // Test basic git functionality
        repo.assert_git_repo();
        assert_eq!(repo.get_git_config("user.name"), Some("Test User".to_string()));
        assert_eq!(repo.get_git_config("user.email"), Some("test@example.com".to_string()));
    }
    
    #[test]
    fn test_repo_file_operations() {
        let repo = TestRepo::new_git();
        
        repo.add_file("test.txt", "test content")
            .assert_file_exists("test.txt")
            .assert_file_content("test.txt", "test content");
    }
    
    #[test]
    fn test_repo_commit_workflow() {
        let repo = TestRepo::new_git();
        
        repo.add_and_commit("README.md", "# Test Project", "Initial commit");
        
        // Verify the commit was created (basic check)
        repo.assert_file_exists("README.md")
            .assert_file_content("README.md", "# Test Project");
    }
    
    #[test]
    fn test_gitx_configuration() {
        let repo = TestRepo::new_git();
        
        repo.setup_gitx_config();
        assert!(repo.is_gitx_configured());
        
        // Test specific config values
        assert_eq!(repo.get_git_config("gitx.github.enabled"), Some("true".to_string()));
        assert_eq!(repo.get_git_config("gitx.github.baseBranch"), Some("main".to_string()));
    }
    
    #[test]
    fn test_builder_pattern() {
        let repo = TestRepoBuilder::new()
            .with_git()
            .with_gitx_config()
            .with_commit("feature.txt", "awesome feature", "Add awesome feature")
            .build();
        
        assert!(repo.is_gitx_configured());
        repo.assert_file_exists("feature.txt")
            .assert_file_content("feature.txt", "awesome feature");
    }
    
    #[test]
    fn test_convenience_constructors() {
        let repo1 = TestRepo::new_configured();
        assert!(repo1.is_gitx_configured());
        
        let repo2 = TestRepo::new_with_commits();
        assert!(repo2.is_gitx_configured());
        repo2.assert_file_exists("initial.txt")
             .assert_file_exists("feature.txt")
             .assert_file_exists("bugfix.txt");
    }
}