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

use anyhow::{Context, Result};
use criterion::{Criterion, criterion_group, criterion_main};
use git2::Repository;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ============================================================================
// GLOBAL SHUTDOWN FLAG
// ============================================================================

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

fn setup_ctrlc_handler() {
    ctrlc::set_handler(move || {
        println!("\n\n⚠️  Ctrl+C received! Finishing current test and generating reports...\n");
        SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

fn should_continue() -> bool {
    !SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

// ============================================================================
// ALGORITHM TRAIT
// ============================================================================

trait DeltaAlgorithm: Send + Sync {
    fn name(&self) -> &str;

    // Regular encode (for algorithms that don't use multi-version)
    fn encode(&self, _base: &[u8], _new: &[u8]) -> Result<Vec<u8>> {
        anyhow::bail!("Not implemented")
    }

    // Multi-version encode (for xpatch_tags)
    fn encode_with_history(
        &self,
        new: &[u8],
        previous_versions: &[(usize, &[u8])],
    ) -> Result<(usize, Vec<u8>)> {
        // Default: just use immediate previous
        if let Some((tag, base)) = previous_versions.first() {
            Ok((*tag, self.encode(base, new)?))
        } else {
            anyhow::bail!("No previous versions")
        }
    }

    fn decode(&self, delta: &[u8], base: &[u8]) -> Result<Vec<u8>>;
}

// xpatch - Sequential mode (no tags, fair comparison to xdelta3(vcdiff))
struct XpatchSequential;

impl DeltaAlgorithm for XpatchSequential {
    fn name(&self) -> &str {
        "xpatch_sequential"
    }

    fn encode(&self, base: &[u8], new: &[u8]) -> Result<Vec<u8>> {
        Ok(xpatch::delta::encode(0, base, new, true))
    }

    fn decode(&self, delta: &[u8], base: &[u8]) -> Result<Vec<u8>> {
        xpatch::delta::decode(base, delta).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// xpatch - With tag optimization (search back N commits for best base)
struct XpatchTags {
    max_search_depth: usize,
}

impl XpatchTags {
    fn new(max_search_depth: usize) -> Self {
        Self { max_search_depth }
    }
}

impl DeltaAlgorithm for XpatchTags {
    fn name(&self) -> &str {
        "xpatch_tags"
    }

    fn encode_with_history(
        &self,
        new: &[u8],
        previous_versions: &[(usize, &[u8])],
    ) -> Result<(usize, Vec<u8>)> {
        let search_depth = self.max_search_depth.min(previous_versions.len());

        let mut best_tag = 0;
        let mut best_delta_size = usize::MAX;
        let mut best_delta = Vec::new();

        // Search through previous N versions
        for i in 0..search_depth {
            let (tag, base) = previous_versions[i];
            let delta = xpatch::delta::encode(tag, base, new, true);

            if delta.len() < best_delta_size {
                best_delta_size = delta.len();
                best_tag = tag;
                best_delta = delta;
            }
        }

        Ok((best_tag, best_delta))
    }

    fn decode(&self, delta: &[u8], base: &[u8]) -> Result<Vec<u8>> {
        xpatch::delta::decode(base, delta).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// vcdiff (VCDIFF standard implementation)
#[cfg(feature = "vcdiff")]
struct VcdiffAlgo;

#[cfg(feature = "vcdiff")]
impl DeltaAlgorithm for VcdiffAlgo {
    fn name(&self) -> &str {
        "vcdiff"
    }

    fn encode(&self, base: &[u8], new: &[u8]) -> Result<Vec<u8>> {
        // Use standard format with checksum for compatibility
        let format = vcdiff::FORMAT_STANDARD | vcdiff::FORMAT_CHECKSUM;
        Ok(vcdiff::encode(base, new, format, true))
    }

    fn decode(&self, delta: &[u8], base: &[u8]) -> Result<Vec<u8>> {
        Ok(vcdiff::decode(base, delta))
    }
}

// gdelta (optional, if feature enabled)
#[cfg(feature = "gdelta")]
struct GdeltaAlgo;

#[cfg(feature = "gdelta")]
impl DeltaAlgorithm for GdeltaAlgo {
    fn name(&self) -> &str {
        "gdelta"
    }

    fn encode(&self, base: &[u8], new: &[u8]) -> Result<Vec<u8>> {
        gdelta::encode(new, base).map_err(Into::into)
    }

    fn decode(&self, delta: &[u8], base: &[u8]) -> Result<Vec<u8>> {
        gdelta::decode(delta, base).map_err(Into::into)
    }
}

// ============================================================================
// STATISTICS HELPERS
// ============================================================================

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn median_u128(values: &mut [u128]) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values.sort();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2
    } else {
        values[mid]
    }
}

fn median_usize(values: &mut [usize]) -> usize {
    if values.is_empty() {
        return 0;
    }
    values.sort();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2
    } else {
        values[mid]
    }
}

// ============================================================================
// RESULT TRACKING
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkResult {
    repo_name: String,
    file_path: String,
    commit_from: String,
    commit_to: String,
    commit_distance: usize,
    file_size: usize,

    algorithm: String,
    tag_used: Option<usize>,
    tag_base_commit: Option<String>,
    tag_base_distance: Option<usize>,

    delta_size: usize,
    compression_ratio: f64,
    encode_us: u128,
    decode_us: u128,
    verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct HardwareInfo {
    cpu: String,
    cores: usize,
    memory_gb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Report {
    generated_at: String,
    hardware: HardwareInfo,
    results: Vec<BenchmarkResult>,
    early_termination: bool,
}

fn collect_hardware_info() -> HardwareInfo {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    HardwareInfo {
        cpu: sys
            .cpus()
            .first()
            .map_or("Unknown".to_string(), |c| c.brand().to_string()),
        cores: sys.cpus().len(),
        memory_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
    }
}

// ============================================================================
// CACHE SYSTEM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedVersion {
    commit_hash: String,
    commit_date: String,
    commit_message: String,
    size_bytes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheManifest {
    repo_name: String,
    files: HashMap<String, Vec<CachedVersion>>,
}

struct Cache {
    root: PathBuf,
    repo_name: String,
    manifest: CacheManifest,
}

impl Cache {
    fn new(root: PathBuf, repo_name: &str) -> Result<Self> {
        fs::create_dir_all(&root)?;

        let manifest_path = root.join("manifest.json");
        let manifest = if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            serde_json::from_str(&content)?
        } else {
            CacheManifest {
                repo_name: repo_name.to_string(),
                files: HashMap::new(),
            }
        };

        Ok(Self {
            root,
            repo_name: repo_name.to_string(),
            manifest,
        })
    }

    fn get_file(&self, file_path: &str, commit_hash: &str) -> Option<Vec<u8>> {
        let safe_path = file_path.replace('/', "___");
        let cache_dir = self
            .root
            .join("files")
            .join(&self.repo_name)
            .join(safe_path);

        if !cache_dir.exists() {
            return None;
        }

        for entry in fs::read_dir(&cache_dir).ok()? {
            let entry = entry.ok()?;
            let filename = entry.file_name().to_string_lossy().to_string();

            if filename.contains(&commit_hash[..8.min(commit_hash.len())]) {
                return fs::read(entry.path()).ok();
            }
        }

        None
    }

    fn save_file(&mut self, file_path: &str, commit: &CommitInfo, content: &[u8]) -> Result<()> {
        let safe_path = file_path.replace('/', "___");
        let cache_dir = self
            .root
            .join("files")
            .join(&self.repo_name)
            .join(&safe_path);
        fs::create_dir_all(&cache_dir)?;

        let filename = format!(
            "{:04}_{}.bin",
            commit.index,
            &commit.hash[..8.min(commit.hash.len())]
        );
        let file_path_full = cache_dir.join(filename);

        fs::write(file_path_full, content)?;

        // Update manifest
        self.manifest
            .files
            .entry(file_path.to_string())
            .or_insert_with(Vec::new)
            .push(CachedVersion {
                commit_hash: commit.hash.clone(),
                commit_date: commit.date.clone(),
                commit_message: commit.message.clone(),
                size_bytes: content.len(),
            });

        Ok(())
    }

    fn get_commits_for_file(&self, file_path: &str) -> Option<&Vec<CachedVersion>> {
        self.manifest.files.get(file_path)
    }

    fn save_manifest(&self) -> Result<()> {
        let manifest_path = self.root.join("manifest.json");
        let json = serde_json::to_string_pretty(&self.manifest)?;
        fs::write(manifest_path, json)?;
        Ok(())
    }
}

// ============================================================================
// GIT OPERATIONS
// ============================================================================

#[derive(Debug, Clone)]
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

fn clone_or_open_repo(url: &str, path: &Path) -> Result<Repository> {
    if path.join(".git").exists() {
        log::info!("Using existing repository at {}", path.display());
        return Repository::open(path).context("Failed to open existing repo");
    }

    log::info!("Cloning {}...", url);
    let mut builder = git2::build::RepoBuilder::new();
    builder
        .clone(url, path)
        .context("Failed to clone repository")
}

fn get_commit_history(repo: &Repository, file_path: &str, limit: usize) -> Result<Vec<CommitInfo>> {
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

        if let Ok(tree) = commit.tree() {
            if let Ok(entry) = tree.get_path(Path::new(file_path)) {
                let blob_id = entry.id();

                // Skip if content didn't actually change
                if last_blob_id == Some(blob_id) {
                    continue;
                }

                commits.push(CommitInfo {
                    hash: commit.id().to_string(),
                    date: commit.time().seconds().to_string(),
                    message: commit.summary().unwrap_or("").to_string(),
                    index: 0, // Will be fixed after reversal
                });
                last_blob_id = Some(blob_id);
            }
        }
    }

    // Reverse to get oldest→newest order and fix indices
    commits.reverse();
    for (idx, commit) in commits.iter_mut().enumerate() {
        commit.index = idx;
    }

    Ok(commits)
}

fn get_file_at_commit(repo: &Repository, commit_hash: &str, file_path: &str) -> Result<Vec<u8>> {
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

fn discover_files(repo: &Repository, mode: FileDiscoveryMode) -> Result<Vec<String>> {
    match mode {
        FileDiscoveryMode::Predefined(files) => Ok(files),
        FileDiscoveryMode::AllAtHead(max_files) => {
            let mut files = get_all_files_at_head(repo)?;
            if max_files > 0 {
                files.truncate(max_files);
            }
            Ok(files)
        }
        FileDiscoveryMode::AllInHistory(max_files) => {
            let mut files = get_all_files_in_history(repo)?;
            if max_files > 0 {
                files.truncate(max_files);
            }
            Ok(files)
        }
    }
}

fn get_all_files_at_head(repo: &Repository) -> Result<Vec<String>> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let tree = commit.tree()?;

    let mut files = Vec::new();
    walk_tree(repo, &tree, Path::new(""), &mut files)?;
    files.sort();
    Ok(files)
}

fn get_all_files_in_history(repo: &Repository) -> Result<Vec<String>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut all_files = HashSet::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;
        collect_files_from_tree(repo, &tree, Path::new(""), &mut all_files)?;
    }

    let mut files: Vec<String> = all_files.into_iter().collect();
    files.sort();
    Ok(files)
}

fn walk_tree(
    repo: &Repository,
    tree: &git2::Tree,
    base_path: &Path,
    files: &mut Vec<String>,
) -> Result<()> {
    for entry in tree.iter() {
        let name = entry.name().unwrap_or("");
        let entry_path = base_path.join(name);

        match entry.kind() {
            Some(git2::ObjectType::Blob) => {
                files.push(entry_path.to_string_lossy().to_string());
            }
            Some(git2::ObjectType::Tree) => {
                if let Ok(subtree) = repo.find_tree(entry.id()) {
                    walk_tree(repo, &subtree, &entry_path, files)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn collect_files_from_tree(
    repo: &Repository,
    tree: &git2::Tree,
    base_path: &Path,
    files: &mut HashSet<String>,
) -> Result<()> {
    for entry in tree.iter() {
        let name = entry.name().unwrap_or("");
        let entry_path = base_path.join(name);

        match entry.kind() {
            Some(git2::ObjectType::Blob) => {
                files.insert(entry_path.to_string_lossy().to_string());
            }
            Some(git2::ObjectType::Tree) => {
                if let Ok(subtree) = repo.find_tree(entry.id()) {
                    collect_files_from_tree(repo, &subtree, &entry_path, files)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// ============================================================================
// BENCHMARKING WITH TAG OPTIMIZATION
// ============================================================================

fn benchmark_file_with_tags(
    repo: &Repository,
    cache: &Option<Arc<Mutex<Cache>>>,
    repo_name: &str,
    file_path: &str,
    max_commits: usize,
    max_tag_depth: usize,
    min_file_size: usize,
    algos: &[Box<dyn DeltaAlgorithm>],
) -> Result<Vec<BenchmarkResult>> {
    // Get commit history
    let commits = if let Some(cache) = cache {
        let cache = cache.lock().unwrap();
        if let Some(cached_versions) = cache.get_commits_for_file(file_path) {
            // Cached versions are already in oldest→newest order from manifest
            cached_versions
                .iter()
                .enumerate()
                .map(|(index, v)| CommitInfo {
                    hash: v.commit_hash.clone(),
                    date: v.commit_date.clone(),
                    message: v.commit_message.clone(),
                    index,
                })
                .collect()
        } else {
            get_commit_history(repo, file_path, max_commits)?
        }
    } else {
        get_commit_history(repo, file_path, max_commits)?
    };

    if commits.len() < 2 {
        anyhow::bail!("Not enough commits for {}", file_path);
    }

    log::debug!(
        "Processing {} with {} commits (oldest: {}, newest: {})",
        file_path,
        commits.len(),
        &commits.first().unwrap().hash[..8],
        &commits.last().unwrap().hash[..8]
    );

    // Load all commit contents upfront for tag search
    let mut commit_data = Vec::new();
    for commit in &commits {
        let content = if let Some(cache) = cache {
            let cache = cache.lock().unwrap();
            cache
                .get_file(file_path, &commit.hash)
                .or_else(|| get_file_at_commit(repo, &commit.hash, file_path).ok())
        } else {
            get_file_at_commit(repo, &commit.hash, file_path).ok()
        };

        if let Some(content) = content {
            commit_data.push((commit.clone(), content));
        }
    }

    if commit_data.len() < 2 {
        anyhow::bail!("Not enough valid commits for {}", file_path);
    }

    // Skip files that are too small (empty or nearly empty)
    let avg_size: usize = commit_data
        .iter()
        .map(|(_, content)| content.len())
        .sum::<usize>()
        / commit_data.len();
    if avg_size < min_file_size {
        anyhow::bail!("File too small (avg {} bytes): {}", avg_size, file_path);
    }

    let mut results = Vec::new();

    // Process each target commit (oldest→newest order)
    // For each commit i, we encode a delta from commit i-1 (older) → i (newer)
    for i in 1..commit_data.len() {
        if !should_continue() {
            break;
        }

        let (target_commit, target_content) = &commit_data[i];
        let (prev_commit, prev_content) = &commit_data[i - 1];

        // For each algorithm
        for algo in algos {
            let result = if algo.name() == "xpatch_tags" {
                // Build list of previous versions for tag search
                let search_depth = max_tag_depth.min(i);
                let previous_versions: Vec<(usize, &[u8])> = (0..search_depth)
                    .map(|j| {
                        let base_idx = i - 1 - j;
                        let tag = j + 1;
                        (tag, commit_data[base_idx].1.as_slice())
                    })
                    .collect();

                let start = Instant::now();
                let (tag_used, delta) =
                    match algo.encode_with_history(target_content, &previous_versions) {
                        Ok(d) => d,
                        Err(e) => {
                            log::debug!("Tag encode failed for {}: {}", file_path, e);
                            continue;
                        }
                    };
                let encode_us = start.elapsed().as_micros();

                let base_idx = i - tag_used;
                let (base_commit, base_content) = &commit_data[base_idx];

                let start = Instant::now();
                let reconstructed = match algo.decode(&delta, base_content) {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!(
                            "Tag decode failed for {} (tag={}, base={}→target={}, base_size={}, delta_size={}, target_size={}): {}",
                            file_path,
                            tag_used,
                            base_commit.hash[..8].to_string(),
                            target_commit.hash[..8].to_string(),
                            base_content.len(),
                            delta.len(),
                            target_content.len(),
                            e
                        );
                        continue;
                    }
                };
                let decode_us = start.elapsed().as_micros();

                let verified = reconstructed == *target_content;

                Some(BenchmarkResult {
                    repo_name: repo_name.to_string(),
                    file_path: file_path.to_string(),
                    commit_from: base_commit.hash[..8].to_string(),
                    commit_to: target_commit.hash[..8].to_string(),
                    commit_distance: target_commit.distance_from(base_commit),
                    file_size: target_content.len(),
                    algorithm: algo.name().to_string(),
                    tag_used: Some(tag_used),
                    tag_base_commit: Some(base_commit.hash[..8].to_string()),
                    tag_base_distance: Some(target_commit.distance_from(base_commit)),
                    delta_size: delta.len(),
                    compression_ratio: if target_content.len() > 0 {
                        delta.len() as f64 / target_content.len() as f64
                    } else {
                        0.0
                    },
                    encode_us,
                    decode_us,
                    verified,
                })
            } else {
                // Standard algorithms use immediate previous
                let start = Instant::now();
                let delta = match algo.encode(prev_content, target_content) {
                    Ok(d) => d,
                    Err(e) => {
                        log::debug!(
                            "Encode failed for {} ({}→{}): {}",
                            file_path,
                            prev_commit.hash[..8].to_string(),
                            target_commit.hash[..8].to_string(),
                            e
                        );
                        continue;
                    }
                };
                let encode_us = start.elapsed().as_micros();

                let start = Instant::now();
                let reconstructed = match algo.decode(&delta, prev_content) {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!(
                            "Decode failed for {} with {} ({}→{}, base_size={}, delta_size={}, target_size={}): {}",
                            file_path,
                            algo.name(),
                            prev_commit.hash[..8].to_string(),
                            target_commit.hash[..8].to_string(),
                            prev_content.len(),
                            delta.len(),
                            target_content.len(),
                            e
                        );
                        continue;
                    }
                };
                let decode_us = start.elapsed().as_micros();

                let verified = reconstructed == *target_content;

                Some(BenchmarkResult {
                    repo_name: repo_name.to_string(),
                    file_path: file_path.to_string(),
                    commit_from: prev_commit.hash[..8].to_string(),
                    commit_to: target_commit.hash[..8].to_string(),
                    commit_distance: 1,
                    file_size: target_content.len(),
                    algorithm: algo.name().to_string(),
                    tag_used: None,
                    tag_base_commit: None,
                    tag_base_distance: None,
                    delta_size: delta.len(),
                    compression_ratio: if target_content.len() > 0 {
                        delta.len() as f64 / target_content.len() as f64
                    } else {
                        0.0
                    },
                    encode_us,
                    decode_us,
                    verified,
                })
            };

            if let Some(result) = result {
                results.push(result);
            }
        }
    }

    Ok(results)
}

// ============================================================================
// REPORT GENERATION
// ============================================================================

fn generate_markdown_report(
    results: &[BenchmarkResult],
    hardware: &HardwareInfo,
    early_termination: bool,
    output_path: &Path,
) -> Result<()> {
    let mut report = String::new();

    report.push_str("# 📊 Git Repository Benchmark Report\n\n");

    if early_termination {
        report.push_str("**⚠️ PARTIAL RESULTS - Benchmark was interrupted**\n\n");
    }

    report.push_str(&format!(
        "**Generated:** {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    // Hardware
    report.push_str("## 💻 Hardware\n\n");
    report.push_str("```\n");
    report.push_str(&format!("CPU:    {}\n", hardware.cpu));
    report.push_str(&format!("Cores:  {}\n", hardware.cores));
    report.push_str(&format!("Memory: {:.1} GB\n", hardware.memory_gb));
    report.push_str("```\n\n");

    // Overview
    let total_tests = results.len();
    let verified = results.iter().filter(|r| r.verified).count();
    let unique_files: std::collections::HashSet<_> = results.iter().map(|r| &r.file_path).collect();
    let files_tested = unique_files.len();

    report.push_str("## 📈 Overview\n\n");
    report.push_str(&format!("- **Files Tested:** {}\n", files_tested));
    report.push_str(&format!("- **Total Tests:** {}\n", total_tests));
    report.push_str(&format!(
        "- **Verified:** {} ({:.1}%)\n\n",
        verified,
        (verified as f64 / total_tests as f64) * 100.0
    ));

    // Algorithm verification status
    report.push_str("## ⚠️ Algorithm Health\n\n");
    report.push_str("| Algorithm | Tests Passed | Tests Failed | Status |\n");
    report.push_str("|-----------|--------------|--------------|--------|\n");

    let algos: Vec<String> = results
        .iter()
        .map(|r| r.algorithm.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    for algo in &algos {
        let algo_results: Vec<_> = results.iter().filter(|r| r.algorithm == *algo).collect();
        let passed = algo_results.iter().filter(|r| r.verified).count();
        let failed = algo_results.len() - passed;
        let status = if failed == 0 {
            "✅ VERIFIED"
        } else {
            "❌ FAILED"
        };
        report.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            algo, passed, failed, status
        ));
    }
    report.push_str("\n");
    report.push_str("*Note: Some algorithms may have fewer tests if they failed to encode/decode certain file versions. Failed tests are skipped and logged as warnings.*\n\n");

    // Filter verified algorithms for rankings
    let verified_algos: Vec<_> = algos
        .iter()
        .filter(|algo| {
            let algo_results: Vec<_> = results.iter().filter(|r| r.algorithm == **algo).collect();
            algo_results.iter().all(|r| r.verified)
        })
        .collect();

    // Algorithm comparison
    report.push_str("## 🏆 Algorithm Rankings\n\n");
    report.push_str("*Only verified algorithms*\n\n");
    report.push_str("### By Compression Ratio (Lower is Better)\n\n");
    report.push_str("| Algorithm | Avg Ratio | Median Ratio | Avg Saved | Median Saved | Avg Encode (µs) | Median Encode (µs) | Avg Decode (µs) | Median Decode (µs) |\n");
    report.push_str("|-----------|-----------|--------------|-----------|--------------|-----------------|--------------------|-----------------|-----------------|\n");

    let mut algo_stats: Vec<_> = verified_algos
        .iter()
        .map(|algo| {
            let algo_results: Vec<_> = results
                .iter()
                .filter(|r| r.algorithm == **algo && r.verified)
                .collect();

            // Calculate averages
            let avg_ratio = algo_results
                .iter()
                .map(|r| r.compression_ratio)
                .sum::<f64>()
                / algo_results.len() as f64;
            let avg_encode =
                algo_results.iter().map(|r| r.encode_us).sum::<u128>() / algo_results.len() as u128;
            let avg_decode =
                algo_results.iter().map(|r| r.decode_us).sum::<u128>() / algo_results.len() as u128;

            // Calculate medians
            let mut ratios: Vec<f64> = algo_results.iter().map(|r| r.compression_ratio).collect();
            let mut encode_times: Vec<u128> = algo_results.iter().map(|r| r.encode_us).collect();
            let mut decode_times: Vec<u128> = algo_results.iter().map(|r| r.decode_us).collect();

            let median_ratio = median(&mut ratios);
            let median_encode = median_u128(&mut encode_times);
            let median_decode = median_u128(&mut decode_times);

            (
                *algo,
                avg_ratio,
                median_ratio,
                avg_encode,
                median_encode,
                avg_decode,
                median_decode,
            )
        })
        .collect();

    algo_stats.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    for (algo, avg_ratio, median_ratio, avg_encode, median_encode, avg_decode, median_decode) in
        &algo_stats
    {
        let avg_saved = if avg_ratio.is_finite() && *avg_ratio > 0.0 {
            format!("{:.1}%", (1.0 - avg_ratio) * 100.0)
        } else {
            "N/A".to_string()
        };

        let median_saved = if median_ratio.is_finite() && *median_ratio > 0.0 {
            format!("{:.1}%", (1.0 - median_ratio) * 100.0)
        } else {
            "N/A".to_string()
        };

        report.push_str(&format!(
            "| {} | {:.4} | {:.4} | {} | {} | {} | {} | {} | {} |\n",
            algo,
            avg_ratio,
            median_ratio,
            avg_saved,
            median_saved,
            avg_encode,
            median_encode,
            avg_decode,
            median_decode
        ));
    }

    // Detailed statistics section
    report.push_str("\n## 📊 Detailed Statistics\n\n");

    for algo in &verified_algos {
        let algo_results: Vec<_> = results
            .iter()
            .filter(|r| r.algorithm == **algo && r.verified)
            .collect();

        if algo_results.is_empty() {
            continue;
        }

        report.push_str(&format!("### {}\n\n", algo));

        // Delta size statistics
        let mut delta_sizes: Vec<usize> = algo_results.iter().map(|r| r.delta_size).collect();
        let avg_delta_size = delta_sizes.iter().sum::<usize>() / delta_sizes.len();
        let median_delta_size = median_usize(&mut delta_sizes);

        // Compression ratio statistics
        let mut ratios: Vec<f64> = algo_results.iter().map(|r| r.compression_ratio).collect();
        let avg_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
        let median_ratio = median(&mut ratios);

        // Space saved statistics
        let avg_saved = if avg_ratio.is_finite() && avg_ratio > 0.0 {
            (1.0 - avg_ratio) * 100.0
        } else {
            0.0
        };
        let median_saved = if median_ratio.is_finite() && median_ratio > 0.0 {
            (1.0 - median_ratio) * 100.0
        } else {
            0.0
        };

        // Timing statistics
        let mut encode_times: Vec<u128> = algo_results.iter().map(|r| r.encode_us).collect();
        let mut decode_times: Vec<u128> = algo_results.iter().map(|r| r.decode_us).collect();
        let avg_encode = encode_times.iter().sum::<u128>() / encode_times.len() as u128;
        let avg_decode = decode_times.iter().sum::<u128>() / decode_times.len() as u128;
        let median_encode = median_u128(&mut encode_times);
        let median_decode = median_u128(&mut decode_times);

        report.push_str("| Metric | Average | Median |\n");
        report.push_str("|--------|---------|--------|\n");
        report.push_str(&format!(
            "| Delta Size | {} bytes | {} bytes |\n",
            avg_delta_size, median_delta_size
        ));
        report.push_str(&format!(
            "| Compression Ratio | {:.4} | {:.4} |\n",
            avg_ratio, median_ratio
        ));
        report.push_str(&format!(
            "| Space Saved | {:.2}% | {:.2}% |\n",
            avg_saved, median_saved
        ));
        report.push_str(&format!(
            "| Encode Time | {} µs | {} µs |\n",
            avg_encode, median_encode
        ));
        report.push_str(&format!(
            "| Decode Time | {} µs | {} µs |\n\n",
            avg_decode, median_decode
        ));
    }

    // Tag optimization analysis
    report.push_str("\n## 💡 Tag Optimization Impact\n\n");

    let seq_results: Vec<_> = results
        .iter()
        .filter(|r| r.algorithm == "xpatch_sequential" && r.verified)
        .collect();
    let tags_results: Vec<_> = results
        .iter()
        .filter(|r| r.algorithm == "xpatch_tags" && r.verified)
        .collect();

    if !seq_results.is_empty() && !tags_results.is_empty() {
        let seq_ratio =
            seq_results.iter().map(|r| r.compression_ratio).sum::<f64>() / seq_results.len() as f64;
        let tags_ratio = tags_results
            .iter()
            .map(|r| r.compression_ratio)
            .sum::<f64>()
            / tags_results.len() as f64;

        // Calculate median ratios
        let mut seq_ratios: Vec<f64> = seq_results.iter().map(|r| r.compression_ratio).collect();
        let mut tags_ratios: Vec<f64> = tags_results.iter().map(|r| r.compression_ratio).collect();
        let seq_median = median(&mut seq_ratios);
        let tags_median = median(&mut tags_ratios);

        if seq_ratio.is_finite() && tags_ratio.is_finite() && seq_ratio > 0.0 {
            let avg_improvement = ((seq_ratio - tags_ratio) / seq_ratio) * 100.0;
            let median_improvement = if seq_median > 0.0 {
                ((seq_median - tags_median) / seq_median) * 100.0
            } else {
                0.0
            };

            report.push_str(&format!(
                "**Average:** Tags provide **{:.1}%** better compression than sequential mode.\n\n",
                avg_improvement
            ));

            report.push_str(&format!(
                "**Median:** Tags provide **{:.1}%** better compression than sequential mode.\n\n",
                median_improvement
            ));

            // Tag usage statistics
            let mut tag_values: Vec<usize> =
                tags_results.iter().filter_map(|r| r.tag_used).collect();
            let mut base_distances: Vec<usize> = tags_results
                .iter()
                .filter_map(|r| r.tag_base_distance)
                .collect();

            let avg_tag = tag_values.iter().sum::<usize>() as f64 / tag_values.len() as f64;
            let avg_base_distance =
                base_distances.iter().sum::<usize>() as f64 / base_distances.len() as f64;
            let median_tag = median_usize(&mut tag_values);
            let median_base_distance = median_usize(&mut base_distances);

            report.push_str(&format!("**Tag Statistics:**\n"));
            report.push_str(&format!(
                "- Average tag value: {:.1} (median: {})\n",
                avg_tag, median_tag
            ));
            report.push_str(&format!(
                "- Average base distance: {:.1} commits back (median: {})\n\n",
                avg_base_distance, median_base_distance
            ));
        } else {
            report.push_str("*Insufficient data for tag optimization analysis*\n\n");
        }
    }

    report.push_str("---\n");
    report.push_str(
        "\n*Commits processed in chronological order (oldest→newest). Run with different repositories and XPATCH_MAX_TAG_DEPTH to explore optimization*\n",
    );

    fs::write(output_path, report)?;
    println!("✅ Report saved to: {}", output_path.display());

    Ok(())
}

fn generate_json_report(
    results: Vec<BenchmarkResult>,
    hardware: HardwareInfo,
    early_termination: bool,
    output_path: &Path,
) -> Result<()> {
    let report = Report {
        generated_at: chrono::Local::now().to_rfc3339(),
        hardware,
        results,
        early_termination,
    };

    let json = serde_json::to_string_pretty(&report)?;
    fs::write(output_path, json)?;
    println!("✅ JSON saved to: {}", output_path.display());

    Ok(())
}

// ============================================================================
// ENVIRONMENT-BASED CONFIGURATION
// ============================================================================

#[derive(Debug)]
struct Config {
    repo: Option<String>,
    preset: Option<String>,
    max_commits: usize,
    output: PathBuf,
    cache_dir: Option<PathBuf>,
    build_cache: bool,
    use_cache: bool,
    max_tag_depth: usize,
    all_files_head: bool,
    all_files: bool,
    max_files: usize,
    parallel_files: bool,
    min_file_size: usize,
}

impl Config {
    fn from_env() -> Result<Self> {
        let repo = std::env::var("XPATCH_REPO").ok();
        let preset = std::env::var("XPATCH_PRESET").ok();

        let max_commits = std::env::var("XPATCH_MAX_COMMITS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        let output = std::env::var("XPATCH_OUTPUT")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./benchmark_results"));

        let cache_dir = std::env::var("XPATCH_CACHE_DIR").ok().map(PathBuf::from);

        let build_cache = std::env::var("XPATCH_BUILD_CACHE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let use_cache = std::env::var("XPATCH_USE_CACHE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let max_tag_depth = std::env::var("XPATCH_MAX_TAG_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(16);

        let all_files_head = std::env::var("XPATCH_ALL_FILES_HEAD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let all_files = std::env::var("XPATCH_ALL_FILES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let max_files = std::env::var("XPATCH_MAX_FILES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let parallel_files = std::env::var("XPATCH_PARALLEL_FILES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let min_file_size = std::env::var("XPATCH_MIN_FILE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        Ok(Self {
            repo,
            preset,
            max_commits,
            output,
            cache_dir,
            build_cache,
            use_cache,
            max_tag_depth,
            all_files_head,
            all_files,
            max_files,
            parallel_files,
            min_file_size,
        })
    }

    fn print_help() {
        println!("Git Repository Benchmark - Environment Variable Configuration");
        println!();
        println!("Required (one of):");
        println!(
            "  XPATCH_REPO=<url>              Repository URL (e.g., https://github.com/rust-lang/rust.git)"
        );
        println!("  XPATCH_PRESET=<name>           Use preset: rust, neovim, tokio, git");
        println!();
        println!("Options:");
        println!(
            "  XPATCH_MAX_COMMITS=<n>         Maximum commits to analyze per file (default: 50, 0=all)"
        );
        println!(
            "  XPATCH_OUTPUT=<path>           Output directory (default: ./benchmark_results)"
        );
        println!("  XPATCH_CACHE_DIR=<path>        Cache directory for extracted versions");
        println!(
            "  XPATCH_BUILD_CACHE=<bool>      Build cache only, don't benchmark (default: false)"
        );
        println!(
            "  XPATCH_USE_CACHE=<bool>        Use existing cache instead of git2 (default: false)"
        );
        println!("  XPATCH_MAX_TAG_DEPTH=<n>       Maximum tag search depth (default: 16)");
        println!("  XPATCH_ALL_FILES_HEAD=<bool>   Test all files at HEAD (default: false)");
        println!(
            "  XPATCH_ALL_FILES=<bool>        Test all files from history (SLOW) (default: false)"
        );
        println!("  XPATCH_MAX_FILES=<n>           Maximum files to test (default: 0=all)");
        println!("  XPATCH_PARALLEL_FILES=<bool>   Process files in parallel (default: false)");
        println!(
            "  XPATCH_MIN_FILE_SIZE=<n>       Minimum average file size in bytes (default: 100)"
        );
        println!();
        println!("Examples:");
        println!("  XPATCH_PRESET=tokio cargo bench --bench git_real_world");
        println!("  XPATCH_PRESET=tokio XPATCH_MAX_COMMITS=0 XPATCH_ALL_FILES=true \\");
        println!(
            "    XPATCH_BUILD_CACHE=true XPATCH_CACHE_DIR=./cache cargo bench --bench git_real_world"
        );
    }
}

#[derive(Debug)]
enum FileDiscoveryMode {
    Predefined(Vec<String>),
    AllAtHead(usize),
    AllInHistory(usize),
}

const PRESETS: &[(&str, &str, &[&str])] = &[
    (
        "rust",
        "https://github.com/rust-lang/rust.git",
        &["Cargo.toml", "compiler/rustc_driver/src/lib.rs"],
    ),
    (
        "neovim",
        "https://github.com/neovim/neovim.git",
        &["src/nvim/main.c", "runtime/lua/vim/_editor.lua"],
    ),
    (
        "tokio",
        "https://github.com/tokio-rs/tokio.git",
        &["tokio/src/runtime/mod.rs"],
    ),
    (
        "git",
        "https://github.com/git/git.git",
        &["builtin/add.c", "diff.c"],
    ),
];

// ============================================================================
// MAIN BENCHMARK RUNNER
// ============================================================================

fn run_git_benchmark(config: Config) -> Result<()> {
    setup_ctrlc_handler();

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let output_dir = config.output.clone();
    fs::create_dir_all(&output_dir)?;

    // Determine repo URL and name
    let (repo_url, repo_name, predefined_files) = if let Some(url) = config.repo {
        let name = url
            .split('/')
            .last()
            .unwrap_or("repo")
            .trim_end_matches(".git")
            .to_string();
        (url, name, Vec::new())
    } else if let Some(preset) = config.preset {
        let preset_config = PRESETS
            .iter()
            .find(|(name, _, _)| *name == preset)
            .ok_or_else(|| anyhow::anyhow!("Unknown preset: {}", preset))?;
        (
            preset_config.1.to_string(),
            preset_config.0.to_string(),
            preset_config.2.iter().map(|s| s.to_string()).collect(),
        )
    } else {
        anyhow::bail!("Must provide XPATCH_REPO or XPATCH_PRESET");
    };

    log::info!("📦 Repository: {}", repo_url);
    log::info!("📊 Tag search depth: {}", config.max_tag_depth);

    let repo_path = output_dir.join("repos").join(&repo_name);
    let repo = clone_or_open_repo(&repo_url, &repo_path)?;

    // File discovery
    let discovery_mode = if config.all_files {
        FileDiscoveryMode::AllInHistory(config.max_files)
    } else if config.all_files_head {
        FileDiscoveryMode::AllAtHead(config.max_files)
    } else {
        FileDiscoveryMode::Predefined(predefined_files)
    };

    let files = discover_files(&repo, discovery_mode)?;
    log::info!("📁 Testing {} files", files.len());

    if files.is_empty() {
        anyhow::bail!("No files found to benchmark");
    }

    // Setup cache
    let cache = if let Some(cache_dir) = config.cache_dir {
        let cache = Arc::new(Mutex::new(Cache::new(cache_dir, &repo_name)?));

        if config.build_cache {
            log::info!("🔨 Building cache...");
            build_cache(&repo, &cache, &repo_name, &files, config.max_commits)?;
            return Ok(());
        }

        Some(cache)
    } else {
        None
    };

    // Build algorithms
    let algos: Vec<Box<dyn DeltaAlgorithm>> = vec![
        Box::new(XpatchSequential),
        Box::new(XpatchTags::new(config.max_tag_depth)),
        #[cfg(feature = "vcdiff")]
        Box::new(VcdiffAlgo),
        #[cfg(feature = "gdelta")]
        Box::new(GdeltaAlgo),
    ];

    log::info!("🔍 Benchmarking with {} algorithms", algos.len());

    // Run benchmarks
    let mp = MultiProgress::new();
    let master_pb = mp.add(ProgressBar::new(files.len() as u64));
    master_pb.set_style(
        ProgressStyle::default_bar()
            .template("⏳ Overall: {bar:40.cyan/blue} {pos}/{len} files | {elapsed} | ETA: {eta}")
            .unwrap(),
    );

    let all_results = Arc::new(Mutex::new(Vec::new()));
    let results_ref = Arc::clone(&all_results);

    if config.parallel_files {
        let repo_path_clone = repo_path.clone();

        files.par_iter().for_each(|file_path| {
            if !should_continue() {
                return;
            }

            // Open repository handle for this thread
            let thread_repo = match Repository::open(&repo_path_clone) {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Failed to open repo for {}: {}", file_path, e);
                    return;
                }
            };

            match benchmark_file_with_tags(
                &thread_repo,
                &cache,
                &repo_name,
                file_path,
                config.max_commits,
                config.max_tag_depth,
                config.min_file_size,
                &algos,
            ) {
                Ok(results) => {
                    results_ref.lock().unwrap().extend(results);
                }
                Err(e) => {
                    log::warn!("Failed {}: {}", file_path, e);
                }
            }

            master_pb.inc(1);
        });
    } else {
        for file_path in &files {
            if !should_continue() {
                break;
            }

            match benchmark_file_with_tags(
                &repo,
                &cache,
                &repo_name,
                file_path,
                config.max_commits,
                config.max_tag_depth,
                config.min_file_size,
                &algos,
            ) {
                Ok(results) => {
                    all_results.lock().unwrap().extend(results);
                }
                Err(e) => {
                    log::warn!("Failed {}: {}", file_path, e);
                }
            }

            master_pb.inc(1);
        }
    }

    master_pb.finish_with_message("✅ Complete");

    // Print summary
    let results = all_results.lock().unwrap().clone();
    let unique_files: HashSet<_> = results.iter().map(|r| &r.file_path).collect();
    let failed_count = results.iter().filter(|r| !r.verified).count();

    println!("\n📊 Benchmark Summary:");
    println!("   Files processed: {}/{}", unique_files.len(), files.len());
    println!("   Total tests: {}", results.len());
    println!(
        "   Verified: {}",
        results.iter().filter(|r| r.verified).count()
    );

    if failed_count > 0 {
        println!("   ⚠️  Failed: {} (check warnings above)", failed_count);
    }

    // Count warnings by algorithm
    let mut algo_test_counts: HashMap<String, usize> = HashMap::new();
    for result in &results {
        *algo_test_counts
            .entry(result.algorithm.clone())
            .or_insert(0) += 1;
    }

    println!("\n   Tests per algorithm:");
    for (algo, count) in algo_test_counts.iter() {
        println!("   - {}: {}", algo, count);
    }

    // Generate reports
    let hardware = collect_hardware_info();
    let early_termination = !should_continue();

    let report_md = output_dir.join(format!("report_{}.md", timestamp));
    let report_json = output_dir.join(format!("report_{}.json", timestamp));

    generate_markdown_report(&results, &hardware, early_termination, &report_md)?;
    generate_json_report(results, hardware, early_termination, &report_json)?;

    Ok(())
}

fn build_cache(
    repo: &Repository,
    cache: &Arc<Mutex<Cache>>,
    repo_name: &str,
    files: &[String],
    max_commits: usize,
) -> Result<()> {
    use crossbeam::channel;
    use rayon::prelude::*;

    let repo_path = repo.path().to_path_buf();
    let (tx, rx): (channel::Sender<(String, CommitInfo, Vec<u8>)>, _) = channel::bounded(5000); // Higher buffer

    let mp = MultiProgress::new();
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("💾 Extracting: {pos} versions")
            .unwrap(),
    );

    let consumer_cache = Arc::clone(cache);
    let consumer_pb = pb.clone();
    let consumer = std::thread::spawn(move || {
        let mut cache_mut = consumer_cache.lock().unwrap();
        let mut count = 0;

        for (file_path, commit, content) in rx.iter() {
            let _ = cache_mut.save_file(&file_path, &commit, &content);
            count += 1;
            consumer_pb.set_position(count as u64);
        }

        cache_mut.save_manifest().ok();
        count
    });

    // Get ALL commits first (once)
    let all_commits = get_commit_history(repo, "", max_commits).unwrap_or_default();

    // Parallel: extract each file × commit combo
    let commit_product: Vec<_> = files
        .iter()
        .flat_map(|f| all_commits.iter().map(move |c| (f.clone(), c.clone())))
        .collect();

    commit_product
        .par_iter()
        .for_each_with(tx.clone(), |tx, (file_path, commit)| {
            let thread_repo = match Repository::open(&repo_path) {
                Ok(r) => r,
                Err(_) => return,
            };

            if let Ok(content) = get_file_at_commit(&thread_repo, &commit.hash, file_path) {
                let _ = tx.send((file_path.clone(), commit.clone(), content));
            }
        });

    drop(tx);
    let count = consumer.join().unwrap_or(0);
    pb.finish_with_message(format!("✅ Cached {} versions", count));

    Ok(())
}

// ============================================================================
// CRITERION INTEGRATION
// ============================================================================

fn git_benchmark(_c: &mut Criterion) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Check for help flag
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        Config::print_help();
        return;
    }

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Configuration error: {}", e);
            eprintln!();
            Config::print_help();
            std::process::exit(1);
        }
    };

    if let Err(e) = run_git_benchmark(config) {
        eprintln!("❌ Benchmark failed: {}", e);
        std::process::exit(1);
    }
}

criterion_group!(benches, git_benchmark);
criterion_main!(benches);
