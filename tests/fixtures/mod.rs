use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub struct TestRepos {
    pub temp_dir: TempDir,
    #[allow(dead_code)]
    pub clean_repo: PathBuf,
    #[allow(dead_code)]
    pub nasty_repo: PathBuf,
    #[allow(dead_code)]
    pub missing_head_repo: PathBuf,
    #[allow(dead_code)]
    pub no_upstream_repo: PathBuf,
    #[allow(dead_code)]
    pub behind_repo: PathBuf,
}

impl TestRepos {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // Create clean repo (all changes pushed)
        let clean_repo = base.join("clean-repo");
        Self::create_clean_repo(&clean_repo);

        // Create nasty repo (has unpushed commits)
        let nasty_repo = base.join("nasty-repo");
        Self::create_nasty_repo(&nasty_repo);

        // Create missing head repo (bare repo with no HEAD)
        let missing_head_repo = base.join("missing-head-repo");
        Self::create_missing_head_repo(&missing_head_repo);

        // Create repo with no upstream configured
        let no_upstream_repo = base.join("no-upstream-repo");
        Self::create_no_upstream_repo(&no_upstream_repo);

        // Create repo that is behind remote (for edge case testing)
        let behind_repo = base.join("behind-repo");
        Self::create_behind_repo(&behind_repo);

        Self {
            temp_dir,
            clean_repo,
            nasty_repo,
            missing_head_repo,
            no_upstream_repo,
            behind_repo,
        }
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    fn create_clean_repo(path: &Path) {
        // Create bare remote
        let remote_path = path.parent().unwrap().join("clean-repo-remote.git");
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_path)
            .output()
            .unwrap();

        // Create local repo
        Command::new("git")
            .args(["init"])
            .arg(path)
            .output()
            .unwrap();

        // Configure git
        Self::git_config(path, "user.name", "Test User");
        Self::git_config(path, "user.email", "test@example.com");

        // Create initial commit
        std::fs::write(path.join("README.md"), "# Clean Repo\n").unwrap();
        Self::git_add_commit(path, "Initial commit");

        // Add remote and push
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&remote_path)
            .current_dir(path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn create_nasty_repo(path: &Path) {
        // Create bare remote
        let remote_path = path.parent().unwrap().join("nasty-repo-remote.git");
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_path)
            .output()
            .unwrap();

        // Create local repo
        Command::new("git")
            .args(["init"])
            .arg(path)
            .output()
            .unwrap();

        // Configure git
        Self::git_config(path, "user.name", "Test User");
        Self::git_config(path, "user.email", "test@example.com");

        // Create initial commit
        std::fs::write(path.join("README.md"), "# Nasty Repo\n").unwrap();
        Self::git_add_commit(path, "Initial commit");

        // Add remote and push
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&remote_path)
            .current_dir(path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(path)
            .output()
            .unwrap();

        // Create unpushed commit
        std::fs::write(path.join("unpushed.txt"), "This is not pushed\n").unwrap();
        Self::git_add_commit(path, "Add unpushed changes");
    }

    fn create_missing_head_repo(path: &Path) {
        // Create bare repo (has no HEAD by default when empty)
        Command::new("git")
            .args(["init", "--bare"])
            .arg(path)
            .output()
            .unwrap();
    }

    fn create_no_upstream_repo(path: &Path) {
        // Create local repo without remote
        Command::new("git")
            .args(["init"])
            .arg(path)
            .output()
            .unwrap();

        // Configure git
        Self::git_config(path, "user.name", "Test User");
        Self::git_config(path, "user.email", "test@example.com");

        // Create initial commit
        std::fs::write(path.join("README.md"), "# No Upstream\n").unwrap();
        Self::git_add_commit(path, "Initial commit");

        // Note: No remote configured, so no upstream
    }

    fn create_behind_repo(path: &Path) {
        // Create bare remote
        let remote_path = path.parent().unwrap().join("behind-repo-remote.git");
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_path)
            .output()
            .unwrap();

        // Create local repo
        Command::new("git")
            .args(["init"])
            .arg(path)
            .output()
            .unwrap();

        // Configure git
        Self::git_config(path, "user.name", "Test User");
        Self::git_config(path, "user.email", "test@example.com");

        // Create initial commit
        std::fs::write(path.join("README.md"), "# Behind Repo\n").unwrap();
        Self::git_add_commit(path, "Initial commit");

        // Add remote and push
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&remote_path)
            .current_dir(path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(path)
            .output()
            .unwrap();

        // Create a commit directly in remote (simulating someone else pushing)
        let temp_clone = path.parent().unwrap().join("behind-repo-temp-clone");
        Command::new("git")
            .args(["clone"])
            .arg(&remote_path)
            .arg(&temp_clone)
            .output()
            .unwrap();

        Self::git_config(&temp_clone, "user.name", "Test User");
        Self::git_config(&temp_clone, "user.email", "test@example.com");
        std::fs::write(temp_clone.join("new.txt"), "New file\n").unwrap();
        Self::git_add_commit(&temp_clone, "Remote commit");

        Command::new("git")
            .args(["push"])
            .current_dir(&temp_clone)
            .output()
            .unwrap();

        // Now local repo is behind
        Command::new("git")
            .args(["fetch"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_config(path: &Path, key: &str, value: &str) {
        Command::new("git")
            .args(["config", key, value])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_add_commit(path: &Path, message: &str) {
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .unwrap();
    }
}
