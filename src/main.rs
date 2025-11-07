use anyhow::{Context, Result};
use clap::Parser;
use git2::{BranchType, Repository};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "nasty-boii")]
#[command(about = "Finds git repos that have changes that are not yet pushed", long_about = None)]
struct Args {
    /// Directory to search (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Number of threads to use (defaults to number of CPU cores)
    #[arg(short, long)]
    threads: Option<usize>,

    /// Log level (off, error, warn, info, debug, trace)
    #[arg(short = 'l', long, default_value = "warn")]
    log_level: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set up tracing
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    info!(
        search_path = %args.path.display(),
        threads = ?args.threads,
        "Starting repository scan"
    );

    // Set up thread pool
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .context("Failed to set thread pool size")?;
        debug!(thread_count = threads, "Configured thread pool");
    }

    // Find all git repositories
    let git_repos = find_git_repos(&args.path)?;

    info!(repo_count = git_repos.len(), "Found git repositories");

    for repo in &git_repos {
        debug!(repo_path = %repo.display(), "Found repository");
    }

    // Check each repository in parallel for unpushed changes
    let nasty_repos: Vec<PathBuf> = git_repos
        .par_iter()
        .filter_map(|repo_path| {
            match has_unpushed_changes(repo_path) {
                Ok(true) => {
                    info!(
                        repo_path = %repo_path.display(),
                        "Repository has unpushed changes"
                    );
                    Some(repo_path.clone())
                }
                Ok(false) => {
                    debug!(
                        repo_path = %repo_path.display(),
                        "Repository is clean"
                    );
                    None
                }
                Err(e) => {
                    warn!(
                        repo_path = %repo_path.display(),
                        error = %e,
                        "Failed to check repository"
                    );
                    None
                }
            }
        })
        .collect();

    info!(
        nasty_count = nasty_repos.len(),
        total_count = git_repos.len(),
        "Scan complete"
    );

    // Print results
    for repo in nasty_repos {
        println!("{}", repo.display());
    }

    Ok(())
}

fn find_git_repos(root: &Path) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories except .git
            let name = e.file_name().to_string_lossy();
            if name == ".git" {
                return true;
            }
            !name.starts_with('.')
        })
    {
        let entry = entry.context("Failed to read directory entry")?;

        if entry.file_type().is_dir() && entry.file_name() == ".git" {
            if let Some(parent) = entry.path().parent() {
                repos.push(parent.to_path_buf());
            }
        }
    }

    Ok(repos)
}

fn has_unpushed_changes(repo_path: &Path) -> Result<bool> {
    let repo = Repository::open(repo_path)
        .context(format!("Failed to open repository at {:?}", repo_path))?;

    // Get the current branch
    let head = repo.head().context("Failed to get HEAD")?;

    if !head.is_branch() {
        // Not on a branch (detached HEAD), skip
        return Ok(false);
    }

    let branch_name = head
        .shorthand()
        .context("Failed to get branch name")?;

    let branch = repo
        .find_branch(branch_name, BranchType::Local)
        .context("Failed to find local branch")?;

    // Get the upstream branch
    let upstream = match branch.upstream() {
        Ok(upstream) => upstream,
        Err(_) => {
            // No upstream branch configured, consider it as having unpushed changes
            // if there are any commits
            return Ok(true);
        }
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
        return Ok(false);
    }

    // Check if local is ahead of remote
    let (ahead, _behind) = repo
        .graph_ahead_behind(local_oid, remote_oid)
        .context("Failed to calculate ahead/behind")?;

    Ok(ahead > 0)
}
