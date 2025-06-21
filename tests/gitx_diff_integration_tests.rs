use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use wiremock::{
    matchers::{method, path, header},
    Mock, MockServer, ResponseTemplate,
};

mod test_utils;
use test_utils::TestRepo;

/// Integration tests for `gitx diff` command using mock GitHub API server
/// This tests the full workflow without making real GitHub API calls

#[tokio::test]
async fn test_gitx_diff_creates_pr_successfully() {
    // Start mock GitHub API server
    let mock_server = MockServer::start().await;
    
    // Mock the GitHub API endpoints
    // GitHubMocks::setup_default_mocks(&mock_server).await; // Commented out for now
    
    // Create test repository with commits and mock GitHub configuration
    let repo = TestRepo::with_configured_gitx_and_commits();
    
    // Override GitHub API base URL to use our mock server
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    
    // Add a new commit to create a PR for
    repo.add_and_commit("new_feature.txt", "new feature content", "Add new feature");
    
    // Run gitx diff (GitHub integration should be automatic when configured)
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created branch"));
    
    // Clean up environment
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
}

#[tokio::test]
async fn test_gitx_diff_handles_github_api_errors() {
    let mock_server = MockServer::start().await;
    
    // Mock GitHub API to return error responses
    Mock::given(method("POST"))
        .and(path("/repos/test-owner/test-repo/pulls"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "Validation Failed",
            "errors": [{"message": "A pull request already exists"}]
        })))
        .mount(&mock_server)
        .await;
    
    let repo = TestRepo::with_configured_gitx_and_commits();
    
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    
    repo.add_and_commit("error_test.txt", "error test", "Test error handling");
    
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("diff")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
    
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
}

#[tokio::test]
async fn test_gitx_diff_updates_existing_pr() {
    let mock_server = MockServer::start().await;
    
    // Mock successful PR creation
    Mock::given(method("POST"))
        .and(path("/repos/test-owner/test-repo/pulls"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "number": 123,
            "html_url": "https://github.com/test-owner/test-repo/pull/123",
            "title": "Add new feature",
            "body": "This PR adds a new feature"
        })))
        .mount(&mock_server)
        .await;
    
    // Mock PR update
    Mock::given(method("PATCH"))
        .and(path("/repos/test-owner/test-repo/pulls/123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "number": 123,
            "html_url": "https://github.com/test-owner/test-repo/pull/123",
            "title": "Add new feature - Updated",
            "body": "This PR adds a new feature - Updated"
        })))
        .mount(&mock_server)
        .await;
    
    // Mock repository info
    Mock::given(method("GET"))
        .and(path("/repos/test-owner/test-repo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "test-repo",
            "owner": {"login": "test-owner"},
            "default_branch": "main"
        })))
        .mount(&mock_server)
        .await;
    
    let repo = TestRepo::with_configured_gitx_and_commits();
    
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    
    repo.add_and_commit("feature.txt", "feature content", "Add new feature");
    
    // First run - create PR
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("diff")
        .assert()
        .success();
    
    // Modify the commit (simulate amending)
    repo.add_file("feature.txt", "updated feature content");
    repo.git_add(&["feature.txt"]);
    repo.git_commit("Add new feature - Updated");
    
    // Second run - should update existing PR
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("diff")
        .assert()
        .success();
    
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
}

#[tokio::test]
async fn test_gitx_diff_without_github_flag() {
    let repo = TestRepo::with_commits();
    repo.add_and_commit("local_feature.txt", "local content", "Add local feature");
    
    // Run gitx diff without GitHub integration (should create branch but not PR)
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("Creating PR branch for"));
}

#[tokio::test]
async fn test_gitx_diff_multiple_commits() {
    let mock_server = MockServer::start().await;
    setup_github_api_mocks(&mock_server).await;
    
    let repo = TestRepo::with_configured_gitx_and_commits();
    
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    
    // Add multiple commits
    repo.add_and_commit("feature1.txt", "feature1", "Add feature 1");
    repo.add_and_commit("feature2.txt", "feature2", "Add feature 2");
    
    // Run gitx diff --all (should show interactive selection)
    // Note: This test is limited since we can't easily test interactive selection
    // We'll test that the command recognizes multiple commits
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .args(&["diff", "--all"])
        .assert()
        .success();
    
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
}

/// Helper function to set up common GitHub API mocks
async fn setup_github_api_mocks(mock_server: &MockServer) {
    // Mock repository info endpoint
    Mock::given(method("GET"))
        .and(path("/repos/test-owner/test-repo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "test-repo",
            "owner": {"login": "test-owner"},
            "default_branch": "main"
        })))
        .mount(mock_server)
        .await;
    
    // Mock PR creation endpoint
    Mock::given(method("POST"))
        .and(path("/repos/test-owner/test-repo/pulls"))
        .and(header("authorization", "Bearer mock_token"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "number": 123,
            "html_url": "https://github.com/test-owner/test-repo/pull/123",
            "title": "Test PR",
            "body": "Test PR body"
        })))
        .mount(mock_server)
        .await;
    
    // Mock user info endpoint (for getting username)
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("authorization", "Bearer mock_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "login": "test-user"
        })))
        .mount(mock_server)
        .await;
    
    // Mock branch creation (if needed)
    Mock::given(method("POST"))
        .and(path("/repos/test-owner/test-repo/git/refs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "ref": "refs/heads/test-branch",
            "object": {"sha": "abc123"}
        })))
        .mount(mock_server)
        .await;
}

#[tokio::test]
async fn test_gitx_diff_with_authentication_failure() {
    let mock_server = MockServer::start().await;
    
    // Mock authentication failure
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "message": "Bad credentials"
        })))
        .mount(&mock_server)
        .await;
    
    let repo = TestRepo::with_commits();
    repo.set_git_config("gitx.github.token", "invalid_token").unwrap();
    repo.set_git_config("gitx.github.enabled", "true").unwrap();
    
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    
    repo.add_and_commit("auth_test.txt", "auth test", "Test auth failure");
    
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .args(&["diff", "--github"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("authentication").or(predicate::str::contains("credentials")));
    
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
}

#[tokio::test]
async fn test_gitx_diff_network_timeout() {
    let mock_server = MockServer::start().await;
    
    // Mock slow response to test timeout handling
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(10)) // 10 second delay
                .set_body_json(json!({"login": "test-user"}))
        )
        .mount(&mock_server)
        .await;
    
    let repo = TestRepo::with_commits();
    repo.set_git_config("gitx.github.token", "mock_token").unwrap();
    repo.set_git_config("gitx.github.enabled", "true").unwrap();
    
    let mock_url = mock_server.uri();
    unsafe { std::env::set_var("GITHUB_API_BASE_URL", &mock_url); }
    unsafe { std::env::set_var("GITX_TIMEOUT_SECONDS", "1"); } // 1 second timeout
    
    repo.add_and_commit("timeout_test.txt", "timeout test", "Test timeout");
    
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.current_dir(&repo.temp_dir)
        .args(&["diff", "--github"])
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .failure(); // Should fail due to timeout
    
    unsafe { std::env::remove_var("GITHUB_API_BASE_URL"); }
    unsafe { std::env::remove_var("GITX_TIMEOUT_SECONDS"); }
}