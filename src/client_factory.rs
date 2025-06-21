use crate::github::GitHubClientTrait;

/// Factory function to create GitHub client - returns real client in production
#[cfg(not(test))]
pub async fn create_github_client() -> Result<Box<dyn GitHubClientTrait>, Box<dyn std::error::Error>> {
    let client = crate::github::GitHubClient::new().await?;
    Ok(Box::new(client))
}

/// Factory function to create GitHub client - returns mock client in tests
#[cfg(test)]
pub async fn create_github_client() -> Result<Box<dyn GitHubClientTrait>, Box<dyn std::error::Error>> {
    let mock_client = crate::mock_github::MockGitHubClient::new();
    Ok(Box::new(mock_client))
}