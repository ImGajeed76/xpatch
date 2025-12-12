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
- **Excellent Compression**: 99.4-99.8% average space savings on real-world code changes
- **Fast Performance**: 40-55 GB/s throughput for typical changes
- **Optional zstd Compression**: Additional compression layer for complex changes
- **Metadata Support**: Embed version tags with zero overhead for values 0-15

## Performance

Tested on real-world git repositories with 1.2+ million actual code changes. All operations are single-threaded.

### Real-World Performance (tokio & mdn/content repositories):

**Sequential Mode** (comparing against immediate previous version):

- Encoding: 10-14 µs median
- Decoding: <1 µs (effectively instant)

**Tag Optimization Mode** (searching 16 previous versions for best base):

- Encoding: 104-208 µs median (slower due to trying multiple bases)
- Decoding: <1 µs (effectively instant)

**Compression Results:**

- Code repositories: 2 bytes median (99.8% space saved)
- Documentation: 23 bytes median (99.4% space saved)
- Sequential mode: 25-68 bytes median (98.3-98.4% space saved)

Most real-world changes compress extremely well due to localized edits. See test_results for detailed benchmark data
across different file types and change patterns.

## Installation

Add xpatch to your `Cargo.toml`:

```toml
[dependencies]
xpatch = "0.2.0"
```

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

See `src/bin/cli/README.md` for detailed CLI documentation.

## Benchmark Results

Tested on **1,359,468 real-world Git commit changes** across two repositories (tokio: 133,728 deltas, mdn/content:
1,225,740 deltas). All measurements are single-threaded performance.

**Hardware**: AMD Ryzen 7 7800X3D (16 threads), 64GB DDR5 RAM, Fedora Linux

### tokio (Rust Async Runtime - Code Repository)

| Algorithm         | Median Delta | Compression Ratio | Space Saved | Median Encode | Median Decode |
|-------------------|--------------|-------------------|-------------|---------------|---------------|
| **xpatch_tags**   | **2 bytes**  | **0.0019**        | **99.8%**   | 208 µs        | 0 µs          |
| xpatch_sequential | 68 bytes     | 0.0165            | 98.4%       | 14 µs         | 0 µs          |
| vcdiff (xdelta3)  | 97 bytes     | 0.0276            | 97.2%       | 15 µs         | 3 µs          |
| gdelta            | 69 bytes     | 0.0180            | 98.2%       | 1 µs          | 0 µs          |

**Tag optimization impact**: 88.7% smaller deltas (median) compared to sequential mode.

### mdn/content (MDN Web Docs - Documentation Repository)

| Algorithm         | Median Delta | Compression Ratio | Space Saved | Median Encode | Median Decode |
|-------------------|--------------|-------------------|-------------|---------------|---------------|
| **xpatch_tags**   | **23 bytes** | **0.0063**        | **99.4%**   | 104 µs        | 0 µs          |
| xpatch_sequential | 25 bytes     | 0.0069            | 99.3%       | 10 µs         | 0 µs          |
| vcdiff (xdelta3)  | 50 bytes     | 0.0169            | 98.3%       | 9 µs          | 2 µs          |
| gdelta            | 26 bytes     | 0.0077            | 99.2%       | 0 µs          | 0 µs          |

**Tag optimization impact**: 8.8% smaller deltas (median) compared to sequential mode.

### Key Insights

- **Code repositories benefit 10x more from tag optimization** (88.7% improvement) than documentation (8.8% improvement)
- The median delta of **2 bytes** on tokio means many changes can be represented by just the header
- Tag system averages 1.9 commits back for tokio (median: 2), showing frequent code reversion patterns

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

- `tag`: User-defined metadata value (tags 0-15 use zero overhead)
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

### Quick Stress Tests

Tests human-focused scenarios (code edits, documentation, config files):

```bash
cargo bench --bench stress
```

### Real-World Git Repository Benchmarks

Test on actual git repositories with environment variable configuration:

```bash
# Use a preset repository
XPATCH_PRESET=tokio cargo bench --bench git_real_world

# Test all files at HEAD
XPATCH_PRESET=tokio XPATCH_ALL_FILES_HEAD=true cargo bench --bench git_real_world

# Build cache for faster repeated runs
XPATCH_PRESET=tokio XPATCH_BUILD_CACHE=true XPATCH_CACHE_DIR=./cache cargo bench --bench git_real_world

# Use cache
XPATCH_PRESET=tokio XPATCH_USE_CACHE=true XPATCH_CACHE_DIR=./cache cargo bench --bench git_real_world

# Customize search depth and other options
XPATCH_PRESET=tokio XPATCH_MAX_TAG_DEPTH=32 XPATCH_MAX_COMMITS=200 cargo bench --bench git_real_world
```

**Available Environment Variables:**

- `XPATCH_PRESET`: Repository preset (rust, neovim, tokio, git)
- `XPATCH_REPO`: Custom repository URL
- `XPATCH_MAX_COMMITS`: Maximum commits per file (default: 50, 0=all)
- `XPATCH_MAX_TAG_DEPTH`: Tag search depth (default: 16)
- `XPATCH_ALL_FILES_HEAD`: Test all files at HEAD
- `XPATCH_ALL_FILES`: Test all files in history (slow)
- `XPATCH_MAX_FILES`: Limit number of files
- `XPATCH_PARALLEL_FILES`: Process files in parallel
- `XPATCH_OUTPUT`: Output directory
- `XPATCH_CACHE_DIR`: Cache directory
- `XPATCH_BUILD_CACHE`: Build cache only
- `XPATCH_USE_CACHE`: Use existing cache

Results are saved to timestamped files in `benchmark_results/` with both Json and Markdown reports.

## Related Projects

- [gdelta](https://github.com/ImGajeed76/gdelta) - General-purpose delta compression algorithm used by xpatch

## License

xpatch is dual-licensed: AGPL-3.0-or-later for open source, with a commercial option for proprietary use.

### The Philosophy

I'm a huge fan of open source. I also don't want massive corporations extracting value from community work without
giving anything back. AGPL solves this - if you modify xpatch and distribute it (including running it as a service),
those improvements stay open.

That said, I'm not trying to build a licensing business here. This is about fairness, not revenue.

### Do You Need a Commercial License?

**Probably not if you're:**

- Building open source software (AGPL is perfect)
- A small team or indie developer
- Experimenting or doing research
- A startup figuring things out

**Maybe if you're:**

- A large company with AGPL restrictions
- Integrating this into proprietary infrastructure at scale
- Need legal certainty for closed-source use

### How Commercial Licensing Works

Email me at xpatch-commercial@alias.oseifert.ch and let's talk.

Small businesses? Probably free - I just want to know who's using it and how.

Larger companies? Yeah, I'll ask for something, but it'll be reasonable. You have the resources to support open source
work, so let's make it fair.

Would rather contribute code than pay? Even better. Help make xpatch better and we'll figure out the licensing stuff.

I'm not interested in complex contracts or pricing games. Just don't be a massive corp that takes community work and
gives nothing back. That's literally the only thing I'm trying to prevent.

### Contributor License Agreement

If you contribute code, you're granting us rights to use it under both AGPL and commercial terms. This sounds scarier
than it is - it just means we can handle licensing requests without tracking down every contributor for permission.

The AGPL version stays open forever. This just gives us flexibility to be reasonable with companies that need commercial
licenses.

See LICENSE-AGPL.txt for the full text, or LICENSE-COMMERCIAL.txt for commercial terms.

## Contributing

Contributions are welcome. Please open an issue or pull request on GitHub.