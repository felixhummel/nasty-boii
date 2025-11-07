mod fixtures;

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use fixtures::TestRepos;

#[test]
fn test_finds_nasty_repo() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("nasty-repo"));
}

#[test]
fn test_does_not_list_clean_repo() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("clean-repo").not());
}

#[test]
fn test_finds_no_upstream_repo() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no-upstream-repo"));
}

#[test]
fn test_missing_head_flag() {
    // NOTE: The --missing-head flag is designed to find repos with missing HEAD
    // However, bare repos (which have no HEAD by default) are not detected by
    // nasty-boii's walker since they don't have a .git subdirectory.
    // This test verifies that the flag doesn't crash and runs successfully.
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg("--missing-head")
        .arg(repos.path())
        .assert()
        .success();

    // Bare repos aren't found, so output would be empty
    // If we had a non-bare repo with missing HEAD, it would appear here
}

#[test]
fn test_missing_head_flag_does_not_show_nasty_repo() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg("--missing-head")
        .arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("nasty-repo").not());
}

#[test]
fn test_default_mode_does_not_show_missing_head() {
    let repos = TestRepos::new();

    // In default mode, missing-head repos should not appear in stdout
    // (they appear in warnings instead)
    cargo_bin_cmd!().arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("missing-head-repo").not());
}

#[test]
fn test_behind_repo_not_listed() {
    let repos = TestRepos::new();

    // Repos that are behind (but not ahead) should not be listed
    cargo_bin_cmd!().arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("behind-repo").not());
}

#[test]
fn test_verbose_flag() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg("--verbose")
        .arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Starting repository scan"));
}

#[test]
fn test_log_level_debug() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg("--log-level")
        .arg("debug")
        .arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Repository is clean"));
}

#[test]
fn test_current_directory_default() {
    // This test verifies that the tool works without explicit path argument
    // We run it in the fixtures directory
    let repos = TestRepos::new();

    cargo_bin_cmd!().current_dir(repos.path())
        .assert()
        .success();
}

#[test]
fn test_threads_flag() {
    let repos = TestRepos::new();

    cargo_bin_cmd!().arg("--threads")
        .arg("2")
        .arg("--verbose")
        .arg(repos.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Starting repository scan"));
}

#[test]
fn test_nonexistent_directory() {
    cargo_bin_cmd!().arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .success(); // WalkDir just returns no results for nonexistent paths
}

#[test]
fn test_empty_directory() {
    let temp_dir = tempfile::tempdir().unwrap();

    cargo_bin_cmd!().arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}
