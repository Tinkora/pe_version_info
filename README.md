# Tinkora PE Version Info

> **Status: Alpha CLI prerelease (`v0.1.0-alpha.2`)** — the native CLI has hosted
> three-platform, clean-consumer, Windows resource, and Authenticode evidence.
> The Codex Skill/plugin remains Draft and is not an Agent-callable MCP release.

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

SVG, PDF, MCP/UI, and manual Explorer UI review are follow-up scope. The
Skill still requires a separate fresh-agent acceptance before it can leave
Draft. Do not use the CLI for signed release binaries without the documented
sign-after-verify pipeline.

## Quick start

```bash
cargo build --locked --release -p pevi_cli
target/release/pevi --help
target/release/pevi inspect --input fixtures/pe32_unsigned.exe --format json
```

Read [configuration](docs/configuration.md), [security and compatibility](docs/security_and_compatibility.md), and [releasing](docs/RELEASING.md) before mutation.

## Install a release binary

The [latest prerelease](https://github.com/Tinkora/pe_version_info/releases)
provides archives for Linux x86-64, macOS Apple Silicon, and Windows x86-64.
Download the archive and its matching `.sha256` file, then verify it before
placing `pevi` on `PATH`:

```bash
gh release download v0.1.0-alpha.2 \
  --repo Tinkora/pe_version_info \
  --pattern 'pevi-v0.1.0-alpha.2-*' \
  --dir release-assets
cd release-assets
sha256sum --check --strict ./*.sha256
tar -xzf pevi-v0.1.0-alpha.2-x86_64-unknown-linux-gnu.tar.gz
install -m 0755 pevi "$HOME/.local/bin/pevi"
pevi --version
```

Windows users should verify the `.exe.sha256` file with `Get-FileHash` and
place the executable in a user-owned directory on `PATH`. Release assets also
include `SHA256SUMS`, an SPDX SBOM, license evidence, and GitHub attestations;
the exact verification commands are documented in
[RELEASING.md](docs/RELEASING.md).

## Development

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), [SUPPORT.md](SUPPORT.md), and [CHANGELOG.md](CHANGELOG.md).
