# Tinkora PE Version Info

> **Status: Draft** — the native core/CLI and Draft Codex Skill have passed hosted native, documentation, and supply-chain checks. Alpha still requires independent Windows resource and Authenticode evidence, clean-consumer/Skill acceptance, and complete release-governance evidence.

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

PE Version Info (`pevi`) inspects and safely updates Windows PE32/PE32+ EXE/DLL `VERSIONINFO` resources and icons on the native host. The core uses `editpe`; the CLI emits one stable JSON object for automation.

Current candidate scope:

- Native Rust core and `pevi` CLI for `inspect`, `plan`, `apply`, `verify`, `init`, and `convert-icon`.
- `en-US` / UTF-16LE (`040904B0`) VERSIONINFO, unknown-string preservation, and PNG/JPEG/ICO icon input.
- Separate output by default, transactional writes, bounded decoding, and explicit two-flag authorization for in-place or certificate-table edits.
- Draft Codex Skill/plugin orchestration. This does not claim Agent-callable MCP support.

SVG, PDF, MCP/UI, and manual Explorer UI review are follow-up scope. Alpha still requires independent Windows API or inspector evidence for resources and Authenticode. Do not use the Draft candidate for signed release binaries without the documented sign-after-verify pipeline.

## Quick start

```bash
cargo build --locked --release -p pevi_cli
target/release/pevi --help
target/release/pevi inspect --input fixtures/pe32_unsigned.exe --format json
```

Read [configuration](docs/configuration.md), [security and compatibility](docs/security_and_compatibility.md), and [releasing](docs/RELEASING.md) before mutation.

## Development

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), [SUPPORT.md](SUPPORT.md), and [CHANGELOG.md](CHANGELOG.md).
