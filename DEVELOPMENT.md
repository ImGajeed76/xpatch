# Development Guide

This guide provides instructions for building, testing, and contributing to xpatch (v0.3.1).

## Table of Contents

- [Quick Start](#quick-start)
- [Initial Setup](#initial-setup)
- [Prerequisites](#prerequisites)
- [Repository Structure](#repository-structure)
- [Building the Project](#building-the-project)
- [Running Tests](#running-tests)
- [Running Benchmarks](#running-benchmarks)
- [Working with Language Bindings](#working-with-language-bindings)
- [Testing Packages Locally](#testing-packages-locally)
- [Code Style and Linting](#code-style-and-linting)
- [Contributing](#contributing)

## Quick Start

For a guided setup experience:

```bash
axogen run setup
```

For quick reference documentation:

```bash
axogen run howto
```

## Initial Setup

After cloning the repository, set up the build automation tools:

```bash
# Install dependencies (including Axogen)
bun install

# Verify Axogen is available
axogen --version

# Run the interactive setup
axogen run setup
```

The setup command will:
- Detect installed development tools (Rust, Python, Node.js, etc.)
- Prompt for package manager preferences
- Install dependencies
- Build all components

### Regenerating Configuration Files

Configuration files (pyproject.toml, package.json, DEVELOPMENT.md) are auto-generated from templates in `axogen/templates/`. To regenerate them:

```bash
axogen generate
```

or

```bash
axogen gen
```

## Prerequisites

### Required Tools

- **Rust**: Version 1.92.0 or later (install via [rustup](https://rustup.rs/))
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  **Note**: This project uses Rust edition 2024, which requires Rust 1.92.0 or later.

### Optional Tools (for bindings)

- **Python 3.8+** with pip (for Python bindings)
- **Node.js 16+** or **Bun** (for Node.js bindings)
- **Maturin** (for building Python bindings)
  ```bash
  pip install maturin
  ```

To check which tools are installed:

```bash
axogen run setup # Will detect and report installed tools
```

## Repository Structure

```
xpatch/
├── Cargo.toml                      # Workspace root configuration
├── README.md                       # Main project documentation
├── DEVELOPMENT.md                  # This file (auto-generated)
├── LICENSE-AGPL.txt                # AGPL license
├── LICENSE-COMMERCIAL.txt          # Commercial license
├── axogen/                         # Build automation configuration
│   ├── commands/                   # Command implementations
│   ├── targets/                    # Config file generators
│   ├── templates/                  # Templates for generated files
│   └── utils/                      # Utility functions
├── crates/
│   ├── xpatch/                     # Core Rust library + CLI
│   │   ├── Cargo.toml              # Core package configuration
│   │   ├── src/
│   │   │   ├── lib.rs              # Library entry point
│   │   │   ├── delta.rs            # Delta encoding/decoding
│   │   │   └── bin/
│   │   │       └── cli.rs          # CLI implementation
│   │   ├── examples/               # Usage examples
│   │   ├── benches/                # Benchmarks
│   │   └── tests/                  # Integration tests
│   ├── xpatch-python/              # Python bindings (PyO3)
│   │   ├── Cargo.toml
│   │   ├── pyproject.toml          # Auto-generated from axogen
│   │   └── src/lib.rs
│   └── xpatch-node/                # Node.js bindings (NAPI-RS)
│       ├── Cargo.toml
│       ├── package.json            # Auto-generated from axogen
│       └── src/lib.rs
└── target/                         # Build artifacts (gitignored)
```

## Building the Project

### Build Everything

```bash
axogen run build all
```

### Build Individual Components

#### Core Rust Library

```bash
axogen run build rust
```

For release builds:

```bash
axogen run build rust --release
```

#### Python Bindings

```bash
axogen run build python
```

For release builds:

```bash
axogen run build python --release
```

#### Node.js Bindings

```bash
axogen run build node
```

For release builds:

```bash
axogen run build node --release
```

### Manual Build Commands

If you prefer to build manually:

**Rust:**
```bash
cargo build --all
cargo build --all --release  # Release mode
```

**Python:**
```bash
cd crates/xpatch-python
maturin develop              # Development
maturin build --release      # Release
```

**Node.js:**
```bash
cd crates/xpatch-node
bun install && bun run build        # If using Bun
npm install && npm run build        # If using npm
```

## Running Tests

### Run All Tests

```bash
axogen run test
```

### Test Individual Components

#### Core Rust Tests

```bash
axogen run test rust
```

#### Python Binding Tests

```bash
axogen run test python
```

#### Node.js Binding Tests

```bash
axogen run test node
```

### Manual Test Commands

**Rust:**
```bash
cargo test -p xpatch
cargo test -p xpatch -- --nocapture    # With output
cargo test -p xpatch test_name         # Specific test
```

**Python:**
```bash
cd crates/xpatch-python
maturin develop
python tests/test_xpatch.py
```

**Node.js:**
```bash
cd crates/xpatch-node
bun test.js    # Or: node test.js
```

## Running Examples

### Using Axogen

```bash
axogen run example list            # List all examples
axogen run example basic            # Run basic example
axogen run example tags             # Run tags example
```

### Manual Commands

```bash
cargo run --example basic
cargo run --example tags
```

### CLI Examples

```bash
# Encode a delta
cargo run -p xpatch --features cli -- encode base.txt new.txt -o patch.xp

# Decode a delta
cargo run -p xpatch --features cli -- decode base.txt patch.xp -o restored.txt

# Show delta information
cargo run -p xpatch --features cli -- info patch.xp
```

## Running Benchmarks

For detailed benchmark information:

```bash
axogen run howto bench
```

### Quick Benchmark Commands

```bash
# Stress test benchmark
cargo bench --bench stress

# Real-world git benchmark
cargo bench --bench git_real_world

# With specific preset
XPATCH_PRESET=tokio cargo bench --bench git_real_world
```

### Environment Variables

- `XPATCH_PRESET`: Choose repository (`tokio`, `mdn`, `all`)
- `XPATCH_MAX_TAG_DEPTH`: Maximum tag depth to test (default: 16)
- `XPATCH_MAX_COMMITS`: Maximum commits to process (default: all)
- `XPATCH_BUILD_CACHE`: Build cache only
- `XPATCH_USE_CACHE`: Use existing cache

## Working with Language Bindings

### Python Development Workflow

```bash
cd crates/xpatch-python
python -m venv venv
source venv/bin/activate     # On Windows: venv\Scripts\activate
pip install maturin
maturin develop
python tests/test_xpatch.py
```

### Node.js Development Workflow

```bash
cd crates/xpatch-node
bun install                  # Or: npm install
bun run build:debug          # Or: npm run build:debug
bun test.js                  # Or: npm test
```

## Testing Packages Locally

Before publishing, you can test the packages in other projects locally without publishing to package registries.

### Testing Rust Library Locally

Use path dependencies in your test project's `Cargo.toml`:

```toml
[dependencies]
xpatch = { path = "/path/to/xpatch/crates/xpatch" }
```

Or use cargo's local registry:

```bash
# In your test project
cargo add --path /path/to/xpatch/crates/xpatch
```

### Testing Python Package Locally

**Option 1: Development install with maturin**

```bash
# In xpatch-python directory
cd crates/xpatch-python
maturin develop

# Now you can import xpatch in any Python script on the same system
python -c "import xpatch; print(xpatch.__version__)"
```

**Option 2: Editable install with pip**

```bash
# In xpatch-python directory
cd crates/xpatch-python
pip install -e .

# Or from another project
pip install -e /path/to/xpatch/crates/xpatch-python
```

**Option 3: Build and install wheel locally**

```bash
# Build the wheel
cd crates/xpatch-python
maturin build --release

# Install the wheel in your test project's venv
pip install target/wheels/xpatch_rs-*.whl
```

### Testing Node.js Package Locally

**Option 1: Using npm link**

```bash
# In xpatch-node directory
cd crates/xpatch-node
npm run build
npm link

# In your test project
npm link xpatch-rs
```

**Option 2: Using bun link**

```bash
# In xpatch-node directory
cd crates/xpatch-node
bun run build
bun link

# In your test project
bun link xpatch-rs
```

**Option 3: Direct path installation**

```bash
# In your test project
npm install /path/to/xpatch/crates/xpatch-node
# Or
bun add /path/to/xpatch/crates/xpatch-node
```

**Option 4: Using package.json dependency**

In your test project's `package.json`:

```json
{
  "dependencies": {
    "xpatch-rs": "file:../path/to/xpatch/crates/xpatch-node"
  }
}
```

Then run `npm install` or `bun install`.

### Unlinking Packages

After testing, unlink the packages:

```bash
# npm
npm unlink xpatch-rs

# bun
bun unlink xpatch-rs

# Or just reinstall from registry
npm install xpatch-rs
bun add xpatch-rs
```

## Code Style and Linting

### Format All Code

```bash
axogen run fmt
```

### Lint All Code

```bash
axogen run lint
```

### Manual Commands

**Rust:**
```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all --all-features -- -D warnings
```

**Python:**
```bash
pip install black mypy ruff
black crates/xpatch-python/
ruff check crates/xpatch-python/
```

## Contributing

### Workflow

1. **Fork the repository** on GitHub

2. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

3. **Make your changes** following the code style guidelines

4. **Run tests and linting**:
   ```bash
   axogen run test
   axogen run lint
   ```

5. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` - New features
   - `fix:` - Bug fixes
   - `docs:` - Documentation changes
   - `refactor:` - Code refactoring
   - `test:` - Test additions or changes
   - `perf:` - Performance improvements

6. **Push to your fork**:
   ```bash
   git push origin feature/my-feature
   ```

7. **Open a Pull Request** on GitHub

### Guidelines

- Write clear commit messages
- Add tests for new features
- Update documentation for API changes
- Ensure all tests pass before submitting PR
- Keep PRs focused on a single feature/fix

## Releasing

For information on how to release a new version:

```bash
axogen run howto release
```

## Cleaning Build Artifacts

```bash
axogen run clean
```

## Debugging

### Enable Debug Features

```bash
cargo build -p xpatch --features debug_all
```

Available debug features:
- `debug_delta_encode`
- `debug_delta_token`
- `debug_delta_analyze`
- `debug_delta_compress`
- `debug_delta_pattern`
- `debug_delta_header`
- `debug_tokenizer`
- `debug_all` (enables all of the above)

### Verbose Logging

```bash
RUST_LOG=debug cargo run -p xpatch --features cli -- encode base.txt new.txt -o patch.xp
```

## Getting Help

For quick reference:
```bash
axogen run howto
```

- **Issues**: Report bugs at [GitHub Issues](https://github.com/ImGajeed76/xpatch/issues)
- **Commercial Support**: Contact xpatch-commercial@alias.oseifert.ch

## License

See [LICENSE-AGPL.txt](LICENSE-AGPL.txt) for open-source licensing and [LICENSE-COMMERCIAL.txt](LICENSE-COMMERCIAL.txt) for commercial licensing options.

---

*This file is auto-generated from `axogen/templates/DEVELOPMENT.md.njk`. To modify, edit the template and run `axogen generate`.*
