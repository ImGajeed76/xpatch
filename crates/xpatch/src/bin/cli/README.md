# xpatch CLI Tool

Command-line interface for the xpatch high-performance delta compression library.

xpatch creates and applies binary deltas between files with intelligent compression and excellent performance.

## Installation

### From crates.io

```bash
cargo install xpatch --features cli
```

### From Source

```bash
git clone https://github.com/ImGajeed76/xpatch
cd xpatch
cargo build --release --features cli
# Binary will be at target/release/xpatch
```

## Quick Start

```bash
# Create a delta
xpatch encode old.bin new.bin -o patch.xdelta

# Apply the delta
xpatch decode old.bin patch.xdelta -o restored.bin

# Verify they match
diff new.bin restored.bin
```

## Commands

### `encode` - Create a Delta

Create a delta patch between two files.

```bash
xpatch encode <BASE> <NEW> -o <OUTPUT> [OPTIONS]
```

**Arguments:**
- `<BASE>` - Base file (original version)
- `<NEW>` - New file (target version)
- `-o, --output <PATH>` - Output delta file (required)

**Options:**
- `-t, --tag <NUMBER>` - User-defined metadata tag (default: 0)
- `-z, --zstd` - Enable zstd compression for complex changes
- `-v, --verify` - Verify delta after creation by decoding and comparing
- `-f, --force` - Overwrite output file if it exists
- `-q, --quiet` - Suppress all output except errors
- `-y, --yes` - Skip memory warning prompts

**Examples:**

```bash
# Basic encoding
xpatch encode v1.0.bin v1.1.bin -o update.xdelta

# With version tag and verification
xpatch encode old.dat new.dat -o patch.xdelta -t 42 --verify

# Enable zstd for better compression on complex changes
xpatch encode base.db updated.db -o delta.xdelta --zstd

# Force overwrite and run quietly
xpatch encode old.txt new.txt -o patch.xdelta -f -q
```

### `decode` - Apply a Delta

Apply a delta patch to reconstruct the new file.

```bash
xpatch decode <BASE> <DELTA> -o <OUTPUT> [OPTIONS]
```

**Arguments:**
- `<BASE>` - Base file (original version, must match encoding base)
- `<DELTA>` - Delta patch file
- `-o, --output <PATH>` - Output file (required)

**Options:**
- `-f, --force` - Overwrite output file if it exists
- `-q, --quiet` - Suppress all output except errors
- `-y, --yes` - Skip memory warning prompts

**Examples:**

```bash
# Basic decoding
xpatch decode v1.0.bin patch.xdelta -o v1.1.bin

# Force overwrite
xpatch decode old.dat delta.xdelta -o new.dat -f

# Quiet mode for scripting
xpatch decode base.bin update.xdelta -o result.bin -q
```

### `info` - Show Delta Information

Display metadata and statistics about a delta file.

```bash
xpatch info <DELTA>
```

**Arguments:**
- `<DELTA>` - Delta patch file

**Example Output:**

```
Tag: 42
Size: 14524 bytes
Algorithm: Tokens
Header size: 2 bytes
```

**Examples:**

```bash
# Show delta information
xpatch info patch.xdelta

# Use in scripts
TAG=$(xpatch info patch.xdelta | grep "Tag:" | cut -d' ' -f2)
echo "Patch version: $TAG"
```

## Features

### Memory Management

xpatch automatically checks available memory before processing:

- **< 80% usage**: Proceeds without warning
- **80-100% usage**: Shows warning, asks for confirmation
- **> 100% usage**: Shows critical warning
- **> Total RAM**: Fails immediately

**Memory Warning Example:**

```
Memory warning: This operation requires ~2.4 GB
   Available: 1.8 GB free (8.0 GB total)

   Loading these files will use 133% of available memory.
   Your system may freeze or crash.

   Continue? [y/N]:
```

Skip prompts with `--yes` flag for automated scripts.

### Progress Output

By default, xpatch shows detailed progress:

```
File sizes: Base: 10.5 MB, New: 10.8 MB
Memory: ~34.2 MB required, 52.32 GB available ✓
Step 1/3: Reading files...
Step 2/3: Encoding delta...
Step 3/3: Writing output...

Success: Created delta.xpatch (10.8 MB, 100.0% of new file)
   Encoding took 123.35ms
```

Use `--quiet` to suppress all output except errors.

### Verification

Use `--verify` to ensure delta correctness:

```bash
xpatch encode base.bin new.bin -o patch.xdelta --verify
```

This decodes the delta and compares it byte-by-byte with the original new file. Recommended for critical data.

### Metadata Tags

Tags are user-defined integers (0-2^64) stored in the delta header. Common uses:
- Version numbers
- Build identifiers
- Timestamps
- Custom application metadata

```bash
xpatch encode v1.bin v2.bin -o patch.xdelta -t 42
xpatch info patch.xdelta  # Shows: Tag: 42
```

### Compression

xpatch uses intelligent internal compression by default. For complex changes with low similarity, enable zstd:

```bash
# Default: automatic algorithm selection
xpatch encode base.bin new.bin -o patch.xdelta

# Force zstd for better compression on complex changes
xpatch encode base.bin new.bin -o patch.xdelta --zstd
```

## Exit Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 0 | Success | Operation completed successfully |
| 1 | General error | File not found, permission denied, etc. |
| 2 | Encode/decode failed | Delta operation failed |
| 4 | Out of memory | Insufficient RAM available |
| 5 | User cancelled | Operation cancelled at prompt |

**Example:**

```bash
xpatch encode base.bin new.bin -o patch.xdelta
if [ $? -eq 0 ]; then
    echo "Success"
else
    echo "Failed with code $?"
fi
```

## Common Errors

**"File not found"**
```
Error: File not found: input.bin
```
Check that all input files exist and paths are correct.

**"Output file already exists"**
```
Error: Output file already exists: output.xdelta
   Use --force to overwrite
```
Use `-f/--force` flag or delete the existing file.

**"Insufficient memory"**
```
Error: Insufficient memory
   Required: ~4.2 GB
   Total RAM: 4.0 GB
```
Process files on a machine with more RAM or use the library API with streaming.

**"Verification failed"**
```
Error: Verification failed: reconstructed output does not match original
```
File may have been modified during encoding. Try encoding again.

**"Decode failed"**
```
Error: Decode failed: invalid delta format
```
Delta file is corrupted or you're using the wrong base file.

## License

xpatch CLI is dual-licensed:

**Open Source**: GNU Affero General Public License v3.0 (AGPL-3.0)  
**Commercial**: Proprietary license available for commercial use in closed-source applications

For commercial licensing inquiries: xpatch-commercial@alias.oseifert.ch

---

**xpatch** - High-performance delta compression for the modern age.