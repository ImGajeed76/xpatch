# xpatch

[![Crates.io](https://img.shields.io/crates/v/xpatch.svg)](https://crates.io/crates/xpatch)
[![Documentation](https://docs.rs/xpatch/badge.svg)](https://docs.rs/xpatch)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

A high-performance delta compression library for Rust that automatically selects the optimal compression algorithm based
on the type of change detected between data versions.

## Features

- **Automatic Algorithm Selection**: Analyzes changes and chooses the best compression strategy
- **Multiple Compression Algorithms**:
    - Simple character insertion (Chars)
    - Token-based compression (Tokens)
    - Byte removal (Remove)
    - Repetitive pattern detection (RepeatChars, RepeatTokens)
    - General-purpose delta compression (GDelta, GDeltaZstd)
- **Excellent Compression Ratios**: Achieves 99.5% average space savings on real-world code changes
- **Fast Performance**: 40-55 GB/s throughput for typical changes
- **Optional zstd Compression**: Additional compression layer for complex changes
- **Metadata Support**: Embed version tags with zero overhead for values 0-15

## Performance

All encoding and decoding operations are single-threaded. The benchmark infrastructure uses multiple threads to process
many deltas in parallel, but each individual encoding is sequential.

**Typical Performance** (simple insertions, deletions, small changes):

- Encoding: 1-5 microseconds (40-55 GB/s)
- Decoding: 1-5 microseconds (40-55 GB/s)

**Slower Cases** (complex changes requiring GDelta):

- Encoding: 50-200 microseconds (0.5-2 GB/s)
- Decoding: 5-50 microseconds (2-20 GB/s)

**Edge Cases** (worst-case scenarios, complete rewrites):

- Encoding: up to 200 microseconds for 100KB files (~500 MB/s)
- Decoding: 30-50 microseconds (~2-3 GB/s)

Most real-world changes fall into the "typical" category. See [test_results](./test_results) for detailed benchmark
data.

## Installation

Add xpatch to your `Cargo.toml`:

```toml
[dependencies]
xpatch = "0.1.0"
```

## License

This project is dual-licensed under:

### Option 1: AGPL-3.0-or-later (Free for Open Source)
Free to use in open source projects that comply with the AGPL license.
If you modify xpatch and distribute it (including as a web service),
you must open-source your modifications under AGPL.

### Option 2: Commercial License (For Proprietary Use)
For companies that want to use xpatch in closed-source products,
a commercial license is available. Pricing is flexible.

**To purchase a commercial license or request a quote:**
Email: xpatch-commercial@alias.oseifert.ch

### Contributor License Agreement
All contributors must sign a CLA that grants us rights to relicense
their contributions under both AGPL and commercial terms.

See LICENSE-AGPL.txt for the full AGPL license text.
See LICENSE-COMMERCIAL.txt for commercial license terms.

## Quick Start

```rust
use xpatch::delta;

fn main() {
    let base_data = b"Hello, world!";
    let new_data = b"Hello, beautiful world!";

    // Encode the difference
    let tag = 0; // User-defined metadata
    let enable_zstd = true;
    let delta = delta::encode(tag, base_data, new_data, enable_zstd);

    println!("Original size: {} bytes", base_data.len());
    println!("Delta size: {} bytes", delta.len());
    println!("Compression ratio: {:.2}%",
             (1.0 - delta.len() as f64 / new_data.len() as f64) * 100.0);

    // Decode to reconstruct new_data
    let reconstructed = delta::decode(base_data, &delta[..]).unwrap();
    assert_eq!(reconstructed, new_data);

    // Extract metadata without decoding
    let extracted_tag = delta::get_tag(&delta[..]).unwrap();
    assert_eq!(extracted_tag, tag);
}
```

### Running the Examples

Try the included examples to see xpatch in action:

```bash
# Basic compression example
cargo run --example basic

# Tags example demonstrating metadata and version optimization
cargo run --example tags

# Expected output will show compression ratios and delta sizes
```

## Command-Line Tool

xpatch includes a convenient CLI tool for working with deltas:

```bash
# Install with CLI support
cargo install xpatch --features cli

# Or build from source
cargo build --release --features cli
```

### Basic Usage

```bash
# Create a delta
xpatch encode base.txt new.txt -o patch.xp

# Apply a delta
xpatch decode base.txt patch.xp -o restored.txt

# Show delta info
xpatch info patch.xp
```

See `src/bin/xpatch/README.md` for detailed CLI documentation.

## Benchmark Results

Tested on 337,378 real-world Git commit diffs across three repositories (Git, Neovim, Tokio). All measurements are
single-threaded performance.

**Hardware**: AMD Ryzen 7 7800X3D (16 threads), 64GB DDR5 RAM, Fedora Linux

| Algorithm | Avg Compression Ratio | Avg Space Savings | Avg Encode Time | Avg Decode Time |
|-----------|-----------------------|-------------------|-----------------|-----------------|
| xpatch    | 0.0043                | 99.6%             | 0.69 ms         | 0.03 ms         |
| xdelta3   | 0.0197                | 98.0%             | 0.12 ms         | 0.01 ms         |
| qbsdiff   | 0.0073                | 99.3%             | 17.29 ms        | 1.66 ms         |

**Win Rate** (best compression in head-to-head comparison): xpatch wins 95.4% of cases.

See the [test_results](./test_results) directory for detailed logs and benchmark data.

## How It Works

xpatch analyzes the change pattern between two byte sequences and automatically selects the most efficient algorithm:

1. **Change Analysis**: Detects whether the change is a simple insertion, removal, or complex modification
2. **Pattern Detection**: Identifies repetitive patterns that can be compressed efficiently
3. **Algorithm Selection**: Tests multiple specialized algorithms and chooses the smallest output
4. **Encoding**: Creates a compact delta with algorithm metadata in the header

For complex changes, xpatch uses [gdelta](https://github.com/ImGajeed76/gdelta), a general-purpose delta compression
algorithm, with optional zstd compression.

## API Documentation

### Encoding

```rust
pub fn encode(tag: usize, base_data: &[u8], new_data: &[u8], enable_zstd: bool) -> Vec<u8>
```

Creates a delta that transforms `base_data` into `new_data`.

- `tag`: User-defined metadata value
- `base_data`: The original data
- `new_data`: The target data
- `enable_zstd`: Enable zstd compression for complex changes (slower but better compression)

**Returns**: Compact delta as a byte vector

### Decoding

```rust
pub fn decode(base_data: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str>
```

Applies a delta to reconstruct the new data.

- `base_data`: The original data the delta was created from
- `delta`: The encoded delta

**Returns**: Reconstructed data or error message

### Metadata Extraction

```rust
pub fn get_tag(delta: &[u8]) -> Result<usize, &'static str>
```

Extracts the tag value from a delta without decoding it.

**Returns**: Tag value or error message

## Understanding Tags

The tag parameter provides a way to embed metadata directly into your deltas. Tags enable an important optimization in
version control systems: you can choose which previous version to use as the base for creating a delta, not just the
immediate predecessor.

### Efficient Storage

Tags from 0-15 use only a single byte in the delta header alongside the algorithm type, adding zero overhead. Larger
tags use variable-length encoding.

### Example: Comparing Against Older Versions

Consider this scenario where data reverts to a previous state:

```rust
use xpatch::delta;

fn main() {
    let v1 = b"Hello";
    let v2 = b"Hello, World!";
    let v3 = b"Hello";  // Same as v1!

    println!("=== Naive Approach ===");
    // Always compare with immediate predecessor
    let delta_v1_to_v2 = delta::encode(0, v1, v2, false);
    println!("v1 -> v2 delta size: {} bytes", delta_v1_to_v2.len());

    let delta_v2_to_v3 = delta::encode(0, v2, v3, false);
    println!("v2 -> v3 delta size: {} bytes", delta_v2_to_v3.len());

    let naive_total = delta_v1_to_v2.len() + delta_v2_to_v3.len();
    println!("Naive total: {} bytes\n", naive_total);

    println!("=== Optimized Approach ===");
    // Compare v3 with v1 instead - they're identical!
    let delta_v1_to_v3 = delta::encode(1, v1, v3, false);
    println!("v1 -> v3 delta size: {} bytes", delta_v1_to_v3.len());
    println!("Tag=1 indicates base version\n");

    // Verify decoding works
    let reconstructed = delta::decode(v1, &delta_v1_to_v3[..]).unwrap();
    assert_eq!(reconstructed, v3);

    let tag = delta::get_tag(&delta_v1_to_v3[..]).unwrap();
    println!("Tag extracted: {}", tag);
}
```

**Output:**

```
=== Naive Approach ===
v1 -> v2 delta size: 9 bytes
v2 -> v3 delta size: 3 bytes
Naive total: 12 bytes

=== Optimized Approach ===
v1 -> v3 delta size: 2 bytes
Tag=1 indicates base version

Tag extracted: 1
```

By checking older versions in your history, you can find the optimal base that produces the smallest delta. The tag
stores which version was used as the base, allowing your decoder to retrieve the correct version during reconstruction.
This is particularly effective when changes are reverted or when data has cyclical patterns.

## Running Benchmarks

The repository includes comprehensive benchmark suites:

### Synthetic Benchmarks

```bash
# Standard compression tests
cargo bench --bench stress --features benchmark

# Library comparison
cargo bench --bench stress_compared --features benchmark
```

### Real-World Git Benchmarks

```bash
# Install xdelta3 (required for git benchmark comparisons)
apt-get install xdelta3  # or: brew install xdelta on macOS

# Run benchmark on selected repositories
cargo run --bin git_benchmark --features benchmark -- \
    --repos git,neovim,tokio \
    --max-commits 100 \
    --threads 8
```

Results are saved to the `benchmark_results` directory with timestamped CSV files. Benchmark scripts are included in the
repository.

## Related Projects

- [gdelta](https://github.com/ImGajeed76/gdelta) - General-purpose delta compression algorithm used by xpatch

## Contributing

Contributions are welcome. Please open an issue or pull request on GitHub.