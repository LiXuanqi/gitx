use crate::github::GitHubClientTrait;

/// Factory function to create GitHub client - returns real client in production
#[cfg(not(test))]
pub async fn create_github_client() -> Result<Box<dyn GitHubClientTrait>, Box<dyn std::error::Error>> {
    // Allow tests to force use of mock client via environment variable
    if std::env::var("GITX_USE_MOCK_GITHUB").is_ok() {
        let mock_client = crate::mock_github::MockGitHubClient::new();
        Ok(Box::new(mock_client))
    } else {
        let client = crate::github::GitHubClient::new().await?;
        Ok(Box::new(client))
    }
}

/// Factory function to create GitHub client - returns mock client in tests
#[cfg(test)]
pub async fn create_github_client() -> Result<Box<dyn GitHubClientTrait>, Box<dyn std::error::Error>> {
    let mock_client = crate::mock_github::MockGitHubClient::new();
    Ok(Box::new(mock_client))
}