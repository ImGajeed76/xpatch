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

//! Quick Benchmark: Human-Focused Changes
//!
//! Tests xpatch on what it's designed for:
//! - Sequential additions (writing code)
//! - Sequential deletions (removing code)
//! - Small scattered edits (bug fixes)
//! - Realistic file sizes (1-500KB)
//! - Real data formats (code, docs, configs)
//!
//! Should run in under 2 minutes for quick feedback.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Mutex;
use std::time::Instant;
use xpatch::delta;

// ============================================================================
// STATISTICS TRACKING
// ============================================================================

#[derive(Clone)]
struct TestResult {
    scenario: String,
    format: String,
    size: usize,
    delta_size: usize,
    compression_ratio: f64,
    encode_us: u128,
    decode_us: u128,
}

static RESULTS: Mutex<Vec<TestResult>> = Mutex::new(Vec::new());

fn record_result(result: TestResult) {
    if let Ok(mut results) = RESULTS.lock() {
        results.push(result);
    }
}

fn print_summary() {
    let results = RESULTS.lock().unwrap();
    if results.is_empty() {
        return;
    }

    println!("\n╔═══════════════════════════════════════════════════════════════════╗");
    println!("║              XPATCH HUMAN-FOCUSED BENCHMARK SUMMARY               ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    // Overall stats
    let avg_ratio = results.iter().map(|r| r.compression_ratio).sum::<f64>() / results.len() as f64;
    let avg_encode = results.iter().map(|r| r.encode_us).sum::<u128>() / results.len() as u128;
    let avg_decode = results.iter().map(|r| r.decode_us).sum::<u128>() / results.len() as u128;

    println!("📊 OVERALL PERFORMANCE:");
    println!("  Total tests:         {}", results.len());
    println!(
        "  Avg compression:     {:.3} ({:.1}% saved)",
        avg_ratio,
        (1.0 - avg_ratio) * 100.0
    );
    println!("  Avg encode time:     {} µs", avg_encode);
    println!("  Avg decode time:     {} µs\n", avg_decode);

    // By scenario
    println!("🎯 BY SCENARIO:");
    let scenarios: Vec<String> = results
        .iter()
        .map(|r| r.scenario.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for scenario in scenarios {
        let scenario_results: Vec<_> = results.iter().filter(|r| r.scenario == scenario).collect();
        let avg = scenario_results
            .iter()
            .map(|r| r.compression_ratio)
            .sum::<f64>()
            / scenario_results.len() as f64;
        println!(
            "  {:<25} {:.3} ({:.1}% saved)",
            scenario,
            avg,
            (1.0 - avg) * 100.0
        );
    }

    // By format
    println!("\n📝 BY FORMAT:");
    let formats: Vec<String> = results
        .iter()
        .map(|r| r.format.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for format in formats {
        let format_results: Vec<_> = results.iter().filter(|r| r.format == format).collect();
        let avg = format_results
            .iter()
            .map(|r| r.compression_ratio)
            .sum::<f64>()
            / format_results.len() as f64;
        println!(
            "  {:<25} {:.3} ({:.1}% saved)",
            format,
            avg,
            (1.0 - avg) * 100.0
        );
    }

    // By size
    println!("\n📏 BY FILE SIZE:");
    let mut by_size: Vec<(&TestResult, String)> =
        results.iter().map(|r| (r, format_size(r.size))).collect();
    by_size.sort_by_key(|(r, _)| r.size);

    let small: Vec<_> = by_size
        .iter()
        .filter(|(r, _)| r.size < 10_000)
        .map(|(r, _)| *r)
        .collect();
    let medium: Vec<_> = by_size
        .iter()
        .filter(|(r, _)| r.size >= 10_000 && r.size < 100_000)
        .map(|(r, _)| *r)
        .collect();
    let large: Vec<_> = by_size
        .iter()
        .filter(|(r, _)| r.size >= 100_000)
        .map(|(r, _)| *r)
        .collect();

    if !small.is_empty() {
        let avg = small.iter().map(|r| r.compression_ratio).sum::<f64>() / small.len() as f64;
        println!(
            "  Small (<10KB):        {:.3} ({:.1}% saved)",
            avg,
            (1.0 - avg) * 100.0
        );
    }
    if !medium.is_empty() {
        let avg = medium.iter().map(|r| r.compression_ratio).sum::<f64>() / medium.len() as f64;
        println!(
            "  Medium (10-100KB):    {:.3} ({:.1}% saved)",
            avg,
            (1.0 - avg) * 100.0
        );
    }
    if !large.is_empty() {
        let avg = large.iter().map(|r| r.compression_ratio).sum::<f64>() / large.len() as f64;
        println!(
            "  Large (>100KB):       {:.3} ({:.1}% saved)",
            avg,
            (1.0 - avg) * 100.0
        );
    }

    println!("\n✅ Benchmark complete! Run 'cargo bench stress' to see detailed timings.\n");
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ============================================================================
// REALISTIC DATA GENERATORS
// ============================================================================

fn generate_rust_code(lines: usize) -> String {
    let mut code = String::from("use std::collections::HashMap;\n\n");
    code.push_str("pub struct Example {\n    data: Vec<String>,\n}\n\n");

    for i in 0..lines {
        code.push_str(&format!(
            "fn function_{}() -> Result<(), Error> {{\n    let x = {};\n    Ok(())\n}}\n\n",
            i, i
        ));
    }

    code
}

fn generate_markdown_docs(sections: usize) -> String {
    let mut doc = String::from("# Project Documentation\n\n");

    for i in 0..sections {
        doc.push_str(&format!("## Section {}\n\n", i));
        doc.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
        doc.push_str("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\n");
        doc.push_str("```rust\nfn example() {\n    println!(\"Hello\");\n}\n```\n\n");
    }

    doc
}

fn generate_json_config(entries: usize) -> String {
    let mut json = String::from("{\n  \"version\": \"1.0.0\",\n  \"settings\": {\n");

    for i in 0..entries {
        json.push_str(&format!("    \"key_{}\": \"value_{}\",\n", i, i));
    }

    json.push_str("    \"enabled\": true\n  }\n}\n");
    json
}

fn generate_log_file(entries: usize) -> String {
    let mut log = String::new();
    let levels = ["INFO", "WARN", "ERROR", "DEBUG"];

    for i in 0..entries {
        let level = levels[i % levels.len()];
        log.push_str(&format!(
            "[2025-01-{:02} 12:00:{:02}] {} [thread-{}] Processing request #{}\n",
            (i / 3600) % 31 + 1,
            i % 60,
            level,
            i % 10,
            i
        ));
    }

    log
}

// ============================================================================
// HUMAN CHANGE SCENARIOS
// ============================================================================

fn apply_sequential_additions(base: &str, lines: usize) -> String {
    let mut result = base.to_string();

    for i in 0..lines {
        result.push_str(&format!("    let new_var_{} = {};\n", i, i));
    }

    result
}

fn apply_sequential_deletions(base: &str, remove_count: usize) -> String {
    let lines: Vec<&str> = base.lines().collect();
    if lines.len() <= remove_count {
        return String::new();
    }

    lines[..lines.len() - remove_count].join("\n")
}

fn apply_scattered_edits(base: &str, edit_count: usize) -> String {
    let mut lines: Vec<String> = base.lines().map(|s| s.to_string()).collect();

    for i in 0..edit_count.min(lines.len()) {
        let idx = (i * lines.len() / edit_count) % lines.len();
        lines[idx] = format!("    // EDITED: {}", lines[idx].trim());
    }

    lines.join("\n")
}

fn apply_variable_rename(base: &str, old: &str, new: &str) -> String {
    base.replace(old, new)
}

// ============================================================================
// BENCHMARK HELPER
// ============================================================================

fn measure_compression(scenario: &str, format: &str, base: &[u8], new: &[u8]) -> TestResult {
    let start = Instant::now();
    let delta = delta::encode(0, base, new, true);
    let encode_us = start.elapsed().as_micros();

    let start = Instant::now();
    let _decoded = delta::decode(base, &delta).unwrap();
    let decode_us = start.elapsed().as_micros();

    TestResult {
        scenario: scenario.to_string(),
        format: format.to_string(),
        size: new.len(),
        delta_size: delta.len(),
        compression_ratio: delta.len() as f64 / new.len() as f64,
        encode_us,
        decode_us,
    }
}

// ============================================================================
// BENCHMARKS
// ============================================================================

fn bench_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_files");
    group.sample_size(20);

    // Small Rust file (5KB)
    let base = generate_rust_code(50);
    let new = apply_sequential_additions(&base, 10);
    let result = measure_compression(
        "sequential_additions",
        "rust_small",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.throughput(Throughput::Bytes(new.len() as u64));
    group.bench_function("rust_sequential_add", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    // Small edits
    let base = generate_rust_code(50);
    let new = apply_scattered_edits(&base, 5);
    let result = measure_compression(
        "scattered_edits",
        "rust_small",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.bench_function("rust_scattered_edits", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    group.finish();
}

fn bench_medium_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("medium_files");
    group.sample_size(15);

    // Medium Rust file (50KB)
    let base = generate_rust_code(500);
    let new = apply_sequential_additions(&base, 20);
    let result = measure_compression(
        "sequential_additions",
        "rust_medium",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.throughput(Throughput::Bytes(new.len() as u64));
    group.bench_function("rust_sequential_add", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    // Variable rename (common refactor)
    let base = generate_rust_code(500);
    let new = apply_variable_rename(&base, "function_", "process_");
    let result = measure_compression(
        "variable_rename",
        "rust_medium",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.bench_function("rust_variable_rename", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    // Deletions
    let base = generate_rust_code(500);
    let new = apply_sequential_deletions(&base, 100);
    let result = measure_compression(
        "sequential_deletions",
        "rust_medium",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.bench_function("rust_sequential_delete", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    group.finish();
}

fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_files");
    group.sample_size(10);

    // Large Rust file (200KB)
    let base = generate_rust_code(2000);
    let new = apply_sequential_additions(&base, 50);
    let result = measure_compression(
        "sequential_additions",
        "rust_large",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.throughput(Throughput::Bytes(new.len() as u64));
    group.bench_function("rust_sequential_add", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    group.finish();
}

fn bench_documentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("documentation");
    group.sample_size(15);

    // Add new section
    let base = generate_markdown_docs(20);
    let new = apply_sequential_additions(&base, 5);
    let result = measure_compression(
        "sequential_additions",
        "markdown",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.throughput(Throughput::Bytes(new.len() as u64));
    group.bench_function("markdown_add_section", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    // Edit existing sections
    let base = generate_markdown_docs(20);
    let new = apply_scattered_edits(&base, 5);
    let result = measure_compression(
        "scattered_edits",
        "markdown",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.bench_function("markdown_edit_sections", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    group.finish();
}

fn bench_config_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_files");
    group.sample_size(15);

    // Add config entries
    let base = generate_json_config(50);
    let new = apply_sequential_additions(&base, 5);
    let result = measure_compression(
        "sequential_additions",
        "json",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.throughput(Throughput::Bytes(new.len() as u64));
    group.bench_function("json_add_entries", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    // Edit values
    let base = generate_json_config(50);
    let new = apply_variable_rename(&base, "value_", "updated_");
    let result = measure_compression("value_updates", "json", base.as_bytes(), new.as_bytes());
    record_result(result);

    group.bench_function("json_update_values", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    group.finish();
}

fn bench_log_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("log_files");
    group.sample_size(15);

    // Append new logs (common scenario)
    let base = generate_log_file(1000);
    let new = apply_sequential_additions(&base, 100);
    let result = measure_compression(
        "sequential_additions",
        "logs",
        base.as_bytes(),
        new.as_bytes(),
    );
    record_result(result);

    group.throughput(Throughput::Bytes(new.len() as u64));
    group.bench_function("logs_append", |b| {
        b.iter(|| {
            let delta = delta::encode(
                black_box(0),
                black_box(base.as_bytes()),
                black_box(new.as_bytes()),
                black_box(true),
            );
            delta::decode(black_box(base.as_bytes()), black_box(&delta)).unwrap()
        });
    });

    group.finish();

    // Print summary at the end
    print_summary();
}

// ============================================================================
// CRITERION CONFIGURATION
// ============================================================================

criterion_group!(
    benches,
    bench_small_files,
    bench_medium_files,
    bench_large_files,
    bench_documentation,
    bench_config_files,
    bench_log_files
);

criterion_main!(benches);
