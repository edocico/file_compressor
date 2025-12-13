# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A file compression/decompression utility written in Rust using the Zstandard (zstd) compression algorithm. Provides both CLI and GUI interfaces. **All user-facing text is in Italian** (error messages, commands, help text, comments).

## Build and Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build (optimized)

# Run CLI
cargo run -- compress <file> --livello 10
cargo run -- decompress <file.zst>
cargo run -- multicompress file1 file2 --output archive.tar.zst
cargo run -- batch "*.log" --livello 5
cargo run -- verifica <file.zst>

# Run GUI
cargo run --bin file_compressor_gui

# Test
cargo test                     # Run all tests
cargo test -- --nocapture      # With output
cargo test <test_name>         # Specific test

# Lint and format
cargo clippy
cargo fmt
```

**Binary outputs:**
- CLI: `target/{debug,release}/file_compressor`
- GUI: `target/{debug,release}/file_compressor_gui`

## Architecture

### Module Structure

| File | Purpose |
|------|---------|
| [src/lib.rs](src/lib.rs) | Core compression library with all compression/decompression logic |
| [src/main.rs](src/main.rs) | CLI application with progress bars |
| [src/gui.rs](src/gui.rs) | egui-based GUI application |

### Core Library (lib.rs)

Key types:
- `CompressOptions` / `DecompressOptions` - Builder pattern for operation configuration
- `CompressionResult` - Stores input/output sizes
- `VerifyResult` - File integrity verification result

Key functions:
- `compress_file()` / `decompress_file()` - Single file operations with progress callbacks
- `compress_directory()` - Creates tar.zst from directory
- `compress_multiple_files()` - Bundles files into tar.zst archive
- `verify_zst()` - Validates file integrity

### CLI Commands (main.rs)

| Command | Description |
|---------|-------------|
| `compress` | Single file/directory compression (level 1-21, default 3) |
| `decompress` | Decompress .zst or extract .tar.zst |
| `multicompress` | Create tar.zst from multiple files |
| `batch` | Compress files matching glob pattern (e.g., `*.log`, `**/*.txt`) |
| `verifica` | Verify .zst file integrity |

### Dependencies

| Crate | Purpose |
|-------|---------|
| zstd | Zstandard compression |
| clap | CLI argument parsing |
| indicatif | Progress bars |
| tar | TAR archive handling |
| rayon | Parallel batch processing |
| glob | Pattern matching for batch operations |
| eframe/rfd | GUI framework and file dialogs |

## Design Patterns

- **Streaming I/O**: Uses 256KB buffers with `BufReader`/`BufWriter` - never loads entire files into memory
- **Progress callbacks**: All operations accept `Box<dyn Fn(u64) + Send>` for progress tracking
- **Force flag**: Requires `--force` to overwrite existing files
- **Output naming**: `file.txt` → `file.txt.zst`, directories → `dirname.tar.zst`
