# xpatch

[![Crates.io](https://img.shields.io/crates/v/xpatch.svg)](https://crates.io/crates/xpatch)
[![npm](https://img.shields.io/npm/v/xpatch-rs.svg)](https://www.npmjs.com/package/xpatch-rs)
[![PyPI](https://img.shields.io/pypi/v/xpatch-rs.svg)](https://pypi.org/project/xpatch-rs/)
[![Documentation](https://docs.rs/xpatch/badge.svg)](https://docs.rs/xpatch)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

A high-performance delta compression library with automatic algorithm selection, available for **Rust**, **C/C++**, **Python**, **Node.js**, **WebAssembly**, and as a **CLI tool**.

## Demo

**[🚀 Try the live demo →](https://github.com/ImGajeed76/xpatch_demo_editor)**

A lightning-fast markdown editor showcasing xpatch's compression and time-travel capabilities. Watch it achieve
crazy space savings while scrubbing through document history like a video timeline.

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
- **Multi-language**: Native bindings for Rust, C/C++, Python, Node.js, and WebAssembly

## Installation

### Rust

```toml
[dependencies]
xpatch = "0.3.1"
```

**Requirements:**
- Rust 1.92.0+ (for Rust edition 2024 support)

### C/C++

```bash
# Build the library (using axogen)
axogen run build c

# Or manually with cargo
cd crates/xpatch-c && cargo build --release

# Build and test the example
axogen run build c --example
```

The build produces:
- **Library**: `target/release/libxpatch_c.{so,dylib,dll}`
- **Header**: `crates/xpatch-c/xpatch.h`

See [crates/xpatch-c/README.md](crates/xpatch-c/README.md) for usage examples and API reference.

### Python

```bash
pip install xpatch-rs
```

### Node.js

```bash
npm install xpatch-rs
```

### WebAssembly

```bash
# Build for web (browser)
axogen run build wasm --target web

# Build for Node.js
axogen run build wasm --target nodejs

# Build for bundlers (webpack, vite, rollup)
axogen run build wasm --target bundler
```

See [crates/xpatch-wasm/README.md](crates/xpatch-wasm/README.md) for usage examples and API reference.

### CLI Tool

```bash
cargo install xpatch --features cli
```

## Quick Start

### Rust

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

### C/C++

```c
#include <stdio.h>
#include <string.h>
#include "xpatch.h"

int main() {
    const char* base = "Hello, World!";
    const char* new = "Hello, C!";

    // Encode
    struct xpatch_XPatchBuffer delta = xpatch_encode(
        0, (const uint8_t*)base, strlen(base),
        (const uint8_t*)new, strlen(new), true
    );

    printf("Compressed %zu → %zu bytes\n", strlen(new), delta.len);

    // Decode
    struct xpatch_XPatchResult result = xpatch_decode(
        (const uint8_t*)base, strlen(base),
        delta.data, delta.len
    );

    if (result.error_message == NULL) {
        printf("Success!\n");
        xpatch_free_buffer(result.buffer);
    }

    xpatch_free_buffer(delta);
    return 0;
}
```

### Python

```python
import xpatch

base = b"Hello, World!"
new = b"Hello, Python!"

# Create delta
delta = xpatch.encode(0, base, new)

# Apply delta
reconstructed = xpatch.decode(base, delta)
assert reconstructed == new

# Extract tag
tag = xpatch.get_tag(delta)
print(f"Compressed {len(new)} → {len(delta)} bytes")
```

### Node.js / TypeScript

```javascript
const xpatch = require('xpatch-rs');

const base = Buffer.from('Hello, World!');
const newData = Buffer.from('Hello, Node!');

// Create delta
const delta = xpatch.encode(0, base, newData);

// Apply delta
const reconstructed = xpatch.decode(base, delta);
console.log(reconstructed.equals(newData)); // true

// Extract tag
const tag = xpatch.getTag(delta);
console.log(`Compressed ${newData.length} → ${delta.length} bytes`);
```

### WebAssembly

```javascript
import init, { encode, decode, get_tag } from './pkg/xpatch_wasm.js';

await init();

const encoder = new TextEncoder();
const base = encoder.encode("Hello, World!");
const newData = encoder.encode("Hello, WASM!");

// Create delta
const delta = encode(0, base, newData, true);

// Apply delta
const reconstructed = decode(base, delta);
console.log(new TextDecoder().decode(reconstructed)); // "Hello, WASM!"

// Extract tag
const tag = get_tag(delta);
console.log(`Compressed ${newData.length} → ${delta.length} bytes`);
```

### CLI

```bash
# Create a delta
xpatch encode base.txt new.txt -o patch.xp

# Apply a delta
xpatch decode base.txt patch.xp -o restored.txt

# Show delta info
xpatch info patch.xp
```

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

## Repository Structure

```
xpatch/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── xpatch/            # Core Rust library with CLI
│   ├── xpatch-c/          # C/C++ bindings (cbindgen + FFI)
│   ├── xpatch-python/     # Python bindings (PyO3 + Maturin)
│   └── xpatch-node/       # Node.js bindings (NAPI-RS)
└── README.md
```

## Development

### Quick Start for Contributors

After cloning the repository:

```bash
# Install dependencies (requires Bun)
bun install

# Run interactive setup (detects tools, builds everything)
axogen run setup
```

### Common Commands

```bash
# Build components
axogen run build rust          # Core Rust library
axogen run build c             # C/C++ bindings
axogen run build python        # Python bindings
axogen run build node          # Node.js bindings
axogen run build all           # Everything

# Run tests
axogen run test                # All tests
axogen run test rust           # Rust only
axogen run test python         # Python only
axogen run test node           # Node only

# Local testing (prepare packages for use in other projects)
axogen run local rust          # Prepare Rust library
axogen run local python        # Prepare Python package
axogen run local node          # Prepare Node.js package

# Code quality
axogen run fmt                 # Format all code
axogen run lint                # Lint all code

# Examples
axogen run example basic       # Run basic example
axogen run example tags        # Run tags example

# Quick reference
axogen run howto               # Show all documentation
axogen run howto build         # Build instructions
axogen run howto bench         # Benchmark guide
axogen run howto local         # Local testing guide
```

For complete development documentation, see [DEVELOPMENT.md](DEVELOPMENT.md).

### Manual Build Commands

If you prefer not to use Axogen:

```bash
# Rust
cargo build --all
cargo test -p xpatch

# Python
cd crates/xpatch-python
pip install maturin
maturin develop

# Node.js
cd crates/xpatch-node
npm install && npm run build
```

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

See the [test_results](./crates/xpatch/test_results) directory for detailed logs and benchmark data.

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

**Rust:**
```rust
pub fn encode(tag: usize, base_data: &[u8], new_data: &[u8], enable_zstd: bool) -> Vec<u8>
```

**Python:**
```python
def encode(tag: int, base_data: bytes, new_data: bytes, enable_zstd: bool = True) -> bytes
```

**Node.js:**
```typescript
function encode(tag: number, baseData: Buffer, newData: Buffer, enableZstd?: boolean): Buffer
```

Creates a delta that transforms `base_data` into `new_data`.

- `tag`: User-defined metadata value (tags 0-15 use zero overhead)
- `base_data`: The original data
- `new_data`: The target data
- `enable_zstd`: Enable zstd compression for complex changes (slower but better compression)

**Returns**: Compact delta as bytes

### Decoding

**Rust:**
```rust
pub fn decode(base_data: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str>
```

**Python:**
```python
def decode(base_data: bytes, delta: bytes) -> bytes  # Raises ValueError on error
```

**Node.js:**
```typescript
function decode(baseData: Buffer, delta: Buffer): Buffer  // Throws Error on failure
```

Applies a delta to reconstruct the new data.

- `base_data`: The original data the delta was created from
- `delta`: The encoded delta

**Returns**: Reconstructed data or error

### Metadata Extraction

**Rust:**
```rust
pub fn get_tag(delta: &[u8]) -> Result<usize, &'static str>
```

**Python:**
```python
def get_tag(delta: bytes) -> int  # Raises ValueError on error
```

**Node.js:**
```typescript
function getTag(delta: Buffer): number  // Throws Error on failure
```

Extracts the tag value from a delta without decoding it.

**Returns**: Tag value or error

## Version Compatibility

**Important**: Always use the **same version of xpatch** for both encoding and decoding deltas.

- **v0.3.0 and later**: Delta format is stable. Deltas created with any v0.3.0+ version can be decoded with any other v0.3.0+ version.
- **Earlier versions**: Format may differ between versions. Use the exact same version for encoding and decoding.

**Cross-language compatibility**: When using the same version, you can encode a delta in one language binding (e.g., Python) and decode it in another (e.g., Rust or Node.js). All language bindings use the same underlying format.

```python
# Example: Encode in Python
delta = xpatch.encode(0, base, new_data)  # Python v0.3.1
```

```javascript
// Decode in Node.js (same version)
const result = xpatch.decode(base, delta);  // Node.js v0.3.1 - works!
```

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

Results are saved to timestamped files in `benchmark_results/` with both JSON and Markdown reports.

## Related Projects

- [gdelta](https://github.com/ImGajeed76/gdelta) - General-purpose delta compression algorithm used by xpatch
- [xpatch Demo Editor](https://github.com/ImGajeed76/xpatch_demo_editor) - Live demo showcasing xpatch capabilities

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
