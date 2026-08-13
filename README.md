# Tinkora PE Version Info

> **Status: Idea / L0** — this repository currently contains the product and implementation plan only.

[简体中文](README.zh-CN.md)

<!-- markdownlint-disable MD033 -->
<p align="center">
  <a href="https://ko-fi.com/tinkora" target="_blank" rel="noopener noreferrer">
    <img
      src="https://ko-fi.com/img/githubbutton_sm.svg"
      alt="Support Tinkora on Ko-fi"
      width="520"
    >
  </a>
</p>
<!-- markdownlint-enable MD033 -->

Cross-platform tooling for inspecting and updating Windows PE `VERSIONINFO`, replacing executable icons, and exposing the workflow to humans and AI agents.

The planned product has three layers:

- `pevi` — a deterministic Rust CLI/library that edits an existing `.exe` or `.dll` on Windows, macOS, and Linux.
- `pevi` Codex Skill — configuration templates and safe agent instructions for repeatable build pipelines.
- Optional MCP server/UI — structured inspection, preview, confirmation, and file selection for hosts that support MCP Apps.

The first implementation should use [`editpe`](https://github.com/Systemcluster/editpe) for cross-platform PE resource parsing and rebuilding. [`winresource`](https://github.com/BenjaminRi/winresource) is useful for build-time resources in Rust applications, but is not the primary engine for modifying arbitrary existing executables.

This is not yet a production tool. Do not use it to modify signed release binaries until signature invalidation, backup, atomic replacement, and post-write verification are implemented and tested.

## Plan

The detailed implementation plan is in [`docs/superpowers/plans/2026_08_13_pe_version_info.md`](docs/superpowers/plans/2026_08_13_pe_version_info.md).

Architecture decisions and research are in [`docs/decisions/`](docs/decisions/).
