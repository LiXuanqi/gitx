use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A Git extension tool"));
}

#[test]
fn test_diff_help() {
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.args(&["diff", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create/update stacked PRs from commits"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("gitx").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure();
}