use assert_cmd::Command;
use predicates::prelude::*;

mod test_utils;
use test_utils::TestRepo;

/// Integration test for simple gitx workflow using CLI with mock client
/// This test verifies the full CLI workflow while using MockGitHubClient automatically in test mode
#[tokio::test]
async fn test_simple_workflow_with_cli_and_mock() {
    // Create test repository with gitx configuration
    let repo = TestRepo::with_gitx();
    
    // Set up mock GitHub token
    repo.set_git_config("gitx.github.token", "mock_token").unwrap();
    
    // Set up local repository as mock remote (allows git push to work)
    let _remote_path = repo.setup_mock_remote();
    
    // Step 1: Add a new file
    repo.add_file("feature.txt", "This is a new feature");
    
    // Step 2: Stage the file
    repo.git_add(&["feature.txt"]);
    
    // Step 3: Commit the staged file
    repo.git_commit("Add new feature");
    
    // Step 4: Run gitx diff to create GitHub PR
    // Force CLI to use MockGitHubClient via environment variable
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd
        .current_dir(&repo.temp_dir)
        .env("GITX_USE_MOCK_GITHUB", "1") // Force CLI binary to use MockGitHubClient
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created GitHub PR #1")) // Mock returns PR #1
        .stdout(predicate::str::contains("(transient branch deleted locally)"))
        .stderr(predicate::str::is_empty()); // Assert no error logs

}