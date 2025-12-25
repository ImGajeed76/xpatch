#!/usr/bin/env node
/**
 * Unit tests for xpatch-rs Node.js bindings (JavaScript)
 */

const xpatch = require('./index.js');

function test_encode_decode() {
    const base = Buffer.from('Hello, World!');
    const newData = Buffer.from('Hello, Node!');

    const delta = xpatch.encode(0, base, newData);
    const reconstructed = xpatch.decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error(`Expected ${newData}, got ${reconstructed}`);
    }
    console.log('✓ test_encode_decode passed');
}

function test_get_tag() {
    const base = Buffer.from('test');
    const newData = Buffer.from('test123');

    const delta = xpatch.encode(42, base, newData);
    const tag = xpatch.getTag(delta);

    if (tag !== 42) {
        throw new Error(`Expected tag 42, got ${tag}`);
    }
    console.log('✓ test_get_tag passed');
}

function test_compression() {
    const base = Buffer.alloc(1000, 'x');
    const newData = Buffer.concat([base, Buffer.alloc(100, 'y')]);

    const delta = xpatch.encode(0, base, newData);

    if (delta.length >= newData.length) {
        throw new Error(`Delta size ${delta.length} should be less than new data size ${newData.length}`);
    }

    const reduction = (100 * (1 - delta.length / newData.length)).toFixed(1);
    console.log(`✓ test_compression passed (compressed ${newData.length} → ${delta.length} bytes, ${reduction}% reduction)`);
}

function test_empty_data() {
    const base = Buffer.from('');
    const newData = Buffer.from('Hello!');

    const delta = xpatch.encode(0, base, newData);
    const reconstructed = xpatch.decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error('Empty base data test failed');
    }
    console.log('✓ test_empty_data passed');
}

function test_large_tag() {
    const base = Buffer.from('test');
    const newData = Buffer.from('test data');

    const largeTag = 999999;
    const delta = xpatch.encode(largeTag, base, newData);
    const tag = xpatch.getTag(delta);

    if (tag !== largeTag) {
        throw new Error(`Expected tag ${largeTag}, got ${tag}`);
    }
    console.log('✓ test_large_tag passed');
}

function test_identical_data() {
    const data = Buffer.from('Same data');

    const delta = xpatch.encode(0, data, data);
    const reconstructed = xpatch.decode(data, delta);

    if (!reconstructed.equals(data)) {
        throw new Error('Identical data test failed');
    }
    console.log(`✓ test_identical_data passed (delta size: ${delta.length} bytes)`);
}

function test_zstd_disabled() {
    const base = Buffer.from('Hello, World!');
    const newData = Buffer.from('Hello, JavaScript!');

    const delta = xpatch.encode(0, base, newData, false);
    const reconstructed = xpatch.decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error('Zstd disabled test failed');
    }
    console.log('✓ test_zstd_disabled passed');
}

function test_buffer_types() {
    // Test with different buffer creation methods
    const base = Buffer.from([72, 101, 108, 108, 111]); // "Hello"
    const newData = Buffer.from('Hello, World!', 'utf8');

    const delta = xpatch.encode(0, base, newData);
    const reconstructed = xpatch.decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error('Buffer types test failed');
    }
    console.log('✓ test_buffer_types passed');
}

// Run all tests
console.log('Running xpatch-rs Node.js binding tests (JavaScript)...\n');

try {
    test_encode_decode();
    test_get_tag();
    test_compression();
    test_empty_data();
    test_large_tag();
    test_identical_data();
    test_zstd_disabled();
    test_buffer_types();

    console.log('\n✅ All JavaScript tests passed!');
} catch (error) {
    console.error('\n❌ Test failed:', error.message);
    process.exit(1);
}
