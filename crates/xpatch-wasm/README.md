# xpatch-rs (WASM)

High-performance delta compression library - universal WebAssembly bindings for browser and Node.js.

```bash
npm install xpatch-rs
```

This is the **recommended package** for most use cases. It works in both browsers and Node.js with a single ~890KB package.

> **Need maximum performance?** For performance-critical Node.js server applications, consider [`xpatch-rs-native`](https://www.npmjs.com/package/xpatch-rs-native) which provides native bindings (~10-30% faster).

## When to Use

- **Browser applications**: Web-based editors, file viewers, collaborative tools
- **Node.js applications**: Universal package that works everywhere
- **Edge computing**: Cloudflare Workers, Deno, Bun
- **Cross-platform consistency**: Same package works on all platforms

## Building

### Prerequisites

```bash
cargo install wasm-pack
```

### Build Commands

Using axogen (from repository root):

```bash
axogen run build wasm --release
axogen run build wasm --target=web
axogen run build wasm --target=nodejs
```

Or directly with wasm-pack:

```bash
cd crates/xpatch-wasm

# For bundlers (webpack, vite, rollup)
wasm-pack build --release --target bundler

# For direct browser use
wasm-pack build --release --target web

# For Node.js
wasm-pack build --release --target nodejs
```

Output will be in `pkg/` directory.

## Usage

### Browser (with Bundler)

```javascript
import init, { encode, decode, version } from './pkg/xpatch_wasm.js';

await init();

console.log(`xpatch version: ${version()}`);

const encoder = new TextEncoder();
const base = encoder.encode("Hello, World!");
const newData = encoder.encode("Hello, WebAssembly!");

const delta = encode(0, base, newData, true);
const reconstructed = decode(base, delta);

console.log(new TextDecoder().decode(reconstructed));
```

### Browser (without Bundler)

```html
<script type="module">
    import init, { encode, decode } from './pkg/xpatch_wasm.js';

    await init();

    const encoder = new TextEncoder();
    const base = encoder.encode("Hello, World!");
    const newData = encoder.encode("Hello, WASM!");

    const delta = encode(0, base, newData, true);
    const reconstructed = decode(base, delta);

    console.log(new TextDecoder().decode(reconstructed));
</script>
```

## API

### `init(input?: string | URL): Promise<void>`

Initialize the WebAssembly module. Must be called first.

### `encode(tag: number, baseData: Uint8Array, newData: Uint8Array, enableZstd: boolean): Uint8Array`

Create a delta between base and new data.

- `tag`: Metadata tag (0-15 have zero overhead)
- `baseData`: Original data
- `newData`: Modified data
- `enableZstd`: Enable zstd compression

Returns delta as Uint8Array.

### `decode(baseData: Uint8Array, delta: Uint8Array): Uint8Array`

Apply delta to reconstruct new data.

- `baseData`: Original data
- `delta`: Delta patch

Returns reconstructed data. Throws error if delta is invalid.

### `get_tag(delta: Uint8Array): number`

Extract metadata tag from delta. Throws error if delta is invalid.

### `version(): string`

Get xpatch library version.

## Examples

See `examples/` directory:
- `browser/` - Interactive browser demo with UI
- `node/` - Node.js usage examples

### Quick Start with axogen

Run examples directly (builds and runs automatically):

```bash
# Browser demo (builds for web, starts server at localhost:8080)
axogen run example browser --lang=wasm

# Node.js example (builds for nodejs, runs all 5 examples)
axogen run example node --lang=wasm

# List all available examples
axogen run example list
```

### Manual Setup

Alternatively, run manually:

```bash
# Browser
axogen run build wasm --target web
cd crates/xpatch-wasm && python3 -m http.server 8080
# Open http://localhost:8080/examples/browser/

# Node.js
axogen run build wasm --target nodejs
node crates/xpatch-wasm/examples/node/example.js
```

## Testing

```bash
# Run Rust tests
cargo test -p xpatch-wasm

# Run WASM tests in browser
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

## Browser Compatibility

Requires WebAssembly support (Chrome 57+, Firefox 52+, Safari 11+).

## Performance

Performance characteristics measured on Node.js (WASM runtime overhead applies):

- **Small data** (~13 bytes): ~10 µs encode, ~1 µs decode
- **Medium data** (10KB): ~18 µs encode, ~5 µs decode
- **Large data** (100KB): ~87 µs encode, ~33 µs decode
- **Tag extraction**: <1 µs
- **Without zstd**: <1 µs for small payloads

Compression ratios on test data:
- 1 byte change in 10KB file: **13 bytes** delta (99.87% compression)
- 1 byte change in 100KB file: **13 bytes** delta (99.99% compression)

**Note:** Browser performance may vary due to JavaScript engine differences. For Node.js applications requiring maximum performance, consider [`xpatch-rs-native`](https://www.npmjs.com/package/xpatch-rs-native) for ~10-30% faster execution.

## Use Cases

Perfect for:
- **Browser-based applications**: Rich text editors, code editors, collaborative tools
- **Client-side versioning**: Local document history without server roundtrips
- **Offline-first apps**: Sync deltas when connection is restored
- **WebAssembly environments**: Cloudflare Workers, edge computing
- **Cross-platform consistency**: Same compression format across all platforms
- **Bandwidth-sensitive applications**: Minimize data transfer in web apps

## TypeScript

TypeScript definitions are automatically generated in `pkg/xpatch_wasm.d.ts`.

## Links

- [GitHub Repository](https://github.com/ImGajeed76/xpatch)
- [Demo Editor](https://github.com/ImGajeed76/xpatch_demo_editor)
- [xpatch-rs-native](https://www.npmjs.com/package/xpatch-rs-native) (Native Node.js bindings for maximum performance)
- [PyPI Package](https://pypi.org/project/xpatch-rs/) (Python bindings)

## License

Dual-licensed: AGPL-3.0-or-later for open source, commercial licensing available.
