# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A file compression/decompression utility written in Rust using the Zstandard (zstd) compression algorithm. Provides both CLI and GUI interfaces.

**Language**: CLI and library use Italian for all user-facing text (error messages, help text, comments). GUI auto-detects system locale and supports both Italian and English.

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
| sys-locale | GUI system locale detection |
| ctrlc | Ctrl+C signal handling for graceful interruption |

## Design Patterns

- **Streaming I/O**: Adaptive buffer sizes (256KB for files <10MB, 1MB for larger) - never loads entire files into memory
- **Auto-parallel**: Files ≥1MB automatically use multi-threaded compression; explicit `--parallel` flag also available
- **Large file optimizations**: Files ≥10MB enable WindowLog(24) and long-distance matching for better compression
- **Progress callbacks**: All operations accept `Box<dyn Fn(u64) + Send + Sync>` for progress tracking
- **Force flag**: Requires `--force` to overwrite existing files
- **Output naming**: `file.txt` → `file.txt.zst`, directories → `dirname.tar.zst`

## Gotchas

- **Italian text in CLI/lib**: All error messages, help text, and comments must be in Italian. GUI handles both Italian/English via sys-locale.
- **Tests require temp files**: Many tests create/delete files in temp directories; ensure cleanup on failure.
- **Windows builds**: `build.rs` embeds Windows resources (icon, metadata) - requires `winresource` build dependency.

## Release Builds

```bash
cargo build --release              # Full LTO, optimized (slower build)
cargo build --profile release-fast # Thin LTO, faster build
```

Multi-platform releases are automated via GitHub Actions (`.github/workflows/release.yml`) triggered by version tags (`v*`):

- Linux: AppImage
- Windows: ZIP archive
- macOS: Universal binary DMG (Intel + Apple Silicon)
