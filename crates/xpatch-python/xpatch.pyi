"""Type stubs for xpatch"""

from typing import Union

def encode(
    tag: int,
    base_data: bytes,
    new_data: bytes,
    enable_zstd: bool = True
) -> bytes:
    """Encode a delta patch between base_data and new_data.

    Args:
        tag: Metadata tag to embed in the delta (0-15 with no overhead)
        base_data: The original data as bytes
        new_data: The new data as bytes
        enable_zstd: Whether to enable zstd compression (default: True)

    Returns:
        The encoded delta patch as bytes

    Example:
        >>> import xpatch
        >>> base = b"Hello, World!"
        >>> new = b"Hello, Rust!"
        >>> delta = xpatch.encode(0, base, new)
        >>> len(delta)
        8
    """
    ...

def decode(
    base_data: bytes,
    delta: bytes
) -> bytes:
    """Decode a delta patch and reconstruct the new data.

    Args:
        base_data: The original data the delta was created from
        delta: The encoded delta patch

    Returns:
        The reconstructed new data as bytes

    Raises:
        ValueError: If the delta is invalid or corrupted

    Example:
        >>> import xpatch
        >>> base = b"Hello, World!"
        >>> new = b"Hello, Rust!"
        >>> delta = xpatch.encode(0, base, new)
        >>> reconstructed = xpatch.decode(base, delta)
        >>> reconstructed == new
        True
    """
    ...

def get_tag(delta: bytes) -> int:
    """Extract the metadata tag from a delta patch.

    Args:
        delta: The delta patch as bytes

    Returns:
        The embedded metadata tag as an integer

    Raises:
        ValueError: If the delta is invalid or corrupted

    Example:
        >>> import xpatch
        >>> base = b"Hello, World!"
        >>> new = b"Hello, Rust!"
        >>> delta = xpatch.encode(42, base, new)
        >>> xpatch.get_tag(delta)
        42
    """
    ...
