# xpatch-rs - Python Bindings

High-performance delta compression library for Python with automatic algorithm selection.

## Installation

```bash
pip install xpatch-rs
```

**Note:** The package is named `xpatch-rs` on PyPI, but you import it as `xpatch` in your code.

Or build from source:

```bash
cd crates/xpatch-python
pip install maturin
maturin develop
```

## Quick Start

```python
import xpatch

# Create a delta patch
base = b"Hello, World!"
new = b"Hello, Rust!"
delta = xpatch.encode(tag=0, base_data=base, new_data=new)

# Apply the patch
reconstructed = xpatch.decode(base_data=base, delta=delta)
assert reconstructed == new

# Extract metadata tag
tag = xpatch.get_tag(delta)
print(f"Tag: {tag}")
```

## API Reference

### `encode(tag, base_data, new_data, enable_zstd=True) -> bytes`

Creates a delta patch between `base_data` and `new_data`.

**Parameters:**
- `tag` (int): Metadata tag to embed (0-15 for no overhead, larger values supported)
- `base_data` (bytes): Original data
- `new_data` (bytes): New data
- `enable_zstd` (bool): Enable zstd compression (default: True)

**Returns:** `bytes` - The encoded delta patch

### `decode(base_data, delta) -> bytes`

Reconstructs `new_data` from `base_data` and a delta patch.

**Parameters:**
- `base_data` (bytes): Original data
- `delta` (bytes): Delta patch created by `encode()`

**Returns:** `bytes` - The reconstructed new data

**Raises:** `ValueError` if delta is invalid

### `get_tag(delta) -> int`

Extracts the metadata tag from a delta patch without decoding.

**Parameters:**
- `delta` (bytes): Delta patch

**Returns:** `int` - The embedded tag

**Raises:** `ValueError` if delta is invalid

## Performance

xpatch achieves exceptional compression ratios on real-world data:

- **99.8% compression** on typical code changes
- **2 byte median delta** for sequential edits
- **Instant decoding** (<1µs for most patches)
- **40-55 GB/s throughput** for encoding

## Use Cases

Perfect for:
- Version control systems
- Document synchronization
- Incremental backups
- Network-efficient updates
- Real-time collaborative editing

## License

This project is dual-licensed:
- **AGPL-3.0-or-later** for open-source use
- **Commercial license** available at xpatch-commercial@alias.oseifert.ch

See [LICENSE-AGPL.txt](../../LICENSE-AGPL.txt) and [LICENSE-COMMERCIAL.txt](../../LICENSE-COMMERCIAL.txt) for details.

## Links

- [GitHub Repository](https://github.com/ImGajeed76/xpatch)
- [Demo Editor](https://github.com/ImGajeed76/xpatch_demo_editor)
- [PyPI Package](https://pypi.org/project/xpatch-rs/)
- [Rust Documentation](https://docs.rs/xpatch)
