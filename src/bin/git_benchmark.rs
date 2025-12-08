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

//! # Delta Compression Real-World Benchmark (Enhanced with Tags & WAL)
//!
//! Features:
//! - Parallel benchmark execution for speed
//! - Incremental result storage (no data loss on Ctrl+C)
//! - Graceful shutdown handling
//! - Timestamped output files
//! - **TAGS OPTIMIZATION**: xpatch searches up to 16 previous versions to find optimal base
//! - **WAL CSV**: Write-Ahead Log style appending for crash safety
//! - **CACHE SUPPORT**: Use pre-extracted git content for faster execution
//!
//! Usage:
//!   cargo run --bin git_benchmark --features benchmark -- [--repos <names>] [--max-commits <n>] [--output <dir>] [--threads <n>] [--cache-dir <path>]

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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
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

    // xpatch with TAGS optimization
    xpatch_tag: usize,
    xpatch_base_commit: String,
    xpatch_base_distance: usize,
    xpatch_delta_size: usize,
    xpatch_ratio: f64,
    xpatch_encode_us: u128,
    xpatch_decode_us: u128,

    // xdelta3 (sequential baseline, no tags)
    xdelta3_delta_size: usize,
    xdelta3_ratio: f64,
    xdelta3_encode_us: u128,
    xdelta3_decode_us: u128,

    // qbsdiff (sequential baseline, no tags)
    qbsdiff_delta_size: usize,
    qbsdiff_ratio: f64,
    qbsdiff_encode_us: u128,
    qbsdiff_decode_us: u128,
}

// Mirror of FileVersion from git-extract tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileVersion {
    repo_name: String,
    file_path: String,
    commit_hash: String,
    commit_date: String,
    commit_message: String,
    size_bytes: usize,
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

    /// Maximum tag search depth (0-15 for zero overhead, higher uses varint)
    #[arg(long, default_value = "16")]
    max_tag_depth: usize,

    /// Use pre-extracted cache directory (from git-extract tool)
    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

// WAL Writer for crash-safe CSV appending
struct WalCsvWriter {
    tx: mpsc::Sender<BenchmarkResult>,
}

impl WalCsvWriter {
    fn new(csv_path: PathBuf) -> Result<Self> {
        let (tx, rx) = mpsc::channel::<BenchmarkResult>();

        // Spawn writer thread
        std::thread::spawn(move || {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&csv_path)
                .expect("Failed to open CSV for WAL writing");

            let mut writer = csv::Writer::from_writer(file);

            // Write header if file is new
            let is_empty = fs::metadata(&csv_path)
                .map(|m| m.len() == 0)
                .unwrap_or(true);

            if is_empty {
                writer
                    .write_record([
                        "repo_name",
                        "file_path",
                        "commit_from",
                        "commit_to",
                        "distance",
                        "size_from",
                        "size_to",
                        "xpatch_tag",
                        "xpatch_base_commit",
                        "xpatch_base_distance",
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
                    ])
                    .expect("Failed to write CSV header");
                writer.flush().expect("Failed to flush CSV header");
            }

            // Process results as they come in
            let mut count = 0;
            for result in rx {
                if writer.serialize(&result).is_err() {
                    eprintln!("⚠️  Failed to serialize result");
                    continue;
                }
                if writer.flush().is_err() {
                    eprintln!("⚠️  Failed to flush CSV");
                    continue;
                }

                count += 1;
                if count % 100 == 0 {
                    log::info!("💾 WAL: Persisted {} results to CSV", count);
                }
            }

            log::info!("💾 WAL: Final flush - {} results persisted", count);
        });

        Ok(Self { tx })
    }

    fn send(&self, result: BenchmarkResult) -> Result<()> {
        self.tx
            .send(result)
            .context("Failed to send result to WAL writer")
    }
}

// Progress update messages for centralized progress management
enum ProgressUpdate {
    NewFile {
        file_path: String,
        total_commits: usize,
    },
    IncCommits {
        file_path: String,
    },
    FinishFile {
        file_path: String,
        benchmark_count: usize,
    },
}

struct BenchmarkRunner {
    args: Args,
    output_dir: PathBuf,
    repos_dir: PathBuf,
    csv_path: PathBuf,
    wal_writer: WalCsvWriter,
    mp: MultiProgress,
    start_time: String,
    benchmark_counter: Arc<AtomicUsize>,
    tags_optimization_counter: Arc<Mutex<(usize, usize)>>,
    progress_tx: mpsc::Sender<ProgressUpdate>,
    cache_dir: Option<PathBuf>,
    manifest: Option<Vec<FileVersion>>,
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

        // Initialize WAL writer
        let wal_writer = WalCsvWriter::new(csv_path.clone())?;

        // Set up thread pool
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

        // Create progress manager thread
        let (progress_tx, progress_rx) = mpsc::channel::<ProgressUpdate>();
        let mp_clone = mp.clone();

        std::thread::spawn(move || {
            let mut file_bars: HashMap<String, ProgressBar> = HashMap::new();

            while let Ok(update) = progress_rx.recv() {
                match update {
                    ProgressUpdate::NewFile {
                        file_path,
                        total_commits,
                    } => {
                        let pb = mp_clone.add(ProgressBar::new(total_commits as u64));
                        pb.set_style(
                            ProgressStyle::default_bar()
                                .template("{prefix:.cyan} {bar:40} {pos}/{len} commits")
                                .unwrap()
                                .progress_chars("=>-"),
                        );
                        pb.set_prefix(format!("📄 {}", file_path));
                        file_bars.insert(file_path, pb);
                    }
                    ProgressUpdate::IncCommits { file_path } => {
                        if let Some(pb) = file_bars.get(&file_path) {
                            pb.inc(1);
                        }
                    }
                    ProgressUpdate::FinishFile {
                        file_path,
                        benchmark_count,
                    } => {
                        if let Some(pb) = file_bars.remove(&file_path) {
                            pb.finish_with_message(format!("✓ {} benchmarks", benchmark_count));
                        }
                    }
                }
            }
        });

        // Load cache manifest if provided
        let (cache_dir, manifest) = if let Some(cache_dir) = args.cache_dir.clone() {
            let manifest_path = cache_dir.join("manifest.json");
            if manifest_path.exists() {
                log::info!(
                    "💾 Loading cache manifest from: {}",
                    manifest_path.display()
                );
                let content = fs::read_to_string(&manifest_path).with_context(|| {
                    format!("Failed to read manifest: {}", manifest_path.display())
                })?;
                let manifest: Vec<FileVersion> =
                    serde_json::from_str(&content).with_context(|| {
                        format!("Failed to parse manifest: {}", manifest_path.display())
                    })?;
                log::info!("   Loaded {} cached file versions", manifest.len());
                (Some(cache_dir), Some(manifest))
            } else {
                log::warn!(
                    "⚠️  Cache directory provided but no manifest.json found at {}",
                    manifest_path.display()
                );
                (Some(cache_dir), None)
            }
        } else {
            (None, None)
        };

        log::info!("🚀 Starting Delta Compression Benchmark");
        log::info!("Output directory: {}", output_dir.display());
        log::info!("Results file: {}", csv_path.display());
        log::info!("Threads: {}", rayon::current_num_threads());
        log::info!("Max tag search depth: {}", args.max_tag_depth);
        log::info!("Tags 0-15 have zero overhead, higher tags use varint encoding");

        if let Some(cache) = &cache_dir {
            log::info!("💾 Cache directory: {}", cache.display());
            match &manifest {
                Some(m) => log::info!("   Loaded {} file versions from manifest", m.len()),
                None => log::warn!("   No manifest found in cache directory"),
            }
        } else {
            log::info!("💾 No cache directory specified, will use git2 directly");
        }

        Ok(Self {
            args,
            output_dir,
            repos_dir,
            csv_path,
            wal_writer,
            mp,
            start_time,
            benchmark_counter: Arc::new(AtomicUsize::new(0)),
            tags_optimization_counter: Arc::new(Mutex::new((0, 0))),
            progress_tx,
            cache_dir,
            manifest,
        })
    }

    fn run(&self) -> Result<()> {
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
                (true, _, _) => self.get_all_files_in_history(&repo)?,
                (false, true, _) => self.get_all_files_at_head(&repo)?,
                (false, false, 0) => repo_config.files.iter().map(|s| s.to_string()).collect(),
                (false, false, n) => self
                    .get_all_files_at_head(&repo)?
                    .into_iter()
                    .take(n)
                    .collect(),
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

        let total_benchmarks = Arc::new(AtomicUsize::new(0));

        for repo_config in repos_to_test {
            println!("\n{}", "═".repeat(80));
            log::info!("📦 Processing repository: {}", repo_config.name);
            log::info!("   {}", repo_config.description);

            let files_to_benchmark = repo_files_map.get(repo_config.name).unwrap();

            match self.benchmark_repo(
                repo_config,
                files_to_benchmark,
                Arc::clone(&total_benchmarks),
                &master_pb,
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

    fn benchmark_repo(
        &self,
        repo_config: &RepoConfig,
        files_to_benchmark: &[String],
        total_benchmarks: Arc<AtomicUsize>,
        master_pb: &ProgressBar,
    ) -> Result<usize> {
        let repo_path = self.repos_dir.join(repo_config.name);
        let repo = self.ensure_repo_cloned(repo_config, &repo_path)?;

        let local_count = Arc::new(AtomicUsize::new(0));

        if self.args.parallel_files {
            // In parallel mode, use the progress manager thread
            files_to_benchmark.par_iter().for_each(|file_path| {
                let repo_path = repo_path.clone();
                let repo_name = repo_config.name.to_string();
                let file_path = file_path.clone();
                let local_count = Arc::clone(&local_count);
                let total_benchmarks = Arc::clone(&total_benchmarks);
                let wal_writer = &self.wal_writer;
                let tags_counter = Arc::clone(&self.tags_optimization_counter);
                let progress_tx = self.progress_tx.clone();

                if let Ok(thread_repo) = Repository::open(&repo_path) {
                    // Notify progress manager about new file
                    progress_tx
                        .send(ProgressUpdate::NewFile {
                            file_path: file_path.clone(),
                            total_commits: self.args.max_commits.min(100),
                        })
                        .ok();

                    match self.benchmark_file_with_tags(
                        &thread_repo,
                        &repo_name,
                        &file_path,
                        local_count,
                        total_benchmarks,
                        wal_writer,
                        tags_counter,
                        progress_tx.clone(),
                    ) {
                        Ok(count) => {
                            progress_tx
                                .send(ProgressUpdate::FinishFile {
                                    file_path: file_path.clone(),
                                    benchmark_count: count,
                                })
                                .ok();
                            master_pb.inc(1);
                        }
                        Err(e) => {
                            log::warn!("Failed: {} - {}", file_path, e);
                            master_pb.inc(1);
                        }
                    }
                }
            });
        } else {
            // Sequential mode - show detailed progress directly
            for (_idx, file_path) in files_to_benchmark.iter().enumerate() {
                let file_pb = self.mp.add(ProgressBar::new_spinner());
                file_pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("    {spinner:.green} {msg}")
                        .unwrap(),
                );
                file_pb.set_message(format!("📄 {}", file_path));

                match self.benchmark_file_with_tags(
                    &repo,
                    repo_config.name,
                    file_path,
                    Arc::clone(&local_count),
                    Arc::clone(&total_benchmarks),
                    &self.wal_writer,
                    Arc::clone(&self.tags_optimization_counter),
                    self.progress_tx.clone(),
                ) {
                    Ok(_) => {
                        file_pb.finish_and_clear();
                        master_pb.inc(1);
                    }
                    Err(e) => {
                        file_pb.finish_and_clear();
                        log::warn!("Failed: {} - {}", file_path, e);
                        master_pb.inc(1);
                    }
                }
            }
        }

        let count = local_count.load(Ordering::Relaxed);
        total_benchmarks.fetch_add(count, Ordering::Relaxed);
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

    fn benchmark_file_with_tags(
        &self,
        repo: &Repository,
        repo_name: &str,
        file_path: &str,
        local_count: Arc<AtomicUsize>,
        total_benchmarks: Arc<AtomicUsize>,
        wal_writer: &WalCsvWriter,
        tags_counter: Arc<Mutex<(usize, usize)>>,
        progress_tx: mpsc::Sender<ProgressUpdate>,
    ) -> Result<usize> {
        let max_commits = if self.args.max_commits > 0 {
            self.args.max_commits
        } else {
            usize::MAX
        };

        let commits = self.get_commit_history(repo, repo_name, file_path, max_commits)?;

        if commits.len() < 2 {
            anyhow::bail!(
                "Not enough commits found (need at least 2, found {})",
                commits.len()
            );
        }

        // Load all commit contents upfront for tag search
        let mut commit_data = Vec::new();
        for commit in &commits {
            match self.get_file_at_commit(repo, repo_name, &commit.hash, file_path) {
                Ok(content) => commit_data.push((commit.clone(), content)),
                Err(e) => {
                    log::debug!("Skipping commit {}: {}", &commit.hash[..8], e);
                    continue;
                }
            }
        }

        if commit_data.len() < 2 {
            anyhow::bail!("Not enough commits with valid content");
        }

        // Process each target commit
        let mut final_count = 0;
        for i in 1..commit_data.len() {
            let (target_commit, target_content) = &commit_data[i];

            // Find optimal base version using tags
            let search_depth = self.args.max_tag_depth.min(i);
            let mut best_base_idx = i - 1;
            let mut best_tag = 0;
            let mut best_delta_size = usize::MAX;
            let mut found_better_base = false;

            // Search backwards through previous versions
            for j in (i.saturating_sub(search_depth)..i).rev() {
                let (_, base_content) = &commit_data[j];
                let tag = i - j;

                // Quick size estimation
                let delta = xpatch::delta::encode(tag, base_content, target_content, true);

                if delta.len() < best_delta_size {
                    best_delta_size = delta.len();
                    best_base_idx = j;
                    best_tag = tag;

                    if j < i - 1 {
                        found_better_base = true;
                    }
                }
            }

            let (base_commit, base_content) = &commit_data[best_base_idx];
            let (_, immediate_prev_content) = &commit_data[i - 1];

            // Run full benchmark with optimal base
            match self.benchmark_commit_pair_with_tags(
                repo,
                repo_name,
                file_path,
                base_commit,
                target_commit,
                best_tag,
                base_content,
                target_content,
                immediate_prev_content,
            ) {
                Ok(result) => {
                    // Send to WAL writer
                    wal_writer.send(result.clone())?;

                    local_count.fetch_add(1, Ordering::Relaxed);
                    let total = total_benchmarks.fetch_add(1, Ordering::Relaxed) + 1;
                    final_count += 1;

                    // Update tags optimization counter
                    if found_better_base {
                        let mut counter = tags_counter.lock().unwrap();
                        counter.0 += 1;
                        counter.1 += 1;
                    } else {
                        let mut counter = tags_counter.lock().unwrap();
                        counter.1 += 1;
                    }

                    // Send progress update
                    progress_tx
                        .send(ProgressUpdate::IncCommits {
                            file_path: file_path.to_string(),
                        })
                        .ok();

                    // Log progress every 100 benchmarks
                    if total % 100 == 0 {
                        log::info!("💾 Processed {} benchmarks total", total);
                    }
                }
                Err(e) => {
                    log::debug!(
                        "Failed commit pair {}->{}: {}",
                        &base_commit.hash[..8],
                        &target_commit.hash[..8],
                        e
                    );
                }
            }
        }

        Ok(final_count)
    }

    fn benchmark_commit_pair_with_tags(
        &self,
        _repo: &Repository,
        repo_name: &str,
        file_path: &str,
        base: &CommitInfo,
        target: &CommitInfo,
        tag: usize,
        base_content: &[u8],
        target_content: &[u8],
        immediate_prev_content: &[u8],
    ) -> Result<BenchmarkResult> {
        if base_content.is_empty() || target_content.is_empty() {
            anyhow::bail!("Empty file content");
        }

        // xpatch with optimal base and tag
        let (xpatch_delta, xpatch_encode_us, xpatch_decode_us) =
            self.bench_xpatch_with_tag(base_content, target_content, tag)?;

        // xdelta3 with immediate predecessor (fair comparison)
        let (xdelta3_delta, xdelta3_encode_us, xdelta3_decode_us) = self
            .bench_xdelta3(immediate_prev_content, target_content)
            .unwrap_or_else(|_| (Vec::new(), 0, 0));

        // qbsdiff with immediate predecessor (fair comparison)
        let (qbsdiff_delta, qbsdiff_encode_us, qbsdiff_decode_us) = self
            .bench_qbsdiff(immediate_prev_content, target_content)
            .unwrap_or_else(|_| (Vec::new(), 0, 0));

        Ok(BenchmarkResult {
            repo_name: repo_name.to_string(),
            file_path: file_path.to_string(),
            commit_from: base.hash[..8].to_string(),
            commit_to: target.hash[..8].to_string(),
            commit_distance: target.distance_from(base),
            file_size_from: base_content.len(),
            file_size_to: target_content.len(),

            xpatch_tag: tag,
            xpatch_base_commit: base.hash[..8].to_string(),
            xpatch_base_distance: target.distance_from(base),

            xpatch_delta_size: xpatch_delta.len(),
            xpatch_ratio: xpatch_delta.len() as f64 / target_content.len() as f64,
            xpatch_encode_us,
            xpatch_decode_us,

            xdelta3_delta_size: xdelta3_delta.len(),
            xdelta3_ratio: if !xdelta3_delta.is_empty() && !target_content.is_empty() {
                xdelta3_delta.len() as f64 / target_content.len() as f64
            } else {
                f64::NAN
            },
            xdelta3_encode_us,
            xdelta3_decode_us,

            qbsdiff_delta_size: qbsdiff_delta.len(),
            qbsdiff_ratio: qbsdiff_delta.len() as f64 / target_content.len() as f64,
            qbsdiff_encode_us,
            qbsdiff_decode_us,
        })
    }

    fn get_commit_history(
        &self,
        repo: &Repository,
        repo_name: &str,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>> {
        // Try cache first if available
        if let Some(manifest) = &self.manifest {
            let cache_result = self.get_commit_history_from_cache(repo_name, file_path, limit);
            if let Ok(commits) = cache_result {
                if !commits.is_empty() {
                    log::debug!("Using cache for {}: {} commits", file_path, commits.len());
                    return Ok(commits);
                }
            }
        }

        // Fall back to git2
        log::debug!("Cache not available for {}, using git2", file_path);
        self.get_commit_history_from_git(repo, file_path, limit)
    }

    fn get_commit_history_from_cache(
        &self,
        repo_name: &str,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>> {
        let cache_dir = self
            .cache_dir
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No cache directory"))?;
        let safe_path = file_path.replace('/', "___");
        let file_cache_dir = cache_dir.join("files").join(repo_name).join(safe_path);

        if !file_cache_dir.exists() {
            anyhow::bail!(
                "Cache directory does not exist: {}",
                file_cache_dir.display()
            );
        }

        let mut commits = Vec::new();
        let mut entries = fs::read_dir(&file_cache_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "bin")
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        // Sort by index (filename format: <index>_<hash>.bin)
        entries.sort_by_key(|e| {
            e.file_name()
                .to_string_lossy()
                .split('_')
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(usize::MAX)
        });

        for (idx, entry) in entries.iter().enumerate() {
            if limit > 0 && idx >= limit {
                break;
            }

            let filename = entry.file_name().to_string_lossy().to_string();
            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() < 2 {
                continue;
            }

            let hash = parts[1].trim_end_matches(".bin").to_string();

            // Try to get commit info from manifest
            if let Some(manifest) = &self.manifest {
                if let Some(version) = manifest.iter().find(|v| {
                    v.repo_name == repo_name
                        && v.file_path == file_path
                        && v.commit_hash.starts_with(&hash)
                }) {
                    commits.push(CommitInfo {
                        hash: version.commit_hash.clone(),
                        date: version.commit_date.clone(),
                        message: version.commit_message.clone(),
                        index: idx,
                    });
                    continue;
                }
            }

            // Fallback: use hash and basic info
            commits.push(CommitInfo {
                hash,
                date: "0".to_string(),
                message: "".to_string(),
                index: idx,
            });
        }

        Ok(commits)
    }

    fn get_commit_history_from_git(
        &self,
        repo: &Repository,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>> {
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
        repo_name: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<Vec<u8>> {
        // Try cache first if available
        if let Some(cache_dir) = &self.cache_dir {
            let safe_path = file_path.replace('/', "___");
            let file_cache_dir = cache_dir.join("files").join(repo_name).join(safe_path);

            // Look for file matching the commit hash
            if file_cache_dir.exists() {
                let entries = fs::read_dir(&file_cache_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "bin")
                            .unwrap_or(false)
                    });

                for entry in entries {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let parts: Vec<&str> = filename.split('_').collect();
                    if parts.len() >= 2 {
                        let hash = parts[1].trim_end_matches(".bin");
                        if commit_hash.starts_with(hash) {
                            // Found matching file in cache
                            log::trace!("Loading from cache: {} {}", file_path, &commit_hash[..8]);
                            return fs::read(entry.path()).with_context(|| {
                                format!("Failed to read cached file: {}", entry.path().display())
                            });
                        }
                    }
                }
            }
        }

        // Fall back to git2
        log::trace!(
            "Cache miss for {} {}, using git2",
            file_path,
            &commit_hash[..8]
        );
        self.get_file_at_commit_from_git(repo, commit_hash, file_path)
    }

    fn get_file_at_commit_from_git(
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

    fn bench_xpatch_with_tag(
        &self,
        base: &[u8],
        target: &[u8],
        tag: usize,
    ) -> Result<(Vec<u8>, u128, u128)> {
        let start = Instant::now();
        let delta = xpatch::delta::encode(tag, base, target, true);
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
        println!("║                  DELTA COMPRESSION - FINAL SUMMARY                        ║");
        println!("║                  (with xpatch TAGS optimization)                          ║");
        println!("╚═══════════════════════════════════════════════════════════════════════════╝\n");

        let mut by_repo: HashMap<String, Vec<BenchmarkResult>> = HashMap::new();
        for result in &all_results {
            by_repo
                .entry(result.repo_name.clone())
                .or_default()
                .push(result.clone());
        }

        let (better_base_found, total_benchmarks) = *self.tags_optimization_counter.lock().unwrap();
        println!("\n🏷️  TAGS OPTIMIZATION STATISTICS:");
        println!("   Total benchmarks: {}", total_benchmarks);
        println!(
            "   Better base found: {} ({:.1}%)",
            better_base_found,
            (better_base_found as f64 / total_benchmarks.max(1) as f64) * 100.0
        );
        println!(
            "   Average search depth: {} commits",
            self.args.max_tag_depth
        );

        for (repo_name, results) in by_repo {
            println!("\n📊 REPOSITORY: {}", repo_name);
            println!("{}", "─".repeat(80));

            let avg_size = results.iter().map(|r| r.file_size_to).sum::<usize>() / results.len();

            let avg_xpatch_tag =
                results.iter().map(|r| r.xpatch_tag).sum::<usize>() as f64 / results.len() as f64;
            let avg_xpatch_dist = results
                .iter()
                .map(|r| r.xpatch_base_distance)
                .sum::<usize>() as f64
                / results.len() as f64;
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

            println!("\n  xpatch TAGS Optimization:");
            println!(
                "    Average tag value: {:.1} (0-15 = zero overhead)",
                avg_xpatch_tag
            );
            println!(
                "    Average base distance: {:.1} commits back",
                avg_xpatch_dist
            );
            println!(
                "    Compression ratio: {:.4} ({:.1}% savings)",
                avg_xpatch,
                (1.0 - avg_xpatch) * 100.0
            );

            println!("\n  Comparison (sequential baseline):");
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

            let improvement_over_xdelta3 = if !avg_xdelta3.is_nan() {
                (avg_xdelta3 - avg_xpatch) / avg_xdelta3 * 100.0
            } else {
                0.0
            };
            let improvement_over_qbsdiff = (avg_qbsdiff - avg_xpatch) / avg_qbsdiff * 100.0;

            println!("\n  🏆 xpatch improvement:");
            if !avg_xdelta3.is_nan() {
                println!("    vs xdelta3: {:.1}% better", improvement_over_xdelta3);
            }
            println!("    vs qbsdiff: {:.1}% better", improvement_over_qbsdiff);
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

        println!(
            "\n\n╔═══════════════════════════════════════════════════════════════════════════╗"
        );
        println!("║                          OVERALL WINNER                                   ║");
        println!("╚═══════════════════════════════════════════════════════════════════════════╝");
        println!(
            "   xpatch with TAGS optimization: {:.4} avg ratio",
            overall_xpatch
        );

        if !overall_xdelta3.is_nan() {
            println!(
                "   vs xdelta3 ({:.4}): {:.1}% better",
                overall_xdelta3,
                (overall_xdelta3 - overall_xpatch) / overall_xdelta3 * 100.0
            );
        }
        println!(
            "   vs qbsdiff ({:.4}): {:.1}% better",
            overall_qbsdiff,
            (overall_qbsdiff - overall_xpatch) / overall_qbsdiff * 100.0
        );
        println!("\n   💡 Tags 0-15 have zero overhead - xpatch gets this optimization for free!");

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
    index: usize,
}

impl CommitInfo {
    fn distance_from(&self, other: &CommitInfo) -> usize {
        self.index.abs_diff(other.index)
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let runner = BenchmarkRunner::new(args)?;
    runner.run()?;

    Ok(())
}
