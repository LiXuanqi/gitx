use assert_cmd::Command;
use wiremock::MockServer;

mod test_utils;
use test_utils::{TestRepo };

/// Integration test for simple gitx workflow
/// This test focuses on a single, straightforward workflow scenario
#[tokio::test]
async fn test_simple_workflow() {
    // Start mock GitHub API server
    let mock_server = MockServer::start().await;
    
    
    // Create test repository with mock GitHub configuration (no commits)
    let repo = TestRepo::with_gitx();
    
    // Set up mock GitHub token
    repo.set_git_config("gitx.github.token", "mock_token").unwrap();
    
    // Set up local repository as mock remote (allows git push to work)
    let _remote_path = repo.setup_mock_remote();
    
    // Override GitHub API base URL to use our mock server
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    
    // Step 1: Add a new file
    repo.add_file("feature.txt", "This is a new feature");
    
    // Step 2: Stage the file
    repo.git_add(&["feature.txt"]);
    
    // Step 3: Commit the staged file
    repo.git_commit("Add new feature");
    
    // Debug: Check git remote and config
    let debug_output = std::process::Command::new("git")
        .args(&["remote", "-v"])
        .current_dir(&repo.temp_dir)
        .output()
        .unwrap();
    println!("Git remotes: {}", String::from_utf8_lossy(&debug_output.stdout));
    
    let debug_output = std::process::Command::new("git")
        .args(&["config", "--list"])
        .current_dir(&repo.temp_dir)
        .output()
        .unwrap();
    println!("Git config: {}", String::from_utf8_lossy(&debug_output.stdout));

    // Step 4: Run gitx diff to create GitHub PR
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    let assert_result = cmd.current_dir(&repo.temp_dir)
        .arg("diff")
        .assert()
        .success();

    // Print the captured output
    let output = assert_result.get_output();
    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    
    // Clean up environment
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
}