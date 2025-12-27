# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **C/C++ Bindings** (`crates/xpatch-c/`):
    - FFI wrapper with 6 core functions (encode, decode, get_tag, free_buffer, free_error, version)
    - Auto-generated header file via cbindgen with comprehensive documentation
    - 16 comprehensive unit tests covering memory safety, threading, edge cases, and error handling
    - Input validation on all FFI functions to prevent undefined behavior
    - Distribution package (library, header, README) in `dist/` directory
    - Full axogen integration (`build c`, `test c`, `example --lang=c`)
    - Complete documentation with API reference, best practices, and troubleshooting
    - C and C++ usage examples with Makefile
    - Enables Go and other language bindings through C FFI layer
- **WebAssembly Bindings** (`crates/xpatch-wasm/`):
    - wasm-bindgen wrapper with 4 core functions (encode, decode, get_tag, version)
    - Auto-generated TypeScript definitions with comprehensive JSDoc documentation
    - 7 comprehensive tests covering all functions, error handling, and edge cases
    - Support for web, Node.js, and bundler targets via wasm-pack
    - Interactive browser demo with real-time compression visualization
    - Node.js example demonstrating all API features
    - Full axogen integration (`build wasm --target=[web|nodejs|bundler]`, `test wasm`)
    - Benchmarked performance metrics (16µs encode, 2µs decode for small data)
    - Complete documentation with API reference, performance data, and use cases
- **Version Compatibility Documentation**: Added version compatibility section in README clarifying that delta format is stable from v0.3.0 onwards

## [0.3.1] - 2025-12-27

### Added

- **Type stubs (.pyi)** for Python bindings to provide better IDE IntelliSense and autocomplete

[0.3.1]: https://github.com/ImGajeed76/xpatch/releases/tag/v0.3.1

## [0.3.0] - 2025-12-27

### Added

- **Multi-Language Bindings**:
    - **Python bindings** (PyO3 + Maturin): Published as `xpatch-rs` on PyPI
    - **Node.js bindings** (NAPI-RS): Published as `xpatch-rs` on npm
    - Full API parity with Rust library across all bindings
    - Comprehensive test suites for Python and Node.js
- **Axogen Build Automation System**:
    - Interactive setup command for detecting and configuring development tools
    - Unified build, test, and local packaging commands for all languages
    - Auto-generation of configuration files (pyproject.toml, package.json, DEVELOPMENT.md)
    - Built-in howto documentation system
- **CharsZstd Algorithm**: New compression algorithm that applies zstd to continuous character additions for improved compression on large insertions
- **Documentation**:
    - Comprehensive DEVELOPMENT.md with setup, build, test, and contribution guidelines
    - Crate-level README.md for Rust package on crates.io
    - Documented Rust edition 2024 requirement (Rust 1.92.0+)
    - Demo editor reference showcasing xpatch capabilities

### Changed

- **Repository Structure**: Reorganized into workspace with separate crates for core library, Python bindings, and Node.js bindings
- **Licensing Documentation**: Clarified dual-license philosophy and improved commercial licensing explanation
- **Requirements**:
    - Documented Rust 1.92.0+ requirement for Rust edition 2024 support
    - Standardized Node.js version requirement to 16+
- **README Improvements**: Enhanced README flow with better feature descriptions and licensing clarity

### Fixed

- Broken CHANGELOG repository link (xpatch-lib → ImGajeed76)
- Clippy warnings

[0.3.0]: https://github.com/ImGajeed76/xpatch/releases/tag/v0.3.0

## [0.2.0] - 2025-12-11

### Added

- **Enhanced CLI Tool** (`src/bin/cli.rs`):
    - Memory usage warnings with system checks before processing large files
    - Progress indicators for multistep operations
    - Colored output with `owo-colors`
    - Verification mode (`--verify`) to ensure delta correctness
    - Better error messages with specific exit codes
    - Force overwrite (`--force`) and quiet mode (`--quiet`) flags
- **Comprehensive Real-World Benchmarking**:
    - New `git_real_world` benchmark suite testing on actual git repositories
    - Tested on **1.2 million real-world deltas** across 30,719 files
    - Support for multiple algorithms: xpatch (sequential + tags), vcdiff, gdelta
    - Parallel processing
    - Hardware info collection in reports
- **vcdiff Algorithm Support**: Added vcdiff (VCDIFF standard implementation) as benchmark comparison
- **Cache System** for benchmark data: Extract git file versions once, reuse across runs
- **Detailed Markdown Reports**: Auto-generated benchmark reports with statistics, rankings, and tag optimization
  analysis

### Changed

- **Benchmark Infrastructure Completely Rewritten**:
    - Moved from synthetic-only tests to real-world git repository analysis
    - Environment variable configuration system replacing CLI arguments
    - Support for repository presets (rust, neovim, tokio, git)
    - File discovery modes: predefined files, all at HEAD, all in history
    - Parallel file processing option
- **Updated Dependencies**:
    - `gdelta` upgraded from 0.1.1 to 0.2.1
    - Added `vcdiff` 0.1.0 for benchmark comparisons
    - Added `crossbeam` 0.8.4 for concurrent cache building
- **Stress Benchmark Refocus**: Now tests "human-focused" scenarios (code edits, docs, configs, logs) instead of
  synthetic data
- **CLI Moved**: Binary moved from `src/bin/xpatch.rs` to `src/bin/cli.rs` with full rewrite
- **Optimized `encode_remove`**: Now encodes length (`end - start`) instead of absolute `end` position, reducing bytes
  needed for large file removals

### Deprecated

This release is not compatible with v0.1.0.

### Performance

Tested on **1,359,468 real-world git commit changes** across tokio (133,728 deltas) and mdn/content (1,225,740 deltas):

**tokio (Rust async runtime, code repository):**

- **xpatch with tags**: 2 bytes median, 0.0019 compression ratio (99.8% space saved)
- **xpatch sequential**: 68 bytes median, 0.0165 ratio (98.4% saved)
- vcdiff (xdelta3): 97 bytes median, 0.0276 ratio (97.2% saved)
- gdelta: 69 bytes median, 0.0180 ratio (98.2% saved)
- **Tag optimization impact**: 88.7% smaller deltas vs sequential mode (median)

**mdn/content (MDN Web Docs, documentation repository):**

- **xpatch with tags**: 23 bytes median, 0.0063 compression ratio (99.4% space saved)
- **xpatch sequential**: 25 bytes median, 0.0069 ratio (99.3% saved)
- vcdiff: 50 bytes median, 0.0169 ratio (98.3% saved)
- gdelta: 26 bytes median, 0.0077 ratio (99.2% saved)
- **Tag optimization impact**: 8.8% smaller deltas vs sequential mode (median)

### Removed

- **git_benchmark Binary**: Replaced by the new `git_real_world` benchmark infrastructure
- **stress_compared Benchmark**: Removed in favor of focused human-scenario tests in main stress benchmark and gdelta's
  comprehensive benchmark suite including xpatch
- Old CLI at `src/bin/xpatch.rs` (replaced with enhanced version)

[0.2.0]: https://github.com/ImGajeed76/xpatch/releases/tag/v0.2.0

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