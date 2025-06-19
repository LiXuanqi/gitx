use git2::{Repository, Oid};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Metadata stored for each commit that has a PR branch
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommitMetadata {
    pub pr_branch_name: String,
    pub github_pr_number: Option<u64>,
    pub status: PRStatus,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub original_commit_id: String,
    #[serde(default)]
    pub incremental_commits: Vec<IncrementalCommit>,
}

/// Information about an incremental commit added to a PR branch
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IncrementalCommit {
    pub commit_id: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub commit_type: IncrementalCommitType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IncrementalCommitType {
    /// Original commit was amended
    AmendedCommit,
    /// New commit added to this feature
    AdditionalCommit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PRStatus {
    BranchCreated,
    PRCreated,
    PRMerged,
    Cancelled,
}

/// Git notes namespace for storing gitx metadata
const GITX_NOTES_REF: &str = "refs/notes/gitx-metadata";

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
    
    /// Mark as merged
    #[allow(dead_code)]
    pub fn mark_merged(mut self) -> Self {
        self.status = PRStatus::PRMerged;
        self.last_updated = Utc::now();
        self
    }
    
    /// Add an incremental commit to this PR
    pub fn add_incremental_commit(mut self, commit_id: String, message: String, commit_type: IncrementalCommitType) -> Self {
        let incremental_commit = IncrementalCommit {
            commit_id,
            message,
            created_at: Utc::now(),
            commit_type,
        };
        
        self.incremental_commits.push(incremental_commit);
        self.last_updated = Utc::now();
        self
    }
    
    /// Check if the original commit has been changed (amended)
    pub fn is_commit_changed(&self, current_commit_id: &str) -> bool {
        // If original_commit_id is empty (backward compatibility), assume no change
        if self.original_commit_id.is_empty() {
            false
        } else {
            self.original_commit_id != current_commit_id
        }
    }
}

/// Store commit metadata in git notes
pub fn store_commit_metadata(commit_id: &Oid, metadata: &CommitMetadata) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    
    // Serialize metadata to JSON
    let json_data = serde_json::to_string_pretty(metadata)
        .map_err(|e| git2::Error::from_str(&format!("Failed to serialize metadata: {}", e)))?;
    
    // Get or create the notes reference
    let signature = repo.signature()?;
    
    // Write the note
    repo.note(&signature, &signature, Some(GITX_NOTES_REF), *commit_id, &json_data, false)?;
    
    Ok(())
}

/// Retrieve commit metadata from git notes
pub fn get_commit_metadata(commit_id: &Oid) -> Result<Option<CommitMetadata>, git2::Error> {
    let repo = Repository::open(".")?;
    
    // Try to get the note
    match repo.find_note(Some(GITX_NOTES_REF), *commit_id) {
        Ok(note) => {
            let note_content = note.message().unwrap_or("");
            
            // Deserialize JSON
            match serde_json::from_str::<CommitMetadata>(note_content) {
                Ok(metadata) => Ok(Some(metadata)),
                Err(e) => {
                    eprintln!("Warning: Failed to parse metadata for commit {}: {}", commit_id, e);
                    Ok(None)
                }
            }
        }
        Err(_) => Ok(None), // Note doesn't exist
    }
}

/// Update existing commit metadata
pub fn update_commit_metadata(commit_id: &Oid, metadata: &CommitMetadata) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    
    // Serialize metadata to JSON
    let json_data = serde_json::to_string_pretty(metadata)
        .map_err(|e| git2::Error::from_str(&format!("Failed to serialize metadata: {}", e)))?;
    
    // Get signature
    let signature = repo.signature()?;
    
    // Update the note (force=true to overwrite)
    repo.note(&signature, &signature, Some(GITX_NOTES_REF), *commit_id, &json_data, true)?;
    
    Ok(())
}

/// List all commits that have PR metadata
pub fn list_all_pr_commits() -> Result<Vec<(Oid, CommitMetadata)>, git2::Error> {
    let repo = Repository::open(".")?;
    let mut results = Vec::new();
    
    // Try to get notes iterator
    match repo.notes(Some(GITX_NOTES_REF)) {
        Ok(notes) => {
            // Iterate through all notes
            for note_result in notes {
                let (_note_oid, annotated_oid) = note_result?;
                
                // Get the note content
                if let Ok(note) = repo.find_note(Some(GITX_NOTES_REF), annotated_oid) {
                    if let Some(content) = note.message() {
                        // Try to deserialize
                        if let Ok(metadata) = serde_json::from_str::<CommitMetadata>(content) {
                            results.push((annotated_oid, metadata));
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Notes reference doesn't exist yet, return empty list
        }
    }
    
    Ok(results)
}

/// Remove metadata for a commit (cleanup)
#[allow(dead_code)]
pub fn remove_commit_metadata(commit_id: &Oid) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let signature = repo.signature()?;
    repo.note_delete(*commit_id, Some(GITX_NOTES_REF), &signature, &signature)?;
    Ok(())
}

/// Check if a commit has PR metadata
pub fn has_pr_metadata(commit_id: &Oid) -> bool {
    get_commit_metadata(commit_id).unwrap_or(None).is_some()
}

/// Information about a PR for status display
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

/// Get all PRs for status display
pub fn get_all_pr_status() -> Result<Vec<PRStatusInfo>, git2::Error> {
    let repo = Repository::open(".")?;
    let all_pr_commits = list_all_pr_commits()?;
    let mut status_infos = Vec::new();
    
    for (commit_oid, metadata) in all_pr_commits {
        // Get commit information
        if let Ok(commit) = repo.find_commit(commit_oid) {
            let commit_message = commit.message().unwrap_or("").to_string();
            let latest_incremental = metadata.incremental_commits.last().cloned();
            
            status_infos.push(PRStatusInfo {
                commit_id: commit_oid.to_string(),
                commit_message,
                branch_name: metadata.pr_branch_name.clone(),
                pr_number: metadata.github_pr_number,
                status: metadata.status.clone(),
                created_at: metadata.created_at,
                last_updated: metadata.last_updated,
                incremental_count: metadata.incremental_commits.len(),
                latest_incremental,
            });
        }
    }
    
    // Sort by creation time (newest first)
    status_infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    Ok(status_infos)
}

/// Check if a commit at the current position differs from its stored metadata
/// Returns (has_metadata, needs_incremental_update)
#[allow(dead_code)]
pub fn check_commit_for_updates(current_oid: &Oid) -> Result<(bool, bool), git2::Error> {
    match get_commit_metadata(current_oid)? {
        Some(metadata) => {
            let current_commit_id = current_oid.to_string();
            let needs_update = metadata.is_commit_changed(&current_commit_id);
            Ok((true, needs_update))
        }
        None => Ok((false, false)),
    }
}

/// Find commits that need incremental updates
#[allow(dead_code)]
pub fn find_commits_needing_updates() -> Result<Vec<(Oid, CommitMetadata)>, git2::Error> {
    let all_pr_commits = list_all_pr_commits()?;
    let mut needs_updates = Vec::new();
    
    let repo = Repository::open(".")?;
    
    for (stored_oid, metadata) in all_pr_commits {
        // Try to find the commit at the stored position
        if let Ok(_stored_commit) = repo.find_commit(stored_oid) {
            let current_commit_id = stored_oid.to_string();
            
            // Check if the commit content has changed (this would happen if rebased/amended)
            if metadata.is_commit_changed(&current_commit_id) {
                needs_updates.push((stored_oid, metadata));
            }
        }
    }
    
    Ok(needs_updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile;

    fn create_test_repo() -> Result<(Repository, tempfile::TempDir), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let repo = Repository::init(&temp_dir)?;
        
        // Configure user for commits
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;
        
        Ok((repo, temp_dir))
    }

    #[test]
    fn test_commit_metadata_creation() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "1234567890abcdef".to_string()
        );
        
        assert_eq!(metadata.pr_branch_name, "gitx/test/feature");
        assert_eq!(metadata.original_commit_id, "1234567890abcdef");
        assert!(metadata.github_pr_number.is_none());
        assert!(matches!(metadata.status, PRStatus::BranchCreated));
        assert!(metadata.incremental_commits.is_empty());
    }

    #[test]
    fn test_metadata_with_pr_number() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "1234567890abcdef".to_string()
        ).with_pr_number(123);
        
        assert_eq!(metadata.github_pr_number, Some(123));
        assert!(matches!(metadata.status, PRStatus::PRCreated));
    }

    #[test]
    fn test_incremental_commit_handling() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "1234567890abcdef".to_string()
        ).add_incremental_commit(
            "abcdef1234567890".to_string(),
            "Updated feature implementation".to_string(),
            IncrementalCommitType::AmendedCommit
        );
        
        assert_eq!(metadata.incremental_commits.len(), 1);
        assert_eq!(metadata.incremental_commits[0].commit_id, "abcdef1234567890");
        assert_eq!(metadata.incremental_commits[0].message, "Updated feature implementation");
        assert!(matches!(metadata.incremental_commits[0].commit_type, IncrementalCommitType::AmendedCommit));
    }

    #[test]
    fn test_commit_change_detection() {
        let metadata = CommitMetadata::new_branch_created(
            "gitx/test/feature".to_string(),
            "1234567890abcdef".to_string()
        );
        
        assert!(!metadata.is_commit_changed("1234567890abcdef"));
        assert!(metadata.is_commit_changed("abcdef1234567890"));
    }

    #[test]
    fn test_has_pr_metadata_false_for_nonexistent() {
        let (_repo, temp_dir) = create_test_repo().expect("Failed to create test repo");
        
        // Change to test repo directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();
        
        // Test with a random OID
        let test_oid = Oid::from_str("1234567890123456789012345678901234567890").unwrap();
        assert!(!has_pr_metadata(&test_oid));
        
        // Restore directory
        env::set_current_dir(original_dir).unwrap();
    }
}