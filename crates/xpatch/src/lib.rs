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

//! # xpatch
//!
//! A high-performance delta compression library with automatic algorithm selection.
//!
//! ## Quick Start
//! ```
//! use xpatch::delta;
//!
//! let base = b"Hello, world!";
//! let new = b"Hello, beautiful world!";
//! let delta = delta::encode(0, base, new, true);
//! let decoded = delta::decode(base, &delta).unwrap();
//! assert_eq!(decoded, new);
//! ```

pub(crate) mod debug;
pub mod delta;
pub mod token_list;
pub mod tokenizer;
pub mod varint;

// Re-export main public API
pub use delta::{Algorithm, decode, encode, get_tag};
