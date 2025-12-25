#![deny(clippy::all)]

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

use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Encode a delta patch between base_data and new_data.
///
/// @param tag - Metadata tag to embed in the delta (0-15 with no overhead)
/// @param baseData - The original data as a Buffer
/// @param newData - The new data as a Buffer
/// @param enableZstd - Whether to enable zstd compression (default: true)
/// @returns The encoded delta patch as a Buffer
///
/// @example
/// ```javascript
/// const xpatch = require('xpatch-rs');
/// const base = Buffer.from('Hello, World!');
/// const newData = Buffer.from('Hello, Node!');
/// const delta = xpatch.encode(0, base, newData);
/// console.log(`Delta size: ${delta.length} bytes`);
/// ```
#[napi]
pub fn encode(
    tag: u32,
    base_data: Buffer,
    new_data: Buffer,
    enable_zstd: Option<bool>,
) -> Result<Buffer> {
    let enable_zstd = enable_zstd.unwrap_or(true);
    let result = xpatch::encode(tag as usize, &base_data, &new_data, enable_zstd);
    Ok(Buffer::from(result))
}

/// Decode a delta patch to reconstruct new_data from base_data.
///
/// @param baseData - The original data as a Buffer
/// @param delta - The delta patch as a Buffer
/// @returns The reconstructed new data as a Buffer
/// @throws {Error} If the delta is invalid or corrupted
///
/// @example
/// ```javascript
/// const xpatch = require('xpatch-rs');
/// const base = Buffer.from('Hello, World!');
/// const newData = Buffer.from('Hello, Node!');
/// const delta = xpatch.encode(0, base, newData);
/// const decoded = xpatch.decode(base, delta);
/// console.log(decoded.equals(newData)); // true
/// ```
#[napi]
pub fn decode(base_data: Buffer, delta: Buffer) -> Result<Buffer> {
    match xpatch::decode(&base_data, &delta) {
        Ok(result) => Ok(Buffer::from(result)),
        Err(error) => Err(Error::from_reason(error)),
    }
}

/// Extract the metadata tag from a delta patch.
///
/// @param delta - The delta patch as a Buffer
/// @returns The embedded metadata tag as a number
/// @throws {Error} If the delta is invalid or corrupted
///
/// @example
/// ```javascript
/// const xpatch = require('xpatch-rs');
/// const base = Buffer.from('Hello, World!');
/// const newData = Buffer.from('Hello, Node!');
/// const delta = xpatch.encode(42, base, newData);
/// const tag = xpatch.getTag(delta);
/// console.log(`Tag: ${tag}`); // Tag: 42
/// ```
#[napi]
pub fn get_tag(delta: Buffer) -> Result<u32> {
    match xpatch::get_tag(&delta) {
        Ok(tag) => Ok(tag as u32),
        Err(error) => Err(Error::from_reason(error)),
    }
}
