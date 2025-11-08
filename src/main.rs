use anyhow::{Context, Result};
use clap::Parser;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use nasty_boii::{check_repo_status, RepoStatus};
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

    /// List repos with missing HEAD (default log level becomes error)
    #[arg(long)]
    missing_head: bool,

    /// Path to file containing exclude patterns (gitignore-style, one per line)
    #[arg(long)]
    exclude_from: Option<PathBuf>,
}

/// Load gitignore patterns from the exclude file if provided.
fn load_gitignore(exclude_file: Option<&PathBuf>, base_path: &Path) -> Result<Option<Gitignore>> {
    if let Some(exclude_file) = exclude_file {
        let mut builder = GitignoreBuilder::new(base_path);
        if let Some(err) = builder.add(exclude_file) {
            return Err(err).context(format!(
                "Failed to read exclude file: {}",
                exclude_file.display()
            ));
        }
        Ok(Some(
            builder
                .build()
                .context("Failed to build gitignore matcher")?,
        ))
    } else {
        Ok(None)
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set up tracing
    let default_log_level = if args.missing_head {
        "error"
    } else {
        &args.log_level
    };
    let log_level = if args.verbose {
        "info"
    } else {
        default_log_level
    };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    fmt().with_env_filter(env_filter).with_target(false).init();

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

    // Load exclude patterns if provided
    let gitignore = load_gitignore(args.exclude_from.as_ref(), &args.path)?;

    // Find git repositories and check them in parallel
    let missing_head_mode = args.missing_head;
    WalkDir::new(&args.path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Always allow the root directory (depth 0)
            if e.depth() == 0 {
                return true;
            }

            // Check gitignore patterns if configured
            if let Some(ref gi) = gitignore {
                // Use matched_path_or_any_parents to check if this path or any parent is ignored
                let is_dir = e.file_type().is_dir();
                match gi.matched_path_or_any_parents(e.path(), is_dir) {
                    ignore::Match::Ignore(_) => {
                        debug!(
                            path = %e.path().display(),
                            "Excluding path based on pattern"
                        );
                        return false;
                    }
                    ignore::Match::None | ignore::Match::Whitelist(_) => {
                        // Continue with other filters
                    }
                }
            }

            // Skip hidden directories except .git
            let name = e.file_name().to_string_lossy();
            if name == ".git" {
                return true;
            }

            !name.starts_with('.')
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_dir() && e.file_name() == ".git")
        .filter_map(|e| e.path().parent().map(std::path::Path::to_path_buf))
        .par_bridge()
        .for_each(|repo_path| {
            info!(repo_path = %repo_path.display(), "Found repository");

            match check_repo_status(&repo_path) {
                Ok(RepoStatus::HasUnpushed) => {
                    if !missing_head_mode {
                        println!("{}", repo_path.display());
                    }
                }
                Ok(RepoStatus::MissingHead) => {
                    if missing_head_mode {
                        println!("{}", repo_path.display());
                    } else {
                        warn!(
                            repo_path = %repo_path.display(),
                            "Repository has no HEAD"
                        );
                    }
                }
                Ok(RepoStatus::Clean) => {
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
