use git2::{Repository, Oid};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Type of incremental commit
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IncrementalCommitType {
    /// Original commit was amended (git commit --amend)
    AmendedCommit,
    /// New commit added to this feature
    AdditionalCommit,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PRStatus {
    BranchCreated,
    PRCreated,
    PRMerged,
    Cancelled,
}

/// Git notes namespace for storing gitx metadata
const GITX_NOTES_REF: &str = "refs/notes/gitx-metadata";

/// Metadata about a commit and its associated PR
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommitMetadata {
    pub pr_branch_name: String,
    #[serde(default)]
    pub github_pr_number: Option<u64>,
    pub status: PRStatus,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub original_commit_id: String,
    #[serde(default)]
    pub incremental_commits: Vec<IncrementalCommit>,
}

/// Information about an incremental commit
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IncrementalCommit {
    pub commit_id: String,
    pub message: String,
    pub commit_type: IncrementalCommitType,
    pub created_at: DateTime<Utc>,
}

impl CommitMetadata {
    /// Create new metadata for a branch that was just created
    pub fn new_branch_created(pr_branch_name: String, original_commit_id: String) -> Self {
        let now = Utc::now();
        Self {
            pr_branch_name,
            github_pr_number: None,
            status: PRStatus::BranchCreated,
            created_at: now,
            last_updated: now,
            original_commit_id,
            incremental_commits: Vec::new(),
        }
    }
    
    /// Update metadata with GitHub PR number
    pub fn with_pr_number(mut self, pr_number: u64) -> Self {
        self.github_pr_number = Some(pr_number);
        self.status = PRStatus::PRCreated;
        self.last_updated = Utc::now();
        self
    }
    
    /// Add an incremental commit
    pub fn add_incremental_commit(mut self, commit_id: String, message: String, commit_type: IncrementalCommitType) -> Self {
        let incremental_commit = IncrementalCommit {
            commit_id,
            message,
            commit_type,
            created_at: Utc::now(),
        };
        
        self.incremental_commits.push(incremental_commit);
        self.last_updated = Utc::now();
        self
    }
    
    /// Mark as merged
    #[allow(dead_code)]
    pub fn mark_merged(mut self) -> Self {
        self.status = PRStatus::PRMerged;
        self.last_updated = Utc::now();
        self
    }
    
    /// Check if the current commit ID differs from the original stored commit ID
    pub fn is_commit_changed(&self, current_commit_id: &str) -> bool {
        self.original_commit_id != current_commit_id
    }
}

/// Store metadata for a commit using git notes
pub fn store_commit_metadata(commit_id: &Oid, metadata: &CommitMetadata) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let signature = repo.signature()?;
    
    let json = serde_json::to_string_pretty(metadata)?;
    
    // Store as a git note
    repo.note(&signature, &signature, Some(GITX_NOTES_REF), *commit_id, &json, false)?;
    
    Ok(())
}

/// Update existing metadata for a commit
pub fn update_commit_metadata(commit_id: &Oid, metadata: &CommitMetadata) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let signature = repo.signature()?;
    
    let json = serde_json::to_string_pretty(metadata)?;
    
    // Update the git note (force overwrite)
    repo.note(&signature, &signature, Some(GITX_NOTES_REF), *commit_id, &json, true)?;
    
    Ok(())
}

/// Get metadata for a commit
pub fn get_commit_metadata(commit_id: &Oid) -> Result<Option<CommitMetadata>, Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    
    match repo.find_note(Some(GITX_NOTES_REF), *commit_id) {
        Ok(note) => {
            if let Some(content) = note.message() {
                let metadata: CommitMetadata = serde_json::from_str(content)?;
                Ok(Some(metadata))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None), // Note doesn't exist
    }
}

/// Check if a commit has PR metadata
pub fn has_pr_metadata(commit_id: &Oid) -> bool {
    match get_commit_metadata(commit_id) {
        Ok(Some(_)) => true,
        _ => false,
    }
}

/// Information needed to display PR status
#[derive(Debug, Clone)]
pub struct PRStatusInfo {
    pub commit_id: String,
    pub commit_message: String,
    pub branch_name: String,
    pub pr_number: Option<u64>,
    pub status: PRStatus,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub incremental_count: usize,
    pub latest_incremental: Option<IncrementalCommit>,
}

impl PRStatusInfo {
    /// Create from commit metadata and message
    pub fn from_commit_and_metadata(commit_id: String, commit_message: String, metadata: &CommitMetadata) -> Self {
        let latest_incremental = metadata.incremental_commits.last().cloned();
        
        Self {
            commit_id,
            commit_message,
            branch_name: metadata.pr_branch_name.clone(),
            pr_number: metadata.github_pr_number,
            status: metadata.status.clone(),
            created_at: metadata.created_at,
            last_updated: metadata.last_updated,
            incremental_count: metadata.incremental_commits.len(),
            latest_incremental,
        }
    }
}

/// Get status information for all PRs
pub fn get_all_pr_status() -> Result<Vec<PRStatusInfo>, Box<dyn std::error::Error>> {
    // TODO: Fix git2 notes API usage
    // For now return empty list to allow compilation
    Ok(Vec::new())
}

/// Remove metadata for a commit (cleanup)
#[allow(dead_code)]
pub fn remove_commit_metadata(commit_id: &Oid) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let signature = repo.signature()?;
    repo.note_delete(*commit_id, Some(GITX_NOTES_REF), &signature, &signature)?;
    Ok(())
}

/// List all commits that have PR metadata
#[allow(dead_code)]
pub fn list_all_pr_commits() -> Result<Vec<(Oid, CommitMetadata)>, git2::Error> {
    // TODO: Fix git2 notes API usage
    // For now return empty list to allow compilation
    Ok(Vec::new())
}

/// Check if a commit at the current position differs from its stored metadata
/// Returns (has_metadata, needs_incremental_update)
#[allow(dead_code)]
pub fn check_commit_for_updates(current_oid: &Oid) -> Result<(bool, bool), git2::Error> {
    match get_commit_metadata(current_oid) {
        Ok(Some(metadata)) => {
            let current_commit_id = current_oid.to_string();
            let needs_update = metadata.is_commit_changed(&current_commit_id);
            Ok((true, needs_update))
        }
        Ok(None) => Ok((false, false)),
        Err(_) => Ok((false, false)),
    }
}

/// Find commits that need incremental updates
#[allow(dead_code)]
pub fn find_commits_needing_updates() -> Result<Vec<(Oid, CommitMetadata)>, git2::Error> {
    let all_pr_commits = list_all_pr_commits()?;
    let mut needs_updates = Vec::new();
    
    let repo = Repository::open(".")?;
    
    for (commit_oid, metadata) in all_pr_commits {
        let current_commit = repo.find_commit(commit_oid)?;
        let current_commit_id = current_commit.id().to_string();
        
        if metadata.is_commit_changed(&current_commit_id) {
            needs_updates.push((commit_oid, metadata));
        }
    }
    
    Ok(needs_updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use git2::{Repository, Signature};
    use std::fs;
    use std::path::Path;

    fn create_test_repo() -> Result<(Repository, TempDir), git2::Error> {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(&temp_dir)?;
        
        // Configure user for commits
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        
        // Create initial commit
        let signature = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            // Create a test file
            let test_file_path = temp_dir.path().join("test.txt");
            fs::write(&test_file_path, "test content").unwrap();
            index.add_path(Path::new("test.txt"))?;
            index.write()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        let _commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;
        drop(tree); // Explicitly drop to release the borrow
        
        Ok((repo, temp_dir))
    }

    #[test]
    fn test_commit_metadata_creation() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "abc123".to_string()
        );
        
        assert_eq!(metadata.pr_branch_name, "gitx/test/feature");
        assert_eq!(metadata.original_commit_id, "abc123");
        assert_eq!(metadata.status, PRStatus::BranchCreated);
        assert!(metadata.incremental_commits.is_empty());
        assert_eq!(metadata.incremental_commits.len(), 0);
        assert!(metadata.github_pr_number.is_none());
    }

    #[test]
    fn test_commit_metadata_with_pr_number() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "abc123".to_string()
        ).with_pr_number(42);
        
        assert_eq!(metadata.github_pr_number, Some(42));
        assert_eq!(metadata.status, PRStatus::PRCreated);
    }

    #[test]
    fn test_add_incremental_commit() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "abc123".to_string()
        ).add_incremental_commit(
            "def456".to_string(),
            "Fix issue with tests".to_string(),
            IncrementalCommitType::AmendedCommit
        );
        
        assert_eq!(metadata.incremental_commits.len(), 1);
        
        let inc_commit = &metadata.incremental_commits[0];
        assert_eq!(inc_commit.commit_id, "def456");
        assert_eq!(inc_commit.message, "Fix issue with tests");
        assert!(matches!(inc_commit.commit_type, IncrementalCommitType::AmendedCommit));
        
        // Verify the incremental commit was added
        let latest = &metadata.incremental_commits[0];
        assert_eq!(latest.commit_id, "def456");
    }

    #[test]
    fn test_commit_changed_detection() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "abc123".to_string()
        );
        
        // Same commit ID should not be changed
        assert!(!metadata.is_commit_changed("abc123"));
        
        // Different commit ID should be changed
        assert!(metadata.is_commit_changed("def456"));
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "abc123".to_string()
        ).with_pr_number(42)
        .add_incremental_commit(
            "def456".to_string(),
            "Update feature".to_string(),
            IncrementalCommitType::AdditionalCommit
        );
        
        // Serialize to JSON
        let json = serde_json::to_string(&metadata).expect("Failed to serialize");
        
        // Deserialize back
        let deserialized: CommitMetadata = serde_json::from_str(&json).expect("Failed to deserialize");
        
        assert_eq!(deserialized.pr_branch_name, metadata.pr_branch_name);
        assert_eq!(deserialized.original_commit_id, metadata.original_commit_id);
        assert_eq!(deserialized.github_pr_number, metadata.github_pr_number);
        assert_eq!(deserialized.incremental_commits.len(), metadata.incremental_commits.len());
        assert_eq!(deserialized.incremental_commits.len(), metadata.incremental_commits.len());
    }

    #[test]
    fn test_store_and_retrieve_metadata() {
        let (repo, _temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change working directory to the test repo
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(_temp_dir.path()).unwrap();
        
        // Get the HEAD commit
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        let commit_id = commit.id();
        
        // Create metadata
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            commit_id.to_string()
        );
        
        // Store metadata
        store_commit_metadata(&commit_id, &metadata).expect("Failed to store metadata");
        
        // Retrieve metadata
        let retrieved = get_commit_metadata(&commit_id).expect("Failed to get metadata");
        assert!(retrieved.is_some());
        
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.pr_branch_name, "gitx/test/feature");
        assert_eq!(retrieved.original_commit_id, commit_id.to_string());
        
        // Test has_pr_metadata
        assert!(has_pr_metadata(&commit_id));
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that old metadata format (without new fields) can still be deserialized
        let old_format_json = r#"{
            "pr_branch_name": "gitx/test/old-feature",
            "original_commit_id": "old123",
            "status": "BranchCreated",
            "created_at": "2023-01-01T00:00:00Z",
            "last_updated": "2023-01-01T00:00:00Z",
            "incremental_commits": []
        }"#;
        
        let metadata: CommitMetadata = serde_json::from_str(old_format_json)
            .expect("Failed to deserialize old format");
        
        // New fields should have default values
        assert_eq!(metadata.incremental_commits.len(), 0);
        assert!(metadata.github_pr_number.is_none());
    }
}