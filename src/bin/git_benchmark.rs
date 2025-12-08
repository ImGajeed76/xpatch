// xpatch - High-performance delta compression library
// Copyright (c) 2025 Oliver Seifert
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// Commercial License Option:
// For commercial use in proprietary software, a commercial license is
// available. Contact xpatch-commercial@alias.oseifert.ch for details.

//! # Delta Compression Real-World Benchmark (Enhanced)
//!
//! Features:
//! - Parallel benchmark execution for speed
//! - Incremental result storage (no data loss on Ctrl+C)
//! - Graceful shutdown handling
//! - Timestamped output files
//!
//! Usage:
//!   cargo run --bin git_benchmark --features benchmark -- [--repos <names>] [--max-commits <n>] [--output <dir>] [--threads <n>]

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use git2::Repository;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkResult {
    repo_name: String,
    file_path: String,
    commit_from: String,
    commit_to: String,
    commit_distance: usize,
    file_size_from: usize,
    file_size_to: usize,

    xpatch_delta_size: usize,
    xpatch_ratio: f64,
    xpatch_encode_us: u128,
    xpatch_decode_us: u128,

    xdelta3_delta_size: usize,
    xdelta3_ratio: f64,
    xdelta3_encode_us: u128,
    xdelta3_decode_us: u128,

    qbsdiff_delta_size: usize,
    qbsdiff_ratio: f64,
    qbsdiff_encode_us: u128,
    qbsdiff_decode_us: u128,
}

#[derive(Debug)]
struct RepoConfig {
    name: &'static str,
    url: &'static str,
    files: &'static [&'static str],
    description: &'static str,
}

const REPOSITORIES: &[RepoConfig] = &[
    RepoConfig {
        name: "git",
        url: "https://github.com/git/git.git",
        files: &["builtin/add.c", "diff.c", "revision.c", "Makefile"],
        description: "Git version control system (~300MB, 50k commits)",
    },
    RepoConfig {
        name: "linux",
        url: "https://github.com/torvalds/linux.git",
        files: &["kernel/sched/core.c", "fs/ext4/inode.c", "Makefile"],
        description: "Linux kernel (~4GB, 1M+ commits) - SLOW!",
    },
    RepoConfig {
        name: "rust",
        url: "https://github.com/rust-lang/rust.git",
        files: &["compiler/rustc_driver/src/lib.rs", "library/std/src/lib.rs"],
        description: "Rust compiler (~800MB, 150k commits)",
    },
    RepoConfig {
        name: "neovim",
        url: "https://github.com/neovim/neovim.git",
        files: &["src/nvim/main.c", "runtime/lua/vim/_editor.lua"],
        description: "Neovim editor (~200MB, 40k commits)",
    },
    RepoConfig {
        name: "tokio",
        url: "https://github.com/tokio-rs/tokio.git",
        files: &["tokio/src/runtime/mod.rs", "tokio/src/net/tcp/stream.rs"],
        description: "Tokio async runtime (~100MB, 10k commits)",
    },
];

#[derive(Parser, Debug)]
#[command(name = "delta-benchmark")]
#[command(about = "Real-world delta compression benchmark", long_about = None)]
struct Args {
    /// Repository names to benchmark (git, linux, rust, neovim, tokio)
    #[arg(short, long, value_delimiter = ',', default_value = "git,neovim,tokio")]
    repos: Vec<String>,

    /// Maximum commits to analyze per file (0 = unlimited)
    #[arg(short, long, default_value = "100")]
    max_commits: usize,

    /// Output directory for results
    #[arg(short, long, default_value = "./benchmark_results")]
    output: PathBuf,

    /// Skip missing libraries (xdelta3) instead of failing
    #[arg(long, default_value = "false")]
    skip_missing_libs: bool,

    /// Number of parallel threads (0 = auto-detect)
    #[arg(short, long, default_value = "0")]
    threads: usize,

    /// Maximum files to test per repository (0 = use predefined list)
    #[arg(long, default_value = "0")]
    max_files: usize,

    /// Discover ALL files from current HEAD (unlimited)
    #[arg(long, default_value = "false")]
    all_files_head: bool,

    /// Discover ALL files from entire git history (unlimited, VERY SLOW)
    #[arg(long, default_value = "false")]
    all_files: bool,

    /// Process multiple files in parallel (may increase memory usage)
    #[arg(long, default_value = "false")]
    parallel_files: bool,
}

struct BenchmarkRunner {
    args: Args,
    output_dir: PathBuf,
    repos_dir: PathBuf,
    csv_path: PathBuf,
    csv_writer: Arc<Mutex<csv::Writer<std::fs::File>>>,
    mp: MultiProgress,
    #[allow(dead_code)]
    start_time: String,
    benchmark_counter: Arc<Mutex<usize>>,
}

impl BenchmarkRunner {
    fn new(args: Args) -> Result<Self> {
        let start_time = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let output_dir = args.output.clone();
        let repos_dir = output_dir.join("repos");

        fs::create_dir_all(&output_dir)
            .with_context(|| format!("Failed to create output dir: {}", output_dir.display()))?;
        fs::create_dir_all(&repos_dir)
            .with_context(|| format!("Failed to create repos dir: {}", repos_dir.display()))?;

        // Create timestamped CSV file
        let csv_path = output_dir.join(format!("results_{}.csv", start_time));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&csv_path)?;

        let mut writer = csv::Writer::from_writer(file);

        // Write headers
        writer.write_record([
            "repo_name",
            "file_path",
            "commit_from",
            "commit_to",
            "distance",
            "size_from",
            "size_to",
            "xpatch_delta",
            "xpatch_ratio",
            "xpatch_encode_us",
            "xpatch_decode_us",
            "xdelta3_delta",
            "xdelta3_ratio",
            "xdelta3_encode_us",
            "xdelta3_decode_us",
            "qbsdiff_delta",
            "qbsdiff_ratio",
            "qbsdiff_encode_us",
            "qbsdiff_decode_us",
        ])?;
        writer.flush()?;

        // Set up thread pool
        let threads = if args.threads == 0 {
            num_cpus::get()
        } else {
            args.threads
        };
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok(); // Ignore error if already initialized

        Ok(Self {
            args,
            output_dir,
            repos_dir,
            csv_path: csv_path.clone(),
            csv_writer: Arc::new(Mutex::new(writer)),
            mp: MultiProgress::new(),
            start_time,
            benchmark_counter: Arc::new(Mutex::new(0)),
        })
    }

    fn run(&self) -> Result<()> {
        log::info!("🚀 Starting Delta Compression Benchmark");
        log::info!("Output directory: {}", self.output_dir.display());
        log::info!("Results file: {}", self.csv_path.display());
        log::info!("Threads: {}", rayon::current_num_threads());

        // Set up Ctrl+C handler
        let csv_path = self.csv_path.clone();
        ctrlc::set_handler(move || {
            println!("\n\n⚠️  Ctrl+C received! Shutting down gracefully...");
            println!("💾 Results saved to: {}", csv_path.display());
            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");

        self.check_dependencies()?;

        let repos_to_test = self.get_selected_repos()?;

        let mut total_files = 0;
        let mut repo_files_map = HashMap::new();

        for repo_config in &repos_to_test {
            let repo_path = self.repos_dir.join(repo_config.name);
            let repo = self.ensure_repo_cloned(repo_config, &repo_path)?;

            let files = match (
                self.args.all_files,
                self.args.all_files_head,
                self.args.max_files,
            ) {
                (true, _, _) => {
                    // --all-files: discover from history, unlimited
                    self.get_all_files_in_history(&repo)?
                }
                (false, true, _) => {
                    // --all-files-head: discover from HEAD, unlimited
                    self.get_all_files_at_head(&repo)?
                }
                (false, false, 0) => {
                    // Default: use predefined list
                    repo_config.files.iter().map(|s| s.to_string()).collect()
                }
                (false, false, n) => {
                    // --max-files N: discover from HEAD, limited
                    self.get_all_files_at_head(&repo)?
                        .into_iter()
                        .take(n)
                        .collect()
                }
            };

            total_files += files.len();
            repo_files_map.insert(repo_config.name, files);
        }

        let master_pb = self.mp.add(ProgressBar::new(total_files as u64));
        master_pb.set_style(
            ProgressStyle::default_bar()
                .template("\n╔═ Overall Progress ═╗ {bar:50.cyan/blue} {pos}/{len} files | {elapsed_precise} elapsed | ETA: {eta}\n")
                .unwrap()
                .progress_chars("█▓░"),
        );
        master_pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let total_benchmarks = Arc::new(Mutex::new(0usize));
        let mut processed_files = 0;

        for repo_config in repos_to_test {
            println!("\n{}", "═".repeat(80));
            log::info!("📦 Processing repository: {}", repo_config.name);
            log::info!("   {}", repo_config.description);

            // Get pre-discovered files
            let files_to_benchmark = repo_files_map.get(repo_config.name).unwrap();

            match self.benchmark_repo(
                repo_config,
                files_to_benchmark,
                Arc::clone(&total_benchmarks),
                &master_pb,
                &mut processed_files,
            ) {
                Ok(count) => {
                    log::info!("   ✓ Completed: {} benchmarks", count);
                }
                Err(e) => {
                    log::error!("   ✗ Failed: {}", e);
                    if !self.args.skip_missing_libs {
                        return Err(e);
                    }
                }
            }
        }

        master_pb.finish_with_message("✅ All files processed");

        {
            let mut writer = self.csv_writer.lock().unwrap();
            writer.flush()?;
        }

        self.print_final_summary()?;

        log::info!("\n✅ Benchmark complete!");
        log::info!("💾 Results saved to: {}", self.csv_path.display());
        Ok(())
    }

    fn check_dependencies(&self) -> Result<()> {
        log::info!("🔍 Checking dependencies...");

        if std::process::Command::new("xdelta3")
            .arg("--version")
            .output()
            .is_err()
        {
            if self.args.skip_missing_libs {
                log::warn!("⚠️  xdelta3 command not found, will skip xdelta3 benchmarks");
            } else {
                anyhow::bail!("xdelta3 not found. Install it or use --skip-missing-libs");
            }
        }

        Ok(())
    }

    fn get_selected_repos(&self) -> Result<Vec<&RepoConfig>> {
        let selected: Vec<&RepoConfig> = REPOSITORIES
            .iter()
            .filter(|r| self.args.repos.contains(&r.name.to_string()))
            .collect();

        if selected.is_empty() {
            anyhow::bail!(
                "No valid repositories selected. Choose from: {}",
                REPOSITORIES
                    .iter()
                    .map(|r| r.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Ok(selected)
    }

    fn benchmark_file_sequential(
        &self,
        repo: &Repository,
        repo_name: &str,
        file_path: &str,
        local_count: Arc<Mutex<usize>>,
        csv_writer: &Arc<Mutex<csv::Writer<std::fs::File>>>,
        benchmark_counter: &Arc<Mutex<usize>>,
    ) -> Result<usize> {
        let commits = self.get_commit_history(repo, file_path)?;

        let total_commits = if self.args.max_commits > 0 {
            commits.len().min(self.args.max_commits)
        } else {
            commits.len()
        };

        if total_commits < 2 {
            anyhow::bail!(
                "Not enough commits found (need at least 2, found {})",
                commits.len()
            );
        }

        // Create progress bar for commits
        let commit_pb = self.mp.add(ProgressBar::new((total_commits - 1) as u64));
        commit_pb.set_style(
            ProgressStyle::default_bar()
                .template("    {bar:40} {pos}/{len} commits | ETA: {eta}")
                .unwrap()
                .progress_chars("=>-"),
        );
        commit_pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let mut count = 0;
        for i in 0..total_commits - 1 {
            let from = &commits[i];
            let to = &commits[i + 1];
            let distance = i + 1;

            match self.benchmark_commit_pair(repo, repo_name, file_path, from, to, distance) {
                Ok(result) => {
                    // Write result immediately
                    if let Ok(mut writer) = csv_writer.lock() {
                        let _ = writer.serialize(&result);
                        let _ = writer.flush();

                        *local_count.lock().unwrap() += 1;
                        count += 1;

                        // Update global counter
                        let total = {
                            let mut counter = benchmark_counter.lock().unwrap();
                            *counter += 1;
                            *counter
                        };

                        // Update progress bar every 10 benchmarks
                        if total % 10 == 0 {
                            commit_pb.set_message(format!("[{} total]", total));
                        }
                    }
                }
                Err(e) => {
                    log::debug!(
                        "Failed commit pair {}->{}: {}",
                        &from.hash[..8],
                        &to.hash[..8],
                        e
                    );
                }
            }
            commit_pb.inc(1);
        }

        commit_pb.finish_with_message(format!("✓ {} benchmarks", count));
        Ok(count)
    }

    fn benchmark_repo(
        &self,
        repo_config: &RepoConfig,
        files_to_benchmark: &[String],
        total_benchmarks: Arc<Mutex<usize>>,
        master_pb: &ProgressBar,
        _processed_files: &mut usize,
    ) -> Result<usize> {
        let repo_path = self.repos_dir.join(repo_config.name);
        let repo = self.ensure_repo_cloned(repo_config, &repo_path)?;

        let repo_pb = self
            .mp
            .add(ProgressBar::new(files_to_benchmark.len() as u64));
        repo_pb.set_style(
            ProgressStyle::default_bar()
                .template("  [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} files | ETA: {eta} | {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );
        repo_pb.set_message(format!("Repo: {}", repo_config.name));
        repo_pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let local_count = Arc::new(Mutex::new(0usize));

        use std::sync::atomic::{AtomicUsize, Ordering};
        let files_completed = Arc::new(AtomicUsize::new(0));

        if self.args.parallel_files {
            files_to_benchmark.par_iter().for_each(|file_path| {
                let repo_path = repo_path.clone();
                let repo_name = repo_config.name.to_string();
                let file_path = file_path.clone();
                let local_count = Arc::clone(&local_count);
                let csv_writer = Arc::clone(&self.csv_writer);
                let benchmark_counter = Arc::clone(&self.benchmark_counter);
                let mp = self.mp.clone();

                if let Ok(thread_repo) = Repository::open(&repo_path) {
                    let file_pb = mp.add(ProgressBar::new_spinner());
                    file_pb.set_style(
                        ProgressStyle::default_spinner()
                            .template("    {spinner:.green} {msg}")
                            .unwrap(),
                    );
                    file_pb.set_message(format!("📄 {}", file_path));

                    match self.benchmark_file_sequential(
                        &thread_repo,
                        &repo_name,
                        &file_path,
                        local_count,
                        &csv_writer,
                        &benchmark_counter,
                    ) {
                        Ok(count) => {
                            file_pb.finish_with_message(format!(
                                "✅ {} ({} benchmarks)",
                                file_path, count
                            ));
                        }
                        Err(e) => {
                            file_pb.finish_with_message(format!("❌ {} - {}", file_path, e));
                            log::warn!("Failed to benchmark {}: {}", file_path, e);
                        }
                    }

                    let completed = files_completed.fetch_add(1, Ordering::Relaxed);
                    if completed % 3 == 0 {
                        master_pb.inc(3);
                        repo_pb.inc(3);
                    }
                }
            });

            let final_completed = files_completed.load(Ordering::Relaxed);
            let remainder = final_completed % 3;
            if remainder > 0 {
                master_pb.inc(remainder as u64);
                repo_pb.inc(remainder as u64);
            }
        } else {
            // Sequential processing (original working code)
            for (idx, file_path) in files_to_benchmark.iter().enumerate() {
                repo_pb.set_position(idx as u64);

                let file_pb = self.mp.add(ProgressBar::new_spinner());
                file_pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("    {spinner:.green} {msg}")
                        .unwrap(),
                );
                file_pb.set_message(format!("📄 {}", file_path));

                match self.benchmark_file_parallel(
                    &repo,
                    repo_config.name,
                    file_path,
                    Arc::clone(&local_count),
                ) {
                    Ok(count) => {
                        file_pb.finish_with_message(format!(
                            "✅ {} ({} benchmarks)",
                            file_path, count
                        ));
                        master_pb.inc(1);
                    }
                    Err(e) => {
                        file_pb.finish_with_message(format!("❌ {} - {}", file_path, e));
                        log::warn!("Failed to benchmark {}: {}", file_path, e);
                        master_pb.inc(1);
                    }
                }
            }
        }

        repo_pb.finish_with_message(format!("✅ {}", repo_config.name));

        let count = *local_count.lock().unwrap();
        *total_benchmarks.lock().unwrap() += count;
        Ok(count)
    }

    fn ensure_repo_cloned(&self, repo_config: &RepoConfig, repo_path: &Path) -> Result<Repository> {
        if repo_path.join(".git").exists() {
            log::info!("   Using existing repository at {}", repo_path.display());
            return Repository::open(repo_path).with_context(|| {
                format!("Failed to open existing repo at {}", repo_path.display())
            });
        }

        log::info!("   Cloning {}...", repo_config.url);
        let pb = self.mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Cloning repository...");

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.download_tags(git2::AutotagOption::All);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder
            .clone(repo_config.url, repo_path)
            .with_context(|| format!("Failed to clone {}", repo_config.url))?;

        pb.finish_with_message("✓ Clone complete");
        Ok(repo)
    }

    fn benchmark_file_parallel(
        &self,
        repo: &Repository,
        repo_name: &str,
        file_path: &str,
        local_count: Arc<Mutex<usize>>,
    ) -> Result<usize> {
        let commits = self.get_commit_history(repo, file_path)?;

        let total_commits = if self.args.max_commits > 0 {
            commits.len().min(self.args.max_commits)
        } else {
            commits.len()
        };

        if total_commits < 2 {
            anyhow::bail!(
                "Not enough commits found (need at least 2, found {})",
                commits.len()
            );
        }

        let commit_pb = self.mp.add(ProgressBar::new((total_commits - 1) as u64));
        commit_pb.set_style(
            ProgressStyle::default_bar()
                .template("    {bar:40} {pos}/{len} commits | ETA: {eta} | {per_sec:.1}")
                .unwrap()
                .progress_chars("=>-"),
        );
        commit_pb.enable_steady_tick(std::time::Duration::from_millis(100));

        // Prepare commit pairs
        let mut pairs = Vec::new();
        for i in 0..total_commits - 1 {
            pairs.push((commits[i].clone(), commits[i + 1].clone(), i + 1));
        }

        // Process pairs in parallel
        let csv_writer: Arc<Mutex<csv::Writer<std::fs::File>>> = Arc::clone(&self.csv_writer);
        let benchmark_counter = Arc::clone(&self.benchmark_counter);
        let repo_path = repo.path().parent().unwrap().to_path_buf();
        let repo_name = repo_name.to_string();
        let file_path = file_path.to_string();
        let pb = commit_pb.clone();

        pairs.par_iter().for_each(|(from, to, distance)| {
            // Each thread opens its own repo connection
            if let Ok(thread_repo) = Repository::open(&repo_path) {
                match self.benchmark_commit_pair(
                    &thread_repo,
                    &repo_name,
                    &file_path,
                    from,
                    to,
                    *distance,
                ) {
                    Ok(result) => {
                        // Write result immediately
                        if let Ok(mut writer) = csv_writer.lock() {
                            let _ = writer.serialize(&result);
                            let _ = writer.flush();
                            *local_count.lock().unwrap() += 1;

                            // Update global counter
                            let total = {
                                let mut counter = benchmark_counter.lock().unwrap();
                                *counter += 1;
                                *counter
                            };

                            // Update progress bar with total count every 10 benchmarks
                            if total % 10 == 0 {
                                pb.set_message(format!("[{} total]", total));
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!(
                            "Failed commit pair {}->{}: {}",
                            &from.hash[..8],
                            &to.hash[..8],
                            e
                        );
                    }
                }
                pb.inc(1);
            }
        });

        let final_count = *local_count.lock().unwrap();
        commit_pb.finish_with_message(format!("✓ {} benchmarks", final_count));
        Ok(final_count)
    }

    fn get_commit_history(&self, repo: &Repository, file_path: &str) -> Result<Vec<CommitInfo>> {
        let mut revwalk = repo.revwalk()?;

        if revwalk.push_head().is_err() {
            log::debug!("HEAD not found, trying to find a branch...");
            let mut found_branch = false;

            for branch_name in &["main", "master", "develop"] {
                if let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Local)
                    && let Some(target) = branch.get().target()
                {
                    revwalk.push(target)?;
                    found_branch = true;
                    log::debug!("Using branch: {}", branch_name);
                    break;
                }
            }

            if !found_branch {
                for r in (repo.references()?).flatten() {
                    if let Some(target) = r.target() {
                        revwalk.push(target)?;
                        log::debug!("Using reference: {:?}", r.name());
                        found_branch = true;
                        break;
                    }
                }
            }

            if !found_branch {
                anyhow::bail!("Could not find any valid reference to start commit walk");
            }
        }

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            if let Ok(tree) = commit.tree()
                && tree.get_path(Path::new(file_path)).is_ok()
            {
                commits.push(CommitInfo {
                    hash: commit.id().to_string(),
                    date: commit.time().seconds().to_string(),
                    message: commit.summary().unwrap_or("").to_string(),
                });
            }
        }

        Ok(commits)
    }

    fn benchmark_commit_pair(
        &self,
        repo: &Repository,
        repo_name: &str,
        file_path: &str,
        from: &CommitInfo,
        to: &CommitInfo,
        distance: usize,
    ) -> Result<BenchmarkResult> {
        let content_from = self.get_file_at_commit(repo, &from.hash, file_path)?;
        let content_to = self.get_file_at_commit(repo, &to.hash, file_path)?;

        if content_from.is_empty() || content_to.is_empty() {
            anyhow::bail!("Empty file content");
        }

        let (xpatch_delta, xpatch_encode_us, xpatch_decode_us) =
            self.bench_xpatch(&content_from, &content_to)?;

        let (xdelta3_delta, xdelta3_encode_us, xdelta3_decode_us) = self
            .bench_xdelta3(&content_from, &content_to)
            .unwrap_or_else(|_| (Vec::new(), 0, 0));

        let (qbsdiff_delta, qbsdiff_encode_us, qbsdiff_decode_us) =
            self.bench_qbsdiff(&content_from, &content_to)?;

        Ok(BenchmarkResult {
            repo_name: repo_name.to_string(),
            file_path: file_path.to_string(),
            commit_from: from.hash[..8].to_string(),
            commit_to: to.hash[..8].to_string(),
            commit_distance: distance,
            file_size_from: content_from.len(),
            file_size_to: content_to.len(),

            xpatch_delta_size: xpatch_delta.len(),
            xpatch_ratio: xpatch_delta.len() as f64 / content_to.len() as f64,
            xpatch_encode_us,
            xpatch_decode_us,

            xdelta3_delta_size: xdelta3_delta.len(),
            xdelta3_ratio: if !xdelta3_delta.is_empty() && !content_to.is_empty() {
                xdelta3_delta.len() as f64 / content_to.len() as f64
            } else {
                f64::NAN
            },
            xdelta3_encode_us,
            xdelta3_decode_us,

            qbsdiff_delta_size: qbsdiff_delta.len(),
            qbsdiff_ratio: qbsdiff_delta.len() as f64 / content_to.len() as f64,
            qbsdiff_encode_us,
            qbsdiff_decode_us,
        })
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

    fn bench_xpatch(&self, base: &[u8], target: &[u8]) -> Result<(Vec<u8>, u128, u128)> {
        let start = Instant::now();
        let delta = xpatch::delta::encode(0, base, target, true);
        let encode_time = start.elapsed().as_micros();

        let start = Instant::now();
        let _decoded = xpatch::delta::decode(base, &delta[..])
            .map_err(|e| anyhow::anyhow!("xpatch decode failed: {}", e))?;
        let decode_time = start.elapsed().as_micros();

        Ok((delta, encode_time, decode_time))
    }

    fn bench_xdelta3(&self, base: &[u8], target: &[u8]) -> Result<(Vec<u8>, u128, u128)> {
        let start = Instant::now();
        let delta = xdelta3::encode(base, target)
            .ok_or_else(|| anyhow::anyhow!("xdelta3 encode failed"))?;
        let encode_time = start.elapsed().as_micros();

        let start = Instant::now();
        let _decoded = xdelta3::decode(&delta[..], base)
            .ok_or_else(|| anyhow::anyhow!("xdelta3 decode failed"))?;
        let decode_time = start.elapsed().as_micros();

        Ok((delta, encode_time, decode_time))
    }

    fn bench_qbsdiff(&self, base: &[u8], target: &[u8]) -> Result<(Vec<u8>, u128, u128)> {
        let start = Instant::now();
        let mut patch = Vec::new();
        qbsdiff::Bsdiff::new(base, target)
            .compare(Cursor::new(&mut patch))
            .map_err(|e| anyhow::anyhow!("qbsdiff compare failed: {:?}", e))?;
        let encode_time = start.elapsed().as_micros();

        let start = Instant::now();
        let patcher = qbsdiff::Bspatch::new(&patch[..])
            .map_err(|e| anyhow::anyhow!("qbsdiff patch creation failed: {:?}", e))?;
        let mut decoded = Vec::new();
        patcher
            .apply(base, Cursor::new(&mut decoded))
            .map_err(|e| anyhow::anyhow!("qbsdiff apply failed: {:?}", e))?;
        let decode_time = start.elapsed().as_micros();

        Ok((patch, encode_time, decode_time))
    }

    fn print_final_summary(&self) -> Result<()> {
        // Read all results from CSV
        let mut reader = csv::Reader::from_path(&self.csv_path)?;
        let mut all_results: Vec<BenchmarkResult> = Vec::new();

        for r in reader.deserialize::<BenchmarkResult>().flatten() {
            all_results.push(r);
        }

        if all_results.is_empty() {
            log::warn!("No results to summarize");
            return Ok(());
        }

        println!(
            "\n\n╔═══════════════════════════════════════════════════════════════════════════╗"
        );
        println!("║                  DELTA COMPRESSION - FINAL SUMMARY                         ║");
        println!("╚═══════════════════════════════════════════════════════════════════════════╝\n");

        let mut by_repo: HashMap<String, Vec<BenchmarkResult>> = HashMap::new();
        for result in &all_results {
            by_repo
                .entry(result.repo_name.clone())
                .or_default()
                .push(result.clone());
        }

        for (repo_name, results) in by_repo {
            println!("\n📊 REPOSITORY: {}", repo_name);
            println!("{}", "─".repeat(80));

            let avg_size = results.iter().map(|r| r.file_size_to).sum::<usize>() / results.len();
            let avg_xpatch =
                results.iter().map(|r| r.xpatch_ratio).sum::<f64>() / results.len() as f64;
            let avg_xdelta3 = results
                .iter()
                .filter(|r| !r.xdelta3_ratio.is_nan())
                .map(|r| r.xdelta3_ratio)
                .sum::<f64>()
                / results
                    .iter()
                    .filter(|r| !r.xdelta3_ratio.is_nan())
                    .count()
                    .max(1) as f64;
            let avg_qbsdiff =
                results.iter().map(|r| r.qbsdiff_ratio).sum::<f64>() / results.len() as f64;

            println!("  Benchmarks run: {}", results.len());
            println!("  Average file size: {:.1} KB", avg_size as f64 / 1024.0);

            println!("\n  Average Compression Ratios:");
            println!(
                "    xpatch:  {:.4} ({:.1}% savings)",
                avg_xpatch,
                (1.0 - avg_xpatch) * 100.0
            );
            if !avg_xdelta3.is_nan() {
                println!(
                    "    xdelta3: {:.4} ({:.1}% savings)",
                    avg_xdelta3,
                    (1.0 - avg_xdelta3) * 100.0
                );
            } else {
                println!("    xdelta3: N/A (library unavailable)");
            }
            println!(
                "    qbsdiff: {:.4} ({:.1}% savings)",
                avg_qbsdiff,
                (1.0 - avg_qbsdiff) * 100.0
            );
        }

        let overall_xpatch =
            all_results.iter().map(|r| r.xpatch_ratio).sum::<f64>() / all_results.len() as f64;
        let overall_xdelta3 = all_results
            .iter()
            .filter(|r| !r.xdelta3_ratio.is_nan())
            .map(|r| r.xdelta3_ratio)
            .sum::<f64>()
            / all_results
                .iter()
                .filter(|r| !r.xdelta3_ratio.is_nan())
                .count()
                .max(1) as f64;
        let overall_qbsdiff =
            all_results.iter().map(|r| r.qbsdiff_ratio).sum::<f64>() / all_results.len() as f64;

        println!("\n\n🏆 OVERALL WINNER:");
        let winner = if overall_xpatch < overall_qbsdiff
            && (overall_xdelta3.is_nan() || overall_xpatch < overall_xdelta3)
        {
            format!("xpatch ({:.4} avg ratio)", overall_xpatch)
        } else if !overall_xdelta3.is_nan() && overall_xdelta3 < overall_qbsdiff {
            format!("xdelta3 ({:.4} avg ratio)", overall_xdelta3)
        } else {
            format!("qbsdiff ({:.4} avg ratio)", overall_qbsdiff)
        };
        println!("   {}", winner);

        Ok(())
    }

    // Get all files at HEAD (fast)
    fn get_all_files_at_head(&self, repo: &Repository) -> Result<Vec<String>> {
        let pb = self.mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Walking repository tree...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let tree = commit.tree()?;

        let mut files = Vec::new();
        self.walk_tree(repo, &tree, Path::new(""), &mut files)?;

        files.sort();
        pb.finish_with_message(format!("✓ Found {} files", files.len()));
        Ok(files)
    }

    // Get all files from history (comprehensive but slow)
    fn get_all_files_in_history(&self, repo: &Repository) -> Result<Vec<String>> {
        let pb = self.mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Walking full commit history...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut all_files = std::collections::HashSet::new();
        let mut commit_count = 0;

        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            let tree = commit.tree()?;

            self.collect_files_from_tree(repo, &tree, Path::new(""), &mut all_files)?;

            commit_count += 1;
            if commit_count % 100 == 0 {
                pb.set_message(format!(
                    "Scanned {} commits, found {} files...",
                    commit_count,
                    all_files.len()
                ));
            }
        }

        pb.finish_with_message(format!(
            "✓ Scanned {} commits, found {} unique files",
            commit_count,
            all_files.len()
        ));

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitInfo {
    hash: String,
    date: String,
    message: String,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let runner = BenchmarkRunner::new(args)?;
    runner.run()?;

    Ok(())
}
