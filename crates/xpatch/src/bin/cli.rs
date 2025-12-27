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

//! # xpatch CLI
//!
//! High-performance delta compression tool with intelligent compression.
//!
//! ## Usage
//!
//! Create a delta patch:
//! ```bash
//! xpatch encode base.bin new.bin -o patch.xdelta
//! ```
//!
//! Apply a delta patch:
//! ```bash
//! xpatch decode base.bin patch.xdelta -o new.bin
//! ```
//!
//! Show delta information:
//! ```bash
//! xpatch info patch.xdelta
//! ```

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;
use sysinfo::System;

// ============================================================================
// CLI Structure
// ============================================================================

/// High-performance delta compression tool
#[derive(Parser)]
#[command(name = "xpatch")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a delta patch from base to new file
    Encode {
        /// Base file (original version)
        base: PathBuf,

        /// New file (target version)
        new: PathBuf,

        /// Output delta file
        #[arg(short, long)]
        output: PathBuf,

        /// User-defined metadata tag (e.g., version number, build ID)
        #[arg(short, long, default_value = "0")]
        tag: usize,

        /// Enable zstd compression for complex changes
        #[arg(short, long)]
        zstd: bool,

        /// Verify delta after creation by decoding and comparing
        #[arg(short, long)]
        verify: bool,

        /// Skip memory warning prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Overwrite output file if it exists
        #[arg(short, long)]
        force: bool,

        /// Suppress output except errors
        #[arg(short, long)]
        quiet: bool,
    },
    /// Apply a delta patch to reconstruct the new file
    Decode {
        /// Base file (original version)
        base: PathBuf,

        /// Delta patch file
        delta: PathBuf,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,

        /// Skip memory warning prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Overwrite output file if it exists
        #[arg(short, long)]
        force: bool,

        /// Suppress output except errors
        #[arg(short, long)]
        quiet: bool,
    },
    /// Show information about a delta file
    Info {
        /// Delta patch file
        delta: PathBuf,
    },
}

// ============================================================================
// Exit Codes
// ============================================================================

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_ENCODE_DECODE_FAILED: i32 = 2;
const EXIT_OUT_OF_MEMORY: i32 = 4;
const EXIT_USER_CANCELLED: i32 = 5;

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Encode {
            base,
            new,
            output,
            tag,
            zstd,
            verify,
            yes,
            force,
            quiet,
        } => handle_encode(&base, &new, &output, tag, zstd, verify, yes, force, quiet),
        Commands::Decode {
            base,
            delta,
            output,
            yes,
            force,
            quiet,
        } => handle_decode(&base, &delta, &output, yes, force, quiet),
        Commands::Info { delta } => handle_info(&delta),
    };

    match result {
        Ok(()) => process::exit(EXIT_SUCCESS),
        Err(e) => {
            eprintln!("{} {}", "Error:".bright_red().bold(), e);

            // Determine exit code based on error message
            let exit_code = if e.to_string().contains("out of memory")
                || e.to_string().contains("Out of memory")
                || e.to_string().contains("Insufficient memory")
            {
                EXIT_OUT_OF_MEMORY
            } else if e.to_string().contains("cancelled") || e.to_string().contains("Cancelled") {
                EXIT_USER_CANCELLED
            } else if e.to_string().contains("encode") || e.to_string().contains("decode") {
                EXIT_ENCODE_DECODE_FAILED
            } else {
                EXIT_ERROR
            };

            process::exit(exit_code);
        }
    }
}

// ============================================================================
// Command Handlers
// ============================================================================

/// Handle the encode subcommand
#[allow(clippy::too_many_arguments)]
fn handle_encode(
    base_path: &Path,
    new_path: &Path,
    output_path: &Path,
    tag: usize,
    zstd: bool,
    verify: bool,
    yes: bool,
    force: bool,
    quiet: bool,
) -> Result<()> {
    // Validate input files
    if !base_path.exists() {
        bail!("File not found: {}", base_path.display());
    }
    if !new_path.exists() {
        bail!("File not found: {}", new_path.display());
    }

    // Check if output exists
    if output_path.exists() && !force {
        bail!(
            "Output file already exists: {}\n   Use --force to overwrite",
            output_path.display()
        );
    }

    // Get file sizes
    let base_size = fs::metadata(base_path)
        .context("Failed to read base file metadata")?
        .len();
    let new_size = fs::metadata(new_path)
        .context("Failed to read new file metadata")?
        .len();

    if !quiet {
        println!(
            "{} Base: {}, New: {}",
            "File sizes:".bright_cyan(),
            format_bytes(base_size),
            format_bytes(new_size)
        );
    }

    // Memory check
    let required = estimate_encode_memory(base_size, new_size);
    check_memory(required, yes, quiet)?;

    // Read files
    if !quiet {
        let total_steps = if verify { 4 } else { 3 };
        println!(
            "{} Reading files...",
            format!("Step 1/{}:", total_steps).bright_cyan()
        );
    }

    let base_data = fs::read(base_path)
        .with_context(|| format!("Failed to read base file: {}", base_path.display()))?;
    let new_data = fs::read(new_path)
        .with_context(|| format!("Failed to read new file: {}", new_path.display()))?;

    // Encode
    if !quiet {
        let total_steps = if verify { 4 } else { 3 };
        println!(
            "{} Encoding delta...",
            format!("Step 2/{}:", total_steps).bright_cyan()
        );
    }

    let start = Instant::now();
    let delta = xpatch::delta::encode(tag, &base_data, &new_data, zstd);
    let encode_time = start.elapsed();

    // Write output
    if !quiet {
        let total_steps = if verify { 4 } else { 3 };
        println!(
            "{} Writing output...",
            format!("Step 3/{}:", total_steps).bright_cyan()
        );
    }

    fs::write(output_path, &delta)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Verify if requested
    let verify_result = if verify {
        if !quiet {
            println!("{} Verifying delta...", "Step 4/4:".bright_cyan());
        }

        let verify_start = Instant::now();

        // Decode and compare
        let reconstructed = xpatch::delta::decode(&base_data, &delta)
            .map_err(|e| anyhow::anyhow!("Verification decode failed: {}", e))?;

        let verify_time = verify_start.elapsed();

        // Compare
        if reconstructed != new_data {
            bail!(
                "Verification failed: reconstructed output does not match original new file\n   \
                 Expected {} bytes, got {} bytes",
                new_data.len(),
                reconstructed.len()
            );
        }

        Some(verify_time)
    } else {
        None
    };

    // Success message
    if !quiet {
        println!();
        println!(
            "{} Created {} ({}, {:.1}% of new file)",
            "Success:".bright_green().bold(),
            output_path.display(),
            format_bytes(delta.len() as u64),
            (delta.len() as f64 / new_size as f64) * 100.0
        );
        print!("   Encoding took {}", format_duration(encode_time));
        if let Some(verify_time) = verify_result {
            print!(", verification took {}", format_duration(verify_time));
        }
        println!();
    }

    Ok(())
}

/// Handle the decode subcommand
fn handle_decode(
    base_path: &Path,
    delta_path: &Path,
    output_path: &Path,
    yes: bool,
    force: bool,
    quiet: bool,
) -> Result<()> {
    // Validate input files
    if !base_path.exists() {
        bail!("File not found: {}", base_path.display());
    }
    if !delta_path.exists() {
        bail!("File not found: {}", delta_path.display());
    }

    // Check if output exists
    if output_path.exists() && !force {
        bail!(
            "Output file already exists: {}\n   Use --force to overwrite",
            output_path.display()
        );
    }

    // Get file sizes
    let base_size = fs::metadata(base_path)
        .context("Failed to read base file metadata")?
        .len();
    let delta_size = fs::metadata(delta_path)
        .context("Failed to read delta file metadata")?
        .len();

    if !quiet {
        println!(
            "{} Base: {}, Delta: {}",
            "File sizes:".bright_cyan(),
            format_bytes(base_size),
            format_bytes(delta_size)
        );
    }

    // Memory check (estimate output size as ~base_size)
    let required = estimate_decode_memory(base_size, delta_size);
    check_memory(required, yes, quiet)?;

    // Read files
    if !quiet {
        println!("{} Reading files...", "Step 1/3:".bright_cyan());
    }

    let base_data = fs::read(base_path)
        .with_context(|| format!("Failed to read base file: {}", base_path.display()))?;
    let delta_data = fs::read(delta_path)
        .with_context(|| format!("Failed to read delta file: {}", delta_path.display()))?;

    // Decode
    if !quiet {
        println!("{} Decoding delta...", "Step 2/3:".bright_cyan());
    }

    let start = Instant::now();
    let output_data = xpatch::delta::decode(&base_data, &delta_data)
        .map_err(|e| anyhow::anyhow!("Decode failed: {}", e))?;
    let decode_time = start.elapsed();

    // Write output
    if !quiet {
        println!("{} Writing output...", "Step 3/3:".bright_cyan());
    }

    fs::write(output_path, &output_data)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Success message
    if !quiet {
        println!();
        println!(
            "{} Created {} ({})",
            "Success:".bright_green().bold(),
            output_path.display(),
            format_bytes(output_data.len() as u64)
        );
        println!("   Decoding took {}", format_duration(decode_time));
    }

    Ok(())
}

/// Handle the info subcommand
fn handle_info(delta_path: &Path) -> Result<()> {
    // Validate input file
    if !delta_path.exists() {
        bail!("File not found: {}", delta_path.display());
    }

    // Read delta file
    let delta_data = fs::read(delta_path)
        .with_context(|| format!("Failed to read delta file: {}", delta_path.display()))?;

    // Get tag
    let tag = xpatch::delta::get_tag(&delta_data)
        .map_err(|e| anyhow::anyhow!("Failed to read delta tag: {}", e))?;

    println!("Tag: {}", tag);
    println!("Size: {} bytes", delta_data.len());

    // Try to decode header for additional info
    match xpatch::delta::decode_header(&delta_data) {
        Ok((algo, _, header_bytes)) => {
            println!("Algorithm: {:?}", algo);
            println!("Header size: {} bytes", header_bytes);
        }
        Err(_) => {
            // Don't fail if header can't be decoded
        }
    }

    Ok(())
}

// ============================================================================
// Memory Management
// ============================================================================

/// Estimate memory required for encoding
fn estimate_encode_memory(base_size: u64, new_size: u64) -> u64 {
    // base + new + delta (worst case = new) + 20% overhead
    base_size + new_size + new_size + (base_size / 5)
}

/// Estimate memory required for decoding
fn estimate_decode_memory(base_size: u64, delta_size: u64) -> u64 {
    // base + delta + output (estimate as base) + 20% overhead
    base_size + delta_size + base_size + (base_size / 5)
}

/// Check if sufficient memory is available
fn check_memory(required: u64, skip_prompt: bool, quiet: bool) -> Result<()> {
    let mut sys = System::new_all();
    sys.refresh_memory();

    let available = sys.available_memory();
    let total = sys.total_memory();

    // Check if totally insufficient (even if all apps closed)
    if required > total {
        bail!(
            "Insufficient memory\n   Required: ~{}\n   Total RAM: {}\n\n   \
             These files cannot be processed on this system.",
            format_bytes(required),
            format_bytes(total)
        );
    }

    // Calculate usage percentage
    let usage_pct = (required as f64 / available as f64) * 100.0;

    // Show status if not quiet
    if !quiet && usage_pct < 80.0 {
        println!(
            "{} ~{} required, {} available {}",
            "Memory:".bright_cyan(),
            format_bytes(required),
            format_bytes(available),
            "✓".bright_green()
        );
    }

    // Warn if high memory usage
    if usage_pct >= 80.0 {
        eprintln!();
        eprintln!(
            "{} This operation requires ~{}",
            "Memory warning:".bright_yellow().bold(),
            format_bytes(required)
        );
        eprintln!(
            "   Available: {} free ({} total)",
            format_bytes(available),
            format_bytes(total)
        );
        eprintln!();

        if usage_pct >= 100.0 {
            eprintln!(
                "   Loading these files will use {:.0}% of available memory.",
                usage_pct
            );
            eprintln!(
                "   {}",
                "Your system may freeze or crash.".bright_red().bold()
            );
        } else {
            eprintln!(
                "   Loading these files will use {:.0}% of available memory.",
                usage_pct
            );
            eprintln!("   System may slow down temporarily.");
        }
        eprintln!();

        if skip_prompt {
            eprintln!("   {} Continuing anyway (--yes flag)", "⚠".bright_yellow());
            eprintln!();
        } else {
            eprint!("   Continue? [y/N]: ");
            io::stderr().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                bail!("Cancelled by user");
            }
            eprintln!();
        }
    }

    Ok(())
}

// ============================================================================
// Utilities
// ============================================================================

/// Format bytes in human-readable form
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration in human-readable form
fn format_duration(duration: std::time::Duration) -> String {
    let nanos = duration.as_nanos();

    if nanos < 1_000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.1}μs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.3}s", duration.as_secs_f64())
    }
}
