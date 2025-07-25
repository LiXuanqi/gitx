use serde::{Deserialize, Serialize};
use crate::metadata::CommitMetadata;
use crate::git_utils::GitUtils;

/// GitHub repository information
#[derive(Debug, Clone)]
pub struct GitHubRepo {
    pub owner: String,
    pub name: String,
}

/// Information about a created/updated PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRInfo {
    pub number: u64,
    pub url: String,
    pub title: String,
}

/// GitHub PR status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPRStatus {
    pub number: u64,
    pub state: String,       // "open", "closed", "merged"
    pub title: String,
    pub url: String,
    pub mergeable: Option<bool>,
    pub draft: bool,
}

/// Generate PR body content from commit metadata
pub fn generate_pr_body(metadata: &CommitMetadata, commit_message: &str) -> String {
    let mut body = String::new();
    
    // Add main commit message
    if commit_message.lines().count() > 1 {
        body.push_str("## Description\n\n");
        body.push_str(&commit_message.lines().skip(1).collect::<Vec<_>>().join("\n"));
        body.push_str("\n\n");
    }
    
    // Add incremental commits if any
    if !metadata.incremental_commits.is_empty() {
        body.push_str("## Updates\n\n");
        for (i, inc_commit) in metadata.incremental_commits.iter().enumerate() {
            body.push_str(&format!(
                "{}. **{}** ({})\n   - {}\n",
                i + 1,
                format_commit_type(&inc_commit.commit_type),
                inc_commit.created_at.format("%Y-%m-%d %H:%M UTC"),
                inc_commit.message.lines().next().unwrap_or("")
            ));
        }
        body.push_str("\n");
    }
    
    // Add metadata footer
    body.push_str("---\n");
    body.push_str(&format!("*Generated by gitx - Branch: `{}`*\n", metadata.pr_branch_name));
    body.push_str(&format!("*Created: {}*\n", metadata.created_at.format("%Y-%m-%d %H:%M UTC")));
    if !metadata.incremental_commits.is_empty() {
        body.push_str(&format!("*Last updated: {}*\n", metadata.last_updated.format("%Y-%m-%d %H:%M UTC")));
    }
    
    body
}

/// Get GitHub repository info from git remote
pub fn get_github_repo_from_remote() -> Result<GitHubRepo, Box<dyn std::error::Error>> {
    let remote_url = GitUtils::get_remote_url()?;
    let (owner, name) = GitUtils::parse_github_url(&remote_url)?;
    Ok(GitHubRepo { owner, name })
}

/// Check if GitHub token is available
pub fn check_github_token() -> bool {
    crate::config::get_github_token().is_some()
}

/// Format commit type for display
fn format_commit_type(commit_type: &crate::metadata::IncrementalCommitType) -> &'static str {
    match commit_type {
        crate::metadata::IncrementalCommitType::AmendedCommit => "Amended",
        crate::metadata::IncrementalCommitType::AdditionalCommit => "Additional",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_repo_creation() {
        let repo = GitHubRepo {
            owner: "testowner".to_string(),
            name: "testrepo".to_string(),
        };
        
        assert_eq!(repo.owner, "testowner");
        assert_eq!(repo.name, "testrepo");
    }

    #[test]
    fn test_pr_info_creation() {
        let pr_info = PRInfo {
            number: 42,
            url: "https://github.com/owner/repo/pull/42".to_string(),
            title: "Add new feature".to_string(),
        };
        
        assert_eq!(pr_info.number, 42);
        assert_eq!(pr_info.url, "https://github.com/owner/repo/pull/42");
        assert_eq!(pr_info.title, "Add new feature");
    }

    #[test]
    fn test_github_pr_status_creation() {
        let status = GitHubPRStatus {
            number: 123,
            state: "open".to_string(),
            title: "Fix bug".to_string(),
            url: "https://github.com/owner/repo/pull/123".to_string(),
            mergeable: Some(true),
            draft: false,
        };
        
        assert_eq!(status.number, 123);
        assert_eq!(status.state, "open");
        assert!(!status.draft);
        assert_eq!(status.mergeable, Some(true));
    }

    #[test]
    fn test_pr_body_generation() {
        use crate::metadata::{CommitMetadata, IncrementalCommitType};
        
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "abc123".to_string()
        ).add_incremental_commit(
            "def456".to_string(),
            "Fix issue with tests".to_string(),
            IncrementalCommitType::AmendedCommit
        );
        
        let commit_message = "Add new feature\n\nThis adds a really cool feature\nthat does amazing things.";
        
        let body = generate_pr_body(&metadata, commit_message);
        assert!(body.contains("## Description"));
        assert!(body.contains("## Updates"));
        assert!(body.contains("Generated by gitx"));
        assert!(!metadata.incremental_commits.is_empty());
    }
}