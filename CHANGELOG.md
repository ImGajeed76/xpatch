# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-12-07

### Added

- Initial release of **xpatch**, a high-performance delta compression library for Rust
- **Automatic Algorithm Selection**: Analyzes change patterns and dynamically chooses the optimal compression strategy
  from multiple specialized algorithms
- **Multiple Compression Algorithms**:
    - `Chars`: Simple character insertion
    - `Tokens`: Token-based compression
    - `Remove`: Byte removal
    - `RepeatChars` & `RepeatTokens`: Repetitive pattern detection
    - `GDelta` & `GDeltaZstd`: General-purpose delta compression with optional zstd layer
- **Core API**:
    - `delta::encode(tag, base, new, enable_zstd)` - Create compact deltas with embedded metadata
    - `delta::decode(base, delta)` - Reconstruct target data from delta
    - `delta::get_tag(delta)` - Extract metadata without decoding payload
- **Metadata Tags**: User-defined values 0-15 use zero-overhead single-byte encoding; larger values use variable-length
  encoding
- **Example Programs**: `basic.rs` and `tags.rs` demonstrating usage patterns
- **Comprehensive Benchmark Suite**:
    - Synthetic stress tests with Criterion
    - `git_benchmark` binary for testing against real-world repositories (Git, Neovim, Tokio)
    - Head-to-head comparison with xdelta3 and qbsdiff
- **Dual Licensing**: AGPL-3.0-or-later for open source projects, commercial license available for proprietary use

### Performance

- **Typical Changes** (simple insertions/deletions): 40-55 GB/s throughput, 1-5 µs per operation
- **Complex Changes** (GDelta): 0.5-2 GB/s encoding, 2-20 GB/s decoding
- **Average on 337,378 Git commits**: 99.6% space savings (compression ratio 0.0043)
- **Win Rate**: 95.4% against xdelta3 and qbsdiff in compression efficiency
- **Hardware Baseline**: AMD Ryzen 7 7800X3D, 64GB DDR5, Fedora Linux

### Changed

- N/A (Initial release)

### Deprecated

- N/A (Initial release)

### Removed

- N/A (Initial release)

### Fixed

- N/A (Initial release)

### Security

- N/A (Initial release)

[0.1.0]: https://github.com/ImGajeed76/xpatch/releases/tag/v0.1.0