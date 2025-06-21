use git2::Repository;
use url::Url;

/// Git repository utilities
pub struct GitUtils;

impl GitUtils {
    /// Push branch to remote origin
    pub async fn push_branch(branch_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Pushing branch to origin: {}", branch_name);
        
        // Use git command to push the branch
        let output = tokio::process::Command::new("git")
            .args(&["push", "-u", "origin", branch_name])
            .output()
            .await?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to push branch: {}", error).into());
        }
        
        Ok(())
    }
    
    /// Get the current repository's git remote URL
    pub fn get_remote_url() -> Result<String, Box<dyn std::error::Error>> {
        let repo = Repository::open(".")?;
        let remote = repo.find_remote("origin")?;
        let url_str = remote.url().ok_or("No URL found for origin remote")?;
        Ok(url_str.to_string())
    }
    
    /// Check if the current repository has a GitHub remote
    pub fn is_github_repository() -> bool {
        match Self::get_remote_url() {
            Ok(url) => Self::is_github_url(&url),
            Err(_) => false,
        }
    }
    
    /// Check if a URL is a GitHub URL
    pub fn is_github_url(url: &str) -> bool {
        if url.starts_with("git@github.com:") {
            true
        } else if let Ok(parsed_url) = Url::parse(url) {
            parsed_url.host_str() == Some("github.com")
        } else {
            false
        }
    }
    
    /// Parse GitHub repository information from a URL
    pub fn parse_github_url(url: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
        let (owner, name) = if url.starts_with("git@github.com:") {
            // SSH format: git@github.com:owner/repo.git
            let path = url.strip_prefix("git@github.com:").unwrap();
            let path = path.strip_suffix(".git").unwrap_or(path);
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() != 2 {
                return Err("Invalid GitHub SSH URL format".into());
            }
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // HTTPS format: https://github.com/owner/repo.git
            let parsed_url = Url::parse(url)?;
            if parsed_url.host_str() != Some("github.com") {
                return Err("Remote is not a GitHub repository".into());
            }
            
            let path = parsed_url.path().trim_start_matches('/');
            let path = path.strip_suffix(".git").unwrap_or(path);
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() != 2 {
                return Err("Invalid GitHub URL format".into());
            }
            (parts[0].to_string(), parts[1].to_string())
        };
        
        Ok((owner, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_github_url() {
        assert!(GitUtils::is_github_url("https://github.com/owner/repo.git"));
        assert!(GitUtils::is_github_url("git@github.com:owner/repo.git"));
        assert!(!GitUtils::is_github_url("https://gitlab.com/owner/repo.git"));
        assert!(!GitUtils::is_github_url("invalid-url"));
    }

    #[test]
    fn test_parse_github_url_https() {
        let (owner, name) = GitUtils::parse_github_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let (owner, name) = GitUtils::parse_github_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_github_url_without_git_suffix() {
        let (owner, name) = GitUtils::parse_github_url("https://github.com/owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_non_github_url() {
        assert!(GitUtils::parse_github_url("https://gitlab.com/owner/repo.git").is_err());
    }
}