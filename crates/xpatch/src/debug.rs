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

#[cfg(feature = "debug_delta_encode")]
macro_rules! debug_delta_encode {
    ($($arg:tt)*) => (println!("[DELTA][ENCODE] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_delta_encode"))]
macro_rules! debug_delta_encode {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug_delta_token")]
macro_rules! debug_delta_token {
    ($($arg:tt)*) => (println!("[DELTA][TOKEN] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_delta_token"))]
macro_rules! debug_delta_token {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug_delta_analyze")]
macro_rules! debug_delta_analyze {
    ($($arg:tt)*) => (println!("[DELTA][ANALYZE] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_delta_analyze"))]
macro_rules! debug_delta_analyze {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug_delta_compress")]
macro_rules! debug_delta_compress {
    ($($arg:tt)*) => (println!("[DELTA][COMPRESS] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_delta_compress"))]
macro_rules! debug_delta_compress {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug_delta_pattern")]
macro_rules! debug_delta_pattern {
    ($($arg:tt)*) => (println!("[DELTA][PATTERN] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_delta_pattern"))]
macro_rules! debug_delta_pattern {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug_delta_header")]
macro_rules! debug_delta_header {
    ($($arg:tt)*) => (println!("[DELTA][HEADER] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_delta_header"))]
macro_rules! debug_delta_header {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug_tokenizer")]
macro_rules! debug_tokenizer {
    ($($arg:tt)*) => (println!("[TOKENIZER] {}", format_args!($($arg)*)));
}

#[cfg(not(feature = "debug_tokenizer"))]
macro_rules! debug_tokenizer {
    ($($arg:tt)*) => {};
}

pub(crate) use debug_delta_analyze;
pub(crate) use debug_delta_compress;
pub(crate) use debug_delta_encode;
pub(crate) use debug_delta_header;
pub(crate) use debug_delta_pattern;
pub(crate) use debug_delta_token;
pub(crate) use debug_tokenizer;
