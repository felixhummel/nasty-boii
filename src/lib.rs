use anyhow::{Context, Result};
use git2::{BranchType, Repository};
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum RepoStatus {
    Clean,
    HasUnpushed,
    MissingHead,
}

/// Checks the status of a git repository.
///
/// # Errors
/// Returns an error if the repository cannot be opened or if git operations fail.
pub fn check_repo_status(repo_path: &Path) -> Result<RepoStatus> {
    let repo = Repository::open(repo_path).context(format!(
        "Failed to open repository at {}",
        repo_path.display()
    ))?;

    // Get the current branch
    let Ok(head) = repo.head() else {
        // Failed to get HEAD (unborn or missing)
        return Ok(RepoStatus::MissingHead);
    };

    if !head.is_branch() {
        // Not on a branch (detached HEAD), skip
        return Ok(RepoStatus::Clean);
    }

    let branch_name = head.shorthand().context("Failed to get branch name")?;

    let branch = repo
        .find_branch(branch_name, BranchType::Local)
        .context("Failed to find local branch")?;

    // Get the upstream branch
    let Ok(upstream) = branch.upstream() else {
        // No upstream branch configured, consider it as having unpushed changes
        // if there are any commits
        return Ok(RepoStatus::HasUnpushed);
    };

    // Get the local and remote commit OIDs
    let local_oid = branch
        .get()
        .target()
        .context("Failed to get local branch target")?;

    let remote_oid = upstream
        .get()
        .target()
        .context("Failed to get remote branch target")?;

    // Check if the branches point to different commits
    if local_oid == remote_oid {
        return Ok(RepoStatus::Clean);
    }

    // Check if local is ahead of remote
    let (ahead, _behind) = repo
        .graph_ahead_behind(local_oid, remote_oid)
        .context("Failed to calculate ahead/behind")?;

    if ahead > 0 {
        Ok(RepoStatus::HasUnpushed)
    } else {
        Ok(RepoStatus::Clean)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn setup_test_repo(name: &str) -> tempfile::TempDir {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().join(name);

        Command::new("git")
            .args(["init"])
            .arg(&repo_path)
            .output()
            .unwrap();

        // Configure git
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        temp_dir
    }

    #[test]
    fn test_missing_head_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().join("bare-repo");

        // Create bare repo (no HEAD)
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&repo_path)
            .output()
            .unwrap();

        let status = check_repo_status(&repo_path).unwrap();
        assert_eq!(status, RepoStatus::MissingHead);
    }

    #[test]
    fn test_repo_with_no_upstream() {
        let temp_dir = setup_test_repo("no-upstream");
        let repo_path = temp_dir.path().join("no-upstream");

        // Create a commit
        std::fs::write(repo_path.join("test.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // No remote configured, should be HasUnpushed
        let status = check_repo_status(&repo_path).unwrap();
        assert_eq!(status, RepoStatus::HasUnpushed);
    }

    #[test]
    fn test_clean_repo() {
        let temp_dir = setup_test_repo("clean");
        let repo_path = temp_dir.path().join("clean");

        // Create remote
        let remote_path = temp_dir.path().join("remote.git");
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_path)
            .output()
            .unwrap();

        // Create commit
        std::fs::write(repo_path.join("test.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Add remote and push
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&remote_path)
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let status = check_repo_status(&repo_path).unwrap();
        assert_eq!(status, RepoStatus::Clean);
    }

    #[test]
    fn test_repo_with_unpushed_commits() {
        let temp_dir = setup_test_repo("unpushed");
        let repo_path = temp_dir.path().join("unpushed");

        // Create remote
        let remote_path = temp_dir.path().join("remote.git");
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_path)
            .output()
            .unwrap();

        // Create and push initial commit
        std::fs::write(repo_path.join("test.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&remote_path)
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create unpushed commit
        std::fs::write(repo_path.join("unpushed.txt"), "new content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Unpushed commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let status = check_repo_status(&repo_path).unwrap();
        assert_eq!(status, RepoStatus::HasUnpushed);
    }

    #[test]
    fn test_repo_behind_remote() {
        let temp_dir = setup_test_repo("behind");
        let repo_path = temp_dir.path().join("behind");

        // Create remote
        let remote_path = temp_dir.path().join("remote.git");
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_path)
            .output()
            .unwrap();

        // Create and push initial commit
        std::fs::write(repo_path.join("test.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&remote_path)
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Clone and create remote commit
        let clone_path = temp_dir.path().join("clone");
        Command::new("git")
            .args(["clone"])
            .arg(&remote_path)
            .arg(&clone_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&clone_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&clone_path)
            .output()
            .unwrap();
        std::fs::write(clone_path.join("new.txt"), "new").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&clone_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Remote commit"])
            .current_dir(&clone_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["push"])
            .current_dir(&clone_path)
            .output()
            .unwrap();

        // Fetch in original repo
        Command::new("git")
            .args(["fetch"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Should be clean (behind but not ahead)
        let status = check_repo_status(&repo_path).unwrap();
        assert_eq!(status, RepoStatus::Clean);
    }
}
