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

use ::xpatch::delta;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Encode a delta patch between base_data and new_data.
///
/// Args:
///     tag: Metadata tag to embed in the delta (0-15 with no overhead)
///     base_data: The original data as bytes
///     new_data: The new data as bytes
///     enable_zstd: Whether to enable zstd compression (default: True)
///
/// Returns:
///     bytes: The encoded delta patch
///
/// Example:
///     >>> import xpatch
///     >>> base = b"Hello, World!"
///     >>> new = b"Hello, Rust!"
///     >>> delta = xpatch.encode(0, base, new)
///     >>> len(delta)
///     8
#[pyfunction]
#[pyo3(signature = (tag, base_data, new_data, enable_zstd=true))]
fn encode<'py>(
    py: Python<'py>,
    tag: usize,
    base_data: &[u8],
    new_data: &[u8],
    enable_zstd: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let result = delta::encode(tag, base_data, new_data, enable_zstd);
    Ok(PyBytes::new(py, &result[..]))
}

/// Decode a delta patch to reconstruct new_data from base_data.
///
/// Args:
///     base_data: The original data as bytes
///     delta: The delta patch as bytes
///
/// Returns:
///     bytes: The reconstructed new data
///
/// Raises:
///     ValueError: If the delta is invalid or corrupted
///
/// Example:
///     >>> import xpatch
///     >>> base = b"Hello, World!"
///     >>> new = b"Hello, Rust!"
///     >>> delta = xpatch.encode(0, base, new)
///     >>> decoded = xpatch.decode(base, delta)
///     >>> decoded == new
///     True
#[pyfunction]
fn decode<'py>(py: Python<'py>, base_data: &[u8], delta: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    match delta::decode(base_data, delta) {
        Ok(result) => Ok(PyBytes::new(py, &result[..])),
        Err(error) => Err(PyValueError::new_err(error)),
    }
}

/// Extract the metadata tag from a delta patch.
///
/// Args:
///     delta: The delta patch as bytes
///
/// Returns:
///     int: The embedded metadata tag
///
/// Raises:
///     ValueError: If the delta is invalid or corrupted
///
/// Example:
///     >>> import xpatch
///     >>> base = b"Hello, World!"
///     >>> new = b"Hello, Rust!"
///     >>> delta = xpatch.encode(42, base, new)
///     >>> xpatch.get_tag(delta)
///     42
#[pyfunction]
fn get_tag(delta: &[u8]) -> PyResult<usize> {
    match delta::get_tag(delta) {
        Ok(tag) => Ok(tag),
        Err(error) => Err(PyValueError::new_err(error)),
    }
}

/// xpatch - High-performance delta compression library
///
/// This library provides extremely efficient delta compression for byte sequences,
/// achieving median delta sizes of just 2 bytes for typical code changes.
///
/// Features:
/// - 99.8% compression ratio on real-world code changes
/// - Automatic algorithm selection based on change patterns
/// - Embedded metadata tags for versioning
/// - Optional zstd compression layer
///
/// Example:
///     >>> import xpatch
///     >>> base = b"The quick brown fox"
///     >>> new = b"The quick brown cat"
///     >>> delta = xpatch.encode(1, base, new)
///     >>> reconstructed = xpatch.decode(base, delta)
///     >>> reconstructed == new
///     True
#[pymodule]
fn xpatch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(get_tag, m)?)?;
    Ok(())
}
