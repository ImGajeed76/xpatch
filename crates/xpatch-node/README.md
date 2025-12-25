# xpatch-rs - Node.js Bindings

High-performance delta compression library for Node.js with automatic algorithm selection.

## Installation

```bash
npm install xpatch-rs
# or
yarn add xpatch-rs
# or
pnpm add xpatch-rs
```

## Quick Start

```javascript
const xpatch = require('xpatch-rs');

// Create a delta patch
const base = Buffer.from('Hello, World!');
const newData = Buffer.from('Hello, Node!');
const delta = xpatch.encode(0, base, newData);

console.log(`Delta size: ${delta.length} bytes`);

// Apply the patch
const reconstructed = xpatch.decode(base, delta);
console.log(reconstructed.equals(newData)); // true

// Extract metadata tag
const tag = xpatch.getTag(delta);
console.log(`Tag: ${tag}`);
```

## TypeScript Support

TypeScript definitions are included:

```typescript
import { encode, decode, getTag } from 'xpatch-rs';

const base = Buffer.from('Hello, World!');
const newData = Buffer.from('Hello, TypeScript!');

const delta: Buffer = encode(0, base, newData);
const reconstructed: Buffer = decode(base, delta);
const tag: number = getTag(delta);
```

## API Reference

### `encode(tag, baseData, newData, enableZstd?) => Buffer`

Creates a delta patch between `baseData` and `newData`.

**Parameters:**
- `tag` (number): Metadata tag to embed (0-4294967295)
- `baseData` (Buffer): Original data
- `newData` (Buffer): New data
- `enableZstd` (boolean, optional): Enable zstd compression (default: true)

**Returns:** `Buffer` - The encoded delta patch

### `decode(baseData, delta) => Buffer`

Reconstructs `newData` from `baseData` and a delta patch.

**Parameters:**
- `baseData` (Buffer): Original data
- `delta` (Buffer): Delta patch created by `encode()`

**Returns:** `Buffer` - The reconstructed new data

**Throws:** `Error` if delta is invalid

### `getTag(delta) => number`

Extracts the metadata tag from a delta patch without decoding.

**Parameters:**
- `delta` (Buffer): Delta patch

**Returns:** `number` - The embedded tag

**Throws:** `Error` if delta is invalid

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

## Building from Source

```bash
cd crates/xpatch-node
npm install
npm run build
```

For development builds:

```bash
npm run build:debug
```

## Supported Platforms

Pre-built binaries are available for:
- Linux (x64, ARM64, musl)
- macOS (Intel, Apple Silicon)
- Windows (x64, ARM64)

## License

This project is dual-licensed:
- **AGPL-3.0-or-later** for open-source use
- **Commercial license** available at xpatch-commercial@alias.oseifert.ch

See [LICENSE-AGPL.txt](../../LICENSE-AGPL.txt) and [LICENSE-COMMERCIAL.txt](../../LICENSE-COMMERCIAL.txt) for details.

## Links

- [GitHub Repository](https://github.com/ImGajeed76/xpatch)
- [Demo Editor](https://github.com/imgajeed76/xpatch_demo_editor)
- [npm Package](https://www.npmjs.com/package/xpatch-rs)
