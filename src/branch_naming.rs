/// Branch naming utilities for transient PR branches

/// Generate a transient PR branch name from a commit message
/// Format: gitx/{username}/{sanitized-commit-title}
pub fn generate_branch_name(username: &str, commit_message: &str) -> String {
    let sanitized_title = sanitize_commit_title(commit_message);
    format!("gitx/{}/{}", username, sanitized_title)
}

/// Sanitize commit title to be suitable for branch names
/// - Convert to lowercase
/// - Replace spaces and special chars with hyphens
/// - Limit length to 50 characters
/// - Remove consecutive hyphens
fn sanitize_commit_title(commit_message: &str) -> String {
    // Take first line only (commit title)
    let title = commit_message.lines().next().unwrap_or("").trim();
    
    // Convert to lowercase and replace problematic characters
    let mut sanitized = title
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            _ => '-',
        })
        .collect::<String>();
    
    // Remove consecutive hyphens
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    
    // Trim hyphens from start and end
    sanitized = sanitized.trim_matches('-').to_string();
    
    // Limit length to 50 characters
    if sanitized.len() > 50 {
        sanitized.truncate(50);
        sanitized = sanitized.trim_matches('-').to_string();
    }
    
    // Ensure we have something
    if sanitized.is_empty() {
        sanitized = "untitled".to_string();
    }
    
    sanitized
}

/// Check if a branch name follows our transient PR pattern
pub fn is_transient_pr_branch(branch_name: &str) -> bool {
    branch_name.starts_with("gitx/") && branch_name.matches('/').count() == 2
}

/// Extract username from a transient PR branch name
#[allow(dead_code)]
pub fn extract_username(branch_name: &str) -> Option<&str> {
    if !is_transient_pr_branch(branch_name) {
        return None;
    }
    
    let parts: Vec<&str> = branch_name.split('/').collect();
    if parts.len() >= 2 {
        Some(parts[1])
    } else {
        None
    }
}

/// Extract feature name from a transient PR branch name
#[allow(dead_code)]
pub fn extract_feature_name(branch_name: &str) -> Option<&str> {
    if !is_transient_pr_branch(branch_name) {
        return None;
    }
    
    let parts: Vec<&str> = branch_name.split('/').collect();
    if parts.len() >= 3 {
        Some(parts[2])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_commit_title() {
        assert_eq!(sanitize_commit_title("Add user authentication"), "add-user-authentication");
        assert_eq!(sanitize_commit_title("Fix bug with special chars!@#"), "fix-bug-with-special-chars");
        assert_eq!(sanitize_commit_title("Multiple    spaces"), "multiple-spaces");
        assert_eq!(sanitize_commit_title("UPPERCASE"), "uppercase");
        assert_eq!(sanitize_commit_title(""), "untitled");
        assert_eq!(sanitize_commit_title("---"), "untitled");
        
        // Test length limiting
        let long_title = "a".repeat(60);
        let sanitized = sanitize_commit_title(&long_title);
        assert!(sanitized.len() <= 50);
    }

    #[test]
    fn test_generate_branch_name() {
        assert_eq!(
            generate_branch_name("alice", "Add user authentication"),
            "gitx/alice/add-user-authentication"
        );
        assert_eq!(
            generate_branch_name("bob", "Fix login validation"),
            "gitx/bob/fix-login-validation"
        );
    }

    #[test]
    fn test_is_transient_pr_branch() {
        assert!(is_transient_pr_branch("gitx/alice/add-user-auth"));
        assert!(is_transient_pr_branch("gitx/bob/fix-bug"));
        assert!(!is_transient_pr_branch("main"));
        assert!(!is_transient_pr_branch("feature/new-ui"));
        assert!(!is_transient_pr_branch("gitx/alice")); // Missing feature name
        assert!(!is_transient_pr_branch("gitx/alice/feature/nested")); // Too many slashes
    }

    #[test]
    fn test_extract_username() {
        assert_eq!(extract_username("gitx/alice/add-user-auth"), Some("alice"));
        assert_eq!(extract_username("gitx/bob/fix-bug"), Some("bob"));
        assert_eq!(extract_username("main"), None);
        assert_eq!(extract_username("gitx/alice"), None);
    }

    #[test]
    fn test_extract_feature_name() {
        assert_eq!(extract_feature_name("gitx/alice/add-user-auth"), Some("add-user-auth"));
        assert_eq!(extract_feature_name("gitx/bob/fix-bug"), Some("fix-bug"));
        assert_eq!(extract_feature_name("main"), None);
        assert_eq!(extract_feature_name("gitx/alice"), None);
    }
}