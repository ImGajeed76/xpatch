# xpatch C/C++ Bindings

C and C++ bindings for the xpatch delta compression library.

## Features

- Simple C-compatible API with only 3 main functions
- Automatic header generation via cbindgen
- Proper memory management with explicit free functions
- Comprehensive error handling
- Works with both C and C++ code
- Cross-platform support (Linux, macOS, Windows)

## Installation

### Building the Library

```bash
cargo build --release
```

This will create:
- **Linux**: `target/release/libxpatch_c.so`
- **macOS**: `target/release/libxpatch_c.dylib`
- **Windows**: `target/release/xpatch_c.dll`

The C header file is automatically generated at `xpatch.h`.

## API Reference

### Data Types

```c
// Buffer type for encoded/decoded data
typedef struct {
    uint8_t *data;
    uintptr_t len;
} xpatch_XPatchBuffer;

// Result type for operations that can fail
typedef struct {
    xpatch_XPatchBuffer buffer;
    int8_t *error_message;  // NULL on success
} xpatch_XPatchResult;
```

### Functions

#### xpatch_encode

Encode a delta patch between base and new data.

```c
struct xpatch_XPatchBuffer xpatch_encode(
    uintptr_t tag,
    const uint8_t *base_data,
    uintptr_t base_len,
    const uint8_t *new_data,
    uintptr_t new_len,
    bool enable_zstd
);
```

**Parameters:**
- `tag`: Metadata tag (0-15 use zero overhead)
- `base_data`: Pointer to original data
- `base_len`: Length of original data
- `new_data`: Pointer to new data
- `new_len`: Length of new data
- `enable_zstd`: Enable zstd compression

**Returns:** Buffer containing the delta (must be freed with `xpatch_free_buffer`)

#### xpatch_decode

Decode a delta to reconstruct new data from base.

```c
struct xpatch_XPatchResult xpatch_decode(
    const uint8_t *base_data,
    uintptr_t base_len,
    const uint8_t *delta,
    uintptr_t delta_len
);
```

**Parameters:**
- `base_data`: Pointer to original data
- `base_len`: Length of original data
- `delta`: Pointer to delta patch
- `delta_len`: Length of delta patch

**Returns:** Result struct. Check `error_message` for NULL to verify success.

#### xpatch_get_tag

Extract metadata tag from a delta.

```c
int8_t *xpatch_get_tag(
    const uint8_t *delta,
    uintptr_t delta_len,
    uintptr_t *tag_out
);
```

**Parameters:**
- `delta`: Pointer to delta patch
- `delta_len`: Length of delta patch
- `tag_out`: Pointer to store extracted tag

**Returns:** NULL on success, error message on failure

#### Memory Management

```c
void xpatch_free_buffer(struct xpatch_XPatchBuffer buffer);
void xpatch_free_error(int8_t *error_message);
```

**Important:** You must free all buffers and error messages returned by xpatch functions.

#### Version

```c
const int8_t *xpatch_version(void);
```

Returns the library version string (statically allocated, do not free).

## Usage Example

### C

```c
#include <stdio.h>
#include <string.h>
#include "xpatch.h"

int main() {
    const char* base = "Hello, World!";
    const char* new = "Hello, Rust!";

    // Encode
    struct xpatch_XPatchBuffer delta = xpatch_encode(
        0,
        (const uint8_t*)base, strlen(base),
        (const uint8_t*)new, strlen(new),
        true
    );

    printf("Delta size: %zu bytes\n", delta.len);

    // Decode
    struct xpatch_XPatchResult result = xpatch_decode(
        (const uint8_t*)base, strlen(base),
        delta.data, delta.len
    );

    if (result.error_message == NULL) {
        printf("Success! Decoded %zu bytes\n", result.buffer.len);
        xpatch_free_buffer(result.buffer);
    } else {
        fprintf(stderr, "Error: %s\n", (char*)result.error_message);
        xpatch_free_error(result.error_message);
    }

    xpatch_free_buffer(delta);
    return 0;
}
```

### C++

```cpp
#include <iostream>
#include <string>
#include <vector>
#include "xpatch.h"

int main() {
    std::string base = "Hello, World!";
    std::string new_text = "Hello, C++!";

    // Encode
    auto delta = xpatch_encode(
        0,
        reinterpret_cast<const uint8_t*>(base.data()), base.size(),
        reinterpret_cast<const uint8_t*>(new_text.data()), new_text.size(),
        true
    );

    std::cout << "Delta size: " << delta.len << " bytes\n";

    // Decode
    auto result = xpatch_decode(
        reinterpret_cast<const uint8_t*>(base.data()), base.size(),
        delta.data, delta.len
    );

    if (result.error_message == nullptr) {
        std::cout << "Success! Decoded " << result.buffer.len << " bytes\n";
        xpatch_free_buffer(result.buffer);
    } else {
        std::cerr << "Error: " << reinterpret_cast<char*>(result.error_message) << "\n";
        xpatch_free_error(result.error_message);
    }

    xpatch_free_buffer(delta);
    return 0;
}
```

## Building Examples

```bash
cd examples
make
make run
```

## Linking

### GCC/Clang

```bash
gcc -o myapp myapp.c -I/path/to/xpatch-c -L/path/to/target/release -lxpatch_c
```

### CMake

```cmake
add_executable(myapp myapp.c)
target_include_directories(myapp PRIVATE /path/to/xpatch-c)
target_link_libraries(myapp PRIVATE /path/to/target/release/libxpatch_c.so)
```

## Thread Safety

The xpatch library is thread-safe for concurrent encoding and decoding operations. However, individual buffers and results should not be shared between threads without proper synchronization.

## Performance

C bindings add minimal overhead (~few nanoseconds per call). Since xpatch operations typically take 10-200 microseconds, the FFI overhead is negligible (<0.1%).

## Best Practices

### Memory Management

**Always free buffers returned by xpatch functions:**

```c
// ✓ CORRECT
struct xpatch_XPatchBuffer delta = xpatch_encode(/* ... */);
struct xpatch_XPatchResult result = xpatch_decode(/* ... */);

if (result.error_message != NULL) {
    xpatch_free_error(result.error_message);
    xpatch_free_buffer(delta);
    return -1;
}

xpatch_free_buffer(result.buffer);
xpatch_free_buffer(delta);
```

**Never free buffers twice or use after freeing:**

```c
// ✗ WRONG - double free
xpatch_free_buffer(buf);
xpatch_free_buffer(buf);  // CRASH!

// ✗ WRONG - use after free
xpatch_free_buffer(buf);
printf("%zu\n", buf.len);  // UNDEFINED BEHAVIOR!
```

### Error Handling

**Always check error_message before using result data:**

```c
// ✓ CORRECT
struct xpatch_XPatchResult result = xpatch_decode(/* ... */);
if (result.error_message != NULL) {
    fprintf(stderr, "Error: %s\n", (char*)result.error_message);
    xpatch_free_error(result.error_message);
    return -1;
}
// Now safe to use result.buffer
xpatch_free_buffer(result.buffer);
```

**For xpatch_get_tag, NULL means success:**

```c
uintptr_t tag;
int8_t *error = xpatch_get_tag(delta.data, delta.len, &tag);
if (error != NULL) {
    xpatch_free_error(error);
    return -1;
}
```

### Input Validation

Passing NULL pointers with non-zero lengths will cause the function to return an empty/error result:

```c
// Returns empty buffer
struct xpatch_XPatchBuffer delta = xpatch_encode(0, NULL, 10, /* ... */);
if (delta.data == NULL) {
    fprintf(stderr, "Invalid input\n");
}
```

## Troubleshooting

### Linker Errors

**"undefined reference to xpatch_encode"**

Add `-L` for library path and `-l` for linking:

```bash
gcc -o myapp myapp.c -I/path/to/xpatch-c -L/path/to/target/release -lxpatch_c
```

**"cannot open shared object file: libxpatch_c.so"**

Set library path or install system-wide:

```bash
# Temporary
export LD_LIBRARY_PATH=/path/to/target/release:$LD_LIBRARY_PATH

# System-wide (Linux)
sudo cp target/release/libxpatch_c.so /usr/local/lib/
sudo ldconfig
```

### Runtime Crashes

Common causes of segmentation faults:

1. **Null pointer with non-zero length:**
   ```c
   xpatch_encode(0, NULL, 10, /* ... */);  // Returns empty buffer, check result
   ```

2. **Double free:**
   ```c
   xpatch_free_buffer(buf);
   xpatch_free_buffer(buf);  // CRASH
   ```

3. **Using buffer after free:**
   ```c
   xpatch_free_buffer(buf);
   printf("%zu\n", buf.len);  // CRASH
   ```

### Decode Failures

**"Invalid delta format" or "Checksum mismatch"**

- Ensure you're using the **exact same base data** for encoding and decoding
- Check that delta wasn't corrupted during transmission/storage
- Verify entire delta was read (not truncated)

### Platform-Specific

- **Linux**: Use `libxpatch_c.so`
- **macOS**: Use `libxpatch_c.dylib`, set `DYLD_LIBRARY_PATH` if needed
- **Windows**: Use `xpatch_c.dll` (not `libxpatch_c.dll`)

## License

Dual licensed under AGPL-3.0-or-later for open source use, with commercial licensing available.

See the main [LICENSE files](../../LICENSE-AGPL.txt) for details.
