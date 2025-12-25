#!/usr/bin/env python3
"""Unit tests for xpatch Python bindings."""

import xpatch


def test_encode_decode():
    """Test basic encode/decode round-trip."""
    base = b"Hello, World!"
    new = b"Hello, Python!"

    delta = xpatch.encode(0, base, new)
    reconstructed = xpatch.decode(base, delta)

    assert reconstructed == new, f"Expected {new}, got {reconstructed}"
    print("✓ test_encode_decode passed")


def test_get_tag():
    """Test tag extraction from delta."""
    base = b"test"
    new = b"test123"

    delta = xpatch.encode(42, base, new)
    tag = xpatch.get_tag(delta)

    assert tag == 42, f"Expected tag 42, got {tag}"
    print("✓ test_get_tag passed")


def test_compression():
    """Test that repeating compression actually reduces size."""
    base = b"x" * 1000
    new = b"x" * 1000 + b"y" * 100

    delta = xpatch.encode(0, base, new)

    assert len(delta) < len(new), f"Delta size {len(delta)} should be less than new data size {len(new)}"
    print(f"✓ test_compression passed (compressed {len(new)} → {len(delta)} bytes, {100 * (1 - len(delta)/len(new)):.1f}% reduction)")


def test_empty_data():
    """Test handling of empty data."""
    base = b""
    new = b"Hello!"

    delta = xpatch.encode(0, base, new)
    reconstructed = xpatch.decode(base, delta)

    assert reconstructed == new
    print("✓ test_empty_data passed")


def test_large_tag():
    """Test with large tag values."""
    base = b"test"
    new = b"test data"

    # Test with a large tag value
    large_tag = 999999
    delta = xpatch.encode(large_tag, base, new)
    tag = xpatch.get_tag(delta)

    assert tag == large_tag, f"Expected tag {large_tag}, got {tag}"
    print("✓ test_large_tag passed")


def test_identical_data():
    """Test encoding identical data."""
    data = b"Same data"

    delta = xpatch.encode(0, data, data)
    reconstructed = xpatch.decode(data, delta)

    assert reconstructed == data
    print(f"✓ test_identical_data passed (delta size: {len(delta)} bytes)")


def test_zstd_disabled():
    """Test encoding with zstd disabled."""
    base = b"Hello, World!"
    new = b"Hello, Rust!"

    delta = xpatch.encode(0, base, new, enable_zstd=False)
    reconstructed = xpatch.decode(base, delta)

    assert reconstructed == new
    print("✓ test_zstd_disabled passed")


if __name__ == "__main__":
    print("Running xpatch Python binding tests...\n")

    test_encode_decode()
    test_get_tag()
    test_compression()
    test_empty_data()
    test_large_tag()
    test_identical_data()
    test_zstd_disabled()

    print("\n✅ All tests passed!")
