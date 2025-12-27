// Simple WASM performance benchmark
const {encode, decode, get_tag} = require('./pkg/xpatch_wasm.js');

function benchmark(name, fn, iterations = 1000) {
    const start = process.hrtime.bigint();
    for (let i = 0; i < iterations; i++) {
        fn();
    }
    const end = process.hrtime.bigint();
    const totalNs = Number(end - start);
    const avgMs = (totalNs / iterations / 1_000_000).toFixed(3);
    const avgUs = (totalNs / iterations / 1_000).toFixed(1);
    console.log(`${name.padEnd(40)} ${avgUs.padStart(8)} µs/op  (${iterations} iterations)`);
    return {avgMs: parseFloat(avgMs), avgUs: parseFloat(avgUs)};
}

console.log('WASM Performance Benchmarks');
console.log('='.repeat(70));
console.log();

// Test 1: Small text changes
const smallBase = Buffer.from('Hello, World!');
const smallNew = Buffer.from('Hello, WASM!');
const results = {};

results.smallEncode = benchmark(
    'Small encode (13 bytes, with zstd)',
    () => encode(0, smallBase, smallNew, true)
);

const smallDelta = encode(0, smallBase, smallNew, true);
results.smallDecode = benchmark(
    'Small decode (13 bytes)',
    () => decode(smallBase, smallDelta)
);

results.getTag = benchmark(
    'Get tag',
    () => get_tag(smallDelta),
    10000
);

console.log();

// Test 2: Medium data
const mediumBase = Buffer.alloc(10000, 'A');
const mediumNew = Buffer.alloc(10000, 'A');
for (let i = 5000; i < 5100; i++) {
    mediumNew[i] = 'B'.charCodeAt(0);
}

results.mediumEncode = benchmark(
    'Medium encode (10KB, 100 byte diff, zstd)',
    () => encode(0, mediumBase, mediumNew, true),
    500
);

const mediumDelta = encode(0, mediumBase, mediumNew, true);
results.mediumDecode = benchmark(
    'Medium decode (10KB)',
    () => decode(mediumBase, mediumDelta),
    500
);

console.log();

// Test 3: Larger data
const largeBase = Buffer.alloc(100000, 'X');
const largeNew = Buffer.alloc(100000, 'X');
for (let i = 50000; i < 50500; i++) {
    largeNew[i] = 'Y'.charCodeAt(0);
}

results.largeEncode = benchmark(
    'Large encode (100KB, 500 byte diff, zstd)',
    () => encode(0, largeBase, largeNew, true),
    100
);

const largeDelta = encode(0, largeBase, largeNew, true);
results.largeDecode = benchmark(
    'Large decode (100KB)',
    () => decode(largeBase, largeDelta),
    100
);

console.log();

// Test 4: Without compression
results.noZstdEncode = benchmark(
    'Small encode (13 bytes, no zstd)',
    () => encode(0, smallBase, smallNew, false)
);

const noZstdDelta = encode(0, smallBase, smallNew, false);
results.noZstdDecode = benchmark(
    'Small decode (no zstd)',
    () => decode(smallBase, noZstdDelta)
);

console.log();
console.log('='.repeat(70));
console.log();
console.log('Summary:');
console.log(`  Encode (small):  ~${results.smallEncode.avgUs} µs`);
console.log(`  Decode (small):  ~${results.smallDecode.avgUs} µs`);
console.log(`  Encode (100KB):  ~${results.largeEncode.avgUs} µs`);
console.log(`  Decode (100KB):  ~${results.largeDecode.avgUs} µs`);
console.log(`  Get tag:         ~${results.getTag.avgUs} µs`);
console.log();

// Compression stats
console.log('Compression Results:');
console.log(`  Small (13 bytes):   ${smallDelta.length} bytes delta`);
console.log(`  Medium (10KB):      ${mediumDelta.length} bytes delta`);
console.log(`  Large (100KB):      ${largeDelta.length} bytes delta`);
