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
// Commercial License Option:
// For commercial use in proprietary software, a commercial license is
// available. Contact xpatch-commercial@alias.oseifert.ch for details.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "xpatch")]
#[command(about = "High-performance delta compression tool", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a delta between two files
    Encode {
        /// Base file path (use '-' for stdin)
        base: String,
        /// New file path (use '-' for stdin)
        new: String,
        /// Output delta file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// User-defined metadata tag (default: 0)
        #[arg(short, long, default_value = "0")]
        tag: usize,
        /// Enable zstd compression for complex changes
        #[arg(short, long)]
        zstd: bool,
    },
    /// Apply a delta to reconstruct a file
    Decode {
        /// Base file path (use '-' for stdin)
        base: String,
        /// Delta file path (use '-' for stdin)
        delta: String,
        /// Output reconstructed file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show information about a delta file
    Info {
        /// Delta file path (use '-' for stdin)
        delta: String,
    },
}

/// Read data from file or stdin
fn read_input(path: &str) -> Result<Vec<u8>> {
    if path == "-" {
        let mut buffer = Vec::new();
        io::stdin()
            .read_to_end(&mut buffer)
            .context("Failed to read from stdin")?;
        Ok(buffer)
    } else {
        fs::read(path).with_context(|| format!("Failed to read file: {}", path))
    }
}

/// Write data to file or stdout
fn write_output(path: Option<PathBuf>, data: &[u8]) -> Result<()> {
    if let Some(path) = path {
        fs::write(&path, data)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
    } else {
        io::stdout()
            .write_all(data)
            .context("Failed to write to stdout")?;
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            base,
            new,
            output,
            tag,
            zstd,
        } => {
            let start = Instant::now();
            let base_data = read_input(&base)?;
            let new_data = read_input(&new)?;
            let read_time = start.elapsed();

            let start = Instant::now();
            let delta = xpatch::delta::encode(tag, &base_data, &new_data, zstd);
            let encode_time = start.elapsed();

            write_output(output, &delta)?;

            eprintln!(
                "Encoded {} → {} bytes in {:?} (read: {:?})",
                base_data.len(),
                delta.len(),
                encode_time,
                read_time
            );
        }
        Commands::Decode {
            base,
            delta,
            output,
        } => {
            let start = Instant::now();
            let base_data = read_input(&base)?;
            let delta_data = read_input(&delta)?;
            let read_time = start.elapsed();

            let start = Instant::now();
            let decoded = xpatch::delta::decode(&base_data, &delta_data)
                .map_err(|e| anyhow::anyhow!("Decode failed: {}", e))?;
            let decode_time = start.elapsed();

            write_output(output, &decoded)?;

            eprintln!(
                "Decoded {} → {} bytes in {:?} (read: {:?})",
                delta_data.len(),
                decoded.len(),
                decode_time,
                read_time
            );
        }
        Commands::Info { delta } => {
            let delta_data = read_input(&delta)?;

            let tag = xpatch::delta::get_tag(&delta_data)
                .map_err(|e| anyhow::anyhow!("Failed to read delta info: {}", e))?;

            println!("Tag: {}", tag);
            println!("Size: {} bytes", delta_data.len());

            // Try to decode header to show algorithm (optional enhancement)
            match xpatch::delta::decode_header(&delta_data) {
                Ok((algo, _, header_bytes)) => {
                    println!("Algorithm: {:?}", algo);
                    println!("Header size: {} bytes", header_bytes);
                }
                Err(_) => {
                    // Don't fail info command if header can't be decoded
                }
            }
        }
    }

    Ok(())
}
