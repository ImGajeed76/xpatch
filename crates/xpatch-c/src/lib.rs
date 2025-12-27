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

use std::panic;
use std::ptr;
use std::slice;

/// A buffer returned from xpatch functions.
/// The caller is responsible for freeing this buffer using xpatch_free_buffer.
#[repr(C)]
pub struct XPatchBuffer {
    /// Pointer to the data
    pub data: *mut u8,
    /// Length of the data in bytes
    pub len: usize,
}

/// Result type for operations that can fail.
/// If error_message is not NULL, the operation failed and the message describes the error.
/// The caller is responsible for freeing the error message using xpatch_free_error.
#[repr(C)]
pub struct XPatchResult {
    /// The result buffer (valid only if error_message is NULL)
    pub buffer: XPatchBuffer,
    /// Error message (NULL on success, non-NULL on error)
    pub error_message: *mut i8,
}

/// Encode a delta patch between base_data and new_data.
///
/// # Parameters
/// - `tag`: Metadata tag to embed in the delta (0-15 with no overhead)
/// - `base_data`: Pointer to the original data
/// - `base_len`: Length of the original data in bytes
/// - `new_data`: Pointer to the new data
/// - `new_len`: Length of the new data in bytes
/// - `enable_zstd`: Whether to enable zstd compression (true recommended)
///
/// # Returns
/// An XPatchBuffer containing the encoded delta. The caller must free this buffer
/// using xpatch_free_buffer when done.
///
/// # Safety
/// - `base_data` must point to valid memory of at least `base_len` bytes
/// - `new_data` must point to valid memory of at least `new_len` bytes
/// - The returned buffer must be freed with xpatch_free_buffer
///
/// # Example
/// ```c
/// const char* base = "Hello, World!";
/// const char* new = "Hello, Rust!";
/// XPatchBuffer delta = xpatch_encode(0, base, strlen(base), new, strlen(new), true);
/// // Use delta...
/// xpatch_free_buffer(delta);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xpatch_encode(
    tag: usize,
    base_data: *const u8,
    base_len: usize,
    new_data: *const u8,
    new_len: usize,
    enable_zstd: bool,
) -> XPatchBuffer {
    // Input validation
    if base_data.is_null() && base_len > 0 {
        return XPatchBuffer {
            data: ptr::null_mut(),
            len: 0,
        };
    }
    if new_data.is_null() && new_len > 0 {
        return XPatchBuffer {
            data: ptr::null_mut(),
            len: 0,
        };
    }

    let result = panic::catch_unwind(|| {
        // Safety: validated above
        let base = if base_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(base_data, base_len) }
        };
        let new = if new_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(new_data, new_len) }
        };

        let delta = xpatch::encode(tag, base, new, enable_zstd);
        let mut boxed = delta.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len();
        std::mem::forget(boxed); // Prevent deallocation

        XPatchBuffer { data, len }
    });

    match result {
        Ok(buffer) => buffer,
        Err(_) => XPatchBuffer {
            data: ptr::null_mut(),
            len: 0,
        },
    }
}

/// Decode a delta patch to reconstruct new_data from base_data.
///
/// # Parameters
/// - `base_data`: Pointer to the original data
/// - `base_len`: Length of the original data in bytes
/// - `delta`: Pointer to the delta patch
/// - `delta_len`: Length of the delta patch in bytes
///
/// # Returns
/// An XPatchResult. On success, error_message is NULL and buffer contains the reconstructed data.
/// On failure, error_message contains a description of the error.
/// The caller must free the buffer with xpatch_free_buffer and error with xpatch_free_error.
///
/// # Safety
/// - `base_data` must point to valid memory of at least `base_len` bytes
/// - `delta` must point to valid memory of at least `delta_len` bytes
/// - The returned buffer must be freed with xpatch_free_buffer
/// - The returned error message (if not NULL) must be freed with xpatch_free_error
///
/// # Example
/// ```c
/// XPatchResult result = xpatch_decode(base, base_len, delta.data, delta.len);
/// if (result.error_message == NULL) {
///     // Use result.buffer...
///     xpatch_free_buffer(result.buffer);
/// } else {
///     fprintf(stderr, "Error: %s\n", result.error_message);
///     xpatch_free_error(result.error_message);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xpatch_decode(
    base_data: *const u8,
    base_len: usize,
    delta: *const u8,
    delta_len: usize,
) -> XPatchResult {
    // Input validation
    if (base_data.is_null() && base_len > 0) || (delta.is_null() && delta_len > 0) {
        let error_msg = "Invalid null pointer\0";
        let error_ptr = error_msg.as_ptr() as *mut i8;
        return XPatchResult {
            buffer: XPatchBuffer {
                data: ptr::null_mut(),
                len: 0,
            },
            error_message: error_ptr,
        };
    }

    let result = panic::catch_unwind(|| {
        // Safety: validated above
        let base = if base_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(base_data, base_len) }
        };
        let delta_slice = if delta_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(delta, delta_len) }
        };

        match xpatch::decode(base, delta_slice) {
            Ok(decoded) => {
                let mut boxed = decoded.into_boxed_slice();
                let data = boxed.as_mut_ptr();
                let len = boxed.len();
                std::mem::forget(boxed); // Prevent deallocation

                XPatchResult {
                    buffer: XPatchBuffer { data, len },
                    error_message: ptr::null_mut(),
                }
            }
            Err(error) => {
                let error_msg = format!("{}\0", error);
                let error_ptr = error_msg.as_ptr() as *mut i8;
                std::mem::forget(error_msg); // Prevent deallocation

                XPatchResult {
                    buffer: XPatchBuffer {
                        data: ptr::null_mut(),
                        len: 0,
                    },
                    error_message: error_ptr,
                }
            }
        }
    });

    match result {
        Ok(res) => res,
        Err(_) => {
            let panic_msg = "Rust panic occurred\0";
            let error_ptr = panic_msg.as_ptr() as *mut i8;

            XPatchResult {
                buffer: XPatchBuffer {
                    data: ptr::null_mut(),
                    len: 0,
                },
                error_message: error_ptr,
            }
        }
    }
}

/// Extract the metadata tag from a delta patch.
///
/// # Parameters
/// - `delta`: Pointer to the delta patch
/// - `delta_len`: Length of the delta patch in bytes
/// - `tag_out`: Pointer to store the extracted tag value
///
/// # Returns
/// An error message string (NULL on success, non-NULL on error).
/// The caller must free the error message using xpatch_free_error if not NULL.
///
/// # Safety
/// - `delta` must point to valid memory of at least `delta_len` bytes
/// - `tag_out` must point to valid memory for a usize
/// - The returned error message (if not NULL) must be freed with xpatch_free_error
///
/// # Example
/// ```c
/// usize tag;
/// char* error = xpatch_get_tag(delta.data, delta.len, &tag);
/// if (error == NULL) {
///     printf("Tag: %zu\n", tag);
/// } else {
///     fprintf(stderr, "Error: %s\n", error);
///     xpatch_free_error(error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xpatch_get_tag(
    delta: *const u8,
    delta_len: usize,
    tag_out: *mut usize,
) -> *mut i8 {
    // Input validation
    if (delta.is_null() && delta_len > 0) || tag_out.is_null() {
        let error_msg = "Invalid null pointer\0";
        return error_msg.as_ptr() as *mut i8;
    }

    let result = panic::catch_unwind(|| {
        // Safety: validated above
        let delta_slice = if delta_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(delta, delta_len) }
        };

        match xpatch::get_tag(delta_slice) {
            Ok(tag) => {
                unsafe { *tag_out = tag };
                ptr::null_mut()
            }
            Err(error) => {
                let error_msg = format!("{}\0", error);
                let error_ptr = error_msg.as_ptr() as *mut i8;
                std::mem::forget(error_msg); // Prevent deallocation
                error_ptr
            }
        }
    });

    match result {
        Ok(res) => res,
        Err(_) => {
            let panic_msg = "Rust panic occurred\0";
            panic_msg.as_ptr() as *mut i8
        }
    }
}

/// Free a buffer returned by xpatch_encode or xpatch_decode.
///
/// # Parameters
/// - `buffer`: The buffer to free
///
/// # Safety
/// - `buffer` must have been returned by xpatch_encode or from a successful xpatch_decode
/// - `buffer` must not be used after calling this function
/// - This function must be called exactly once per buffer
///
/// # Example
/// ```c
/// XPatchBuffer delta = xpatch_encode(...);
/// // Use delta...
/// xpatch_free_buffer(delta);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xpatch_free_buffer(buffer: XPatchBuffer) {
    if !buffer.data.is_null() && buffer.len > 0 {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(buffer.data, buffer.len));
        }
    }
}

/// Free an error message returned by xpatch functions.
///
/// # Parameters
/// - `error_message`: The error message to free
///
/// # Safety
/// - `error_message` must have been returned by a xpatch function
/// - `error_message` must not be used after calling this function
/// - This function must be called exactly once per error message
///
/// # Example
/// ```c
/// char* error = xpatch_get_tag(...);
/// if (error != NULL) {
///     fprintf(stderr, "Error: %s\n", error);
///     xpatch_free_error(error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xpatch_free_error(error_message: *mut i8) {
    if !error_message.is_null() {
        unsafe {
            // We need to reconstruct the original String to free it properly
            // The error messages are created with format!("{}\0", ...) so we need to find the length
            let mut len = 0;
            while *error_message.offset(len) != 0 {
                len += 1;
            }
            let _ =
                String::from_raw_parts(error_message as *mut u8, len as usize, len as usize + 1);
        }
    }
}

/// Get the version string of the xpatch library.
///
/// # Returns
/// A null-terminated string containing the version. This string is statically allocated
/// and must NOT be freed.
///
/// # Example
/// ```c
/// const char* version = xpatch_version();
/// printf("xpatch version: %s\n", version);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn xpatch_version() -> *const i8 {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const i8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_encode_decode_roundtrip() {
        let base = b"Hello, World!";
        let new = b"Hello, Rust!";

        unsafe {
            // Encode
            let delta = xpatch_encode(0, base.as_ptr(), base.len(), new.as_ptr(), new.len(), true);
            assert!(!delta.data.is_null());
            assert!(delta.len > 0);

            // Decode
            let result = xpatch_decode(base.as_ptr(), base.len(), delta.data, delta.len);
            assert!(result.error_message.is_null());
            assert!(!result.buffer.data.is_null());
            assert_eq!(result.buffer.len, new.len());

            // Verify
            let decoded = slice::from_raw_parts(result.buffer.data, result.buffer.len);
            assert_eq!(decoded, new);

            // Cleanup
            xpatch_free_buffer(delta);
            xpatch_free_buffer(result.buffer);
        }
    }

    #[test]
    fn test_get_tag() {
        let base = b"Hello, World!";
        let new = b"Hello, Rust!";
        let tag_value = 42;

        unsafe {
            // Encode with tag
            let delta = xpatch_encode(
                tag_value,
                base.as_ptr(),
                base.len(),
                new.as_ptr(),
                new.len(),
                true,
            );

            // Get tag
            let mut tag: usize = 0;
            let error = xpatch_get_tag(delta.data, delta.len, &mut tag);
            assert!(error.is_null());
            assert_eq!(tag, tag_value);

            // Cleanup
            xpatch_free_buffer(delta);
        }
    }

    #[test]
    fn test_version() {
        let version = xpatch_version();
        assert!(!version.is_null());

        unsafe {
            // Verify it's a valid null-terminated string
            let mut len = 0;
            while *version.offset(len) != 0 {
                len += 1;
                assert!(len < 100, "Version string too long");
            }
            assert!(len > 0, "Version string is empty");
        }
    }

    #[test]
    fn test_empty_data() {
        let base = b"";
        let new = b"";

        unsafe {
            let delta = xpatch_encode(0, base.as_ptr(), 0, new.as_ptr(), 0, false);
            assert!(!delta.data.is_null());
            assert!(delta.len > 0);

            let result = xpatch_decode(base.as_ptr(), 0, delta.data, delta.len);
            assert!(result.error_message.is_null());
            assert_eq!(result.buffer.len, 0);

            xpatch_free_buffer(delta);
            xpatch_free_buffer(result.buffer);
        }
    }

    #[test]
    fn test_identical_data() {
        let data = b"Hello, World!";

        unsafe {
            let delta = xpatch_encode(
                0,
                data.as_ptr(),
                data.len(),
                data.as_ptr(),
                data.len(),
                false,
            );
            assert!(!delta.data.is_null());
            // Delta should be very small for identical data
            assert!(delta.len < 10);

            let result = xpatch_decode(data.as_ptr(), data.len(), delta.data, delta.len);
            assert!(result.error_message.is_null());
            assert_eq!(result.buffer.len, data.len());

            let decoded = slice::from_raw_parts(result.buffer.data, result.buffer.len);
            assert_eq!(decoded, data);

            xpatch_free_buffer(delta);
            xpatch_free_buffer(result.buffer);
        }
    }

    #[test]
    fn test_large_data() {
        let base = vec![b'A'; 1024 * 1024]; // 1MB
        let mut new = base.clone();
        new[512 * 1024] = b'B'; // Change one byte in the middle

        unsafe {
            let delta = xpatch_encode(0, base.as_ptr(), base.len(), new.as_ptr(), new.len(), true);
            assert!(!delta.data.is_null());
            // Delta should be much smaller than 1MB
            assert!(delta.len < 1024);

            let result = xpatch_decode(base.as_ptr(), base.len(), delta.data, delta.len);
            assert!(result.error_message.is_null());
            assert_eq!(result.buffer.len, new.len());

            let decoded = slice::from_raw_parts(result.buffer.data, result.buffer.len);
            assert_eq!(decoded, &new[..]);

            xpatch_free_buffer(delta);
            xpatch_free_buffer(result.buffer);
        }
    }

    #[test]
    fn test_invalid_delta() {
        let base = b"Hello, World!";
        let invalid_delta = b"this is not a valid delta";

        unsafe {
            let result = xpatch_decode(
                base.as_ptr(),
                base.len(),
                invalid_delta.as_ptr(),
                invalid_delta.len(),
            );

            // Should return an error
            assert!(!result.error_message.is_null());
            assert!(result.buffer.data.is_null());
            assert_eq!(result.buffer.len, 0);

            // Error message should be valid
            let error_str = std::ffi::CStr::from_ptr(result.error_message);
            let error_msg = error_str.to_str().unwrap();
            assert!(!error_msg.is_empty());

            xpatch_free_error(result.error_message);
        }
    }

    #[test]
    fn test_truncated_delta() {
        let base = b"Hello, World!";
        let new = b"Hello, Rust!";

        unsafe {
            // Create a valid delta
            let delta = xpatch_encode(0, base.as_ptr(), base.len(), new.as_ptr(), new.len(), false);

            // Try to decode with truncated delta
            if delta.len > 2 {
                let result = xpatch_decode(
                    base.as_ptr(),
                    base.len(),
                    delta.data,
                    delta.len / 2, // Use only half the delta
                );

                // Should return an error
                assert!(!result.error_message.is_null());
                xpatch_free_error(result.error_message);
            }

            xpatch_free_buffer(delta);
        }
    }

    #[test]
    fn test_tag_zero_overhead() {
        let base = b"Hello";
        let new = b"World";

        unsafe {
            // Test tags 0-15 (should have no overhead)
            for tag in 0..=15 {
                let delta = xpatch_encode(
                    tag,
                    base.as_ptr(),
                    base.len(),
                    new.as_ptr(),
                    new.len(),
                    false,
                );

                let mut extracted_tag: usize = 999;
                let error = xpatch_get_tag(delta.data, delta.len, &mut extracted_tag);
                assert!(error.is_null());
                assert_eq!(extracted_tag, tag);

                xpatch_free_buffer(delta);
            }
        }
    }

    #[test]
    fn test_large_tag() {
        let base = b"Hello";
        let new = b"World";
        let large_tag = 1000;

        unsafe {
            let delta = xpatch_encode(
                large_tag,
                base.as_ptr(),
                base.len(),
                new.as_ptr(),
                new.len(),
                false,
            );

            let mut extracted_tag: usize = 0;
            let error = xpatch_get_tag(delta.data, delta.len, &mut extracted_tag);
            assert!(error.is_null());
            assert_eq!(extracted_tag, large_tag);

            xpatch_free_buffer(delta);
        }
    }

    #[test]
    fn test_with_without_zstd() {
        let base = vec![b'X'; 10000];
        let new = vec![b'Y'; 10000];

        unsafe {
            // Without zstd
            let delta_no_zstd =
                xpatch_encode(0, base.as_ptr(), base.len(), new.as_ptr(), new.len(), false);

            // With zstd
            let delta_with_zstd =
                xpatch_encode(0, base.as_ptr(), base.len(), new.as_ptr(), new.len(), true);

            // Both should work
            let result1 = xpatch_decode(
                base.as_ptr(),
                base.len(),
                delta_no_zstd.data,
                delta_no_zstd.len,
            );
            let result2 = xpatch_decode(
                base.as_ptr(),
                base.len(),
                delta_with_zstd.data,
                delta_with_zstd.len,
            );

            assert!(result1.error_message.is_null());
            assert!(result2.error_message.is_null());

            xpatch_free_buffer(delta_no_zstd);
            xpatch_free_buffer(delta_with_zstd);
            xpatch_free_buffer(result1.buffer);
            xpatch_free_buffer(result2.buffer);
        }
    }

    #[test]
    fn test_multiple_encode_decode() {
        // Test that we can encode/decode multiple times without issues
        let base = b"Version 1";
        let v2 = b"Version 2";
        let v3 = b"Version 3";

        unsafe {
            let delta1 = xpatch_encode(0, base.as_ptr(), base.len(), v2.as_ptr(), v2.len(), false);
            let delta2 = xpatch_encode(1, v2.as_ptr(), v2.len(), v3.as_ptr(), v3.len(), false);

            let result1 = xpatch_decode(base.as_ptr(), base.len(), delta1.data, delta1.len);
            assert!(result1.error_message.is_null());

            let decoded_v2 = slice::from_raw_parts(result1.buffer.data, result1.buffer.len);
            assert_eq!(decoded_v2, v2);

            let result2 = xpatch_decode(
                result1.buffer.data,
                result1.buffer.len,
                delta2.data,
                delta2.len,
            );
            assert!(result2.error_message.is_null());

            let decoded_v3 = slice::from_raw_parts(result2.buffer.data, result2.buffer.len);
            assert_eq!(decoded_v3, v3);

            xpatch_free_buffer(delta1);
            xpatch_free_buffer(delta2);
            xpatch_free_buffer(result1.buffer);
            xpatch_free_buffer(result2.buffer);
        }
    }

    #[test]
    fn test_thread_safety() {
        // Test that encoding/decoding can happen concurrently from multiple threads
        let base = b"Hello, World!";
        let new = b"Hello, Rust!";

        let handles: Vec<_> = (0..4)
            .map(|i| {
                let base = base.to_vec();
                let new = new.to_vec();

                thread::spawn(move || unsafe {
                    for _ in 0..100 {
                        let delta = xpatch_encode(
                            i,
                            base.as_ptr(),
                            base.len(),
                            new.as_ptr(),
                            new.len(),
                            true,
                        );

                        let result =
                            xpatch_decode(base.as_ptr(), base.len(), delta.data, delta.len);
                        assert!(result.error_message.is_null());

                        let mut tag: usize = 0;
                        let error = xpatch_get_tag(delta.data, delta.len, &mut tag);
                        assert!(error.is_null());
                        assert_eq!(tag, i);

                        xpatch_free_buffer(delta);
                        xpatch_free_buffer(result.buffer);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_binary_data() {
        // Test with binary data (not just text)
        let base: Vec<u8> = (0..=255).cycle().take(1000).collect();
        let mut new = base.clone();
        new[500] = 0xFF;

        unsafe {
            let delta = xpatch_encode(0, base.as_ptr(), base.len(), new.as_ptr(), new.len(), false);

            let result = xpatch_decode(base.as_ptr(), base.len(), delta.data, delta.len);
            assert!(result.error_message.is_null());

            let decoded = slice::from_raw_parts(result.buffer.data, result.buffer.len);
            assert_eq!(decoded, &new[..]);

            xpatch_free_buffer(delta);
            xpatch_free_buffer(result.buffer);
        }
    }

    #[test]
    fn test_free_null_buffer() {
        // Test that freeing a null/empty buffer doesn't crash
        unsafe {
            let null_buffer = XPatchBuffer {
                data: ptr::null_mut(),
                len: 0,
            };
            xpatch_free_buffer(null_buffer); // Should not crash
        }
    }

    #[test]
    fn test_free_null_error() {
        // Test that freeing a null error doesn't crash
        unsafe {
            xpatch_free_error(ptr::null_mut()); // Should not crash
        }
    }
}
