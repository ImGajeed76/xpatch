// Example usage of xpatch WASM bindings in Node.js
const { encode, decode, get_tag, version } = require('../../pkg/xpatch_wasm.js');

console.log('xpatch WebAssembly Example');
console.log('==========================\n');

console.log(`Using xpatch version: ${version()}\n`);

// Example 1: Basic encode/decode
console.log('Example 1: Basic Encode/Decode');
console.log('-------------------------------');
const base1 = Buffer.from('Hello, World!');
const new1 = Buffer.from('Hello, Node.js!');

const delta1 = encode(0, base1, new1, true);
console.log(`Original: "${new1.toString()}"`);
console.log(`Base: "${base1.toString()}"`);
console.log(`Delta size: ${delta1.length} bytes`);
console.log(`Compression: ${((1 - delta1.length / new1.length) * 100).toFixed(1)}% saved`);

const reconstructed1 = decode(base1, delta1);
console.log(`Decoded: "${Buffer.from(reconstructed1).toString()}"`);
console.log(`Match: ${Buffer.from(reconstructed1).equals(new1)}\n`);

// Example 2: Tag extraction
console.log('Example 2: Tag Extraction');
console.log('-------------------------');
const base2 = Buffer.from('Version 1');
const new2 = Buffer.from('Version 2');

const delta2 = encode(42, base2, new2, false);
const tag = get_tag(delta2);
console.log(`Encoded with tag: 42`);
console.log(`Extracted tag: ${tag}`);
console.log(`Match: ${tag === 42}\n`);

// Example 3: Large data
console.log('Example 3: Large Data');
console.log('---------------------');
const largeBase = Buffer.alloc(10000, 'A');
const largeNew = Buffer.alloc(10000, 'A');
// Change a few bytes
largeNew[5000] = 'B'.charCodeAt(0);
largeNew[5001] = 'B'.charCodeAt(0);
largeNew[5002] = 'B'.charCodeAt(0);

const largeDelta = encode(0, largeBase, largeNew, true);
console.log(`Original size: ${largeNew.length} bytes`);
console.log(`Delta size: ${largeDelta.length} bytes`);
console.log(`Compression: ${((1 - largeDelta.length / largeNew.length) * 100).toFixed(2)}% saved`);

const reconstructedLarge = decode(largeBase, largeDelta);
console.log(`Match: ${Buffer.from(reconstructedLarge).equals(largeNew)}\n`);

// Example 4: Error handling
console.log('Example 4: Error Handling');
console.log('-------------------------');
const invalidDelta = Buffer.from([0xFF, 0xFF, 0xFF, 0xFF]);

try {
    decode(base1, invalidDelta);
    console.log('ERROR: Should have thrown!');
} catch (e) {
    console.log(`Correctly rejected invalid delta: ${e.message}\n`);
}

// Example 5: Zero-overhead tags (0-15)
console.log('Example 5: Zero-Overhead Tags');
console.log('-----------------------------');
const baseSmall = Buffer.from('a');
const newSmall = Buffer.from('b');

console.log('Testing tags 0-15 (should have minimal overhead):');
for (let tag = 0; tag <= 15; tag++) {
    const delta = encode(tag, baseSmall, newSmall, false);
    const extractedTag = get_tag(delta);
    if (tag % 4 === 0 || tag === 15) {
        console.log(`  Tag ${tag.toString().padStart(2)}: ${delta.length} bytes, extracted: ${extractedTag}`);
    }
}

console.log('\nAll examples completed successfully!');
