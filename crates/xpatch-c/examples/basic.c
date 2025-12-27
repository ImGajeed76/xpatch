/*
 * xpatch C bindings example
 *
 * This example demonstrates the basic usage of xpatch from C code:
 * - Encoding a delta between two byte arrays
 * - Decoding the delta to reconstruct the new data
 * - Extracting metadata tags from deltas
 */

#include <stdio.h>
#include <string.h>
#include "../xpatch.h"

int main() {
    // Example data
    const char* base = "Hello, World!";
    const char* new_text = "Hello, Rust!";

    printf("xpatch C bindings example\n");
    printf("==========================\n\n");

    // Show version
    const char* version = (const char*)xpatch_version();
    printf("Using xpatch version: %s\n\n", version);

    // Encode delta
    printf("Original: %s\n", base);
    printf("New:      %s\n\n", new_text);

    struct xpatch_XPatchBuffer delta = xpatch_encode(
        42,  // tag value
        (const uint8_t*)base,
        strlen(base),
        (const uint8_t*)new_text,
        strlen(new_text),
        true  // enable zstd compression
    );

    if (delta.data == NULL) {
        fprintf(stderr, "Failed to encode delta\n");
        return 1;
    }

    printf("Delta size: %zu bytes\n", delta.len);
    printf("Compression: %zu -> %zu bytes (%.1f%% saved)\n\n",
           strlen(new_text), delta.len,
           100.0 * (1.0 - (double)delta.len / strlen(new_text)));

    // Extract tag
    uintptr_t tag;
    int8_t* tag_error = xpatch_get_tag(delta.data, delta.len, &tag);

    if (tag_error != NULL) {
        fprintf(stderr, "Failed to get tag: %s\n", (char*)tag_error);
        xpatch_free_error(tag_error);
        xpatch_free_buffer(delta);
        return 1;
    }

    printf("Extracted tag: %zu\n\n", tag);

    // Decode delta
    struct xpatch_XPatchResult result = xpatch_decode(
        (const uint8_t*)base,
        strlen(base),
        delta.data,
        delta.len
    );

    if (result.error_message != NULL) {
        fprintf(stderr, "Failed to decode: %s\n", (char*)result.error_message);
        xpatch_free_error(result.error_message);
        xpatch_free_buffer(delta);
        return 1;
    }

    // Verify the result
    if (result.buffer.len != strlen(new_text)) {
        fprintf(stderr, "Decoded length mismatch: expected %zu, got %zu\n",
                strlen(new_text), result.buffer.len);
        xpatch_free_buffer(result.buffer);
        xpatch_free_buffer(delta);
        return 1;
    }

    if (memcmp(result.buffer.data, new_text, result.buffer.len) != 0) {
        fprintf(stderr, "Decoded data mismatch\n");
        xpatch_free_buffer(result.buffer);
        xpatch_free_buffer(delta);
        return 1;
    }

    // Print decoded result
    printf("Decoded: ");
    fwrite(result.buffer.data, 1, result.buffer.len, stdout);
    printf("\n\n");

    printf("✓ Success! Encoding and decoding work correctly.\n");

    // Clean up
    xpatch_free_buffer(result.buffer);
    xpatch_free_buffer(delta);

    return 0;
}
