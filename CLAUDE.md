# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A command-line file compression/decompression utility written in Rust using the Zstandard (zstd) compression algorithm. The application is Italian-language focused with all user-facing messages in Italian.

## Build and Development Commands

### Building
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Build output locations:
# - Debug: ./target/debug/file_compressor
# - Release: ./target/release/file_compressor
```

### Running
```bash
# Run directly with cargo
cargo run -- compress <file> --livello 10
cargo run -- decompress <file.zst>

# Run the built binary
./target/debug/file_compressor compress <file>
./target/release/file_compressor decompress <file.zst>
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test <test_name>
```

### Other Commands
```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Lint with clippy
cargo clippy
```

## Architecture

### Single-File Structure
The entire application is contained in [src/main.rs](src/main.rs). This is a simple CLI tool without complex module organization.

### Key Components

1. **CLI Parser** (lines 8-41): Uses `clap` derive macros to define the command-line interface with two subcommands:
   - `compress`: Compresses a file with configurable compression level (1-21, default 3)
   - `decompress`: Decompresses .zst files

2. **Compression Logic** (lines 77-126 in `compress_file`):
   - Uses streaming compression via `zstd::Encoder` to handle large files without loading them entirely into memory
   - 64KB buffer size for both input and output
   - Output file naming: appends `.zst` to the full filename (e.g., `file.txt` → `file.txt.zst`)
   - Includes file size statistics in MB

3. **Decompression Logic** (lines 129-168 in `decompress_file`):
   - Uses streaming decompression via `zstd::Decoder`
   - 64KB buffer size for both input and output
   - Validates that input has `.zst` extension
   - Output file naming: removes `.zst` extension

4. **Force Flag Behavior**: Both operations check for existing output files and error unless `--force` flag is provided

### Dependencies
- **zstd** (0.13): Zstandard compression library
- **clap** (4.5): Command-line argument parser with derive feature

## Language and Localization

All user-facing strings, comments, and documentation are in Italian. When modifying this code:
- Keep error messages in Italian
- Keep command descriptions and help text in Italian
- Keep println! output messages in Italian
- Comments should remain in Italian

## Memory Efficiency

The application uses streaming I/O throughout to handle files of any size without loading them completely into memory. When modifying file operations, maintain this streaming approach using `BufReader`, `BufWriter`, and the streaming encoder/decoder APIs.
