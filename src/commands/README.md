# Command Handlers

This directory contains the command handler modules for gitx. Each command has its own module to keep the code organized and maintainable.

## Structure

- `mod.rs` - Module declarations
- `branch.rs` - Handles `gitx branch` command for interactive branch switching
- `commit.rs` - Handles `gitx commit` command (git commit passthrough)
- `diff.rs` - Handles `gitx diff` command for creating/updating stacked PRs
- `init.rs` - Handles `gitx init` command for interactive configuration
- `land.rs` - Handles `gitx land` command for cleaning up merged PRs
- `prs.rs` - Handles `gitx prs` command for displaying PR status
- `status.rs` - Handles `gitx status` command (git status passthrough)

## Design Pattern

Each handler module exports a function that:
1. Takes the parsed command arguments as parameters
2. Returns `Result<(), Box<dyn std::error::Error>>`
3. Contains all the logic for that specific command
4. May be async if the command requires async operations (GitHub API calls)

This pattern keeps the main.rs file clean and makes each command's logic easy to find and modify.