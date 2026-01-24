---
name: release-prep
description: Prepare a new release - update version in Cargo.toml, verify build and tests, summarize changes, create git tag
disable-model-invocation: true
---

# Release Preparation Checklist

Follow these steps to prepare a new release:

## 1. Determine Version
Ask the user for the new version number if not provided. Follow semantic versioning:
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes

## 2. Update Cargo.toml
Update the `version` field in Cargo.toml to the new version.

## 3. Verify Build
Run `cargo build --release` to ensure the release build succeeds.

## 4. Run Tests
Run `cargo test` to verify all tests pass (currently 39 tests in lib.rs).

## 5. Run Quality Checks
Run `cargo fmt -- --check` and `cargo clippy -- -D warnings` to match CI requirements.

## 6. Generate Changelog Summary
Use `git log` to summarize changes since the last tag:
```bash
git log $(git describe --tags --abbrev=0)..HEAD --oneline
```

Present a formatted changelog to the user for review.

## 7. Create Git Tag
After user approval, create an annotated tag:
```bash
git tag -a v<version> -m "Release v<version>: <brief summary>"
```

## 8. Final Instructions
Remind the user to push the tag to trigger the release workflow:
```bash
git push origin v<version>
```

This will trigger `.github/workflows/release.yml` which builds:
- Linux: AppImage
- Windows: ZIP archive
- macOS: Universal binary DMG (Intel + Apple Silicon)
