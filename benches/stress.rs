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
use std::sync::Mutex;
use std::time::Instant;
use xpatch::delta;

// ============================================================================
// COMPRESSION STATISTICS
// ============================================================================

#[derive(Clone)]
struct CompressionStats {
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
            "║                         UNIFIED BENCHMARK SUMMARY                                  ║"
        );
        println!(
            "╚════════════════════════════════════════════════════════════════════════════════════╝\n"
        );

        // Calculate aggregates
        let avg_ratio: f64 =
            stats_list.iter().map(|s| s.delta_ratio).sum::<f64>() / stats_list.len() as f64;
        let avg_savings: f64 =
            stats_list.iter().map(|s| s.space_savings).sum::<f64>() / stats_list.len() as f64;
        let best_ratio = stats_list
            .iter()
            .map(|s| s.delta_ratio)
            .fold(f64::INFINITY, f64::min);
        let worst_ratio = stats_list
            .iter()
            .map(|s| s.delta_ratio)
            .fold(f64::NEG_INFINITY, f64::max);

        println!("📊 COMPRESSION METRICS:");
        println!("  Average delta ratio:     {:.3}", avg_ratio);
        println!("  Average space savings:   {:.1}%", avg_savings);
        println!("  Best compression:        {:.3}", best_ratio);
        println!("  Worst compression:       {:.3}\n", worst_ratio);

        let avg_encode_time: u128 =
            stats_list.iter().map(|s| s.encode_time_us).sum::<u128>() / stats_list.len() as u128;
        let avg_decode_time: u128 =
            stats_list.iter().map(|s| s.decode_time_us).sum::<u128>() / stats_list.len() as u128;

        println!("⚡ PERFORMANCE METRICS:");
        println!("  Average encode time:     {} µs", avg_encode_time);
        println!("  Average decode time:     {} µs\n", avg_decode_time);

        // Detailed table
        println!(
            "{:<40} {:>10} {:>10} {:>8} {:>9} {:>12} {:>12}",
            "Test Case", "Original", "Delta", "Ratio", "Savings", "Encode (µs)", "Decode (µs)"
        );
        println!("{}", "─".repeat(110));

        for stat in stats_list.iter() {
            println!(
                "{:<40} {:>10} {:>10} {:>8.3} {:>8.1}% {:>12} {:>12}",
                truncate_str(&stat.test_name, 40),
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

fn truncate_text(base: &[u8], remove_size: usize) -> Vec<u8> {
    let new_len = base.len().saturating_sub(remove_size);
    base[..new_len].to_vec()
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

fn complete_replacement(base: &[u8]) -> Vec<u8> {
    vec![b'Z'; base.len()]
}

// ============================================================================
// HELPER: Run and collect stats
// ============================================================================

fn measure_and_collect(test_name: &str, base: &[u8], new_data: &[u8], enable_zstd: bool) {
    // Measure encode time
    let start = Instant::now();
    let delta = delta::encode(0, base, new_data, enable_zstd);
    let encode_time = start.elapsed().as_micros();

    // Measure decode time
    let start = Instant::now();
    let _decoded = delta::decode(base, &delta[..]).unwrap();
    let decode_time = start.elapsed().as_micros();

    let stats = CompressionStats::calculate(
        test_name.to_string(),
        base,
        new_data,
        &delta[..],
        encode_time,
        decode_time,
    );

    collect_stats(stats);
}

// ============================================================================
// BENCHMARK: DIFFERENT SIZES
// ============================================================================

fn bench_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sizes");
    group.sample_size(10); // Reduced from 20

    for size in [100_000, 1_000_000, 5_000_000] {
        let base = generate_realistic_text(size);
        let new_data = small_insert_at(&base[..], 0.5);

        // Collect stats once
        measure_and_collect(
            &format!("size_{}", format_bytes(size)).as_str(),
            &base[..],
            &new_data[..],
            false,
        );

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("encode", size),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(false),
                    )
                });
            },
        );

        let delta = delta::encode(0, &base[..], &new_data[..], false);
        group.bench_with_input(
            BenchmarkId::new("decode", size),
            &(&base, &delta),
            |b, (base, delta)| {
                b.iter(|| delta::decode(black_box(&base[..]), black_box(&delta[..])).unwrap());
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK: DATA PATTERNS
// ============================================================================

fn bench_data_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_patterns");
    let size = 100_000;

    let test_cases = vec![
        ("realistic", generate_realistic_text(size)),
        ("code", generate_code_text(size)),
        ("repetitive", generate_repetitive_data(size)),
        ("json", generate_json_data(size)),
    ];

    for (name, base) in test_cases {
        let new_data = small_insert_at(&base[..], 0.5);

        // Collect stats once
        measure_and_collect(
            &format!("pattern_{}", name).as_str(),
            &base[..],
            &new_data[..],
            false,
        );

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("encode", name),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(false),
                    )
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK: CHANGE TYPES (Key representative cases)
// ============================================================================

fn bench_change_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("change_types");
    let size = 100_000;
    let base = generate_realistic_text(size);

    group.throughput(Throughput::Bytes(size as u64));

    // Representative change types
    let test_cases = vec![
        ("insert_start", small_insert_at(&base[..], 0.0)),
        ("insert_middle", small_insert_at(&base[..], 0.5)),
        ("insert_end", small_insert_at(&base[..], 1.0)),
        ("append_small", append_text(&base[..], 100)),
        ("append_large", append_text(&base[..], 10000)),
        ("truncate_small", truncate_text(&base[..], 100)),
        ("truncate_large", truncate_text(&base[..], 10000)),
        ("mixed", mixed_operations(&base[..])),
        ("worst_case", complete_replacement(&base[..])),
    ];

    for (name, new_data) in &test_cases {
        // Collect stats once
        measure_and_collect(
            &format!("change_{}", name).as_str(),
            &base[..],
            &new_data[..],
            true,
        );

        group.bench_with_input(
            BenchmarkId::new("encode", name),
            &(&base, new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(true),
                    )
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK: ZSTD COMPARISON
// ============================================================================

fn bench_zstd_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("zstd_comparison");
    let size = 100_000;
    let base = generate_realistic_text(size);
    let new_data = mixed_operations(&base[..]);

    group.throughput(Throughput::Bytes(size as u64));

    // Without zstd
    measure_and_collect("zstd_disabled", &base[..], &new_data[..], false);

    group.bench_with_input(
        BenchmarkId::new("encode", "without_zstd"),
        &(&base, &new_data),
        |b, (base, new_data)| {
            b.iter(|| {
                delta::encode(
                    black_box(0),
                    black_box(&base[..]),
                    black_box(&new_data[..]),
                    black_box(false),
                )
            });
        },
    );

    // With zstd
    measure_and_collect("zstd_enabled", &base[..], &new_data[..], true);

    group.bench_with_input(
        BenchmarkId::new("encode", "with_zstd"),
        &(&base, &new_data),
        |b, (base, new_data)| {
            b.iter(|| {
                delta::encode(
                    black_box(0),
                    black_box(&base[..]),
                    black_box(&new_data[..]),
                    black_box(true),
                )
            });
        },
    );

    group.finish();
}

// ============================================================================
// BENCHMARK: ROUNDTRIP
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for size in [10_000, 100_000, 1_000_000] {
        let base = generate_realistic_text(size);
        let new_data = mixed_operations(&base[..]);

        // Collect stats for roundtrip
        measure_and_collect(
            &format!("roundtrip_{}", format_bytes(size)).as_str(),
            &base[..],
            &new_data[..],
            true,
        );

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("full_cycle", size),
            &(&base, &new_data),
            |b, (base, new_data)| {
                b.iter(|| {
                    let delta = delta::encode(
                        black_box(0),
                        black_box(&base[..]),
                        black_box(&new_data[..]),
                        black_box(true),
                    );

                    let decoded =
                        delta::decode(black_box(&base[..]), black_box(&delta[..])).unwrap();

                    assert_eq!(decoded.len(), new_data.len());
                    black_box(decoded)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK: EDGE CASES
// ============================================================================

fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_cases");

    // Empty to data
    let base_empty = Vec::new();
    let new_from_empty = b"Hello, World!".to_vec();
    measure_and_collect(
        "edge_empty_to_data",
        &base_empty[..],
        &new_from_empty[..],
        false,
    );

    group.bench_with_input(
        BenchmarkId::new("encode", "empty_to_data"),
        &(&base_empty, &new_from_empty),
        |b, (base, new_data)| {
            b.iter(|| {
                delta::encode(
                    black_box(0),
                    black_box(&base[..]),
                    black_box(&new_data[..]),
                    black_box(false),
                )
            });
        },
    );

    // Data to empty
    let base_data = b"Hello, World!".to_vec();
    let new_empty = Vec::new();
    measure_and_collect("edge_data_to_empty", &base_data[..], &new_empty[..], false);

    group.bench_with_input(
        BenchmarkId::new("encode", "data_to_empty"),
        &(&base_data, &new_empty),
        |b, (base, new_data)| {
            b.iter(|| {
                delta::encode(
                    black_box(0),
                    black_box(&base[..]),
                    black_box(&new_data[..]),
                    black_box(false),
                )
            });
        },
    );

    // Single byte change
    let base_single = generate_realistic_text(10000);
    let mut new_single = base_single.to_owned();
    new_single[5000] = b'X';
    measure_and_collect("edge_single_byte", &base_single[..], &new_single[..], false);

    group.bench_with_input(
        BenchmarkId::new("encode", "single_byte_change"),
        &(&base_single, &new_single),
        |b, (base, new_data)| {
            b.iter(|| {
                delta::encode(
                    black_box(0),
                    black_box(&base[..]),
                    black_box(&new_data[..]),
                    black_box(false),
                )
            });
        },
    );

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
        .warm_up_time(std::time::Duration::from_secs(2)) // Reduced from 3
}

// ============================================================================
// FINALIZE AND PRINT SUMMARY
// ============================================================================

criterion_group! {
    name = core_benches;
    config = configure_criterion();
    targets = bench_sizes, bench_data_patterns, bench_change_types
}

criterion_group! {
    name = advanced_benches;
    config = configure_criterion();
    targets = bench_zstd_comparison, bench_roundtrip, bench_edge_cases
}

// Use criterion_main which will print our summary at the end
criterion_main!(core_benches, advanced_benches);
