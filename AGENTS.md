# Agent Instructions

This repository is a Rust CLI/library and Codex plugin project for Windows PE resource editing.

- Treat existing PE files as user data. Never overwrite without an explicit output path or an explicit `--in-place` confirmation flow.
- Reject Authenticode-signed inputs by default. Resource edits invalidate the signature; require an explicit override and report the post-edit state.
- Use `editpe` for cross-platform parsing and rebuilding of existing PE resources. Use `winresource` only for build-time resource fixture experiments or documenting its narrower role.
- Keep image conversion deterministic and lossless by default: preserve aspect ratio, do not crop, use the first PDF page, and record the source format and renderer in the report.
- Keep public documentation in English with a complete Chinese counterpart where the repository standard requires it.
- Add regression tests for every parser, resource, path, security, and output behavior change.

## Commit Language

- Write commit subjects and bodies in English and follow Conventional Commits.
- This repository-level rule overrides any global preference for another commit-message language.

## Public Source Language

- Write new public code comments in English.
- Keep public documentation in English with a complete Chinese counterpart where required.
