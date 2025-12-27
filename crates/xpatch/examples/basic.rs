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

use xpatch::delta;

fn main() {
    let base_data = b"Hello, world!";
    let new_data = b"Hello, beautiful world!";

    // Encode the difference
    let tag = 0; // User-defined metadata
    let enable_zstd = true;
    let delta = delta::encode(tag, base_data, new_data, enable_zstd);

    println!("Original size: {} bytes", base_data.len());
    println!("Delta size: {} bytes", delta.len());
    println!(
        "Compression ratio: {:.2}%",
        (1.0 - delta.len() as f64 / new_data.len() as f64) * 100.0
    );

    // Decode to reconstruct new_data
    let reconstructed = delta::decode(base_data, &delta[..]).unwrap();
    assert_eq!(reconstructed, new_data);

    // Extract metadata without decoding
    let extracted_tag = delta::get_tag(&delta[..]).unwrap();
    assert_eq!(extracted_tag, tag);
}
