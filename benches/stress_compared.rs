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

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Cursor;
use std::sync::Mutex;
use std::time::Instant;
use xpatch::delta;

// ============================================================================
// COMPRESSION STATISTICS
// ============================================================================

#[derive(Clone)]
struct CompressionStats {
    library: String,
    test_name: String,
    original_size: usize,
    delta_size: usize,
    delta_ratio: f64,
    space_savings: f64,
    encode_time_us: u128,
    decode_time_us: u128,
}

impl CompressionStats {
    fn calculate(
        library: String,
        test_name: String,
        base: &[u8],
        new_data: &[u8],
        delta: &[u8],
        encode_time_us: u128,
        decode_time_us: u128,
    ) -> Self {
        let original_size = base.len();
        let new_size = new_data.len();
        let delta_size = delta.len();
        let delta_ratio = if new_size > 0 {
            delta_size as f64 / new_size as f64
        } else {
            0.0
        };
        let space_savings = if original_size > 0 {
            (1.0 - delta_size as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        Self {
            library,
            test_name,
            original_size,
            delta_size,
            delta_ratio,
            space_savings,
            encode_time_us,
            decode_time_us,
        }
    }
}

// Global stats collector using Mutex for thread safety
static STATS_COLLECTOR: Mutex<Vec<CompressionStats>> = Mutex::new(Vec::new());

fn collect_stats(stats: CompressionStats) {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.push(stats);
    }
}

fn print_unified_summary() {
    if let Ok(stats_list) = STATS_COLLECTOR.lock() {
        if stats_list.is_empty() {
            return;
        }

        println!(
            "\n╔════════════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║                    LIBRARY COMPARISON - UNIFIED SUMMARY                            ║"
        );
        println!(
            "╚════════════════════════════════════════════════════════════════════════════════════╝\n"
        );

        // Group by library
        let mut by_library: std::collections::HashMap<String, Vec<&CompressionStats>> =
            std::collections::HashMap::new();
        for stat in stats_list.iter() {
            by_library
                .entry(stat.library.clone())
                .or_insert_with(Vec::new)
                .push(stat);
        }

        println!("📊 LIBRARY COMPARISON:\n");
        println!(
            "{:<15} {:>10} {:>10} {:>10} {:>12} {:>12}",
            "Library", "Avg Ratio", "Best", "Worst", "Avg Enc (µs)", "Avg Dec (µs)"
        );
        println!("{}", "─".repeat(80));

        for (lib, stats) in by_library.iter() {
            let avg_ratio: f64 =
                stats.iter().map(|s| s.delta_ratio).sum::<f64>() / stats.len() as f64;
            let best_ratio = stats
                .iter()
                .map(|s| s.delta_ratio)
                .fold(f64::INFINITY, f64::min);
            let worst_ratio = stats
                .iter()
                .map(|s| s.delta_ratio)
                .fold(f64::NEG_INFINITY, f64::max);
            let avg_encode: u128 =
                stats.iter().map(|s| s.encode_time_us).sum::<u128>() / stats.len() as u128;
            let avg_decode: u128 =
                stats.iter().map(|s| s.decode_time_us).sum::<u128>() / stats.len() as u128;

            println!(
                "{:<15} {:>10.3} {:>10.3} {:>10.3} {:>12} {:>12}",
                lib, avg_ratio, best_ratio, worst_ratio, avg_encode, avg_decode
            );
        }

        // Detailed table
        println!("\n📈 DETAILED RESULTS:\n");
        println!(
            "{:<15} {:<30} {:>10} {:>10} {:>8} {:>9} {:>12} {:>12}",
            "Library",
            "Test Case",
            "Original",
            "Delta",
            "Ratio",
            "Savings",
            "Encode (µs)",
            "Decode (µs)"
        );
        println!("{}", "─".repeat(120));

        for stat in stats_list.iter() {
            println!(
                "{:<15} {:<30} {:>10} {:>10} {:>8.3} {:>8.1}% {:>12} {:>12}",
                stat.library,
                truncate_str(&stat.test_name, 30),
                format_bytes(stat.original_size),
                format_bytes(stat.delta_size),
                stat.delta_ratio,
                stat.space_savings,
                stat.encode_time_us,
                stat.decode_time_us
            );
        }
        println!("\n");
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ============================================================================
// TEST DATA GENERATORS
// ============================================================================

fn generate_realistic_text(size: usize) -> Vec<u8> {
    let paragraphs = vec![
        "The quick brown fox jumps over the lazy dog. ",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ",
        "In a world where technology evolves rapidly, adaptation is key. ",
        "Performance optimization requires careful measurement and analysis. ",
        "Data compression algorithms trade space for time complexity. ",
    ];

    let mut result = Vec::with_capacity(size);
    let mut idx = 0;
    while result.len() < size {
        result.extend_from_slice(paragraphs[idx % paragraphs.len()].as_bytes());
        idx += 1;
    }
    result.truncate(size);
    result
}

fn generate_code_text(size: usize) -> Vec<u8> {
    let code_lines = vec![
        "fn main() {\n",
        "    let x = 42;\n",
        "    println!(\"Hello, world!\");\n",
        "    for i in 0..10 {\n",
        "        process_item(i);\n",
        "    }\n",
        "}\n",
    ];

    let mut result = Vec::with_capacity(size);
    let mut idx = 0;
    while result.len() < size {
        result.extend_from_slice(code_lines[idx % code_lines.len()].as_bytes());
        idx += 1;
    }
    result.truncate(size);
    result
}

fn generate_repetitive_data(size: usize) -> Vec<u8> {
    let pattern = b"AAAAAAAAAA";
    let mut result = Vec::with_capacity(size);
    while result.len() < size {
        result.extend_from_slice(pattern);
    }
    result.truncate(size);
    result
}

fn generate_json_data(size: usize) -> Vec<u8> {
    let template = r#"{"id":12345,"name":"example_user","email":"user@example.com","data":{"nested":true,"values":[1,2,3,4,5]},"timestamp":1234567890},"#;
    let mut result = Vec::with_capacity(size);
    result.push(b'[');
    while result.len() < size - 1 {
        result.extend_from_slice(template.as_bytes());
    }
    result.push(b']');
    result.truncate(size);
    result
}

// ============================================================================
// CHANGE GENERATORS
// ============================================================================

fn small_insert_at(base: &[u8], position: f32) -> Vec<u8> {
    let insert_pos = (base.len() as f32 * position) as usize;
    let insertion = b" [INSERTED] ";

    let mut result = Vec::with_capacity(base.len() + insertion.len());
    result.extend_from_slice(&base[..insert_pos]);
    result.extend_from_slice(insertion);
    result.extend_from_slice(&base[insert_pos..]);
    result
}

fn append_text(base: &[u8], append_size: usize) -> Vec<u8> {
    let mut result = base.to_vec();
    result.extend(vec![b'X'; append_size]);
    result
}

fn mixed_operations(base: &[u8]) -> Vec<u8> {
    let mut result = base.to_vec();

    if result.len() < 1000 {
        return result;
    }

    // Insert at 1/4
    let pos1 = result.len() / 4;
    result.splice(pos1..pos1, b"[NEW]".iter().cloned());

    // Remove at 1/2
    let pos2 = result.len() / 2;
    result.drain(pos2..pos2 + 20.min(result.len() - pos2));

    // Replace at 3/4
    let pos3 = result.len() * 3 / 4;
    for i in pos3..pos3 + 30.min(result.len() - pos3) {
        result[i] = b'M';
    }

    result
}

// ============================================================================
// LIBRARY-SPECIFIC HELPERS
// ============================================================================

// xpatch
fn measure_xpatch(test_name: &str, base: &[u8], new_data: &[u8], enable_zstd: bool) {
    let start = Instant::now();
    let delta = delta::encode(0, base, new_data, enable_zstd);
    let encode_time = start.elapsed().as_micros();

    let start = Instant::now();
    let _decoded = delta::decode(base, &delta[..]).unwrap();
    let decode_time = start.elapsed().as_micros();

    let stats = CompressionStats::calculate(
        "xpatch".to_string(),
        test_name.to_string(),
        base,
        new_data,
        &delta[..],
        encode_time,
        decode_time,
    );

    collect_stats(stats);
}

// xdelta3
fn measure_xdelta3(test_name: &str, base: &[u8], new_data: &[u8]) {
    // Skip empty data
    if base.is_empty() || new_data.is_empty() {
        eprintln!("⚠️  Skipping xdelta3 for {}: empty data", test_name);
        return;
    }

    let start = Instant::now();
    let delta = match xdelta3::encode(base, new_data) {
        Some(d) => d,
        None => {
            eprintln!("⚠️  xdelta3 encode failed for {}", test_name);
            return;
        }
    };
    let encode_time = start.elapsed().as_micros();

    let start = Instant::now();
    let _decoded = match xdelta3::decode(&delta[..], base) {
        Some(d) => d,
        None => {
            eprintln!("⚠️  xdelta3 decode failed for {}", test_name);
            return;
        }
    };
    let decode_time = start.elapsed().as_micros();

    let stats = CompressionStats::calculate(
        "xdelta3".to_string(),
        test_name.to_string(),
        base,
        new_data,
        &delta[..],
        encode_time,
        decode_time,
    );

    collect_stats(stats);
}

// qbsdiff
fn measure_qbsdiff(test_name: &str, base: &[u8], new_data: &[u8]) {
    // Skip empty data
    if base.is_empty() || new_data.is_empty() {
        eprintln!("⚠️  Skipping qbsdiff for {}: empty data", test_name);
        return;
    }

    let start = Instant::now();
    let mut patch = Vec::new();
    if let Err(e) = qbsdiff::Bsdiff::new(base, new_data).compare(Cursor::new(&mut patch)) {
        eprintln!("⚠️  qbsdiff compare failed for {}: {:?}", test_name, e);
        return;
    }
    let encode_time = start.elapsed().as_micros();

    let start = Instant::now();
    let patcher = match qbsdiff::Bspatch::new(&patch[..]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "⚠️  qbsdiff patch creation failed for {}: {:?}",
                test_name, e
            );
            return;
        }
    };
    let mut decoded = Vec::new();
    if let Err(e) = patcher.apply(base, Cursor::new(&mut decoded)) {
        eprintln!("⚠️  qbsdiff apply failed for {}: {:?}", test_name, e);
        return;
    }
    let decode_time = start.elapsed().as_micros();

    let stats = CompressionStats::calculate(
        "qbsdiff".to_string(),
        test_name.to_string(),
        base,
        new_data,
        &patch[..],
        encode_time,
        decode_time,
    );

    collect_stats(stats);
}

fn measure_zstd(test_name: &str, base: &[u8], new_data: &[u8]) {
    use std::fs;
    use std::process::Command;

    // Skip empty data
    if base.is_empty() || new_data.is_empty() {
        eprintln!("⚠️  Skipping zstd for {}: empty data", test_name);
        return;
    }

    // Check if zstd is available
    if Command::new("zstd").arg("--version").output().is_err() {
        eprintln!(
            "⚠️  zstd command not found. Skipping zstd benchmark for {}",
            test_name
        );
        return;
    }

    let pid = std::process::id();
    let safe_test_name = test_name.replace("/", "_").replace(" ", "_");
    let base_file = format!("/tmp/xpatch_bench_base_{}_{}.tmp", pid, safe_test_name);
    let new_file = format!("/tmp/xpatch_bench_new_{}_{}.tmp", pid, safe_test_name);
    let patch_file = format!("/tmp/xpatch_bench_patch_{}_{}.tmp", pid, safe_test_name);
    let decoded_file = format!("/tmp/xpatch_bench_decoded_{}_{}.tmp", pid, safe_test_name);

    // Write data to files
    if let Err(e) = fs::write(&base_file, base) {
        eprintln!(
            "⚠️  Failed to write base file for zstd ({}): {}",
            test_name, e
        );
        return;
    }
    if let Err(e) = fs::write(&new_file, new_data) {
        eprintln!(
            "⚠️  Failed to write new file for zstd ({}): {}",
            test_name, e
        );
        let _ = fs::remove_file(&base_file);
        return;
    }

    // Measure encoding time
    let start = Instant::now();
    let encode_output = Command::new("zstd")
        .arg("--patch-from")
        .arg(&base_file)
        .arg(&new_file)
        .arg("-f")
        .arg("-o")
        .arg(&patch_file)
        .output();
    let encode_time = start.elapsed().as_micros();

    let encode_success = match encode_output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!(
                    "⚠️  zstd encode failed for {}: {}",
                    test_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            output.status.success()
        }
        Err(e) => {
            eprintln!(
                "⚠️  Failed to execute zstd command for {}: {}",
                test_name, e
            );
            false
        }
    };

    if !encode_success {
        let _ = fs::remove_file(&base_file);
        let _ = fs::remove_file(&new_file);
        let _ = fs::remove_file(&patch_file);
        return;
    }

    // Read patch size
    let patch_data = match fs::read(&patch_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!(
                "⚠️  Failed to read patch file for zstd ({}): {}",
                test_name, e
            );
            let _ = fs::remove_file(&base_file);
            let _ = fs::remove_file(&new_file);
            let _ = fs::remove_file(&patch_file);
            return;
        }
    };

    // Measure decoding time (NOTE: --patch-from is REQUIRED here)
    let start = Instant::now();
    let decode_output = Command::new("zstd")
        .arg("-d")
        .arg("--patch-from")
        .arg(&base_file) // CRITICAL: Must specify base file for decoding
        .arg(&patch_file)
        .arg("-f")
        .arg("-o")
        .arg(&decoded_file)
        .output();
    let decode_time = start.elapsed().as_micros();

    let decode_success = match decode_output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!(
                    "⚠️  zstd decode failed for {}: {}",
                    test_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            output.status.success()
        }
        Err(e) => {
            eprintln!("⚠️  Failed to execute zstd decode for {}: {}", test_name, e);
            false
        }
    };

    // Verify decoded data matches new_data
    if decode_success {
        match fs::read(&decoded_file) {
            Ok(decoded_data) => {
                if decoded_data != new_data {
                    eprintln!(
                        "⚠️  zstd decode verification failed for {}: decoded data doesn't match",
                        test_name
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Failed to read decoded file for zstd ({}): {}",
                    test_name, e
                );
            }
        }
    }

    // Clean up files
    let _ = fs::remove_file(&base_file);
    let _ = fs::remove_file(&new_file);
    let _ = fs::remove_file(&patch_file);
    let _ = fs::remove_file(&decoded_file);

    if !decode_success {
        return;
    }

    let stats = CompressionStats::calculate(
        "zstd".to_string(),
        test_name.to_string(),
        base,
        new_data,
        &patch_data,
        encode_time,
        decode_time,
    );

    collect_stats(stats);
}

// ============================================================================
// BENCHMARK: SIZE COMPARISON
// ============================================================================

fn bench_library_comparison_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("library_comparison_sizes");
    group.sample_size(10);

    for size in [100_000, 1_000_000] {
        let base = generate_realistic_text(size);
        let new_data = small_insert_at(&base[..], 0.5);

        let test_name = format!("size_{}", format_bytes(size));

        // Collect stats for all libraries
        measure_xpatch(&test_name.as_str(), &base[..], &new_data[..], true);
        measure_xdelta3(&test_name.as_str(), &base[..], &new_data[..]);
        measure_qbsdiff(&test_name.as_str(), &base[..], &new_data[..]);
        measure_zstd(&test_name.as_str(), &base[..], &new_data[..]);

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark xpatch
        group.bench_with_input(
            BenchmarkId::new("xpatch", size),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let delta = delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(true),
                    );
                    let _decoded =
                        delta::decode(black_box(&base[..]), black_box(&delta[..])).unwrap();
                });
            },
        );

        // Benchmark xdelta3
        group.bench_with_input(
            BenchmarkId::new("xdelta3", size),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    if let Some(delta) =
                        xdelta3::encode(black_box(&base[..]), black_box(&new_data[..]))
                    {
                        let _ = xdelta3::decode(black_box(&base[..]), black_box(&delta[..]));
                    }
                });
            },
        );

        // Benchmark qbsdiff
        group.bench_with_input(
            BenchmarkId::new("qbsdiff", size),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let mut patch = Vec::new();
                    if qbsdiff::Bsdiff::new(black_box(&base[..]), black_box(&new_data[..]))
                        .compare(Cursor::new(&mut patch))
                        .is_ok()
                    {
                        if let Ok(patcher) = qbsdiff::Bspatch::new(black_box(&patch[..])) {
                            let mut decoded = Vec::new();
                            let _ = patcher.apply(black_box(&base[..]), Cursor::new(&mut decoded));
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK: DATA PATTERN COMPARISON
// ============================================================================

fn bench_library_comparison_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("library_comparison_patterns");
    let size = 100_000;

    let test_cases = vec![
        ("realistic", generate_realistic_text(size)),
        ("code", generate_code_text(size)),
        ("repetitive", generate_repetitive_data(size)),
        ("json", generate_json_data(size)),
    ];

    for (name, base) in test_cases {
        let new_data = small_insert_at(&base[..], 0.5);
        let test_name = format!("pattern_{}", name);

        // Collect stats for all libraries
        measure_xpatch(&test_name.as_str(), &base[..], &new_data[..], true);
        measure_xdelta3(&test_name.as_str(), &base[..], &new_data[..]);
        measure_qbsdiff(&test_name.as_str(), &base[..], &new_data[..]);
        measure_zstd(&test_name.as_str(), &base[..], &new_data[..]);

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark xpatch
        group.bench_with_input(
            BenchmarkId::new("xpatch", name),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let delta = delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(true),
                    );
                    let _decoded =
                        delta::decode(black_box(&base[..]), black_box(&delta[..])).unwrap();
                });
            },
        );

        // Benchmark xdelta3
        group.bench_with_input(
            BenchmarkId::new("xdelta3", name),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    if let Some(delta) =
                        xdelta3::encode(black_box(&base[..]), black_box(&new_data[..]))
                    {
                        let _ = xdelta3::decode(black_box(&base[..]), black_box(&delta[..]));
                    }
                });
            },
        );

        // Benchmark qbsdiff
        group.bench_with_input(
            BenchmarkId::new("qbsdiff", name),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let mut patch = Vec::new();
                    if qbsdiff::Bsdiff::new(black_box(&base[..]), black_box(&new_data[..]))
                        .compare(Cursor::new(&mut patch))
                        .is_ok()
                    {
                        if let Ok(patcher) = qbsdiff::Bspatch::new(black_box(&patch[..])) {
                            let mut decoded = Vec::new();
                            let _ = patcher.apply(black_box(&base[..]), Cursor::new(&mut decoded));
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK: CHANGE TYPE COMPARISON
// ============================================================================

fn bench_library_comparison_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("library_comparison_changes");
    let size = 100_000;
    let base = generate_realistic_text(size);

    group.throughput(Throughput::Bytes(size as u64));

    let test_cases = vec![
        ("insert_middle", small_insert_at(&base[..], 0.5)),
        ("append_small", append_text(&base[..], 100)),
        ("append_large", append_text(&base[..], 10000)),
        ("mixed", mixed_operations(&base[..])),
    ];

    for (name, new_data) in &test_cases {
        let test_name = format!("change_{}", name);

        // Collect stats for all libraries
        measure_xpatch(&test_name.as_str(), &base[..], &new_data[..], true);
        measure_xdelta3(&test_name.as_str(), &base[..], &new_data[..]);
        measure_qbsdiff(&test_name.as_str(), &base[..], &new_data[..]);
        measure_zstd(&test_name.as_str(), &base[..], &new_data[..]);

        // Benchmark xpatch
        group.bench_with_input(
            BenchmarkId::new("xpatch", name),
            &(&base, new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let delta = delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(true),
                    );
                    let _decoded =
                        delta::decode(black_box(&base[..]), black_box(&delta[..])).unwrap();
                });
            },
        );

        // Benchmark xdelta3
        group.bench_with_input(
            BenchmarkId::new("xdelta3", name),
            &(&base, new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    if let Some(delta) =
                        xdelta3::encode(black_box(&base[..]), black_box(&new_data[..]))
                    {
                        let _ = xdelta3::decode(black_box(&base[..]), black_box(&delta[..]));
                    }
                });
            },
        );

        // Benchmark qbsdiff
        group.bench_with_input(
            BenchmarkId::new("qbsdiff", name),
            &(&base, new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let mut patch = Vec::new();
                    if qbsdiff::Bsdiff::new(black_box(&base[..]), black_box(&new_data[..]))
                        .compare(Cursor::new(&mut patch))
                        .is_ok()
                    {
                        if let Ok(patcher) = qbsdiff::Bspatch::new(black_box(&patch[..])) {
                            let mut decoded = Vec::new();
                            let _ = patcher.apply(black_box(&base[..]), Cursor::new(&mut decoded));
                        }
                    }
                });
            },
        );
    }

    group.finish();

    // Print summary at the very end
    print_unified_summary();
}

// ============================================================================
// CRITERION CONFIGURATION
// ============================================================================

fn configure_criterion() -> Criterion {
    Criterion::default()
        .with_output_color(true)
        .significance_level(0.1)
        .noise_threshold(0.05)
        .warm_up_time(std::time::Duration::from_secs(2))
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group! {
    name = library_comparison;
    config = configure_criterion();
    targets =
        bench_library_comparison_sizes,
        bench_library_comparison_patterns,
        bench_library_comparison_changes
}

criterion_main!(library_comparison);
