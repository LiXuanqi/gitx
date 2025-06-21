use octocrab::Octocrab;
use async_trait::async_trait;
use crate::metadata::CommitMetadata;
use crate::github_utils::{generate_pr_body, get_github_repo_from_remote};

// Re-export commonly used items
pub use crate::github_utils::{GitHubRepo, PRInfo, GitHubPRStatus, check_github_token};

/// Trait for GitHub API operations to enable dependency injection and mocking
#[async_trait]
pub trait GitHubClientTrait {
    async fn create_pr(
        &self,
        branch_name: &str,
        title: &str,
        body: &str,
        base_branch: &str,
    ) -> Result<PRInfo, Box<dyn std::error::Error>>;
    
    async fn update_pr(
        &self,
        pr_number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    
    async fn get_pr_status(&self, pr_number: u64) -> Result<GitHubPRStatus, Box<dyn std::error::Error>>;
    
    async fn get_multiple_pr_statuses(&self, pr_numbers: &[u64]) -> Result<Vec<GitHubPRStatus>, Box<dyn std::error::Error>>;
}

/// GitHub API client wrapper
pub struct GitHubClient {
    octocrab: Octocrab,
    repo: crate::github_utils::GitHubRepo,
}

#[async_trait]
impl GitHubClientTrait for GitHubClient {
    async fn create_pr(
        &self,
        branch_name: &str,
        title: &str,
        body: &str,
        base_branch: &str,
    ) -> Result<PRInfo, Box<dyn std::error::Error>> {
        self.create_pr_impl(branch_name, title, body, base_branch).await
    }
    
    async fn update_pr(
        &self,
        pr_number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.update_pr_impl(pr_number, title, body).await
    }
    
    async fn get_pr_status(&self, pr_number: u64) -> Result<GitHubPRStatus, Box<dyn std::error::Error>> {
        self.get_pr_status_impl(pr_number).await
    }
    
    async fn get_multiple_pr_statuses(&self, pr_numbers: &[u64]) -> Result<Vec<GitHubPRStatus>, Box<dyn std::error::Error>> {
        self.get_multiple_pr_statuses_impl(pr_numbers).await
    }
}

impl GitHubClient {
    /// Create a new GitHub client
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Get GitHub token from config or environment
        let token = crate::config::get_github_token()
            .ok_or("GitHub token not configured. Run 'gitx init' to set up.")?;
        
        // Initialize octocrab with token
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()?;
        
        // Get repository info from git remote
        let repo = Self::get_github_repo_from_remote()?;
        
        Ok(Self { octocrab, repo })
    }
    
    /// Create a new pull request (implementation)
    pub async fn create_pr_impl(
        &self,
        branch_name: &str,
        title: &str,
        body: &str,
        base_branch: &str,
    ) -> Result<PRInfo, Box<dyn std::error::Error>> {
        println!("Creating PR: {} -> {}", branch_name, base_branch);
        
        let pr = self
            .octocrab
            .pulls(&self.repo.owner, &self.repo.name)
            .create(title, branch_name, base_branch)
            .body(body)
            .send()
            .await?;
        
        Ok(PRInfo {
            number: pr.number,
            url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
            title: pr.title.unwrap_or_default(),
        })
    }
    
    /// Update an existing pull request (implementation)
    pub async fn update_pr_impl(
        &self,
        pr_number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Updating PR #{}", pr_number);
        
        let pulls = self.octocrab.pulls(&self.repo.owner, &self.repo.name);
        let mut update = pulls.update(pr_number);
        
        if let Some(title) = title {
            update = update.title(title);
        }
        
        if let Some(body) = body {
            update = update.body(body);
        }
        
        update.send().await?;
        
        Ok(())
    }
    
    
    /// Get GitHub repository info from git remote
    fn get_github_repo_from_remote() -> Result<crate::github_utils::GitHubRepo, Box<dyn std::error::Error>> {
        get_github_repo_from_remote()
    }
}


impl GitHubClient {
    /// Get PR status from GitHub (implementation)
    pub async fn get_pr_status_impl(&self, pr_number: u64) -> Result<GitHubPRStatus, Box<dyn std::error::Error>> {
        let pr = self
            .octocrab
            .pulls(&self.repo.owner, &self.repo.name)
            .get(pr_number)
            .await?;
        
        Ok(GitHubPRStatus {
            number: pr.number,
            state: pr.state.map(|s| format!("{:?}", s).to_lowercase()).unwrap_or_default(),
            title: pr.title.unwrap_or_default(),
            url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
            mergeable: pr.mergeable,
            draft: pr.draft.unwrap_or(false),
        })
    }

    /// Get multiple PR statuses efficiently (implementation)
    pub async fn get_multiple_pr_statuses_impl(&self, pr_numbers: &[u64]) -> Result<Vec<GitHubPRStatus>, Box<dyn std::error::Error>> {
        let mut statuses = Vec::new();
        
        // Note: In a production system, you'd want to batch these requests
        // For now, we'll do them sequentially to avoid rate limiting
        for &pr_number in pr_numbers {
            match self.get_pr_status_impl(pr_number).await {
                Ok(status) => statuses.push(status),
                Err(e) => {
                    eprintln!("Warning: Failed to get status for PR #{}: {}", pr_number, e);
                }
            }
        }
        
        Ok(statuses)
    }
}

