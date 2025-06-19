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
    pub fn new_branch_created(pr_branch_name: String) -> Self {
        let now = Utc::now();
        Self {
            pr_branch_name,
            github_pr_number: None,
            status: PRStatus::BranchCreated,
            created_at: now,
            last_updated: now,
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
    pub fn mark_merged(mut self) -> Self {
        self.status = PRStatus::PRMerged;
        self.last_updated = Utc::now();
        self
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
        let metadata = CommitMetadata::new_branch_created("gitx/test/feature".to_string());
        
        assert_eq!(metadata.pr_branch_name, "gitx/test/feature");
        assert!(metadata.github_pr_number.is_none());
        assert!(matches!(metadata.status, PRStatus::BranchCreated));
    }

    #[test]
    fn test_metadata_with_pr_number() {
        let metadata = CommitMetadata::new_branch_created("gitx/test/feature".to_string())
            .with_pr_number(123);
        
        assert_eq!(metadata.github_pr_number, Some(123));
        assert!(matches!(metadata.status, PRStatus::PRCreated));
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