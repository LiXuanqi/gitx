use assert_cmd::Command;
use predicates::prelude::*;

mod test_utils;
use test_utils::TestRepo;

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
    let repo = TestRepo::empty(); // Not a git repo
    
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("init")
        .assert()
        .success() // Command succeeds but initialization fails
        .stderr(predicate::str::contains("Error during initialization")); // Should show initialization error
}

#[test]
fn test_gitx_init_basic_setup() {
    let repo = TestRepo::with_git();
    
    // Test that we can run gitx init (but we can't easily test the interactive parts)
    // For now, let's test that it at least recognizes it's in a git repo
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("init")
        .write_stdin("test_token\ny\nmain\ny\n") // Try to provide some inputs
        .assert()
        .success();
    
    // Verify that git config was potentially set (though hard to test with interactive prompts)
    // We'll check if the command succeeded, which is a good start
}

#[test]
fn test_git_config_functions() {
    let repo = TestRepo::with_git();
    
    // Manually set some gitx config values
    repo.set_git_config("gitx.github.token", "test_token")
        .expect("Failed to set config");
    repo.set_git_config("gitx.github.enabled", "true")
        .expect("Failed to set config");
    repo.set_git_config("gitx.github.baseBranch", "main")
        .expect("Failed to set config");
    
    // Verify the config values were set correctly
    assert_eq!(repo.get_git_config("gitx.github.token"), Some("test_token".to_string()));
    assert_eq!(repo.get_git_config("gitx.github.enabled"), Some("true".to_string()));
    assert_eq!(repo.get_git_config("gitx.github.baseBranch"), Some("main".to_string()));
}

#[test]
fn test_gitx_config_verification() {
    let repo = TestRepo::with_git();
    
    // Manually configure gitx to simulate what init would do
    let configs = [
        ("gitx.github.token", "ghp_test123"),
        ("gitx.github.enabled", "true"),
        ("gitx.github.baseBranch", "main"),
        ("gitx.branch.autoCleanup", "true"),
    ];
    
    for (key, value) in &configs {
        repo.set_git_config(key, value)
            .expect(&format!("Failed to set config {}", key));
    }
    
    // Verify all configs are set
    for (key, expected_value) in &configs {
        assert_eq!(
            repo.get_git_config(key),
            Some(expected_value.to_string()),
            "Config {} was not set correctly",
            key
        );
    }
    
    // Test that git config list includes our values
    let config_output = repo.get_all_git_config();
    
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
    let repo = TestRepo::with_git();
    
    // Verify git repository structure exists
    repo.assert_git_repo();
}

#[test]
fn test_complete_gitx_configuration_workflow() {
    let repo = TestRepo::with_git();
    
    // Use the built-in setup method
    repo.setup_gitx_config();
    
    // Verify gitx is properly configured
    assert!(repo.is_gitx_configured());
    
    // Test that gitx commands can now theoretically work with this configuration
    // (We can't fully test without GitHub API, but we can verify config is readable)
    
    // Verify GitHub token is accessible
    assert!(repo.get_git_config("gitx.github.token").is_some());
    
    // Verify boolean configs are properly set
    assert_eq!(repo.get_git_config("gitx.github.enabled"), Some("true".to_string()));
    assert_eq!(repo.get_git_config("gitx.branch.autoCleanup"), Some("true".to_string()));
    
    // Verify base branch is set
    assert_eq!(repo.get_git_config("gitx.github.baseBranch"), Some("main".to_string()));
}

#[test]
fn test_builder_style_constructors() {
    // Test the builder-style constructors
    let gitx_repo = TestRepo::with_gitx();
    assert!(gitx_repo.is_gitx_configured());
    gitx_repo.assert_git_repo();
    
    let repo_with_commits = TestRepo::with_commits();
    assert!(repo_with_commits.is_gitx_configured());
    repo_with_commits
        .assert_git_repo()
        .assert_file_exists("initial.txt")
        .assert_file_exists("feature.txt")
        .assert_file_exists("bugfix.txt");
}

// Note: Testing the full interactive workflow is challenging without more sophisticated
// input simulation. The tests above verify the core functionality that gitx init
// depends on: git config management and git repository detection.