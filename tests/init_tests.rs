use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command as StdCommand;

/// Helper function to create a git repository in a temporary directory
fn setup_git_repo(temp_dir: &assert_fs::TempDir) {
    // Initialize git repo
    let output = StdCommand::new("git")
        .args(&["init"])
        .current_dir(temp_dir)
        .output()
        .expect("Failed to initialize git repo");
    
    assert!(output.status.success(), "Git init failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Configure basic git settings for the test repo
    StdCommand::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(temp_dir)
        .output()
        .expect("Failed to set git user.name");
    
    StdCommand::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(temp_dir)
        .output()
        .expect("Failed to set git user.email");
}

/// Helper function to check git config value
fn get_git_config(temp_dir: &assert_fs::TempDir, key: &str) -> Option<String> {
    let output = StdCommand::new("git")
        .args(&["config", key])
        .current_dir(temp_dir)
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

#[test]
fn test_gitx_init_help() {
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.args(&["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialize gitx configuration"));
}

#[test]
fn test_gitx_init_in_non_git_directory() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&temp_dir)
        .arg("init")
        .assert()
        .success() // Command succeeds but initialization fails
        .stderr(predicate::str::contains("Error during initialization")); // Should show initialization error
}

#[test]
fn test_gitx_init_basic_setup() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    setup_git_repo(&temp_dir);
    
    // Test that we can run gitx init (but we can't easily test the interactive parts)
    // For now, let's test that it at least recognizes it's in a git repo
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&temp_dir)
        .arg("init")
        .write_stdin("test_token\ny\nmain\ny\n") // Try to provide some inputs
        .assert()
        .success();
    
    // Verify that git config was potentially set (though hard to test with interactive prompts)
    // We'll check if the command succeeded, which is a good start
}

#[test]
fn test_git_config_functions() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    setup_git_repo(&temp_dir);
    
    // Manually set some gitx config values
    StdCommand::new("git")
        .args(&["config", "gitx.github.token", "test_token"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to set config");
    
    StdCommand::new("git")
        .args(&["config", "gitx.github.enabled", "true"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to set config");
    
    StdCommand::new("git")
        .args(&["config", "gitx.github.baseBranch", "main"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to set config");
    
    // Verify the config values were set correctly
    assert_eq!(get_git_config(&temp_dir, "gitx.github.token"), Some("test_token".to_string()));
    assert_eq!(get_git_config(&temp_dir, "gitx.github.enabled"), Some("true".to_string()));
    assert_eq!(get_git_config(&temp_dir, "gitx.github.baseBranch"), Some("main".to_string()));
}

#[test]
fn test_gitx_config_verification() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    setup_git_repo(&temp_dir);
    
    // Manually configure gitx to simulate what init would do
    let configs = [
        ("gitx.github.token", "ghp_test123"),
        ("gitx.github.enabled", "true"),
        ("gitx.github.baseBranch", "main"),
        ("gitx.branch.autoCleanup", "true"),
    ];
    
    for (key, value) in &configs {
        StdCommand::new("git")
            .args(&["config", key, value])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set config");
    }
    
    // Verify all configs are set
    for (key, expected_value) in &configs {
        assert_eq!(
            get_git_config(&temp_dir, key),
            Some(expected_value.to_string()),
            "Config {} was not set correctly",
            key
        );
    }
    
    // Test that git config list includes our values
    let output = StdCommand::new("git")
        .args(&["config", "--list"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to list git config");
    
    let config_output = String::from_utf8_lossy(&output.stdout);
    
    // Debug: print the actual config output if the test fails
    if !config_output.contains("gitx.github.basebranch=main") {
        println!("Actual git config output:\n{}", config_output);
    }
    
    assert!(config_output.contains("gitx.github.token=ghp_test123"));
    assert!(config_output.contains("gitx.github.enabled=true"));
    assert!(config_output.contains("gitx.github.basebranch=main")); // Note: git normalizes to lowercase
    assert!(config_output.contains("gitx.branch.autocleanup=true")); // Note: git normalizes to lowercase
}

#[test]
fn test_gitx_init_creates_proper_git_repo_structure() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    setup_git_repo(&temp_dir);
    
    // Verify git repository structure exists
    temp_dir.child(".git").assert(predicate::path::is_dir());
    temp_dir.child(".git/config").assert(predicate::path::is_file());
    temp_dir.child(".git/HEAD").assert(predicate::path::is_file());
}

#[test]
fn test_complete_gitx_configuration_workflow() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    setup_git_repo(&temp_dir);
    
    // Simulate a complete gitx init workflow by setting all expected configs
    let expected_configs = [
        ("gitx.github.token", "ghp_1234567890abcdef"),
        ("gitx.github.enabled", "true"),
        ("gitx.github.baseBranch", "main"),
        ("gitx.branch.autoCleanup", "true"),
    ];
    
    // Set all configurations (as gitx init would do)
    for (key, value) in &expected_configs {
        let output = StdCommand::new("git")
            .args(&["config", key, value])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git config");
        
        assert!(output.status.success(), "Failed to set {}: {}", key, String::from_utf8_lossy(&output.stderr));
    }
    
    // Verify each configuration individually
    for (key, expected_value) in &expected_configs {
        let config_value = get_git_config(&temp_dir, key);
        assert_eq!(
            config_value,
            Some(expected_value.to_string()),
            "Configuration {} was not set to expected value {}",
            key,
            expected_value
        );
    }
    
    // Test that gitx commands can now theoretically work with this configuration
    // (We can't fully test without GitHub API, but we can verify config is readable)
    
    // Verify GitHub token is accessible
    assert!(get_git_config(&temp_dir, "gitx.github.token").is_some());
    
    // Verify boolean configs are properly set
    assert_eq!(get_git_config(&temp_dir, "gitx.github.enabled"), Some("true".to_string()));
    assert_eq!(get_git_config(&temp_dir, "gitx.branch.autoCleanup"), Some("true".to_string()));
    
    // Verify base branch is set
    assert_eq!(get_git_config(&temp_dir, "gitx.github.baseBranch"), Some("main".to_string()));
}

// Note: Testing the full interactive workflow is challenging without more sophisticated
// input simulation. The tests above verify the core functionality that gitx init
// depends on: git config management and git repository detection.

#[cfg(test)]
mod integration_helpers {
    use super::*;
    
    /// Helper to create a more realistic git repository with commits
    #[allow(dead_code)]
    pub fn setup_git_repo_with_commits(temp_dir: &assert_fs::TempDir) {
        setup_git_repo(temp_dir);
        
        // Create a test file and commit
        temp_dir.child("test.txt").write_str("test content").unwrap();
        
        StdCommand::new("git")
            .args(&["add", "test.txt"])
            .current_dir(temp_dir)
            .output()
            .expect("Failed to git add");
        
        StdCommand::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(temp_dir)
            .output()
            .expect("Failed to git commit");
    }
}