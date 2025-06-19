use std::process::Command;
use inquire::{Text, Confirm, Select};

/// Initialize gitx configuration interactively
pub fn interactive_init() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Welcome to gitx! Let's set up your configuration.\n");
    
    // Ask for GitHub token
    let github_token = Text::new("GitHub Personal Access Token:")
        .with_help_message("Create one at https://github.com/settings/tokens with 'repo' scope")
        .with_placeholder("ghp_xxxxxxxxxxxxxxxxxxxx")
        .prompt()?;
    
    if !github_token.trim().is_empty() {
        set_git_config("gitx.github.token", &github_token)?;
        println!("✅ GitHub token configured");
    }
    
    // Ask if they want GitHub integration enabled by default
    let enable_github = Confirm::new("Enable GitHub integration by default for this repo?")
        .with_default(true)
        .with_help_message("When enabled, 'gitx diff' will automatically create GitHub PRs")
        .prompt()?;
    
    set_git_config("gitx.github.enabled", &enable_github.to_string())?;
    println!("✅ GitHub integration: {}", if enable_github { "enabled" } else { "disabled" });
    
    // Ask for base branch
    let base_branch_options = vec!["main", "master", "develop", "custom"];
    let base_branch_choice = Select::new("Default base branch for PRs:", base_branch_options)
        .with_help_message("This is the branch your PRs will target")
        .prompt()?;
    
    let base_branch = if base_branch_choice == "custom" {
        Text::new("Enter custom base branch name:")
            .with_default("main")
            .prompt()?
    } else {
        base_branch_choice.to_string()
    };
    
    set_git_config("gitx.github.baseBranch", &base_branch)?;
    println!("✅ Base branch set to: {}", base_branch);
    
    // Ask about branch cleanup
    let auto_cleanup = Confirm::new("Automatically clean up merged branches?")
        .with_default(true)
        .with_help_message("When enabled, 'gitx land' will clean up branches after merge")
        .prompt()?;
    
    set_git_config("gitx.branch.autoCleanup", &auto_cleanup.to_string())?;
    println!("✅ Auto cleanup: {}", if auto_cleanup { "enabled" } else { "disabled" });
    
    println!("\n🎉 gitx configuration complete!");
    println!("\nYour settings have been saved to this repository's git config.");
    println!("You can view them with: git config --list | grep gitx");
    println!("You can modify them with: git config gitx.<setting> <value>");
    
    println!("\n📚 Quick start:");
    println!("  gitx commit -m \"Your change\"     # Create a commit");
    println!("  gitx diff --github               # Create a GitHub PR");
    println!("  gitx prs                         # View PR status");
    
    Ok(())
}

/// Set a git config value for the current repository
fn set_git_config(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(&["config", key, value])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to set git config {}: {}", key, error).into());
    }
    
    Ok(())
}

/// Get a git config value
pub fn get_git_config(key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(&["config", key])
        .output()?;
    
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    } else {
        Ok(None)
    }
}

/// Check if gitx is initialized in the current repo
#[allow(dead_code)]
pub fn is_initialized() -> bool {
    get_git_config("gitx.github.token").unwrap_or(None).is_some()
}

/// Get the configured GitHub token (from repo config or environment)
pub fn get_github_token() -> Option<String> {
    // First try repo-specific config
    if let Ok(Some(token)) = get_git_config("gitx.github.token") {
        return Some(token);
    }
    
    // Fall back to global config
    if let Ok(Some(token)) = get_git_config_global("gitx.github.token") {
        return Some(token);
    }
    
    // Fall back to environment variable
    std::env::var("GITHUB_TOKEN").ok()
}

/// Get a global git config value
fn get_git_config_global(key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(&["config", "--global", key])
        .output()?;
    
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    } else {
        Ok(None)
    }
}

/// Check if GitHub integration is enabled
#[allow(dead_code)]
pub fn is_github_enabled() -> bool {
    get_git_config("gitx.github.enabled")
        .unwrap_or(None)
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Get the configured base branch
#[allow(dead_code)]
pub fn get_base_branch() -> String {
    get_git_config("gitx.github.baseBranch")
        .unwrap_or(None)
        .unwrap_or_else(|| "main".to_string())
}