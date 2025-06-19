use crate::metadata::{PRStatusInfo, PRStatus};
use crate::github::{GitHubClient, GitHubPRStatus};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Display the status of all stacked PRs
pub async fn display_status() -> Result<(), Box<dyn std::error::Error>> {
    let pr_statuses = crate::metadata::get_all_pr_status()?;
    
    if pr_statuses.is_empty() {
        println!("No stacked PRs found.");
        println!("Run 'gitx diff' to create PR branches from your commits.");
        return Ok(());
    }
    
    // Try to get GitHub status if token is available
    let github_statuses = if crate::github::check_github_token() {
        match get_github_statuses(&pr_statuses).await {
            Ok(statuses) => Some(statuses),
            Err(e) => {
                eprintln!("Warning: Could not fetch GitHub PR statuses: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    println!("📋 Stacked PR Status\n");
    
    for (i, pr_status) in pr_statuses.iter().enumerate() {
        display_pr_status(pr_status, github_statuses.as_ref(), i == 0)?;
        
        if i < pr_statuses.len() - 1 {
            println!(); // Add spacing between PRs
        }
    }
    
    // Show summary
    println!("\n{}", "─".repeat(60));
    display_summary(&pr_statuses, github_statuses.as_ref());
    
    Ok(())
}

/// Get GitHub PR statuses for all PRs that have numbers
async fn get_github_statuses(
    pr_statuses: &[PRStatusInfo],
) -> Result<HashMap<u64, GitHubPRStatus>, Box<dyn std::error::Error>> {
    let pr_numbers: Vec<u64> = pr_statuses
        .iter()
        .filter_map(|pr| pr.pr_number)
        .collect();
    
    if pr_numbers.is_empty() {
        return Ok(HashMap::new());
    }
    
    let github_client = GitHubClient::new().await?;
    let statuses = github_client.get_multiple_pr_statuses(&pr_numbers).await?;
    
    let mut status_map = HashMap::new();
    for status in statuses {
        status_map.insert(status.number, status);
    }
    
    Ok(status_map)
}

/// Display status for a single PR
fn display_pr_status(
    pr_status: &PRStatusInfo,
    github_statuses: Option<&HashMap<u64, GitHubPRStatus>>,
    _is_first: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let commit_short = &pr_status.commit_id[..8];
    let title = pr_status.commit_message.lines().next().unwrap_or("Untitled");
    
    // Status indicator
    let (status_icon, _status_color) = get_status_display(&pr_status.status, github_statuses, pr_status.pr_number);
    
    // Main PR line
    println!("🔗 {} {} {}", 
        status_icon,
        commit_short,
        title
    );
    
    // Branch info (remote-only for GitHub PRs)
    if pr_status.pr_number.is_some() {
        println!("   📁 Remote branch: {}", pr_status.branch_name);
    } else {
        println!("   📁 Branch: {}", pr_status.branch_name);
    }
    
    // GitHub PR info if available
    if let Some(pr_number) = pr_status.pr_number {
        if let Some(github_statuses) = github_statuses {
            if let Some(github_status) = github_statuses.get(&pr_number) {
                println!("   🐙 PR #{}: {} ({})", 
                    pr_number, 
                    github_status.state.to_uppercase(),
                    github_status.url
                );
                
                if github_status.draft {
                    println!("   📝 Draft PR");
                }
                
                if let Some(mergeable) = github_status.mergeable {
                    if !mergeable {
                        println!("   ⚠️  Has merge conflicts");
                    }
                }
            } else {
                println!("   🐙 PR #{}: Status unknown", pr_number);
            }
        } else {
            println!("   🐙 PR #{}", pr_number);
        }
    } else {
        println!("   📋 Local only (no GitHub PR)");
    }
    
    // Timing info
    println!("   📅 Created: {} ({})", 
        pr_status.created_at.format("%Y-%m-%d %H:%M UTC"),
        format_relative_time(&pr_status.created_at)
    );
    
    if pr_status.incremental_count > 0 {
        println!("   🔄 {} incremental update{}", 
            pr_status.incremental_count,
            if pr_status.incremental_count == 1 { "" } else { "s" }
        );
        println!("   📅 Last updated: {} ({})", 
            pr_status.last_updated.format("%Y-%m-%d %H:%M UTC"),
            format_relative_time(&pr_status.last_updated)
        );
        
        // Show latest incremental commit
        if let Some(latest) = &pr_status.latest_incremental {
            let inc_title = latest.message.lines().next().unwrap_or("Untitled");
            let inc_type = match latest.commit_type {
                crate::metadata::IncrementalCommitType::AmendedCommit => "Amended",
                crate::metadata::IncrementalCommitType::AdditionalCommit => "Additional",
            };
            println!("   ├─ {}: {}", inc_type, inc_title);
        }
    }
    
    Ok(())
}

/// Get status display information
fn get_status_display(
    local_status: &PRStatus,
    github_statuses: Option<&HashMap<u64, GitHubPRStatus>>,
    pr_number: Option<u64>,
) -> (&'static str, &'static str) {
    // Check GitHub status first if available
    if let (Some(github_statuses), Some(pr_num)) = (github_statuses, pr_number) {
        if let Some(github_status) = github_statuses.get(&pr_num) {
            return match github_status.state.as_str() {
                "open" => if github_status.draft { ("🚧", "yellow") } else { ("🟢", "green") },
                "closed" => ("🔴", "red"),
                "merged" => ("🟣", "purple"),
                _ => ("❓", "gray"),
            };
        }
    }
    
    // Fallback to local status
    match local_status {
        PRStatus::BranchCreated => ("🆕", "blue"),
        PRStatus::PRCreated => ("🟢", "green"),
        PRStatus::PRMerged => ("🟣", "purple"),
        PRStatus::Cancelled => ("❌", "red"),
    }
}

/// Display summary statistics
fn display_summary(
    pr_statuses: &[PRStatusInfo],
    github_statuses: Option<&HashMap<u64, GitHubPRStatus>>,
) {
    let total = pr_statuses.len();
    let with_github_pr = pr_statuses.iter().filter(|pr| pr.pr_number.is_some()).count();
    let local_only = total - with_github_pr;
    
    println!("📊 Summary: {} total PR{}", total, if total == 1 { "" } else { "s" });
    
    if with_github_pr > 0 {
        println!("   🐙 {} with GitHub PR{}", with_github_pr, if with_github_pr == 1 { "" } else { "s" });
        
        // GitHub status breakdown if available
        if let Some(github_statuses) = github_statuses {
            let mut open = 0;
            let mut merged = 0;
            let mut closed = 0;
            let mut draft = 0;
            
            for pr in pr_statuses {
                if let Some(pr_num) = pr.pr_number {
                    if let Some(status) = github_statuses.get(&pr_num) {
                        match status.state.as_str() {
                            "open" => {
                                open += 1;
                                if status.draft { draft += 1; }
                            },
                            "merged" => merged += 1,
                            "closed" => closed += 1,
                            _ => {}
                        }
                    }
                }
            }
            
            if open > 0 { println!("     └─ {} open ({}drafts)", open, if draft > 0 { format!("{} ", draft) } else { "no ".to_string() }); }
            if merged > 0 { println!("     └─ {} merged", merged); }
            if closed > 0 { println!("     └─ {} closed", closed); }
        }
    }
    
    if local_only > 0 {
        println!("   📋 {} local only", local_only);
    }
    
    let total_incremental: usize = pr_statuses.iter().map(|pr| pr.incremental_count).sum();
    if total_incremental > 0 {
        println!("   🔄 {} total incremental update{}", total_incremental, if total_incremental == 1 { "" } else { "s" });
    }
}

/// Format a timestamp as relative time (e.g., "2 hours ago")
fn format_relative_time(timestamp: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*timestamp);
    
    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if duration.num_days() < 30 {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if duration.num_days() < 365 {
        let months = duration.num_days() / 30;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = duration.num_days() / 365;
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}