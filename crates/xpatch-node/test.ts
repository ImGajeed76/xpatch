#!/usr/bin/env bun
/**
 * Unit tests for xpatch-rs Node.js bindings (TypeScript)
 */

import { encode, decode, getTag } from './index.js';

function test_encode_decode(): void {
    const base = Buffer.from('Hello, World!');
    const newData = Buffer.from('Hello, TypeScript!');

    const delta: Buffer = encode(0, base, newData);
    const reconstructed: Buffer = decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error(`Expected ${newData}, got ${reconstructed}`);
    }
    console.log('✓ test_encode_decode passed');
}

function test_get_tag(): void {
    const base = Buffer.from('test');
    const newData = Buffer.from('test123');

    const delta = encode(42, base, newData);
    const tag: number = getTag(delta);

    if (tag !== 42) {
        throw new Error(`Expected tag 42, got ${tag}`);
    }
    console.log('✓ test_get_tag passed');
}

function test_compression(): void {
    const base = Buffer.alloc(1000, 'x');
    const newData = Buffer.concat([base, Buffer.alloc(100, 'y')]);

    const delta = encode(0, base, newData);

    if (delta.length >= newData.length) {
        throw new Error(`Delta size ${delta.length} should be less than new data size ${newData.length}`);
    }

    const reduction = (100 * (1 - delta.length / newData.length)).toFixed(1);
    console.log(`✓ test_compression passed (compressed ${newData.length} → ${delta.length} bytes, ${reduction}% reduction)`);
}

function test_empty_data(): void {
    const base = Buffer.from('');
    const newData = Buffer.from('Hello!');

    const delta = encode(0, base, newData);
    const reconstructed = decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error('Empty base data test failed');
    }
    console.log('✓ test_empty_data passed');
}

function test_large_tag(): void {
    const base = Buffer.from('test');
    const newData = Buffer.from('test data');

    const largeTag = 999999;
    const delta = encode(largeTag, base, newData);
    const tag = getTag(delta);

    if (tag !== largeTag) {
        throw new Error(`Expected tag ${largeTag}, got ${tag}`);
    }
    console.log('✓ test_large_tag passed');
}

function test_identical_data(): void {
    const data = Buffer.from('Same data');

    const delta = encode(0, data, data);
    const reconstructed = decode(data, delta);

    if (!reconstructed.equals(data)) {
        throw new Error('Identical data test failed');
    }
    console.log(`✓ test_identical_data passed (delta size: ${delta.length} bytes)`);
}

function test_zstd_disabled(): void {
    const base = Buffer.from('Hello, World!');
    const newData = Buffer.from('Hello, TypeScript!');

    const delta = encode(0, base, newData, false);
    const reconstructed = decode(base, delta);

    if (!reconstructed.equals(newData)) {
        throw new Error('Zstd disabled test failed');
    }
    console.log('✓ test_zstd_disabled passed');
}

function test_type_safety(): void {
    // TypeScript should enforce correct types
    const base: Buffer = Buffer.from('test');
    const newData: Buffer = Buffer.from('test123');

    const delta: Buffer = encode(0, base, newData);
    const tag: number = getTag(delta);
    const reconstructed: Buffer = decode(base, delta);

    // These should all be properly typed
    if (typeof tag !== 'number') {
        throw new Error('Tag should be a number');
    }
    if (!Buffer.isBuffer(delta)) {
        throw new Error('Delta should be a Buffer');
    }
    if (!Buffer.isBuffer(reconstructed)) {
        throw new Error('Reconstructed should be a Buffer');
    }

    console.log('✓ test_type_safety passed');
}

// Run all tests
console.log('Running xpatch-rs Node.js binding tests (TypeScript)...\n');

try {
    test_encode_decode();
    test_get_tag();
    test_compression();
    test_empty_data();
    test_large_tag();
    test_identical_data();
    test_zstd_disabled();
    test_type_safety();

    console.log('\n✅ All TypeScript tests passed!');
} catch (error) {
    console.error('\n❌ Test failed:', (error as Error).message);
    process.exit(1);
}
