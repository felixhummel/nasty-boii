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

    /// Enable verbose output (equivalent to --log-level info)
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set up tracing
    let log_level = if args.verbose { "info" } else { &args.log_level };
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

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

    // Find git repositories and check them in parallel
    let all_entries: Vec<_> = WalkDir::new(&args.path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories except .git
            let name = e.file_name().to_string_lossy();
            if name == ".git" {
                return true;
            }
            // Don't filter based on name if it's ".", "..", or starts with "./" or "../" (root directory)
            if name == "." || name == ".." || name.starts_with("./") || name.starts_with("../") {
                return true;
            }
            !name.starts_with('.')
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.file_name() == ".git")
        .filter_map(|e| e.path().parent().map(|p| p.to_path_buf()))
        .collect();

    // Process repositories in parallel
    all_entries.par_iter().for_each(|repo_path| {
        info!(repo_path = %repo_path.display(), "Found repository");

        match has_unpushed_changes(repo_path) {
            Ok(true) => {
                println!("{}", repo_path.display());
            }
            Ok(false) => {
                debug!(
                    repo_path = %repo_path.display(),
                    "Repository is clean"
                );
            }
            Err(e) => {
                warn!(
                    repo_path = %repo_path.display(),
                    error = %e,
                    "Failed to check repository"
                );
            }
        }
    });

    Ok(())
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
