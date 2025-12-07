// xpatch - High-performance delta compression library
// Copyright (c) 2025 Oliver Seifert
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// Commercial License Option:
// For commercial use in proprietary software, a commercial license is
// available. Contact xpatch-commercial@alias.oseifert.ch for details.

//! Variable-length integer encoding (varint) for compact storage.
//!
//! This module provides optimized encoding and decoding of unsigned integers
//! using a compact variable-length format. Smaller values use fewer bytes.
//!
//! # Format
//!
//! Each byte uses 7 bits for data and 1 bit as a continuation flag:
//! - Bit 7 (MSB): 1 = more bytes follow, 0 = last byte
//! - Bits 0-6: Data bits (little-endian)
//!
//! # Examples
//!
//! ```
//! # use xpatch::varint::{encode_varint, decode_varint};
//! // Small values use 1 byte
//! assert_eq!(encode_varint(127), vec![127]);
//!
//! // Larger values use multiple bytes
//! assert_eq!(encode_varint(300), vec![0xAC, 0x02]);
//!
//! // Decode returns (value, bytes_consumed)
//! assert_eq!(decode_varint(&[0xAC, 0x02]), (300, 2));
//! ```

/// Encodes an unsigned integer as a variable-length byte sequence.
///
/// # Performance
///
/// - Fast path for values < 128 (single byte, ~90% of cases)
/// - Pre-allocates based on value size
/// - Optimized with inlining hints
///
/// # Examples
///
/// ```
/// # use xpatch::varint::encode_varint;
/// assert_eq!(encode_varint(0), vec![0]);
/// assert_eq!(encode_varint(127), vec![127]);
/// assert_eq!(encode_varint(128), vec![0x80, 0x01]);
/// assert_eq!(encode_varint(16384), vec![0x80, 0x80, 0x01]);
/// ```
#[inline(always)]
pub fn encode_varint(mut value: usize) -> Vec<u8> {
    // Fast path: values < 128 need only 1 byte (~90% of cases)
    if value < 128 {
        return vec![value as u8];
    }

    // Pre-allocate based on value size
    // Formula: ceil(log2(value+1) / 7) = (bits_used + 6) / 7
    let capacity = if value == 0 {
        1
    } else {
        (usize::BITS - value.leading_zeros()).div_ceil(7) as usize
    };

    let mut bytes = Vec::with_capacity(capacity);

    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80; // Set continuation bit
        }
        bytes.push(byte);
        if value == 0 {
            break;
        }
    }

    bytes
}

/// Decodes a variable-length integer from a byte slice.
///
/// Returns a tuple of (decoded_value, bytes_consumed).
///
/// # Performance
///
/// - Fast path for single-byte values (< 128)
/// - Unrolled loop for first 4 bytes (common case)
/// - Handles larger values with fallback loop
///
/// # Panics
///
/// Does not panic on malformed input, but may return incorrect values
/// if the input is not a valid varint. Callers should ensure input validity.
///
/// # Examples
///
/// ```
/// # use xpatch::varint::decode_varint;
/// assert_eq!(decode_varint(&[0]), (0, 1));
/// assert_eq!(decode_varint(&[127]), (127, 1));
/// assert_eq!(decode_varint(&[0x80, 0x01]), (128, 2));
/// assert_eq!(decode_varint(&[0x80, 0x80, 0x01]), (16384, 3));
/// ```
#[inline(always)]
pub fn decode_varint(bytes: &[u8]) -> (usize, usize) {
    // Fast path: single-byte values (< 128)
    if bytes[0] < 128 {
        return (bytes[0] as usize, 1);
    }

    let mut result = 0usize;
    let mut shift = 0;

    // Unroll first 4 iterations (handles most multi-byte varints)
    // This covers values up to 2^28 - 1 = 268,435,455

    // Byte 0 (already checked: has continuation bit)
    result |= ((bytes[0] & 0x7F) as usize) << shift;
    if bytes[0] & 0x80 == 0 {
        return (result, 1);
    }
    shift += 7;

    // Byte 1
    if 1 >= bytes.len() {
        return (result, 1);
    }
    result |= ((bytes[1] & 0x7F) as usize) << shift;
    if bytes[1] & 0x80 == 0 {
        return (result, 2);
    }
    shift += 7;

    // Byte 2
    if 2 >= bytes.len() {
        return (result, 2);
    }
    result |= ((bytes[2] & 0x7F) as usize) << shift;
    if bytes[2] & 0x80 == 0 {
        return (result, 3);
    }
    shift += 7;

    // Byte 3
    if 3 >= bytes.len() {
        return (result, 3);
    }
    result |= ((bytes[3] & 0x7F) as usize) << shift;
    if bytes[3] & 0x80 == 0 {
        return (result, 4);
    }
    shift += 7;

    // Handle remaining bytes (rare, for values > 2^28)
    let mut i = 4;
    while i < bytes.len() {
        let byte = bytes[i];
        result |= ((byte & 0x7F) as usize) << shift;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }

    (result, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_zero() {
        let encoded = encode_varint(0);
        assert_eq!(encoded, vec![0]);
        assert_eq!(decode_varint(&encoded[..]), (0, 1));
    }

    #[test]
    fn test_encode_decode_small_values() {
        // Values < 128 should be 1 byte
        for i in 0..128 {
            let encoded = encode_varint(i);
            assert_eq!(encoded.len(), 1);
            assert_eq!(encoded[0], i as u8);
            assert_eq!(decode_varint(&encoded[..]), (i, 1));
        }
    }

    #[test]
    fn test_encode_decode_medium_values() {
        // Test some specific medium values
        let test_cases = vec![
            (128, vec![0x80, 0x01]),
            (300, vec![0xAC, 0x02]),
            (16384, vec![0x80, 0x80, 0x01]),
        ];

        for (value, expected) in test_cases {
            let encoded = encode_varint(value);
            assert_eq!(encoded, expected);
            assert_eq!(decode_varint(&encoded[..]), (value, encoded.len()));
        }
    }

    #[test]
    fn test_encode_decode_large_values() {
        let test_values = vec![1_000_000, 10_000_000, 100_000_000, usize::MAX / 2];

        for value in test_values {
            let encoded = encode_varint(value);
            let (decoded, bytes_consumed) = decode_varint(&encoded[..]);
            assert_eq!(decoded, value);
            assert_eq!(bytes_consumed, encoded.len());
        }
    }

    #[test]
    fn test_encode_max_value() {
        let value = usize::MAX;
        let encoded = encode_varint(value);
        let (decoded, bytes_consumed) = decode_varint(&encoded[..]);
        assert_eq!(decoded, value);
        assert_eq!(bytes_consumed, encoded.len());
    }

    #[test]
    fn test_roundtrip_random_values() {
        // Test a range of values across different sizes
        let test_values = [
            1, 10, 100, 127, 128, 255, 256, 1000, 10000, 65535, 65536, 100000, 1000000, 16777215,
            16777216,
        ];

        for &value in &test_values {
            let encoded = encode_varint(value);
            let (decoded, bytes_consumed) = decode_varint(&encoded[..]);
            assert_eq!(decoded, value, "Failed roundtrip for value {}", value);
            assert_eq!(bytes_consumed, encoded.len());
        }
    }

    #[test]
    fn test_decode_partial_buffer() {
        // Test decoding with extra bytes after the varint
        let mut buffer = encode_varint(12345);
        buffer.extend_from_slice(&[0xFF, 0xFF, 0xFF]);

        let (decoded, bytes_consumed) = decode_varint(&buffer[..]);
        assert_eq!(decoded, 12345);
        assert!(bytes_consumed < buffer.len());
    }

    #[test]
    fn test_continuation_bits() {
        // Verify continuation bits are set correctly
        let encoded = encode_varint(128);
        assert!(
            encoded[0] & 0x80 != 0,
            "First byte should have continuation bit"
        );
        assert!(
            encoded[1] & 0x80 == 0,
            "Last byte should not have continuation bit"
        );
    }

    #[test]
    fn test_size_efficiency() {
        // Verify we're using the minimum number of bytes
        assert_eq!(encode_varint(0).len(), 1);
        assert_eq!(encode_varint(127).len(), 1);
        assert_eq!(encode_varint(128).len(), 2);
        assert_eq!(encode_varint(16383).len(), 2);
        assert_eq!(encode_varint(16384).len(), 3);
    }

    #[test]
    fn test_encode_capacity_optimization() {
        // Verify pre-allocation is working correctly
        // (This is more of an implementation detail check)
        for value in [0, 127, 128, 16384, 1_000_000] {
            let encoded = encode_varint(value);
            // Capacity should equal length (no over-allocation)
            assert_eq!(
                encoded.capacity(),
                encoded.len(),
                "Over-allocated for value {}",
                value
            );
        }
    }
}
