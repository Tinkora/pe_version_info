# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- Draft native Rust core and `pevi` CLI for PE inspection, VERSIONINFO merging, bounded PNG/JPEG/ICO conversion, transactional output, and stable JSON errors.
- Draft Codex Skill/plugin scaffold with configuration, error-code, and platform references.
- Cross-platform quality, supply-chain, documentation, and release-candidate workflows that retain artifacts as evidence only.
- Apply reports now include verified VERSIONINFO requests and deterministic icon conversion metadata.
- Main-icon verification checks every embedded frame and rejects missing or corrupted non-primary frames.
- `pevi init` now comments optional mutation sections to prevent accidental VERSIONINFO or icon changes.
- Plans expose icon policy and signature consequences; authorized signed edits emit an explicit Authenticode warning.
