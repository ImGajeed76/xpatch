# xpatch

[![Crates.io](https://img.shields.io/crates/v/xpatch.svg)](https://crates.io/crates/xpatch)
[![Documentation](https://docs.rs/xpatch/badge.svg)](https://docs.rs/xpatch)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

A high-performance delta compression library with automatic algorithm selection.

## Features

- **Automatic Algorithm Selection**: Analyzes changes and chooses the best compression strategy
- **Excellent Compression**: 99.4-99.8% average space savings on real-world code changes
- **Fast Performance**: 40-55 GB/s throughput for typical changes, <1 µs decoding
- **Optional zstd Compression**: Additional compression layer for complex changes
- **Metadata Support**: Embed version tags with zero overhead for values 0-15
- **Multi-language**: Also available for [Python](https://pypi.org/project/xpatch-rs/) and [Node.js](https://www.npmjs.com/package/xpatch-rs)

## Installation

```toml
[dependencies]
xpatch = "0.3.1"
```

**Requirements:**
- Rust edition 2024 or later (Rust 1.92.0+)

## Quick Start

```rust
use xpatch::delta;

fn main() {
    let base = b"Hello, World!";
    let new = b"Hello, Rust!";

    // Create delta
    let delta = delta::encode(0, base, new, true);

    // Apply delta
    let reconstructed = delta::decode(base, &delta).unwrap();
    assert_eq!(reconstructed, new);

    // Extract tag
    let tag = delta::get_tag(&delta).unwrap();
    println!("Compressed {} → {} bytes", new.len(), delta.len());
}
```

## API

### `encode`

```rust
pub fn encode(tag: usize, base_data: &[u8], new_data: &[u8], enable_zstd: bool) -> Vec<u8>
```

Creates a delta that transforms `base_data` into `new_data`.

- `tag`: User-defined metadata value (tags 0-15 use zero overhead)
- `base_data`: The original data
- `new_data`: The target data
- `enable_zstd`: Enable zstd compression for complex changes (slower but better compression)

Returns: Compact delta as bytes

### `decode`

```rust
pub fn decode(base_data: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str>
```

Applies a delta to reconstruct the new data.

- `base_data`: The original data the delta was created from
- `delta`: The encoded delta

Returns: Reconstructed data or error

### `get_tag`

```rust
pub fn get_tag(delta: &[u8]) -> Result<usize, &'static str>
```

Extracts the tag value from a delta without decoding it.

Returns: Tag value or error

## CLI Tool

Install the CLI tool with:

```bash
cargo install xpatch --features cli
```

Usage:

```bash
# Create a delta
xpatch encode base.txt new.txt -o patch.xp

# Apply a delta
xpatch decode base.txt patch.xp -o restored.txt

# Show delta info
xpatch info patch.xp
```

## Performance

Tested on 1.2+ million real-world git changes:

- **Code repositories**: 2 bytes median (99.8% space saved)
- **Documentation**: 23 bytes median (99.4% space saved)
- **Encoding**: 10-208 µs depending on optimization mode
- **Decoding**: <1 µs (effectively instant)

## How It Works

xpatch analyzes the change pattern and automatically selects the most efficient algorithm:

1. **Change Analysis**: Detects whether the change is a simple insertion, removal, or complex modification
2. **Pattern Detection**: Identifies repetitive patterns that can be compressed efficiently
3. **Algorithm Selection**: Tests multiple specialized algorithms and chooses the smallest output
4. **Encoding**: Creates a compact delta with algorithm metadata in the header

For complex changes, xpatch uses [gdelta](https://github.com/ImGajeed76/gdelta), a general-purpose delta compression algorithm, with optional zstd compression.

## Examples

See the [examples](examples/) directory:

- [`basic.rs`](examples/basic.rs) - Basic usage
- [`tags.rs`](examples/tags.rs) - Using tags for version optimization

Run examples with:

```bash
cargo run --example basic
cargo run --example tags
```

## Benchmarks

Run the comprehensive benchmark suite:

```bash
# Quick stress tests
cargo bench --bench stress

# Real-world git repository benchmarks
XPATCH_PRESET=tokio cargo bench --bench git_real_world
```

## Related Projects

- [xpatch repository](https://github.com/ImGajeed76/xpatch) - Multi-language bindings and documentation
- [gdelta](https://github.com/ImGajeed76/gdelta) - General-purpose delta compression
- [xpatch Demo Editor](https://github.com/ImGajeed76/xpatch_demo_editor) - Live demo

## License

Dual-licensed under AGPL-3.0-or-later or commercial license. See [LICENSE-AGPL.txt](../../LICENSE-AGPL.txt) and [LICENSE-COMMERCIAL.txt](../../LICENSE-COMMERCIAL.txt).

For commercial licensing inquiries: xpatch-commercial@alias.oseifert.ch
