---
name: test-coverage-reviewer
description: Reviews code changes for test coverage gaps, especially in GUI and CLI modules
tools: [Read, Grep, Glob]
---

# Test Coverage Review

Analyze the codebase to identify test coverage gaps.

## Current Test Distribution
- `src/lib.rs`: Contains 39 tests (core compression library)
- `src/main.rs`: No tests (CLI application)
- `src/gui.rs`: No tests (GUI application)

## Review Process

### 1. Identify Changed Functions
Look for recently modified or new functions that lack test coverage.

### 2. Check Public API Coverage
For each public function in lib.rs, verify there's at least one test covering:
- Happy path (normal operation)
- Error cases (invalid input, file not found, etc.)
- Edge cases (empty files, very large files, special characters)

### 3. Focus Areas
Priority areas for test coverage:

**Compression Operations:**
- `compress_file()` with various compression levels
- `decompress_file()` with corrupted data
- `compress_directory()` with nested structures
- `compress_multiple_files()` with mixed file types

**Error Handling:**
- Invalid file paths
- Permission errors
- Disk space issues
- Interrupted operations (Ctrl+C handling)

**Edge Cases:**
- Empty files
- Files with special characters in names
- Very large files (>1GB)
- Symbolic links

### 4. Report Format
Provide findings as:
1. Functions without tests (critical)
2. Functions with incomplete coverage (important)
3. Suggested test cases to add (actionable)

## Testing Patterns in This Codebase
Tests use temporary directories and files. Follow the existing pattern:
```rust
#[test]
fn test_example() {
    let temp_dir = tempfile::tempdir().unwrap();
    // ... test logic
}
```
