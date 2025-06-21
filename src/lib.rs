pub mod git_ops;
pub mod git_utils;
pub mod github_utils;
pub mod branch_naming;
pub mod metadata;
pub mod github;
pub mod status_display;
pub mod config;
pub mod cli;
pub mod commands;
pub mod client_factory;

// Make mock_github available for CLI integration testing
pub mod mock_github;