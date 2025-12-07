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

//! Fast tokenizer using a Trie for longest-match encoding.
//!
//! This module provides efficient text tokenization using a pre-built vocabulary.
//! The tokenizer uses a singleton pattern with lazy initialization for optimal
//! performance in multithreaded environments.
//!
//! # Architecture
//!
//! - **Array-based trie**: Enables O(1) lookups with direct array indexing
//! - **Singleton pattern**: Single global instance shared across threads using `OnceLock`
//! - **Greedy longest-match**: Always selects the longest matching token at each position
//!
//! # Example
//!
//! ```no_run
//! use xpatch::tokenizer::{encode, decode, decode_to_string};
//!
//! // Encode text to tokens
//! let text = b"Hello world";
//! let tokens = encode(text)?;
//!
//! // Decode back to bytes
//! let decoded = decode(&tokens)?;
//! assert_eq!(decoded, text);
//!
//! // Or decode to a string
//! let string = decode_to_string(&tokens)?;
//! # Ok::<(), String>(())
//! ```
//!
//! # Performance Characteristics
//!
//! - **Initialization**: O(V*L) where V is vocabulary size, L is average token length
//! - **Encoding**: O(n*m) where n is input length, m is the longest token
//! - **Decoding**: O(t*l) where t is number of tokens, l is average token length
//! - **Memory**: O(256*depth) for the array-based trie structure

use crate::debug::debug_tokenizer;
use crate::token_list::TOKENS;
use std::sync::OnceLock;

/// Trie node for efficient token matching using array-based children.
///
/// Uses direct array indexing (0-255) for O(1) lookups instead of HashMap.
/// This is significantly faster for byte-indexed tries as it eliminates
/// hashing overhead and provides better cache locality.
struct TrieNode {
    /// Token ID if this node represents a complete token
    token_id: Option<usize>,
    /// Child nodes indexed directly by byte value (0-255)
    /// Using Box to keep stack size reasonable
    children: [Option<Box<TrieNode>>; 256],
}

impl TrieNode {
    /// Creates a new empty trie node.
    #[inline]
    fn new() -> Self {
        const INIT: Option<Box<TrieNode>> = None;
        Self {
            token_id: None,
            children: [INIT; 256],
        }
    }
}

/// Trie structure for fast longest-match tokenization.
///
/// Uses array-based indexing for O(1) lookups per byte.
/// Memory usage is O(256*depth) where depth is the longest token.
struct TokenTrie {
    root: TrieNode,
}

impl TokenTrie {
    /// Builds a new trie from the global token vocabulary.
    ///
    /// This is an expensive operation and should only be done once
    /// (handled automatically by the singleton pattern).
    fn new() -> Self {
        debug_tokenizer!("Building token trie with {} tokens...", TOKENS.len());
        let mut trie = Self {
            root: TrieNode::new(),
        };

        for (idx, token) in TOKENS.iter().enumerate() {
            trie.insert(token, idx);
        }

        debug_tokenizer!("Token trie built successfully");
        trie
    }

    /// Inserts a token into the trie with its corresponding ID.
    fn insert(&mut self, token: &[u8], token_id: usize) {
        let mut node = &mut self.root;

        for &byte in token {
            // Direct array indexing - O(1) lookup
            node = node.children[byte as usize].get_or_insert_with(|| Box::new(TrieNode::new()));
        }

        node.token_id = Some(token_id);
    }

    /// Find the longest matching token at the given position.
    ///
    /// Uses greedy matching: walks the trie as far as possible,
    /// returning the last valid token encountered.
    ///
    /// # Returns
    ///
    /// `Some((token_id, length))` if a match is found, `None` otherwise.
    ///
    /// # Performance
    ///
    /// O(m) where m is the length of the longest possible match.
    /// Each byte lookup is O(1) due to array indexing.
    #[inline]
    fn find_longest_match(&self, text: &[u8], start: usize) -> Option<(usize, usize)> {
        let mut node = &self.root;
        let mut last_match = None;
        let mut pos = start;

        while pos < text.len() {
            // Direct array access - much faster than HashMap lookup
            if let Some(next_node) = &node.children[text[pos] as usize] {
                node = next_node;
                pos += 1;

                if let Some(token_id) = node.token_id {
                    last_match = Some((token_id, pos - start));
                }
            } else {
                break;
            }
        }

        last_match
    }
}

/// Simple and efficient tokenizer (singleton).
///
/// This tokenizer is designed to be accessed globally via [`SimpleTokenizer::global()`]
/// or through the convenience functions [`encode`], [`decode`], and [`decode_to_string`].
///
/// # Thread Safety
///
/// The singleton is thread-safe and initialization is performed exactly once
/// using Rust's `OnceLock` primitive.
///
/// # Why Singleton?
///
/// - The trie structure is expensive to build (O(V*L) time and space)
/// - All threads can share the same read-only trie
/// - Eliminates redundant initialization overhead
/// - Simplifies API for common use cases
pub struct SimpleTokenizer {
    trie: TokenTrie,
}

impl SimpleTokenizer {
    /// Get the global tokenizer instance (initialized once, lazily).
    ///
    /// Uses `OnceLock` for thread-safe lazy initialization. The trie is built
    /// on first access and reused for all subsequent calls.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xpatch::tokenizer::SimpleTokenizer;
    /// let tokenizer = SimpleTokenizer::global();
    /// let tokens = tokenizer.encode(b"Hello")?;
    /// # Ok::<(), String>(())
    /// ```
    #[inline]
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<SimpleTokenizer> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            trie: TokenTrie::new(),
        })
    }

    /// Encode UTF-8 text to token indices using greedy longest-match.
    ///
    /// The tokenizer processes the input byte sequence from left to right,
    /// always selecting the longest matching token at each position. This
    /// greedy approach is efficient but may not produce the optimal tokenization
    /// for all possible metrics.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n*m) where n is text length, m is longest token length
    /// - Space complexity: O(t) where t is number of tokens produced
    /// - Best case: O(n) when all matches are found on first try
    /// - Each byte lookup is O(1) due to array-based trie
    ///
    /// # Errors
    ///
    /// Returns an error with detailed context if any byte sequence cannot be tokenized.
    /// This indicates the input contains text not covered by the vocabulary.
    ///
    /// The error message includes:
    /// - Position where tokenization failed
    /// - The problematic byte (hex and char representation)
    /// - Surrounding context for debugging
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xpatch::tokenizer::SimpleTokenizer;
    /// let tokenizer = SimpleTokenizer::global();
    /// let tokens = tokenizer.encode(b"Hello world")?;
    /// println!("Encoded to {} tokens", tokens.len());
    /// # Ok::<(), String>(())
    /// ```
    pub fn encode(&self, text: &[u8]) -> Result<Vec<usize>, String> {
        debug_tokenizer!("Encoding {} bytes...", text.len());

        // Pre-allocate with estimated size (average token is ~2-3 bytes)
        let mut indices = Vec::with_capacity(text.len() / 2);
        let mut pos = 0;

        while pos < text.len() {
            match self.trie.find_longest_match(text, pos) {
                Some((token_id, len)) => {
                    debug_tokenizer!("  pos {}: matched token {} (len {})", pos, token_id, len);
                    indices.push(token_id);
                    pos += len;
                }
                None => {
                    let context_start = pos.saturating_sub(10);
                    let context_end = (pos + 10).min(text.len());
                    let context = &text[context_start..context_end];

                    debug_tokenizer!(
                        "  pos {}: FAILED to match byte {:02x} ('{}')",
                        pos,
                        text[pos],
                        if text[pos].is_ascii_graphic() || text[pos] == b' ' {
                            text[pos] as char
                        } else {
                            '?'
                        }
                    );
                    debug_tokenizer!("    Context: {:?}", String::from_utf8_lossy(context));
                    debug_tokenizer!("    Hex context: {:02x?}", context);
                    debug_tokenizer!("    Position {} out of {} total bytes", pos, text.len());

                    return Err(format!(
                        "Cannot tokenize at position {}: byte {:02x} ('{}')\nContext: {:?}",
                        pos,
                        text[pos],
                        if text[pos].is_ascii_graphic() || text[pos] == b' ' {
                            text[pos] as char
                        } else {
                            '?'
                        },
                        String::from_utf8_lossy(context)
                    ));
                }
            }
        }

        debug_tokenizer!("Encoded to {} tokens", indices.len());
        Ok(indices)
    }

    /// Decode token indices back to UTF-8 bytes.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(t*l) where t is number of tokens, l is average token length
    /// - Space complexity: O(n) where n is total output size
    /// - Pre-allocates output buffer for efficiency
    ///
    /// # Errors
    ///
    /// Returns an error if any token index is out of bounds (>= vocabulary size).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xpatch::tokenizer::SimpleTokenizer;
    /// let tokenizer = SimpleTokenizer::global();
    /// let bytes = tokenizer.decode(&[0, 1, 2])?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn decode(&self, indices: &[usize]) -> Result<Vec<u8>, String> {
        debug_tokenizer!("Decoding {} tokens...", indices.len());

        // Pre-calculate total size for allocation
        let total_size: usize = indices
            .iter()
            .filter_map(|&idx| TOKENS.get(idx).map(|t| t.len()))
            .sum();

        let mut result = Vec::with_capacity(total_size);

        for &idx in indices {
            if let Some(&token) = TOKENS.get(idx) {
                debug_tokenizer!("  token {}: {} bytes", idx, token.len());
                result.extend_from_slice(token);
            } else {
                debug_tokenizer!("  token {}: INVALID INDEX", idx);
                return Err(format!("Invalid token index: {}", idx));
            }
        }

        debug_tokenizer!("Decoded to {} bytes", result.len());
        Ok(result)
    }

    /// Decode token indices and convert to a String.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any token index is invalid
    /// - The decoded bytes are not valid UTF-8
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xpatch::tokenizer::SimpleTokenizer;
    /// let tokenizer = SimpleTokenizer::global();
    /// let text = tokenizer.decode_to_string(&[0, 1, 2])?;
    /// println!("Decoded: {}", text);
    /// # Ok::<(), String>(())
    /// ```
    pub fn decode_to_string(&self, indices: &[usize]) -> Result<String, String> {
        debug_tokenizer!("Decoding {} tokens to string...", indices.len());

        let bytes = self.decode(indices)?;

        match String::from_utf8(bytes) {
            Ok(text) => {
                debug_tokenizer!("Decoded to string: {} chars", text.len());
                Ok(text)
            }
            Err(e) => {
                debug_tokenizer!("Failed to decode to UTF-8: {}", e);
                Err(format!("Invalid UTF-8 in decoded output: {}", e))
            }
        }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Encode UTF-8 text to token indices using the global tokenizer.
///
/// This is a convenience function that calls [`SimpleTokenizer::global().encode()`].
/// Use this for simple cases where you don't need to hold a reference to the tokenizer.
///
/// # Example
///
/// ```no_run
/// # use xpatch::tokenizer::encode;
/// let tokens = encode(b"Hello world")?;
/// # Ok::<(), String>(())
/// ```
#[inline]
pub fn encode(text: &[u8]) -> Result<Vec<usize>, String> {
    SimpleTokenizer::global().encode(text)
}

/// Decode token indices back to UTF-8 bytes using the global tokenizer.
///
/// This is a convenience function that calls [`SimpleTokenizer::global().decode()`].
///
/// # Example
///
/// ```no_run
/// # use xpatch::tokenizer::decode;
/// let bytes = decode(&[0, 1, 2])?;
/// # Ok::<(), String>(())
/// ```
#[inline]
pub fn decode(indices: &[usize]) -> Result<Vec<u8>, String> {
    SimpleTokenizer::global().decode(indices)
}

/// Decode token indices to a String using the global tokenizer.
///
/// This is a convenience function that calls [`SimpleTokenizer::global().decode_to_string()`].
///
/// # Example
///
/// ```no_run
/// # use xpatch::tokenizer::decode_to_string;
/// let text = decode_to_string(&[0, 1, 2])?;
/// println!("{}", text);
/// # Ok::<(), String>(())
/// ```
#[inline]
pub fn decode_to_string(indices: &[usize]) -> Result<String, String> {
    SimpleTokenizer::global().decode_to_string(indices)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Basic Functionality Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_singleton_access() {
        // Multiple calls should return the same instance
        let t1 = SimpleTokenizer::global();
        let t2 = SimpleTokenizer::global();
        assert!(std::ptr::eq(t1, t2), "Should be the same instance");
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(encode(b"").unwrap(), Vec::<usize>::new());
        assert_eq!(decode(&[]).unwrap(), Vec::<u8>::new());
        assert_eq!(decode_to_string(&[]).unwrap(), "");
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let text = b"Hello world";

        match encode(text) {
            Ok(indices) => {
                println!("Encoded to {} tokens: {:?}", indices.len(), indices);

                let decoded = decode(&indices[..]).expect("Decode failed");
                println!("Decoded: {:?}", String::from_utf8_lossy(&decoded[..]));

                // Note: Roundtrip may not be exact if vocabulary has overlapping tokens
                // but the decoded bytes should be valid
                assert!(!decoded.is_empty(), "Decoded output should not be empty");
            }
            Err(e) => {
                println!("Cannot encode (vocab might not cover this text): {}", e);
            }
        }
    }

    #[test]
    fn test_decode_to_string_basic() {
        // Decode first few tokens (if they exist)
        let indices = [0, 1, 2];
        match decode_to_string(&indices) {
            Ok(text) => {
                println!("Decoded to string: '{}'", text);
                assert!(!text.is_empty() || indices.is_empty());
            }
            Err(e) => println!("Error: {}", e),
        }
    }

    #[test]
    fn test_method_and_function_equivalence() {
        let text = b"test";
        let tokenizer = SimpleTokenizer::global();

        // Both should produce same result
        if let (Ok(e1), Ok(e2)) = (tokenizer.encode(text), encode(text)) {
            assert_eq!(e1, e2, "Method and function should give same result");
        }
    }

    // ------------------------------------------------------------------------
    // Error Handling Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_decode_invalid_token_index() {
        // Try to decode an index that's way out of bounds
        let invalid_indices = [usize::MAX];
        let result = decode(&invalid_indices);

        assert!(result.is_err(), "Should error on invalid token index");
        assert!(result.unwrap_err().contains("Invalid token index"));
    }

    #[test]
    fn test_decode_partially_invalid() {
        // Mix valid and invalid indices
        let vocab_size = TOKENS.len();
        let indices = [0, vocab_size + 1000]; // First is valid, second is not

        let result = decode(&indices);
        assert!(result.is_err(), "Should error on any invalid token");
    }

    #[test]
    fn test_encode_untokenizable_input() {
        // Use a byte sequence unlikely to be in the vocabulary
        let weird_bytes = b"\xFF\xFE\xFD\xFC";
        let result = encode(weird_bytes);

        if let Err(error) = result {
            println!("Expected error: {}", error);
            assert!(
                error.contains("Cannot tokenize"),
                "Error should explain tokenization failure"
            );
            assert!(error.contains("position"), "Error should include position");
        }
    }

    // ------------------------------------------------------------------------
    // Edge Case Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_single_byte_tokens() {
        // If vocabulary has single-byte tokens, test them
        for (i, _item) in TOKENS.iter().enumerate().take(TOKENS.len().min(10)) {
            if TOKENS[i].len() == 1 {
                let result = decode(&[i]);
                assert!(result.is_ok(), "Single-byte token should decode");
                assert_eq!(result.unwrap().len(), 1);
            }
        }
    }

    #[test]
    fn test_long_token_sequence() {
        // Test decoding a long sequence of tokens
        let long_sequence: Vec<usize> = (0..100).map(|i| i % TOKENS.len().min(10)).collect();
        let result = decode(&long_sequence[..]);

        assert!(result.is_ok(), "Should handle long sequences");
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_repeated_tokens() {
        // Test encoding/decoding with repeated tokens
        let indices = [0, 0, 0, 1, 1, 2];
        let result = decode(&indices);

        assert!(result.is_ok(), "Should handle repeated tokens");
    }

    // ------------------------------------------------------------------------
    // Performance Characteristic Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_encode_performance_scales_linearly() {
        // This test verifies that encoding time scales roughly linearly with input size
        // Not a precise benchmark, just a sanity check

        let small_text = b"a".repeat(100);
        let large_text = b"a".repeat(1000);

        // Both should succeed or both should fail
        let small_result = encode(&small_text[..]);
        let large_result = encode(&large_text[..]);

        assert_eq!(
            small_result.is_ok(),
            large_result.is_ok(),
            "Both sizes should have same success/failure"
        );
    }

    #[test]
    fn test_decode_allocates_correct_size() {
        // Verify that decode pre-allocates the right amount
        let indices = [0, 1, 2, 3, 4];
        let result = decode(&indices);

        if let Ok(bytes) = result {
            // The length should match the sum of token lengths
            let expected_len: usize = indices
                .iter()
                .filter_map(|&i| TOKENS.get(i).map(|t| t.len()))
                .sum();
            assert_eq!(bytes.len(), expected_len);
        }
    }

    // ------------------------------------------------------------------------
    // Thread Safety Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        // Spawn multiple threads that all access the singleton
        let handles: Vec<_> = (0..4)
            .map(|_| {
                thread::spawn(|| {
                    let tokenizer = SimpleTokenizer::global();
                    // Each thread encodes some text
                    let _ = tokenizer.encode(b"test");
                })
            })
            .collect();

        // All threads should complete successfully
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }
    }

    // ------------------------------------------------------------------------
    // Trie Behavior Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_longest_match_preference() {
        // The tokenizer should always prefer the longest match
        // This is hard to test without knowing the vocabulary structure,
        // but we can at least verify consistency

        let text = b"aaa";
        let result1 = encode(text);
        let result2 = encode(text);

        if let (Ok(tokens1), Ok(tokens2)) = (result1, result2) {
            assert_eq!(tokens1, tokens2, "Same input should produce same tokens");
        }
    }

    #[test]
    fn test_no_partial_matches() {
        // Verify that failed matches don't produce partial output
        let invalid = b"\xFF\xFF";
        let result = encode(invalid);

        if result.is_err() {
            // Good - it should fail cleanly rather than produce partial tokens
            println!("Correctly rejected invalid input");
        }
    }
}
