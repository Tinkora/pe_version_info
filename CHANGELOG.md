# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

No changes yet.

## [0.1.0-alpha.2] - 2026-08-14

### Fixed

- Reject no-op apply requests and reserved version-string overrides before writing.
- Render terminal output as human-readable text while preserving the versioned JSON automation envelope.
- Replace ambiguous signature booleans with an explicit `not_checked` validation status.
- Align JSON Schema constraints with runtime language, code-page, version, icon, and crop rules.
- Run clean-consumer acceptance from a copied binary and fixture in an isolated working directory.
- Bind release attestation verification to the reviewed source ref and commit digest.

### Added

- Draft native Rust core and `pevi` CLI for PE inspection, VERSIONINFO merging, bounded PNG/JPEG/ICO conversion, transactional output, and stable JSON errors.
- Draft Codex Skill/plugin scaffold with configuration, error-code, and platform references.
- Cross-platform quality, supply-chain, documentation, and release-candidate workflows that retain artifacts as evidence only.
- Apply reports now include verified VERSIONINFO requests and deterministic icon conversion metadata.
- Main-icon verification checks every embedded frame and rejects missing or corrupted non-primary frames.
- `pevi init` now comments optional mutation sections to prevent accidental VERSIONINFO or icon changes.
- Plans expose icon policy and signature consequences; authorized signed edits emit an explicit Authenticode warning.

- Release SBOM and license evidence now cover only the `pevi_cli` normal/build dependency closure for released targets, exclude development-only crates, and embed the selected packages' actual license and notice texts.

- Release assets are now produced only from protected version tags and published with checksums, SPDX evidence, and GitHub attestations.
- Windows acceptance now proves an Authenticode digest is intact with a custom-validated test chain before a resource edit and reports that the rebuilt output has no signature afterward, without mutating system trust stores.

## [0.1.0-alpha.1] - 2026-08-13

### Initial scope

- Initial cross-platform `pevi` CLI candidate for inspecting and updating PE VERSIONINFO and icons.
- Draft Codex Skill/plugin scaffold and bilingual documentation.
