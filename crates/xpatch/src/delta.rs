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

//! Delta encoding and decoding for efficient storage of sequential data changes.
//!
//! This module provides multiple compression algorithms that can be automatically
//! selected based on the type of change detected. It supports:
//! - Simple character insertion (Chars)
//! - Token-based compression (Tokens)
//! - Continuous removal (Remove)
//! - Repetitive character patterns (RepeatChars)
//! - Repetitive token patterns (RepeatTokens)
//! - General-purpose delta compression (GDelta)
//! - Zstd-compressed character insertion (CharsZstd)
//! - Zstd-compressed general delta (GDeltaZstd)

use crate::debug::{
    debug_delta_analyze, debug_delta_compress, debug_delta_encode, debug_delta_header,
    debug_delta_pattern, debug_delta_token,
};
use crate::tokenizer;
use crate::varint::{decode_varint, encode_varint};
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Available compression algorithms for delta encoding.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum Algorithm {
    /// Simple byte range removal
    Remove = 0,
    /// Character-by-character insertion (Chars)
    Chars = 1,
    /// Token-based compression for text (Tokens)
    Tokens = 2,
    /// General-purpose delta compression (GDelta)
    GDelta = 3,
    /// Repetitive character pattern insertion (RepeatChars)
    RepeatChars = 4,
    /// Repetitive token pattern insertion (RepeatTokens)
    RepeatTokens = 5,
    /// General-purpose delta compression with zstd (GDeltaZstd)
    GDeltaZstd = 6,
    /// Character insertion with zstd compression (CharsZstd)
    CharsZstd = 7,
}

/// Encodes the difference between base data and new data as a compact delta.
///
/// Automatically selects the best compression algorithm based on change analysis.
/// Returns a delta that can be applied to the base data to reconstruct the new data.
///
/// # Arguments
/// * `tag` - User-defined metadata value (can be used as history reference, version tag, etc.)
/// * `base_data` - The base data to compare against
/// * `new_data` - The new data to encode
/// * `enable_zstd` - Whether to enable zstd compression for GDelta
pub fn encode(tag: usize, base_data: &[u8], new_data: &[u8], enable_zstd: bool) -> Vec<u8> {
    debug_delta_encode!("-------------------------------------------");
    let change = analyze_change(base_data, new_data);

    // Try specialized algorithms based on change type
    let (best_algo, best_data) = match change {
        ChangeType::ContinuousAdd { position, data } => {
            debug_delta_compress!("Detected ContinuousAdd at position {}", position);

            // Use character-based (Chars) encoding
            let mut best_algo = Algorithm::Chars;
            let mut best_data = encode_add(position, &data[..]);
            debug_delta_compress!("  {:?}: {} bytes", best_algo, best_data.len());

            // Try token-based (Tokens) encoding for potentially better compression
            if let Ok(token_data) = encode_tokens(position, &data[..])
                && token_data.len() < best_data.len()
            {
                best_algo = Algorithm::Tokens;
                best_data = token_data;
                debug_delta_compress!("  {:?}: {} bytes", best_algo, best_data.len());
            }

            // Try repetitive character pattern (RepeatChars) encoding
            if let Some((pattern, repeat_count)) = detect_repeating_pattern(&data[..])
                && repeat_count >= 2
            {
                if let Ok(repeat_char_data) =
                    encode_repeat_chars(position, &pattern[..], repeat_count)
                    && repeat_char_data.len() < best_data.len()
                {
                    best_algo = Algorithm::RepeatChars;
                    best_data = repeat_char_data;
                    debug_delta_compress!("  {:?}: {} bytes", best_algo, best_data.len());
                }

                // Try repetitive token pattern (RepeatTokens) encoding
                if let Ok(repeat_token_data) =
                    encode_repeat_tokens(position, &pattern[..], repeat_count)
                    && repeat_token_data.len() < best_data.len()
                {
                    best_algo = Algorithm::RepeatTokens;
                    best_data = repeat_token_data;
                    debug_delta_compress!("  {:?}: {} bytes", best_algo, best_data.len());
                }
            }

            // Try zstd compression (CharsZstd) on the raw data
            if enable_zstd
                && let Ok(chars_zstd_data) = encode_chars_zstd(position, &data[..])
                && chars_zstd_data.len() < best_data.len()
            {
                best_algo = Algorithm::CharsZstd;
                best_data = chars_zstd_data;
                debug_delta_compress!("  {:?}: {} bytes", best_algo, best_data.len());
            }

            (best_algo, best_data)
        }
        ChangeType::ContinuousRemove { start, end } => {
            debug_delta_compress!("Detected ContinuousRemove from {} to {}", start, end);
            debug_delta_compress!("  Remove: 3 bytes");
            (Algorithm::Remove, encode_remove(start, end))
        }
        ChangeType::Complex => {
            debug_delta_compress!("Detected Complex change, using GDelta");

            let gdelta_data = gdelta::encode(new_data, base_data).expect("GDelta failed");
            debug_delta_compress!("  GDelta: {} bytes", gdelta_data.len());

            // Try zstd compression on top of gdelta (GDeltaZstd)
            let mut best_algo = Algorithm::GDelta;
            let mut best_data = gdelta_data.to_owned();

            if enable_zstd && let Ok(compressed) = zstd::encode_all(gdelta_data.as_slice(), 3) {
                debug_delta_compress!("  GDeltaZstd: {} bytes", compressed.len());

                if compressed.len() < best_data.len() {
                    best_algo = Algorithm::GDeltaZstd;
                    best_data = compressed;
                }
            }

            (best_algo, best_data)
        }
    };

    debug_delta_compress!("-------------------------------------------");
    debug_delta_compress!(
        "Chose Algorithm: {:?}; With size: {}",
        best_algo,
        best_data.len()
    );
    debug_delta_compress!("-------------------------------------------");

    // Build delta: [header][algorithm_data]
    let header = encode_header(best_algo, tag);

    let mut delta = Vec::with_capacity(header.len() + best_data.len());
    delta.extend(header);
    delta.extend(best_data.as_slice());

    // Debug statistics
    #[cfg(feature = "debug_delta_encode")]
    {
        let original_size = base_data.len();
        let new_size = new_data.len();
        let delta_size = delta.len();
        let delta_ratio = delta_size as f64 / new_size as f64;
        let space_savings = if original_size > 0 {
            (1.0 - delta_size as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        debug_delta_encode!("");
        debug_delta_encode!("=== Compression Analysis ===");
        debug_delta_encode!("Original size: {} bytes", original_size);
        debug_delta_encode!("New size: {} bytes", new_size);
        debug_delta_encode!("Delta size: {} bytes", delta_size);
        debug_delta_encode!("Delta ratio: {:.3} (delta/new)", delta_ratio);
        debug_delta_encode!("Space savings: {:.1}%", space_savings);
        debug_delta_encode!("============================");
        debug_delta_encode!("");
        debug_delta_encode!("-------------------------------------------");
    }

    delta
}

/// Extracts tag from a delta without fully decoding it.
///
/// Returns the user-defined tag value embedded in the delta.
#[inline]
pub fn get_tag(delta: &[u8]) -> Result<usize, &'static str> {
    if delta.is_empty() {
        return Err("Empty delta");
    }
    let (_, tag, _) = decode_header(delta)?;

    Ok(tag)
}

/// Decodes a delta and applies it to base data to reconstruct the new data.
///
/// # Arguments
/// * `base_data` - The base data the delta was created from
/// * `delta` - The encoded delta to apply
#[inline]
pub fn decode(base_data: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str> {
    if delta.is_empty() {
        return Err("Empty delta");
    }

    // Extract delta components
    let (algo_type, _tag, header_bytes) = decode_header(delta)?;
    let delta = &delta[header_bytes..];

    // Decode using the appropriate algorithm
    let decoded = match algo_type {
        Algorithm::Remove => decode_remove(base_data, delta)?,
        Algorithm::Chars => decode_add(base_data, delta)?,
        Algorithm::Tokens => match decode_tokens(base_data, delta) {
            Ok(d) => d,
            Err(_) => return Err("Error while decoding Tokens"),
        },
        Algorithm::RepeatChars => decode_repeat_chars(base_data, delta)?,
        Algorithm::RepeatTokens => match decode_repeat_tokens(base_data, delta) {
            Ok(d) => d,
            Err(_) => return Err("Error while decoding RepeatTokens"),
        },
        Algorithm::GDelta => match gdelta::decode(delta, base_data) {
            Ok(d) => d,
            Err(_) => return Err("Error decoding gdelta"),
        },
        Algorithm::GDeltaZstd => {
            // Decompress with zstd first
            let decompressed = match zstd::decode_all(delta) {
                Ok(d) => d,
                Err(_) => return Err("Error decompressing zstd data"),
            };

            // Then decode with gdelta
            match gdelta::decode(&decompressed[..], base_data) {
                Ok(d) => d,
                Err(_) => return Err("Error decoding gdelta"),
            }
        }
        Algorithm::CharsZstd => match decode_chars_zstd(base_data, delta) {
            Ok(d) => d,
            Err(_) => return Err("Error while decoding CharsZstd"),
        },
    };

    Ok(decoded)
}

// ============================================================================
// CHANGE ANALYSIS
// ============================================================================

/// Classification of the type of change between two byte sequences.
#[derive(Debug, Clone)]
pub enum ChangeType {
    /// A continuous block of bytes was inserted at a single position
    ContinuousAdd { position: usize, data: Vec<u8> },
    /// A continuous block of bytes was removed
    ContinuousRemove { start: usize, end: usize },
    /// Changes are scattered or complex (multiple edits)
    Complex,
}

/// Analyzes the difference between old and new data to classify the change type.
///
/// This helps select the most efficient encoding algorithm.
fn analyze_change(old: &[u8], new: &[u8]) -> ChangeType {
    debug_delta_analyze!(
        "Analyzing change: old={} bytes, new={} bytes",
        old.len(),
        new.len()
    );

    // Check if there's no change
    if old == new {
        debug_delta_analyze!("  ✓ No change detected (returning empty addition)");
        return ChangeType::ContinuousAdd {
            position: 0,
            data: vec![],
        };
    }

    // Check for continuous addition
    if new.len() > old.len() {
        let added_len = new.len() - old.len();
        debug_delta_analyze!("  Checking for continuous addition ({} bytes)", added_len);

        // Find where the insertion begins using optimized comparison
        let position = find_common_prefix(old, new);
        debug_delta_analyze!("  Common prefix ends at position {}", position);

        // Check if everything after the insertion matches
        let expected_end = position + added_len;
        if expected_end <= new.len() {
            let old_suffix = &old[position..];
            let new_suffix = &new[expected_end..];

            if old_suffix == new_suffix {
                debug_delta_analyze!("  ✓ Detected ContinuousAdd at position {}", position);
                return ChangeType::ContinuousAdd {
                    position,
                    data: new[position..expected_end].to_vec(),
                };
            }
        }
        debug_delta_analyze!("  ✗ Not a continuous addition");
    }

    // Check for continuous removal
    if old.len() > new.len() {
        let deleted_len = old.len() - new.len();
        debug_delta_analyze!("  Checking for continuous removal ({} bytes)", deleted_len);

        // Find where the deletion begins using optimized comparison
        let start = find_common_prefix(old, new);
        debug_delta_analyze!("  Common prefix ends at position {}", start);

        // Check if everything after the deletion matches
        let end = start + deleted_len;
        if end <= old.len() {
            let old_suffix = &old[end..];
            let new_suffix = &new[start..];

            if old_suffix == new_suffix {
                debug_delta_analyze!("  ✓ Detected ContinuousRemove from {} to {}", start, end);
                return ChangeType::ContinuousRemove { start, end };
            }
        }
        debug_delta_analyze!("  ✗ Not a continuous removal");
    }

    debug_delta_analyze!("  → Complex change detected");
    ChangeType::Complex
}

/// Optimized common prefix finding with SIMD on x86_64
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_common_prefix_avx2(a: &[u8], b: &[u8]) -> usize {
    unsafe {
        use std::arch::x86_64::*;

        let len = a.len().min(b.len());
        let mut i = 0;

        // Process 32 bytes at a time with AVX2
        while i + 32 <= len {
            let a_vec = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
            let b_vec = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(a_vec, b_vec);
            let mask = _mm256_movemask_epi8(cmp);

            if mask != -1 {
                return i + mask.trailing_ones() as usize;
            }
            i += 32;
        }

        // Handle remaining bytes
        while i < len && a[i] == b[i] {
            i += 1;
        }

        i
    }
}

#[inline]
fn find_common_prefix(a: &[u8], b: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { find_common_prefix_avx2(a, b) };
        }
    }

    // Fallback: compare in chunks for better performance
    let len = a.len().min(b.len());
    let mut i = 0;

    // Process 8 bytes at a time
    while i + 8 <= len {
        if a[i..i + 8] != b[i..i + 8] {
            // Find exact position
            while i < len && a[i] == b[i] {
                i += 1;
            }
            return i;
        }
        i += 8;
    }

    // Handle remaining bytes
    while i < len && a[i] == b[i] {
        i += 1;
    }

    i
}

// ============================================================================
// PATTERN DETECTION
// ============================================================================

/// Detects if the data consists of a repeating pattern.
///
/// Returns the shortest repeating unit and how many times it repeats.
/// Returns None if no repetition is detected or if it's not efficient to encode.
fn detect_repeating_pattern(data: &[u8]) -> Option<(Vec<u8>, usize)> {
    if data.is_empty() || data.len() < 4 {
        return None;
    }

    debug_delta_pattern!("Detecting repeating pattern in {} bytes", data.len());

    let max_pattern_len = data.len() / 2;

    // Try powers of 2 first (common in programming: 1, 2, 4, 8, 16, 32, 64)
    for exp in 0..7 {
        let pattern_len = 1 << exp;
        if pattern_len > max_pattern_len {
            break;
        }

        if data.len().is_multiple_of(pattern_len) && check_pattern_optimized(data, pattern_len) {
            let repeat_count = data.len() / pattern_len;

            debug_delta_pattern!(
                "  ✓ Detected repeating pattern: {} bytes × {} times (power of 2)",
                pattern_len,
                repeat_count
            );
            debug_delta_encode!(
                "  Detected repeating pattern: {} bytes × {} times",
                pattern_len,
                repeat_count
            );

            return Some((data[..pattern_len].to_vec(), repeat_count));
        }
    }

    // Try other common lengths: 3, 5, 6, 7, 9, 10, 12, etc.
    for pattern_len in [3, 5, 6, 7, 9, 10, 12, 15, 20, 24, 30] {
        if pattern_len > max_pattern_len {
            break;
        }

        if data.len().is_multiple_of(pattern_len) && check_pattern_optimized(data, pattern_len) {
            let repeat_count = data.len() / pattern_len;

            debug_delta_pattern!(
                "  ✓ Detected repeating pattern: {} bytes × {} times",
                pattern_len,
                repeat_count
            );
            debug_delta_encode!(
                "  Detected repeating pattern: {} bytes × {} times",
                pattern_len,
                repeat_count
            );

            return Some((data[..pattern_len].to_vec(), repeat_count));
        }
    }

    debug_delta_pattern!("  ✗ No repeating pattern detected");
    None
}

/// Optimized pattern checking using chunk comparison
#[inline]
fn check_pattern_optimized(data: &[u8], pattern_len: usize) -> bool {
    let pattern = &data[..pattern_len];
    data.chunks_exact(pattern_len).all(|chunk| chunk == pattern)
}

// ============================================================================
// HEADER ENCODING
// ============================================================================

/// Encodes the algorithm type and tag into a compact header.
///
/// Uses a 3-bit algorithm identifier and variable-length encoding for the tag.
/// Format: `[3-bit algo][1-bit flag][4/variable-bit tag]`
#[inline]
pub fn encode_header(algo_type: Algorithm, tag: usize) -> Vec<u8> {
    let algo_type = algo_type as u8;

    if tag < 16 {
        // Small tag: fit in lower 4 bits
        debug_delta_header!(
            "Encoding header: algo={:?}, tag={} (small, 1 byte)",
            Algorithm::try_from_primitive(algo_type).unwrap(),
            tag
        );
        vec![(algo_type << 5) | (tag as u8)]
    } else {
        // Large tag: use continuation bytes
        let first_bits = (tag & 0x0F) as u8;
        let mut bytes =
            Vec::with_capacity(1 + ((usize::BITS - (tag >> 4).leading_zeros()) / 7) as usize);
        bytes.push((algo_type << 5) | 0x10 | first_bits);

        let mut remaining = tag >> 4;
        loop {
            let mut byte = (remaining & 0x7F) as u8;
            remaining >>= 7;
            if remaining != 0 {
                byte |= 0x80; // Continuation flag
            }
            bytes.push(byte);
            if remaining == 0 {
                break;
            }
        }

        debug_delta_header!(
            "Encoding header: algo={:?}, tag={} (large, {} bytes)",
            Algorithm::try_from_primitive(algo_type).unwrap(),
            tag,
            bytes.len()
        );
        bytes
    }
}

/// Decodes the algorithm type and tag from a header.
///
/// Returns the algorithm, tag value, and number of bytes consumed.
#[inline]
pub fn decode_header(bytes: &[u8]) -> Result<(Algorithm, usize, usize), &'static str> {
    if bytes.is_empty() {
        return Err("Empty header delta");
    }

    let first_byte = bytes[0];
    let algo_type = first_byte >> 5;
    let algorithm = match Algorithm::try_from_primitive(algo_type) {
        Ok(algo) => algo,
        Err(_) => return Err("Unsupported algorithm"),
    };

    if (first_byte & 0x10) == 0 {
        // Small tag: contained in first byte
        let tag = (first_byte & 0x0F) as usize;
        debug_delta_header!(
            "Decoded header: algo={:?}, tag={} (small, 1 byte)",
            algorithm,
            tag
        );
        Ok((algorithm, tag, 1))
    } else {
        // Large tag: decode continuation bytes
        let first_bits = (first_byte & 0x0F) as usize;
        let mut result = first_bits;
        let mut shift = 4;
        let mut i = 1;

        loop {
            if i >= bytes.len() {
                return Err("Incomplete varint");
            }
            let byte = bytes[i];
            result |= ((byte & 0x7F) as usize) << shift;
            i += 1;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }

        debug_delta_header!(
            "Decoded header: algo={:?}, tag={} (large, {} bytes)",
            algorithm,
            result,
            i
        );
        Ok((algorithm, result, i))
    }
}

// ============================================================================
// CHARS ALGORITHM - Simple character insertion
// ============================================================================

/// Encodes a continuous insertion of characters at a specific position.
#[inline]
fn encode_add(position: usize, data: &[u8]) -> Vec<u8> {
    let mut encoded = encode_varint(position);
    encoded.extend_from_slice(data);
    encoded
}

/// Decodes and applies a character insertion (Chars) to the base data.
#[inline]
fn decode_add(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str> {
    if delta.is_empty() {
        return Err("Empty add delta");
    }

    let (position, varint_len) = decode_varint(delta);
    let bytes_to_insert = &delta[varint_len..];

    if position > base.len() {
        return Err("Insert position out of bounds");
    }

    let mut result = Vec::with_capacity(base.len() + bytes_to_insert.len());
    result.extend_from_slice(&base[..position]);
    result.extend_from_slice(bytes_to_insert);
    result.extend_from_slice(&base[position..]);

    Ok(result)
}

// ============================================================================
// CHARSZSTD ALGORITHM - Character insertion with zstd compression
// ============================================================================

/// Encodes a continuous insertion of characters with zstd compression.
fn encode_chars_zstd(position: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    // Compress the data with zstd
    let compressed = match zstd::encode_all(data, 3) {
        Ok(c) => c,
        Err(e) => return Err(format!("zstd compression failed: {}", e)),
    };

    // Build encoded format: [position][compressed_data]
    let mut encoded = encode_varint(position);
    encoded.extend_from_slice(&compressed[..]);

    Ok(encoded)
}

/// Decodes and applies a zstd-compressed character insertion (CharsZstd) to the base data.
fn decode_chars_zstd(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    if delta.is_empty() {
        return Err("Empty chars zstd delta".to_string());
    }

    // Decode position
    let (position, varint_len) = decode_varint(delta);

    if position > base.len() {
        return Err(format!(
            "Insert position {} out of bounds (base len: {})",
            position,
            base.len()
        ));
    }

    // Decompress the data
    let compressed_data = &delta[varint_len..];
    let bytes_to_insert = match zstd::decode_all(compressed_data) {
        Ok(d) => d,
        Err(e) => return Err(format!("zstd decompression failed: {}", e)),
    };

    // Build result with insertion
    let mut result = Vec::with_capacity(base.len() + bytes_to_insert.len());
    result.extend_from_slice(&base[..position]);
    result.extend_from_slice(&bytes_to_insert);
    result.extend_from_slice(&base[position..]);

    Ok(result)
}

// ============================================================================
// REMOVE ALGORITHM - Continuous byte removal
// ============================================================================

/// Encodes a continuous removal of bytes from start to end position.
#[inline]
fn encode_remove(start: usize, end: usize) -> Vec<u8> {
    let mut encoded = encode_varint(start);
    encoded.extend(encode_varint(end - start));
    encoded
}

/// Decodes and applies a byte range removal (Remove) to the base data.
#[inline]
fn decode_remove(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str> {
    if delta.is_empty() {
        return Err("Empty remove delta");
    }

    let (start, varint_len) = decode_varint(delta);
    let (distance, _) = decode_varint(&delta[varint_len..]);
    let end = start + distance;

    if start > end || end > base.len() {
        return Err("Invalid deletion range");
    }

    let mut result = Vec::with_capacity(base.len() - (end - start));
    result.extend_from_slice(&base[..start]);
    result.extend_from_slice(&base[end..]);

    Ok(result)
}

// ============================================================================
// TOKENS ALGORITHM - Tokenized continuous addition
// ============================================================================

/// Encodes a continuous insertion using tokenization (Tokens) for better compression.
///
/// Particularly effective for text data where tokens can represent common patterns.
fn encode_tokens(position: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    debug_delta_token!(
        "Encoding {} bytes at position {} using tokens...",
        data.len(),
        position
    );

    // Tokenize the data
    let token_indices = tokenizer::encode(data)?;
    debug_delta_token!("  Tokenized to {} tokens", token_indices.len());

    // Estimate capacity: position varint + count varint + token varints
    let estimated_capacity = 2 + token_indices.len() * 2;
    let mut encoded = Vec::with_capacity(estimated_capacity);

    encoded.extend(encode_varint(position));
    encoded.extend(encode_varint(token_indices.len()));

    for &token_id in &token_indices {
        encoded.extend(encode_varint(token_id));
    }

    #[cfg(feature = "debug_delta_token")]
    {
        let original_size = data.len();
        let encoded_size = encoded.len();
        let ratio = encoded_size as f64 / original_size as f64;

        debug_delta_token!("  Original: {} bytes", original_size);
        debug_delta_token!("  Encoded: {} bytes", encoded_size);
        debug_delta_token!("  Ratio: {:.3} ({:.1}% of original)", ratio, ratio * 100.0);
    }

    Ok(encoded)
}

/// Decodes and applies a tokenized insertion (Tokens) to the base data.
fn decode_tokens(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    debug_delta_token!("Decoding tokens delta ({} bytes)...", delta.len());

    if delta.is_empty() {
        return Err("Empty tokens delta".to_string());
    }

    // Decode position
    let (position, mut offset) = decode_varint(delta);
    debug_delta_token!("  Insert position: {}", position);

    if position > base.len() {
        return Err(format!(
            "Insert position {} out of bounds (base len: {})",
            position,
            base.len()
        ));
    }

    // Decode token count
    let (token_count, varint_len) = decode_varint(&delta[offset..]);
    offset += varint_len;
    debug_delta_token!("  Token count: {}", token_count);

    // Decode all token indices
    let mut token_indices = Vec::with_capacity(token_count);
    for _i in 0..token_count {
        if offset >= delta.len() {
            return Err("Incomplete token data".to_string());
        }
        let (token_id, varint_len) = decode_varint(&delta[offset..]);
        debug_delta_token!("    Token {}: id={}", _i, token_id);
        token_indices.push(token_id);
        offset += varint_len;
    }

    // Decode tokens back to bytes
    let bytes_to_insert = tokenizer::decode(&token_indices[..])?;
    debug_delta_token!("  Decoded to {} bytes", bytes_to_insert.len());

    // Build result with insertion
    let mut result = Vec::with_capacity(base.len() + bytes_to_insert.len());
    result.extend_from_slice(&base[..position]);
    result.extend_from_slice(&bytes_to_insert);
    result.extend_from_slice(&base[position..]);

    debug_delta_token!(
        "  Final size: {} bytes (base: {}, added: {})",
        result.len(),
        base.len(),
        bytes_to_insert.len()
    );

    Ok(result)
}

// ============================================================================
// REPEAT CHARS ALGORITHM - Repetitive character pattern insertion
// ============================================================================

/// Encodes a repetitive character pattern insertion (RepeatChars).
///
/// Format: [position][repeat_count][pattern_bytes...]
fn encode_repeat_chars(
    position: usize,
    pattern: &[u8],
    repeat_count: usize,
) -> Result<Vec<u8>, String> {
    debug_delta_encode!(
        "Encoding RepeatChars: {} bytes × {} at position {}",
        pattern.len(),
        repeat_count,
        position
    );

    let mut encoded = Vec::with_capacity(2 + pattern.len());
    encoded.extend(encode_varint(position));
    encoded.extend(encode_varint(repeat_count));
    encoded.extend_from_slice(pattern);

    debug_delta_encode!("  RepeatChars encoded size: {} bytes", encoded.len());

    Ok(encoded)
}

/// Decodes and applies a repetitive character pattern insertion (RepeatChars).
#[inline]
fn decode_repeat_chars(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, &'static str> {
    if delta.is_empty() {
        return Err("Empty repeat chars delta");
    }

    // Decode position
    let (position, mut offset) = decode_varint(delta);

    if position > base.len() {
        return Err("Insert position out of bounds");
    }

    // Decode repeat count
    let (repeat_count, varint_len) = decode_varint(&delta[offset..]);
    offset += varint_len;

    // The rest is the pattern
    let pattern = &delta[offset..];

    if pattern.is_empty() {
        return Err("Empty pattern in repeat chars");
    }

    // Build the repeated data
    let mut bytes_to_insert = Vec::with_capacity(pattern.len() * repeat_count);
    for _ in 0..repeat_count {
        bytes_to_insert.extend_from_slice(pattern);
    }

    // Build result with insertion
    let mut result = Vec::with_capacity(base.len() + bytes_to_insert.len());
    result.extend_from_slice(&base[..position]);
    result.extend_from_slice(&bytes_to_insert);
    result.extend_from_slice(&base[position..]);

    Ok(result)
}

// ============================================================================
// REPEAT TOKENS ALGORITHM - Repetitive token pattern insertion
// ============================================================================

/// Encodes a repetitive token pattern insertion (RepeatTokens).
///
/// Format: [position][repeat_count][pattern_token_count][pattern_token_ids...]
fn encode_repeat_tokens(
    position: usize,
    pattern: &[u8],
    repeat_count: usize,
) -> Result<Vec<u8>, String> {
    debug_delta_token!(
        "Encoding RepeatTokens: {} bytes × {} at position {}",
        pattern.len(),
        repeat_count,
        position
    );

    // Tokenize the pattern once
    let token_indices = tokenizer::encode(pattern)?;
    debug_delta_token!("  Pattern tokenized to {} tokens", token_indices.len());

    // Build encoded format: [position][repeat_count][pattern_token_count][token_ids...]
    let mut encoded = Vec::with_capacity(3 + token_indices.len() * 2);
    encoded.extend(encode_varint(position));
    encoded.extend(encode_varint(repeat_count));
    encoded.extend(encode_varint(token_indices.len()));

    for &token_id in &token_indices {
        encoded.extend(encode_varint(token_id));
    }

    debug_delta_token!("  RepeatTokens encoded size: {} bytes", encoded.len());

    Ok(encoded)
}

/// Decodes and applies a repetitive token pattern insertion (RepeatTokens).
fn decode_repeat_tokens(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    debug_delta_token!("Decoding RepeatTokens delta ({} bytes)...", delta.len());

    if delta.is_empty() {
        return Err("Empty repeat tokens delta".to_string());
    }

    // Decode position
    let (position, mut offset) = decode_varint(delta);
    debug_delta_token!("  Insert position: {}", position);

    if position > base.len() {
        return Err(format!(
            "Insert position {} out of bounds (base len: {})",
            position,
            base.len()
        ));
    }

    // Decode repeat count
    let (repeat_count, varint_len) = decode_varint(&delta[offset..]);
    offset += varint_len;
    debug_delta_token!("  Repeat count: {}", repeat_count);

    // Decode pattern token count
    let (pattern_token_count, varint_len) = decode_varint(&delta[offset..]);
    offset += varint_len;
    debug_delta_token!("  Pattern token count: {}", pattern_token_count);

    // Decode pattern token indices
    let mut pattern_token_indices = Vec::with_capacity(pattern_token_count);
    for _i in 0..pattern_token_count {
        if offset >= delta.len() {
            return Err("Incomplete token data".to_string());
        }
        let (token_id, varint_len) = decode_varint(&delta[offset..]);
        debug_delta_token!("    Pattern token {}: id={}", _i, token_id);
        pattern_token_indices.push(token_id);
        offset += varint_len;
    }

    // Decode the pattern from tokens
    let pattern_bytes = tokenizer::decode(&pattern_token_indices[..])?;
    debug_delta_token!("  Pattern decoded to {} bytes", pattern_bytes.len());

    // Build the repeated data
    let mut bytes_to_insert = Vec::with_capacity(pattern_bytes.len() * repeat_count);
    for _ in 0..repeat_count {
        bytes_to_insert.extend_from_slice(&pattern_bytes);
    }

    debug_delta_token!("  Total insertion: {} bytes", bytes_to_insert.len());

    // Build result with insertion
    let mut result = Vec::with_capacity(base.len() + bytes_to_insert.len());
    result.extend_from_slice(&base[..position]);
    result.extend_from_slice(&bytes_to_insert);
    result.extend_from_slice(&base[position..]);

    debug_delta_token!(
        "  Final size: {} bytes (base: {}, added: {})",
        result.len(),
        base.len(),
        bytes_to_insert.len()
    );

    Ok(result)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // HEADER ENCODING/DECODING TESTS
    // ========================================================================

    #[test]
    fn test_header_small_tag() {
        // Test tags that fit in 4 bits (0-15)
        for tag in 0..16 {
            let header = encode_header(Algorithm::Chars, tag);
            assert_eq!(header.len(), 1, "Small tag should encode to 1 byte");

            let (algo, decoded_tag, bytes_read) = decode_header(&header[..]).unwrap();
            assert_eq!(algo, Algorithm::Chars);
            assert_eq!(decoded_tag, tag);
            assert_eq!(bytes_read, 1);
        }
    }

    #[test]
    fn test_header_large_tag() {
        // Test tags that require continuation bytes
        let test_cases = vec![16, 100, 1000, 10000, 65535, 1_000_000];

        for tag in test_cases {
            for algo in [
                Algorithm::Chars,
                Algorithm::Tokens,
                Algorithm::Remove,
                Algorithm::RepeatChars,
                Algorithm::RepeatTokens,
                Algorithm::GDelta,
                Algorithm::GDeltaZstd,
                Algorithm::CharsZstd,
            ] {
                let header = encode_header(algo, tag);
                assert!(
                    header.len() > 1,
                    "Large tag should encode to multiple bytes"
                );

                let (decoded_algo, decoded_tag, bytes_read) = decode_header(&header[..]).unwrap();
                assert_eq!(decoded_algo, algo);
                assert_eq!(decoded_tag, tag);
                assert_eq!(bytes_read, header.len());
            }
        }
    }

    #[test]
    fn test_header_all_algorithms() {
        let tag = 42;
        let algorithms = vec![
            Algorithm::Remove,
            Algorithm::Chars,
            Algorithm::Tokens,
            Algorithm::GDelta,
            Algorithm::RepeatChars,
            Algorithm::RepeatTokens,
            Algorithm::GDeltaZstd,
            Algorithm::CharsZstd,
        ];

        for algo in algorithms {
            let header = encode_header(algo, tag);
            let (decoded_algo, decoded_tag, _) = decode_header(&header[..]).unwrap();
            assert_eq!(decoded_algo, algo);
            assert_eq!(decoded_tag, tag);
        }
    }

    // ========================================================================
    // CHANGE ANALYSIS TESTS
    // ========================================================================

    #[test]
    fn test_analyze_continuous_add_at_start() {
        let old = b"world";
        let new = b"hello world";

        match analyze_change(old, new) {
            ChangeType::ContinuousAdd { position, data } => {
                assert_eq!(position, 0);
                assert_eq!(&data[..], &b"hello "[..]);
            }
            _ => panic!("Expected ContinuousAdd"),
        }
    }

    #[test]
    fn test_analyze_continuous_add_at_middle() {
        let old = b"helloworld";
        let new = b"hello world";

        match analyze_change(old, new) {
            ChangeType::ContinuousAdd { position, data } => {
                assert_eq!(position, 5);
                assert_eq!(&data[..], &b" "[..]);
            }
            _ => panic!("Expected ContinuousAdd"),
        }
    }

    #[test]
    fn test_analyze_continuous_add_at_end() {
        let old = b"hello";
        let new = b"hello world";

        match analyze_change(old, new) {
            ChangeType::ContinuousAdd { position, data } => {
                assert_eq!(position, 5);
                assert_eq!(&data[..], &b" world"[..]);
            }
            _ => panic!("Expected ContinuousAdd"),
        }
    }

    #[test]
    fn test_analyze_continuous_remove_at_start() {
        let old = b"hello world";
        let new = b"world";

        match analyze_change(old, new) {
            ChangeType::ContinuousRemove { start, end } => {
                assert_eq!(start, 0);
                assert_eq!(end, 6);
            }
            _ => panic!("Expected ContinuousRemove"),
        }
    }

    #[test]
    fn test_analyze_continuous_remove_at_middle() {
        let old = b"hello world";
        let new = b"helloworld";

        match analyze_change(old, new) {
            ChangeType::ContinuousRemove { start, end } => {
                assert_eq!(start, 5);
                assert_eq!(end, 6);
            }
            _ => panic!("Expected ContinuousRemove"),
        }
    }

    #[test]
    fn test_analyze_continuous_remove_at_end() {
        let old = b"hello world";
        let new = b"hello";

        match analyze_change(old, new) {
            ChangeType::ContinuousRemove { start, end } => {
                assert_eq!(start, 5);
                assert_eq!(end, 11);
            }
            _ => panic!("Expected ContinuousRemove"),
        }
    }

    #[test]
    fn test_analyze_complex_change() {
        let old = b"hello world";
        let new = b"hi there universe";

        match analyze_change(old, new) {
            ChangeType::Complex => {}
            _ => panic!("Expected Complex"),
        }
    }

    #[test]
    fn test_analyze_no_change() {
        let old = b"hello world";
        let new = b"hello world";

        match analyze_change(old, new) {
            ChangeType::ContinuousAdd { position, data } => {
                assert_eq!(position, 0);
                assert_eq!(data, vec![]);
            }
            _ => panic!("Expected Complex for identical data"),
        }
    }

    // ========================================================================
    // PATTERN DETECTION TESTS
    // ========================================================================

    #[test]
    fn test_detect_single_char_repeat() {
        let data = b"aaaaaaaaaa"; // 10 'a's
        let result = detect_repeating_pattern(data);
        assert!(result.is_some());
        let (pattern, repeat_count) = result.unwrap();
        assert_eq!(&pattern[..], &b"a"[..]);
        assert_eq!(repeat_count, 10);
    }

    #[test]
    fn test_detect_two_char_repeat() {
        let data = b"ababababab"; // 5 times "ab"
        let result = detect_repeating_pattern(data);
        assert!(result.is_some());
        let (pattern, repeat_count) = result.unwrap();
        assert_eq!(&pattern[..], &b"ab"[..]);
        assert_eq!(repeat_count, 5);
    }

    #[test]
    fn test_detect_four_char_repeat() {
        let data = b"testtest"; // 2 times "test"
        let result = detect_repeating_pattern(data);
        assert!(result.is_some());
        let (pattern, repeat_count) = result.unwrap();
        assert_eq!(&pattern[..], &b"test"[..]);
        assert_eq!(repeat_count, 2);
    }

    #[test]
    fn test_detect_no_repeat() {
        let data = b"abcdefgh";
        let result = detect_repeating_pattern(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_too_short() {
        let data = b"abc";
        let result = detect_repeating_pattern(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_partial_repeat() {
        let data = b"ababac"; // Not a complete repetition
        let result = detect_repeating_pattern(data);
        assert!(result.is_none());
    }

    // ========================================================================
    // CHARS ALGORITHM TESTS
    // ========================================================================

    #[test]
    fn test_chars_roundtrip_simple() {
        let base = b"hello world";
        let new = b"hello beautiful world";
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_chars_insert_at_start() {
        let base = b"world";
        let new = b"hello world";
        let tag = 1;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_chars_insert_at_end() {
        let base = b"hello";
        let new = b"hello world";
        let tag = 2;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_chars_empty_base() {
        let base = b"";
        let new = b"hello";
        let tag = 3;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    // ========================================================================
    // REMOVE ALGORITHM TESTS
    // ========================================================================

    #[test]
    fn test_remove_roundtrip_simple() {
        let base = b"hello beautiful world";
        let new = b"hello world";
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_remove_from_start() {
        let base = b"hello world";
        let new = b"world";
        let tag = 1;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_remove_from_end() {
        let base = b"hello world";
        let new = b"hello";
        let tag = 2;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_remove_single_char() {
        let base = b"hello world";
        let new = b"helloworld";
        let tag = 3;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    // ========================================================================
    // REPEAT CHARS ALGORITHM TESTS
    // ========================================================================

    #[test]
    fn test_repeat_chars_simple() {
        let base = b"start";
        let new = b"startaaaaaaaaaa"; // Added 10 'a's
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_repeat_chars_multi_byte_pattern() {
        let base = b"prefix";
        let new = b"prefixABABABABAB"; // Added 5x "AB"
        let tag = 1;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_repeat_chars_in_middle() {
        let base = b"startsuffix";
        let new = b"start----------suffix"; // Added 10 dashes
        let tag = 2;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    // ========================================================================
    // COMPLEX/GDELTA TESTS
    // ========================================================================

    #[test]
    fn test_complex_change() {
        let base = b"The quick brown fox jumps over the lazy dog";
        let new = b"A fast red wolf leaps across the sleepy cat";
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_multiple_scattered_changes() {
        let base = b"abcdefghijklmnop";
        let new = b"aXcdefYhijklZnop";
        let tag = 1;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    // ========================================================================
    // METADATA TESTS
    // ========================================================================

    #[test]
    fn test_get_tag_small_tag() {
        let base = b"hello";
        let new = b"hello world";
        let tag = 7;

        let delta = encode(tag, base, new, false);
        let extracted_tag = get_tag(&delta[..]).unwrap();

        assert_eq!(extracted_tag, tag);
    }

    #[test]
    fn test_get_tag_large_tag() {
        let base = b"hello";
        let new = b"hello world";
        let tag = 99999;

        let delta = encode(tag, base, new, false);
        let extracted_tag = get_tag(&delta[..]).unwrap();

        assert_eq!(extracted_tag, tag);
    }

    #[test]
    fn test_get_tag_empty_delta() {
        let delta = b"";
        let result = get_tag(&delta[..]);

        assert!(result.is_err());
    }

    // ========================================================================
    // EDGE CASE TESTS
    // ========================================================================

    #[test]
    fn test_empty_to_empty() {
        let base = b"";
        let new = b"";
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_single_byte_change() {
        let base = b"a";
        let new = b"b";
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_large_insertion() {
        let base = b"start";
        let new_content = b"X".repeat(10000);
        let mut new = b"start".to_vec();
        new.extend_from_slice(&new_content[..]);
        let tag = 0;

        let delta = encode(tag, base, &new[..], false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_large_removal() {
        let base_content = b"X".repeat(10000);
        let mut base = b"start".to_vec();
        base.extend_from_slice(&base_content[..]);
        base.extend_from_slice(b"end");

        let new = b"startend";
        let tag = 0;

        let delta = encode(tag, &base[..], new, false);
        let decoded = decode(&base[..], &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_identical_data() {
        let base = b"hello world";
        let new = b"hello world";
        let tag = 0;

        let delta = encode(tag, base, new, false);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    // ========================================================================
    // ERROR HANDLING TESTS
    // ========================================================================

    #[test]
    fn test_decode_empty_delta() {
        let base = b"hello";
        let delta = b"";

        let result = decode(base, &delta[..]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_corrupted_header() {
        let base = b"hello";
        let delta = b"\xFF\xFF\xFF"; // Invalid header

        let result = decode(base, &delta[..]);
        assert!(result.is_err());
    }

    // ========================================================================
    // COMPRESSION EFFECTIVENESS TESTS
    // ========================================================================

    #[test]
    fn test_chars_is_smaller_than_complex() {
        let base = b"The quick brown fox";
        let new = b"The quick brown fox jumps";
        let tag = 0;

        let delta = encode(tag, base, new, false);

        // Chars should produce a very small delta
        assert!(delta.len() < 20);
    }

    #[test]
    fn test_remove_is_smaller_than_complex() {
        let base = b"The quick brown fox jumps";
        let new = b"The quick brown fox";
        let tag = 0;

        let delta = encode(tag, base, new, false);

        // Remove should produce a very small delta
        assert!(delta.len() < 10);
    }

    #[test]
    fn test_repeat_chars_is_smaller_than_chars() {
        let base = b"start";
        let new_content = b"A".repeat(1000);
        let mut new = b"start".to_vec();
        new.extend_from_slice(&new_content[..]);
        let tag = 0;

        let delta = encode(tag, base, &new[..], false);

        // RepeatChars should produce a much smaller delta than raw chars
        assert!(delta.len() < 50);
    }

    // ========================================================================
    // ZSTD COMPRESSION TESTS
    // ========================================================================

    #[test]
    fn test_gdelta_with_zstd() {
        let base = b"The quick brown fox jumps over the lazy dog. ";
        let base_repeated = base.repeat(100);
        let new_repeated = b"A fast red wolf leaps across the sleepy cat. ".repeat(100);
        let tag = 0;

        // Test with zstd enabled
        let delta_with_zstd = encode(tag, &base_repeated[..], &new_repeated[..], true);
        let decoded = decode(&base_repeated[..], &delta_with_zstd[..]).unwrap();

        assert_eq!(&decoded[..], &new_repeated[..]);
    }

    #[test]
    fn test_gdelta_without_zstd() {
        let base = b"The quick brown fox jumps over the lazy dog. ";
        let base_repeated = base.repeat(100);
        let new_repeated = b"A fast red wolf leaps across the sleepy cat. ".repeat(100);
        let tag = 0;

        // Test without zstd
        let delta_without_zstd = encode(tag, &base_repeated[..], &new_repeated[..], false);
        let decoded = decode(&base_repeated[..], &delta_without_zstd[..]).unwrap();

        assert_eq!(&decoded[..], &new_repeated[..]);
    }

    // ========================================================================
    // ROUND-TRIP PROPERTY TESTS
    // ========================================================================

    #[test]
    fn test_roundtrip_property_various_sizes() {
        // Test case 0: empty to hello
        let base0 = b"";
        let new0 = b"hello";
        let delta0 = encode(0, base0, new0, false);
        let decoded0 = decode(base0, &delta0[..]).unwrap();
        assert_eq!(&decoded0[..], &new0[..], "Failed for test case 0");

        // Test case 1: single char
        let base1 = b"a";
        let new1 = b"ab";
        let delta1 = encode(1, base1, new1, false);
        let decoded1 = decode(base1, &delta1[..]).unwrap();
        assert_eq!(&decoded1[..], &new1[..], "Failed for test case 1");

        // Test case 2: hello to empty
        let base2 = b"hello";
        let new2 = b"";
        let delta2 = encode(2, base2, new2, false);
        let decoded2 = decode(base2, &delta2[..]).unwrap();
        assert_eq!(&decoded2[..], &new2[..], "Failed for test case 2");

        // Test case 3: test to testing
        let base3 = b"test";
        let new3 = b"testing";
        let delta3 = encode(3, base3, new3, false);
        let decoded3 = decode(base3, &delta3[..]).unwrap();
        assert_eq!(&decoded3[..], &new3[..], "Failed for test case 3");

        // Test case 4: insertion in middle
        let base4 = b"abcdefghij";
        let new4 = b"abcXYZdefghij";
        let delta4 = encode(4, base4, new4, false);
        let decoded4 = decode(base4, &delta4[..]).unwrap();
        assert_eq!(&decoded4[..], &new4[..], "Failed for test case 4");

        // Test case 5: repeated data
        let base5 = b"x".repeat(100);
        let new5 = b"x".repeat(200);
        let delta5 = encode(5, &base5[..], &new5[..], false);
        let decoded5 = decode(&base5[..], &delta5[..]).unwrap();
        assert_eq!(&decoded5[..], &new5[..], "Failed for test case 5");
    }

    #[test]
    fn test_roundtrip_with_different_tags() {
        let base = b"hello";
        let new = b"hello world";

        for tag in [0, 1, 5, 15, 16, 100, 1000, 65535, 1_000_000] {
            let delta = encode(tag, base, new, false);
            let decoded = decode(base, &delta[..]).unwrap();
            let extracted_tag = get_tag(&delta[..]).unwrap();

            assert_eq!(&decoded[..], &new[..]);
            assert_eq!(extracted_tag, tag);
        }
    }

    // ========================================================================
    // CHARSZSTD ALGORITHM TESTS
    // ========================================================================

    #[test]
    fn test_chars_zstd_large_addition() {
        // Test CharsZstd with a large text that should compress well
        let base = b"";
        let large_text = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100);
        let tag = 0;

        // Encode with zstd enabled
        let delta = encode(tag, base, &large_text[..], true);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &large_text[..]);
    }

    #[test]
    fn test_chars_zstd_middle_insertion() {
        // Test CharsZstd with insertion in the middle
        let base = b"start end";
        let large_text = b"The quick brown fox jumps over the lazy dog. ".repeat(50);
        let mut new = b"start ".to_vec();
        new.extend_from_slice(&large_text[..]);
        new.extend_from_slice(b"end");
        let tag = 0;

        // Encode with zstd enabled
        let delta = encode(tag, base, &new[..], true);
        let decoded = decode(base, &delta[..]).unwrap();

        assert_eq!(&decoded[..], &new[..]);
    }

    #[test]
    fn test_chars_zstd_disabled() {
        // Test that CharsZstd is not used when zstd is disabled
        let base = b"";
        let large_text = b"Lorem ipsum dolor sit amet. ".repeat(100);
        let tag = 0;

        // Encode with zstd disabled
        let delta = encode(tag, base, &large_text[..], false);
        let (algo, _, _) = decode_header(&delta[..]).unwrap();

        // Should not use CharsZstd when disabled
        assert_ne!(algo, Algorithm::CharsZstd);
    }
}
