use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::github::{GitHubClientTrait, PRInfo, GitHubPRStatus};
use crate::github_utils::generate_pr_body;
use crate::metadata::CommitMetadata;

/// Mock GitHub client for testing that stores operations in memory
#[derive(Debug, Clone)]
pub struct MockGitHubClient {
    /// Storage for created PRs: (branch_name -> PR info)
    created_prs: Arc<Mutex<HashMap<String, PRInfo>>>,
    /// Storage for PR statuses: (pr_number -> status)
    pr_statuses: Arc<Mutex<HashMap<u64, GitHubPRStatus>>>,
    /// Counter for generating PR numbers
    next_pr_number: Arc<Mutex<u64>>,
    /// Storage for PR updates: (pr_number -> (title, body))
    pr_updates: Arc<Mutex<HashMap<u64, (Option<String>, Option<String>)>>>,
}

impl MockGitHubClient {
    /// Create a new mock GitHub client
    pub fn new() -> Self {
        Self {
            created_prs: Arc::new(Mutex::new(HashMap::new())),
            pr_statuses: Arc::new(Mutex::new(HashMap::new())),
            next_pr_number: Arc::new(Mutex::new(1)),
            pr_updates: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a predefined PR status for testing
    pub fn add_pr_status(&self, pr_number: u64, status: GitHubPRStatus) {
        let mut statuses = self.pr_statuses.lock().unwrap();
        statuses.insert(pr_number, status);
    }

    /// Get all created PRs for testing verification
    pub fn get_created_prs(&self) -> HashMap<String, PRInfo> {
        self.created_prs.lock().unwrap().clone()
    }

    /// Get all PR updates for testing verification
    pub fn get_pr_updates(&self) -> HashMap<u64, (Option<String>, Option<String>)> {
        self.pr_updates.lock().unwrap().clone()
    }

    /// Check if a PR was created for a specific branch
    pub fn was_pr_created_for_branch(&self, branch_name: &str) -> bool {
        self.created_prs.lock().unwrap().contains_key(branch_name)
    }

    /// Check if a PR was updated
    pub fn was_pr_updated(&self, pr_number: u64) -> bool {
        self.pr_updates.lock().unwrap().contains_key(&pr_number)
    }

    /// Get the body of a created PR
    pub fn get_pr_body(&self, branch_name: &str) -> Option<String> {
        // In a real implementation, we'd store the body
        // For now, return a placeholder
        if self.was_pr_created_for_branch(branch_name) {
            Some(format!("Mock PR body for branch: {}", branch_name))
        } else {
            None
        }
    }
}

#[async_trait]
impl GitHubClientTrait for MockGitHubClient {
    async fn create_pr(
        &self,
        branch_name: &str,
        title: &str,
        body: &str,
        base_branch: &str,
    ) -> Result<PRInfo, Box<dyn std::error::Error>> {
        println!("Mock: Creating PR: {} -> {} with title: {}", branch_name, base_branch, title);
        
        // Generate a new PR number
        let pr_number = {
            let mut counter = self.next_pr_number.lock().unwrap();
            let number = *counter;
            *counter += 1;
            number
        };
        
        let pr_info = PRInfo {
            number: pr_number,
            url: format!("https://github.com/mock/repo/pull/{}", pr_number),
            title: title.to_string(),
        };
        
        // Store the created PR
        {
            let mut prs = self.created_prs.lock().unwrap();
            prs.insert(branch_name.to_string(), pr_info.clone());
        }
        
        // Create a default PR status as "open"
        let status = GitHubPRStatus {
            number: pr_number,
            state: "open".to_string(),
            title: title.to_string(),
            url: pr_info.url.clone(),
            mergeable: Some(true),
            draft: false,
        };
        
        {
            let mut statuses = self.pr_statuses.lock().unwrap();
            statuses.insert(pr_number, status);
        }
        
        Ok(pr_info)
    }
    
    async fn update_pr(
        &self,
        pr_number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Mock: Updating PR #{}", pr_number);
        
        // Store the update
        {
            let mut updates = self.pr_updates.lock().unwrap();
            updates.insert(
                pr_number,
                (title.map(|s| s.to_string()), body.map(|s| s.to_string())),
            );
        }
        
        // Update the PR status if it exists
        {
            let mut statuses = self.pr_statuses.lock().unwrap();
            if let Some(status) = statuses.get_mut(&pr_number) {
                if let Some(new_title) = title {
                    status.title = new_title.to_string();
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_pr_status(&self, pr_number: u64) -> Result<GitHubPRStatus, Box<dyn std::error::Error>> {
        let statuses = self.pr_statuses.lock().unwrap();
        if let Some(status) = statuses.get(&pr_number) {
            Ok(status.clone())
        } else {
            Err(format!("PR #{} not found", pr_number).into())
        }
    }
    
    async fn get_multiple_pr_statuses(&self, pr_numbers: &[u64]) -> Result<Vec<GitHubPRStatus>, Box<dyn std::error::Error>> {
        let mut statuses = Vec::new();
        
        for &pr_number in pr_numbers {
            match self.get_pr_status(pr_number).await {
                Ok(status) => statuses.push(status),
                Err(e) => {
                    eprintln!("Mock: Warning: Failed to get status for PR #{}: {}", pr_number, e);
                }
            }
        }
        
        Ok(statuses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{CommitMetadata, IncrementalCommitType};

    #[tokio::test]
    async fn test_mock_create_pr() {
        let mock = MockGitHubClient::new();
        
        let pr_info = mock.create_pr(
            "feature-branch",
            "Add new feature",
            "This adds a cool feature",
            "main"
        ).await.unwrap();
        
        assert_eq!(pr_info.number, 1);
        assert_eq!(pr_info.title, "Add new feature");
        assert!(pr_info.url.contains("/pull/1"));
        assert!(mock.was_pr_created_for_branch("feature-branch"));
    }

    #[tokio::test]
    async fn test_mock_update_pr() {
        let mock = MockGitHubClient::new();
        
        // Create a PR first
        let pr_info = mock.create_pr(
            "feature-branch",
            "Add new feature",
            "This adds a cool feature",
            "main"
        ).await.unwrap();
        
        // Update the PR
        mock.update_pr(pr_info.number, Some("Updated title"), Some("Updated body")).await.unwrap();
        
        assert!(mock.was_pr_updated(pr_info.number));
        let updates = mock.get_pr_updates();
        let (title, body) = updates.get(&pr_info.number).unwrap();
        assert_eq!(title.as_ref().unwrap(), "Updated title");
        assert_eq!(body.as_ref().unwrap(), "Updated body");
    }

    #[tokio::test]
    async fn test_mock_get_pr_status() {
        let mock = MockGitHubClient::new();
        
        // Create a PR
        let pr_info = mock.create_pr(
            "feature-branch",
            "Add new feature",
            "This adds a cool feature",
            "main"
        ).await.unwrap();
        
        // Get PR status
        let status = mock.get_pr_status(pr_info.number).await.unwrap();
        assert_eq!(status.number, pr_info.number);
        assert_eq!(status.state, "open");
        assert_eq!(status.title, "Add new feature");
    }

    #[tokio::test]
    async fn test_mock_get_multiple_pr_statuses() {
        let mock = MockGitHubClient::new();
        
        // Create two PRs
        let pr1 = mock.create_pr("branch1", "Feature 1", "Body 1", "main").await.unwrap();
        let pr2 = mock.create_pr("branch2", "Feature 2", "Body 2", "main").await.unwrap();
        
        // Get statuses for both
        let statuses = mock.get_multiple_pr_statuses(&[pr1.number, pr2.number]).await.unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0].number, pr1.number);
        assert_eq!(statuses[1].number, pr2.number);
    }

    #[test]
    fn test_mock_generate_pr_body() {
        let metadata = CommitMetadata::new_branch_created(
            "test-branch".to_string(),
            "abc123".to_string()
        );
        
        let body = generate_pr_body(&metadata, "Add feature\n\nThis is a test feature");
        assert!(body.contains("## Description"));
        assert!(body.contains("Generated by gitx"));
    }

    #[tokio::test] 
    async fn test_mock_add_predefined_status() {
        let mock = MockGitHubClient::new();
        
        // Add a predefined status
        let status = GitHubPRStatus {
            number: 42,
            state: "merged".to_string(),
            title: "Test PR".to_string(),
            url: "https://github.com/test/repo/pull/42".to_string(),
            mergeable: None,
            draft: false,
        };
        
        mock.add_pr_status(42, status);
        
        // Verify we can retrieve it
        let retrieved = mock.get_pr_status(42).await.unwrap();
        assert_eq!(retrieved.state, "merged");
        assert_eq!(retrieved.title, "Test PR");
    }
}