# ADR 0001: Use `editpe` for Existing PE Mutation

Date: 2026-08-13  
Status: Accepted for implementation planning

## Context

The product must modify an arbitrary existing EXE/DLL on Windows, macOS, and Linux. `winresource` is documented as a Rust build-script helper and expects a resource compiler for the target build. `rcedit` is mature for common post-build fields but does not expose the full resource model needed for language-table normalization and a cross-platform Rust core.

## Decision

Use `editpe` as the primary parser/rebuilder for existing PE resources. Keep VERSIONINFO merging, signature policy, path safety, reporting, and icon-source conversion in project-owned code around that library.

## Consequences

- Cross-platform mutation is possible without invoking Windows APIs.
- The project owns compatibility tests against fixture PE files.
- The dependency is newer and smaller than mature GUI editors, so pin versions and maintain fixtures.
- `winresource` remains a documented build-time alternative, not a runtime editor.

