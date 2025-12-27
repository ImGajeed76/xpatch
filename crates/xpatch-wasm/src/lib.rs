use wasm_bindgen::prelude::*;

// When the `console_error_panic_hook` feature is enabled, we can call the
// `set_panic_hook` function at least once during initialization, and then
// we will get better error messages if our code ever panics.
//
// For more details see
// https://github.com/rustwasm/console_error_panic_hook#readme
#[cfg(feature = "console_error_panic_hook")]
pub use console_error_panic_hook::set_once as set_panic_hook;

/// Encode a delta patch between base_data and new_data.
///
/// # Parameters
/// - `tag`: Metadata tag to embed in the delta (0-15 with no overhead)
/// - `base_data`: The original data (Uint8Array in JavaScript)
/// - `new_data`: The new data (Uint8Array in JavaScript)
/// - `enable_zstd`: Whether to enable zstd compression
///
/// # Returns
/// A Uint8Array containing the encoded delta.
///
/// # Example (JavaScript)
/// ```js
/// import init, { encode } from './xpatch_wasm.js';
///
/// await init();
///
/// const base = new TextEncoder().encode("Hello, World!");
/// const newData = new TextEncoder().encode("Hello, WASM!");
/// const delta = encode(0, base, newData, true);
/// console.log(`Delta size: ${delta.length} bytes`);
/// ```
#[wasm_bindgen]
pub fn encode(tag: usize, base_data: &[u8], new_data: &[u8], enable_zstd: bool) -> Vec<u8> {
    xpatch::delta::encode(tag, base_data, new_data, enable_zstd)
}

/// Decode a delta patch to reconstruct new_data from base_data.
///
/// # Parameters
/// - `base_data`: The original data (Uint8Array in JavaScript)
/// - `delta`: The delta patch (Uint8Array in JavaScript)
///
/// # Returns
/// A Uint8Array containing the reconstructed data.
///
/// # Throws
/// Throws an error if the delta is invalid or corrupted.
///
/// # Example (JavaScript)
/// ```js
/// import init, { encode, decode } from './xpatch_wasm.js';
///
/// await init();
///
/// const base = new TextEncoder().encode("Hello, World!");
/// const newData = new TextEncoder().encode("Hello, WASM!");
/// const delta = encode(0, base, newData, true);
///
/// // Decode
/// const reconstructed = decode(base, delta);
/// const text = new TextDecoder().decode(reconstructed);
/// console.log(text); // "Hello, WASM!"
/// ```
#[wasm_bindgen]
pub fn decode(base_data: &[u8], delta: &[u8]) -> Result<Vec<u8>, JsValue> {
    xpatch::delta::decode(base_data, delta).map_err(JsValue::from_str)
}

/// Extract the metadata tag from a delta patch.
///
/// # Parameters
/// - `delta`: The delta patch (Uint8Array in JavaScript)
///
/// # Returns
/// The tag value as a number.
///
/// # Throws
/// Throws an error if the delta is invalid.
///
/// # Example (JavaScript)
/// ```js
/// import init, { encode, get_tag } from './xpatch_wasm.js';
///
/// await init();
///
/// const base = new TextEncoder().encode("v1");
/// const newData = new TextEncoder().encode("v2");
/// const delta = encode(42, base, newData, false);
///
/// const tag = get_tag(delta);
/// console.log(`Tag: ${tag}`); // "Tag: 42"
/// ```
#[wasm_bindgen]
pub fn get_tag(delta: &[u8]) -> Result<usize, JsValue> {
    xpatch::delta::get_tag(delta).map_err(JsValue::from_str)
}

/// Get the version string of the xpatch library.
///
/// # Returns
/// A string containing the version number.
///
/// # Example (JavaScript)
/// ```js
/// import init, { version } from './xpatch_wasm.js';
///
/// await init();
///
/// console.log(`xpatch version: ${version()}`);
/// ```
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_encode_decode() {
        let base = b"Hello, World!";
        let new = b"Hello, WASM!";

        let delta = encode(0, base, new, true);
        assert!(!delta.is_empty());

        let reconstructed = decode(base, &delta).unwrap();
        assert_eq!(reconstructed, new);
    }

    #[wasm_bindgen_test]
    fn test_get_tag() {
        let base = b"test";
        let new = b"TEST";

        let delta = encode(42, base, new, false);
        let tag = get_tag(&delta).unwrap();
        assert_eq!(tag, 42);
    }

    #[wasm_bindgen_test]
    fn test_version() {
        let ver = version();
        assert!(!ver.is_empty());
        assert!(ver.contains('.'));
    }

    #[wasm_bindgen_test]
    fn test_decode_invalid_delta() {
        let base = b"test";
        let invalid_delta = vec![0xFF, 0xFF, 0xFF];

        let result = decode(base, &invalid_delta);
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_zero_overhead_tags() {
        let base = b"a";
        let new = b"b";

        for tag in 0..=15 {
            let delta = encode(tag, base, new, false);
            let extracted_tag = get_tag(&delta).unwrap();
            assert_eq!(extracted_tag, tag);
        }
    }

    #[wasm_bindgen_test]
    fn test_empty_data() {
        let base = b"";
        let new = b"";

        let delta = encode(0, base, new, false);
        let reconstructed = decode(base, &delta).unwrap();
        assert_eq!(reconstructed, new);
    }

    #[wasm_bindgen_test]
    fn test_identical_data() {
        let base = b"same";
        let new = b"same";

        let delta = encode(0, base, new, false);
        let reconstructed = decode(base, &delta).unwrap();
        assert_eq!(reconstructed, new);
    }
}
