// xpatch - Git Repository Extractor with Space Limit
// Pre-extracts all file versions for fast benchmark access

use anyhow::{Context, Result};
use clap::Parser;
use git2::Repository;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize)]
struct FileVersion {
    repo_name: String,
    file_path: String,
    commit_hash: String,
    commit_date: String,
    commit_message: String,
    size_bytes: usize,
}

#[derive(Parser, Debug)]
#[command(name = "git-extract")]
#[command(about = "Pre-extract git file versions for fast benchmarking")]
struct Args {
    /// Repository names to extract (git, linux, rust, neovim, tokio)
    #[arg(short, long, value_delimiter = ',', default_value = "git")]
    repos: Vec<String>,

    /// Maximum commits to extract per file (0 = unlimited)
    #[arg(short, long, default_value = "100")]
    max_commits: usize,

    /// Output directory for extracted files
    #[arg(short, long, default_value = "./benchmark_cache")]
    output: PathBuf,

    /// Maximum space to use in GB (0 = unlimited)
    #[arg(long, default_value = "0")]
    max_space: usize,

    /// Number of parallel threads (0 = auto-detect)
    #[arg(short, long, default_value = "0")]
    threads: usize,

    /// Discover ALL files from current HEAD
    #[arg(long, default_value = "false")]
    all_files_head: bool,

    /// Discover ALL files from entire git history (VERY SLOW)
    #[arg(long, default_value = "false")]
    all_files: bool,

    /// Maximum files to extract per repository (0 = use predefined list)
    #[arg(long, default_value = "0")]
    max_files: usize,
}

const REPOSITORIES: &[(&str, &str, &[&str])] = &[
    (
        "git",
        "https://github.com/git/git.git",
        &["builtin/add.c", "diff.c", "revision.c", "Makefile"],
    ),
    (
        "linux",
        "https://github.com/torvalds/linux.git",
        &["kernel/sched/core.c", "fs/ext4/inode.c", "Makefile"],
    ),
    (
        "rust",
        "https://github.com/rust-lang/rust.git",
        &["compiler/rustc_driver/src/lib.rs", "library/std/src/lib.rs"],
    ),
    (
        "neovim",
        "https://github.com/neovim/neovim.git",
        &["src/nvim/main.c", "runtime/lua/vim/_editor.lua"],
    ),
    (
        "tokio",
        "https://github.com/tokio-rs/tokio.git",
        &["tokio/src/runtime/mod.rs", "tokio/src/net/tcp/stream.rs"],
    ),
];

struct Extractor {
    args: Args,
    cache_dir: PathBuf,
    repos_dir: PathBuf,
    manifest_path: PathBuf,
    mp: MultiProgress,
    total_extracted: Arc<Mutex<usize>>,
    total_bytes: Arc<Mutex<usize>>,
    max_space_bytes: usize,
    space_limit_reached: Arc<Mutex<bool>>,
}

impl Extractor {
    fn new(args: Args) -> Result<Self> {
        let cache_dir = args.output.clone();
        let repos_dir = cache_dir.join("repos");
        let manifest_path = cache_dir.join("manifest.json");
        let max_space_bytes = args.max_space * 1024 * 1024 * 1024; // Convert GB to bytes

        fs::create_dir_all(&cache_dir)?;
        fs::create_dir_all(&repos_dir)?;

        let threads = if args.threads == 0 {
            num_cpus::get()
        } else {
            args.threads
        };
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok();

        let mp = MultiProgress::new();
        mp.set_move_cursor(true);

        log::info!("🚀 Git Repository Extractor");
        log::info!("Cache directory: {}", cache_dir.display());
        log::info!("Threads: {}", threads);

        if max_space_bytes > 0 {
            log::info!("Max space limit: {} GB", args.max_space);
        } else {
            log::info!("Max space limit: UNLIMITED");
        }

        Ok(Self {
            args,
            cache_dir,
            repos_dir,
            manifest_path,
            mp,
            total_extracted: Arc::new(Mutex::new(0)),
            total_bytes: Arc::new(Mutex::new(0)),
            max_space_bytes,
            space_limit_reached: Arc::new(Mutex::new(false)),
        })
    }

    fn run(&self) -> Result<()> {
        let selected: Vec<&'static str> = REPOSITORIES
            .iter()
            .filter(|(name, _, _)| self.args.repos.contains(&name.to_string()))
            .map(|(name, _, _)| *name)
            .collect();

        if selected.is_empty() {
            anyhow::bail!(
                "No valid repositories selected. Choose from: {}",
                REPOSITORIES
                    .iter()
                    .map(|(name, _, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        // Create space usage progress bar if limit is set
        let space_pb = if self.max_space_bytes > 0 {
            let pb = self.mp.add(ProgressBar::new(self.max_space_bytes as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("🗄️  Space: {bar:40.green/yellow} {bytes}/{total_bytes} | {bytes_per_sec} | ETA: {eta}")
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb.set_message("Disk usage");
            Some(pb)
        } else {
            None
        };

        let mut all_versions = Vec::new();

        for repo_name in selected {
            println!("\n{}", "═".repeat(80));
            log::info!("📦 Processing repository: {}", repo_name);

            let repo_path = self.repos_dir.join(repo_name);
            let repo = self.clone_or_open_repo(repo_name, &repo_path)?;

            let files = self.discover_files(&repo, repo_name)?;
            log::info!("   Found {} files to extract", files.len());

            let repo_pb = self.mp.add(ProgressBar::new(files.len() as u64));
            repo_pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "  [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} files | ETA: {eta}",
                    )
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            repo_pb.enable_steady_tick(std::time::Duration::from_millis(100));

            // Create thread-safe manifest container
            let manifest = Arc::new(Mutex::new(Vec::new()));
            let space_limit_reached = Arc::clone(&self.space_limit_reached);
            let repo_path_clone = repo_path.clone();
            let repo_name_clone = repo_name.to_string();
            let repo_pb_clone = repo_pb.clone();
            let space_pb_clone = space_pb.clone();

            // Process files in parallel
            files.par_iter().for_each(|file_path| {
                // Early exit if space limit already reached
                if *space_limit_reached.lock().unwrap() {
                    return;
                }

                // Open repository handle for this thread
                let thread_repo = match Repository::open(&repo_path_clone) {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!("Failed to open repo for {}: {}", file_path, e);
                        repo_pb_clone.inc(1);
                        return;
                    }
                };

                // Extract versions for this file
                match self.extract_file_versions(&thread_repo, &repo_name_clone, file_path) {
                    Ok(versions) => {
                        if !versions.is_empty() {
                            let total_size: usize = versions.iter().map(|v| v.size_bytes).sum();

                            // Check space limit before adding these versions
                            let mut current_bytes = self.total_bytes.lock().unwrap();
                            if self.max_space_bytes > 0
                                && *current_bytes + total_size > self.max_space_bytes
                            {
                                *space_limit_reached.lock().unwrap() = true;
                                drop(current_bytes); // Release lock
                                log::warn!("   Space limit reached while processing {}", file_path);
                                return;
                            }

                            // Add to global manifest
                            let mut manifest_guard = manifest.lock().unwrap();
                            manifest_guard.extend(versions);

                            // Update counters
                            *self.total_extracted.lock().unwrap() += manifest_guard.len();
                            *current_bytes += total_size;
                        }
                    }
                    Err(e) => {
                        log::debug!("Failed to extract {}: {}", file_path, e);
                    }
                }

                // Update progress
                repo_pb_clone.inc(1);

                // Update space progress bar
                if let Some(pb) = &space_pb_clone {
                    let current_bytes = *self.total_bytes.lock().unwrap();
                    pb.set_position(current_bytes as u64);
                }
            });

            // Merge parallel results into all_versions
            all_versions.extend(manifest.lock().unwrap().drain(..));

            repo_pb.finish_with_message(format!("✅ {}", repo_name));

            // Stop if space limit reached
            if *self.space_limit_reached.lock().unwrap() {
                break;
            }
        }

        // Final space update
        if let Some(pb) = &space_pb {
            pb.finish_with_message(format!(
                "💾 {:.2} GB used",
                *self.total_bytes.lock().unwrap() as f64 / 1024.0 / 1024.0 / 1024.0
            ));
        }

        // Save manifest
        self.save_manifest(&all_versions)?;

        // Print summary
        let total_files = all_versions.len();
        let total_bytes = *self.total_bytes.lock().unwrap();
        let total_versions = all_versions.iter().map(|v| v.size_bytes).count();
        let limit_reached = *self.space_limit_reached.lock().unwrap();

        println!("\n{}", "═".repeat(80));
        println!("📊 EXTRACTION COMPLETE");
        if limit_reached {
            println!("⚠️  SPACE LIMIT REACHED");
        }
        println!("{}", "═".repeat(80));
        println!("Total files extracted: {}", total_files);
        println!("Total versions: {}", total_versions);
        println!(
            "Total disk space: {:.2} MB",
            total_bytes as f64 / 1024.0 / 1024.0
        );
        if self.max_space_bytes > 0 {
            let used_pct = (total_bytes as f64 / self.max_space_bytes as f64) * 100.0;
            println!(
                "Space limit: {} GB ({:.1}% used)",
                self.args.max_space,
                used_pct.min(100.0)
            );
        }
        println!("Cache location: {}", self.cache_dir.display());

        Ok(())
    }

    fn clone_or_open_repo(&self, repo_name: &str, repo_path: &Path) -> Result<Repository> {
        if repo_path.join(".git").exists() {
            log::info!("   Using existing repository");
            return Repository::open(repo_path)
                .with_context(|| format!("Failed to open repo at {}", repo_path.display()));
        }

        let repo_url = REPOSITORIES
            .iter()
            .find(|(name, _, _)| *name == repo_name)
            .map(|(_, url, _)| *url)
            .unwrap();

        log::info!("   Cloning {}...", repo_url);
        let pb = self.mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Cloning repository...");

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.download_tags(git2::AutotagOption::All);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder
            .clone(repo_url, repo_path)
            .with_context(|| format!("Failed to clone {}", repo_url))?;

        pb.finish_with_message("✓ Clone complete");
        Ok(repo)
    }

    fn discover_files(&self, repo: &Repository, repo_name: &str) -> Result<Vec<String>> {
        let files = match (
            self.args.all_files,
            self.args.all_files_head,
            self.args.max_files,
        ) {
            (true, _, _) => self.get_all_files_in_history(repo)?,
            (false, true, _) => self.get_all_files_at_head(repo)?,
            (false, false, 0) => REPOSITORIES
                .iter()
                .find(|(name, _, _)| *name == repo_name)
                .map(|(_, _, files)| files.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            (false, false, n) => {
                let mut files = self.get_all_files_at_head(repo)?;
                files.truncate(n);
                files
            }
        };

        Ok(files)
    }

    fn extract_file_versions(
        &self,
        repo: &Repository,
        repo_name: &str,
        file_path: &str,
    ) -> Result<Vec<FileVersion>> {
        // Check space limit before starting this file
        if *self.space_limit_reached.lock().unwrap() {
            anyhow::bail!("Space limit already reached");
        }

        let max_commits = if self.args.max_commits > 0 {
            self.args.max_commits
        } else {
            usize::MAX
        };

        let commits = self.get_commit_history(repo, file_path, max_commits)?;
        if commits.is_empty() {
            anyhow::bail!("No commits found");
        }

        // Create directory for this file
        let safe_path = file_path.replace('/', "___");
        let file_cache_dir = self
            .cache_dir
            .join("files")
            .join(repo_name)
            .join(&safe_path);
        fs::create_dir_all(&file_cache_dir)?;

        let mut versions = Vec::new();

        let mut count = 0;
        for (idx, commit) in commits.iter().enumerate() {
            // Check space limit before each write
            if *self.space_limit_reached.lock().unwrap() {
                break;
            }

            let content = self.get_file_at_commit(repo, &commit.hash, file_path)?;

            // Check if this write would exceed space limit
            let current_bytes = *self.total_bytes.lock().unwrap();
            if self.max_space_bytes > 0 && current_bytes + content.len() > self.max_space_bytes {
                *self.space_limit_reached.lock().unwrap() = true;
                log::warn!("   Writing {} would exceed space limit", file_path);
                break;
            }

            // Save to file: <cache>/files/<repo>/<safe_path>/<index>_<hash>.bin
            let version_filename = format!("{:04}_{}.bin", idx, &commit.hash[..8]);
            let version_path = file_cache_dir.join(&version_filename);

            let mut file = File::create(&version_path)?;
            file.write_all(&content)?;

            // Update counters
            *self.total_extracted.lock().unwrap() += 1;
            *self.total_bytes.lock().unwrap() += content.len();
            count += 1;

            // Add to manifest
            versions.push(FileVersion {
                repo_name: repo_name.to_string(),
                file_path: file_path.to_string(),
                commit_hash: commit.hash.clone(),
                commit_date: commit.date.clone(),
                commit_message: commit.message.clone(),
                size_bytes: content.len(),
            });
        }

        Ok(versions)
    }

    fn get_commit_history(
        &self,
        repo: &Repository,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>> {
        let mut revwalk = repo.revwalk()?;

        if revwalk.push_head().is_err() {
            for branch_name in &["main", "master", "develop"] {
                if let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Local) {
                    if let Some(target) = branch.get().target() {
                        revwalk.push(target)?;
                        break;
                    }
                }
            }
        }

        let mut commits = Vec::new();
        let mut last_blob_id: Option<git2::Oid> = None;

        for oid in revwalk {
            if limit > 0 && commits.len() >= limit {
                break;
            }

            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            if let Ok(tree) = commit.tree()
                && let Ok(entry) = tree.get_path(Path::new(file_path))
            {
                let blob_id = entry.id();

                if last_blob_id != Some(blob_id) {
                    commits.push(CommitInfo {
                        hash: commit.id().to_string(),
                        date: commit.time().seconds().to_string(),
                        message: commit.summary().unwrap_or("").to_string(),
                        index: commits.len(),
                    });
                    last_blob_id = Some(blob_id);
                }
            }
        }

        Ok(commits)
    }

    fn get_file_at_commit(
        &self,
        repo: &Repository,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<Vec<u8>> {
        let oid = git2::Oid::from_str(commit_hash)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(Path::new(file_path))?;
        let object = entry.to_object(repo)?;
        let blob = object
            .as_blob()
            .ok_or_else(|| anyhow::anyhow!("Not a blob"))?;
        Ok(blob.content().to_vec())
    }

    fn get_all_files_at_head(&self, repo: &Repository) -> Result<Vec<String>> {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let tree = commit.tree()?;

        let mut files = Vec::new();
        self.walk_tree(repo, &tree, Path::new(""), &mut files)?;
        files.sort();
        Ok(files)
    }

    fn get_all_files_in_history(&self, repo: &Repository) -> Result<Vec<String>> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut all_files = std::collections::HashSet::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            let tree = commit.tree()?;
            self.collect_files_from_tree(repo, &tree, Path::new(""), &mut all_files)?;
        }

        let mut files: Vec<String> = all_files.into_iter().collect();
        files.sort();
        Ok(files)
    }

    fn walk_tree(
        &self,
        repo: &Repository,
        tree: &git2::Tree,
        base_path: &Path,
        files: &mut Vec<String>,
    ) -> Result<()> {
        for entry in tree {
            let name = entry.name().unwrap_or("");
            let entry_path = base_path.join(name);

            match entry.kind() {
                Some(git2::ObjectType::Blob) => {
                    files.push(entry_path.to_string_lossy().to_string());
                }
                Some(git2::ObjectType::Tree) => {
                    if let Ok(subtree) = repo.find_tree(entry.id()) {
                        self.walk_tree(repo, &subtree, &entry_path, files)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_files_from_tree(
        &self,
        repo: &Repository,
        tree: &git2::Tree,
        base_path: &Path,
        files: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        for entry in tree {
            let name = entry.name().unwrap_or("");
            let entry_path = base_path.join(name);

            match entry.kind() {
                Some(git2::ObjectType::Blob) => {
                    files.insert(entry_path.to_string_lossy().to_string());
                }
                Some(git2::ObjectType::Tree) => {
                    if let Ok(subtree) = repo.find_tree(entry.id()) {
                        self.collect_files_from_tree(repo, &subtree, &entry_path, files)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn save_manifest(&self, manifest: &[FileVersion]) -> Result<()> {
        let json = serde_json::to_string_pretty(manifest)?;
        let mut file = File::create(&self.manifest_path)?;
        file.write_all(json.as_bytes())?;
        log::info!("💾 Manifest saved to: {}", self.manifest_path.display());
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct CommitInfo {
    hash: String,
    date: String,
    message: String,
    index: usize,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let extractor = Extractor::new(args)?;
    extractor.run()?;

    println!("\n✅ Extraction complete!");
    println!(
        "Next step: Run git_benchmark with --cache-dir {}",
        extractor.cache_dir.display()
    );

    Ok(())
}
