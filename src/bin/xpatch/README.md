# xpatch CLI Tool

Command-line interface for the xpatch delta compression library.

## Installation

```bash
cargo install xpatch --features cli
```

Or build from source:

```bash
cargo build --release --features cli
```

## Usage

### Encode
Create a delta between two files:
```bash
xpatch encode base.txt new.txt -o patch.xp
xpatch encode base.txt new.txt > patch.xp
cat new.txt | xpatch encode base.txt - > patch.xp
```

### Decode
Apply a delta to reconstruct a file:
```bash
xpatch decode base.txt patch.xp -o restored.txt
xpatch decode base.txt patch.xp > restored.txt
```

### Info
Show delta metadata:
```bash
xpatch info patch.xp
```

## Options

- Use `-` as a filename for stdin/stdout
- Add metadata tags with `-t` (useful for version control)
- Enable zstd compression with `-z` for better ratios on complex changes

## Examples

Version control workflow:
```bash
# Create patches between versions
xpatch encode v1.c v2.c -t 1 -o patch_v1_v2.xp
xpatch encode v1.c v3.c -t 2 -o patch_v1_v3.xp

# Later, reconstruct any version
xpatch decode v1.c patch_v1_v2.xp -o v2_restored.c
xpatch decode v1.c patch_v1_v3.xp -o v3_restored.c

# Check which base version was used
xpatch info patch_v1_v3.xp
# Output: Tag: 2
```

Pipeline compression:
```bash
# Compress a stream of changes
cat log.txt | xpatch encode previous_log.txt - -z > log_delta.xp
```

## License
Same dual-license as the library: AGPL-3.0 or commercial.