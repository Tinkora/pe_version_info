# PE Version Info Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `subagent-driven-development` or `executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform Rust CLI/library, Codex Skill, and optional MCP adapter that safely inspects and updates Windows PE `VERSIONINFO` and executable icons from reproducible configuration.

**Architecture:** Keep `pe_version_info_core` as the only behavior source. The `pevi` CLI owns local automation and JSON reports; the Codex Skill orchestrates the CLI without reimplementing binary logic; an optional MCP server and MCP Apps UI add structured inspection, preview, confirmation, and file selection while preserving a no-UI path.

**Tech Stack:** Rust 2024, `editpe` for existing PE resource parsing/rebuilding, `clap` for CLI, `serde`/`serde_json`/`toml`/`schemars` for configuration and reports, `image` for raster/ICO, `resvg`/`usvg` for SVG, optional `pdfium-render` plus a pinned PDFium runtime for PDF, MCP SDK/transport selected during the MCP implementation task, GitHub Actions, Codex plugin manifest and Skill.

---

## Scope and non-negotiable rules

- Modify existing EXE/DLL files cross-platform; do not limit the core to `build.rs`.
- Use `editpe` as the primary PE editor. Use `winresource` only in documentation or build-time fixtures; do not claim it edits arbitrary existing binaries.
- Default to a separate output file. Never overwrite the input without `--in-place --confirm-in-place`.
- Reject inputs with an Authenticode certificate table by default. Proceed only with two explicit flags and report that the resulting signature is invalid or absent.
- Preserve unrelated resources and unknown VERSIONINFO strings by default.
- Accept relative and absolute icon paths. Resolve relative paths from the configuration file directory, not the process working directory.
- Support PNG, JPEG, ICO, SVG, and PDF page 1 in Alpha. Unknown formats fail with a stable error; “any format” means an extensible input boundary, not an unbounded decoder promise.
- Preserve aspect ratio and do not crop by default. Use transparent letterboxing unless an explicit background or `cover`/`allow_crop` policy is configured.
- Keep the CLI offline. The plugin may use a verified release cache, but must never download arbitrary URLs supplied by the model.
- Use JSON output as the machine interface, stable error codes, no TTY prompts in non-interactive mode, and explicit confirmation for destructive writes.
- Treat PDFium as an optional native runtime with documented provenance, checksums, and licenses before release.

## Planned repository map

```text
pe_version_info/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── deny.toml
├── crates/
│   ├── pe_version_info_core/
│   │   ├── src/lib.rs
│   │   ├── src/config.rs
│   │   ├── src/error.rs
│   │   ├── src/icon.rs
│   │   ├── src/pe.rs
│   │   ├── src/report.rs
│   │   ├── src/signature.rs
│   │   ├── src/version_info.rs
│   │   └── tests/
│   ├── pevi_cli/
│   │   ├── src/main.rs
│   │   └── tests/cli.rs
│   └── pevi_mcp/
│       ├── src/main.rs
│       └── tests/protocol.rs
├── plugin/
│   └── pe-version-info/
│       ├── .codex-plugin/plugin.json
│       ├── skills/pe-version-info/SKILL.md
│       ├── references/configuration.md
│       ├── references/error-codes.md
│       ├── references/platforms.md
│       ├── assets/icon.svg
│       └── .mcp.json
├── fixtures/
│   ├── pe32_unsigned.exe
│   ├── pe64_unsigned.exe
│   ├── signed.exe
│   ├── no_version_info.exe
│   ├── malformed_resources.exe
│   └── icons/
├── tests/
│   ├── fixtures.rs
│   └── schema_contract.rs
├── docs/
└── .github/workflows/
```

## Task 1: Establish the Rust workspace and dependency policy

**Files:**

- Create: `Cargo.toml`
- Create: `Cargo.lock`
- Create: `rust-toolchain.toml`
- Create: `deny.toml`
- Create: `crates/pe_version_info_core/src/lib.rs`
- Create: `crates/pevi_cli/src/main.rs`
- Create: `tests/fixtures.rs`

- [ ] **Step 1: Create the workspace manifest**

Use a Rust 2024 workspace with application lockfile committed and explicit versions. Start with:

```toml
[workspace]
members = [
    "crates/pe_version_info_core",
    "crates/pevi_cli",
]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/Tinkora/pe_version_info"
rust-version = "1.95"

[workspace.dependencies]
assert_cmd = "2.2"
clap = { version = "4.6", features = ["derive"] }
editpe = "0.2.4"
image = { version = "0.25", default-features = false, features = ["jpeg", "ico", "png"] }
schemars = "1.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tempfile = "3.27"
thiserror = "2.0"
toml = "1.1"

[profile.release]
lto = "thin"
strip = true
```

Do not add PDFium or SVG dependencies until their isolated tasks pass and their native/runtime licensing is recorded.

- [ ] **Step 2: Add the toolchain and dependency policy**

Pin the repository toolchain in `rust-toolchain.toml`; configure `deny.toml` for advisories, licenses, bans, and sources. Use the Tinkora repository standard as the checklist and document any exception in `docs/decisions/`.

- [ ] **Step 3: Add compile-only crate skeletons**

Create minimal library and binary crates. `pe_version_info_core::VERSION_SCHEMA_VERSION` must be `1`. `pevi` must parse a `--help` command and exit successfully.

- [ ] **Step 4: Verify the baseline**

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all commands exit `0`; no behavior is implemented yet.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml deny.toml crates tests
git commit -F - <<'EOF'
建立 PE Version Info Rust 工作区与依赖基线
EOF
```

## Task 2: Define configuration, schemas, and stable errors before editing bytes

**Files:**

- Create: `crates/pe_version_info_core/src/config.rs`
- Create: `crates/pe_version_info_core/src/error.rs`
- Create: `crates/pe_version_info_core/src/report.rs`
- Create: `schemas/pevi_config_v1.json`
- Create: `schemas/pevi_report_v1.json`
- Test: `crates/pe_version_info_core/tests/config_tests.rs`

- [ ] **Step 1: Write configuration parsing tests**

Cover these exact cases:

```rust
#[test]
fn resolves_relative_paths_from_config_directory() { /* config path != cwd */ }

#[test]
fn rejects_input_equal_to_output_without_in_place_confirmation() { /* stable code */ }

#[test]
fn rejects_invalid_semver_component() { /* > 65535 */ }

#[test]
fn rejects_unknown_language_in_alpha() { /* no silent fallback */ }

#[test]
fn rejects_crop_without_allow_crop() { /* contain is safe default */ }
```

Each test must assert the stable error code, not an English sentence.

- [ ] **Step 2: Define the typed configuration**

Implement `Config`, `Policy`, `VersionConfig`, `VersionStrings`, and `IconConfig` with `serde` and `schemars`. Use `schema_version: u32` and `#[serde(deny_unknown_fields)]` for the public configuration. Represent versions as a validated four-component type rather than a free string after parsing.

- [ ] **Step 3: Define stable error codes**

At minimum implement:

```text
config_invalid
config_version_unsupported
path_not_found
path_not_regular_file
input_output_same
unsupported_input_extension
invalid_pe
unsupported_pe_architecture
signed_input_rejected
signature_invalidation_not_acknowledged
resource_malformed
version_info_malformed
icon_invalid
icon_source_too_large
icon_crop_not_allowed
pdf_runtime_unavailable
write_failed
verification_failed
```

Expose `code()`, a human message, and safe structured details. Do not include environment dumps or unrelated directory listings.

- [ ] **Step 4: Generate and test JSON schemas**

Generate versioned schemas from the Rust types, then add contract tests that reject unknown `schema_version` values and unknown required fields. Commit generated schemas because they are public contracts.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p pe_version_info_core --test config_tests --locked
cargo test --workspace --locked
```

Expected: all configuration and schema tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/pe_version_info_core schemas
git commit -F - <<'EOF'
定义配置文件、JSON Schema 与稳定错误码
EOF
```

## Task 3: Implement read-only PE inspection with `editpe`

**Files:**

- Create: `crates/pe_version_info_core/src/pe.rs`
- Create: `crates/pe_version_info_core/src/version_info.rs`
- Create: `crates/pe_version_info_core/src/signature.rs`
- Modify: `crates/pe_version_info_core/src/lib.rs`
- Create: `fixtures/README.md`
- Test: `crates/pe_version_info_core/tests/inspect_tests.rs`

- [ ] **Step 1: Add fixture generation instructions**

Document how to obtain reproducible PE32 and PE32+ fixtures and how to generate an unsigned Rust fixture with `winresource` on Windows. Do not commit proprietary binaries. If binary fixtures are committed, record their source, license, SHA-256, PE architecture, and whether they are signed.

- [ ] **Step 2: Write inspection tests**

Cover PE32, PE32+, missing VERSIONINFO, multiple string tables, existing main icon, malformed/truncated resource directory, and certificate-table detection. Tests must assert that malformed input returns an error and never panics.

- [ ] **Step 3: Implement PE classification**

Use `editpe::Image::parse` and expose a read-only `PeInspection` containing architecture, subsystem, resource summary, input SHA-256, and certificate-table presence. Do not call a Windows-only API.

- [ ] **Step 4: Implement VERSIONINFO reading**

Use `ResourceDirectory::get_version_info()`, `VersionInfo`, `VersionStringTable`, and `vars`. Normalize known language table keys into a public locale/code-page representation. Preserve raw values in a diagnostic field only when needed for troubleshooting.

- [ ] **Step 5: Implement signature detection policy input**

Detect the PE certificate-table directory without claiming cryptographic trust. The result fields must distinguish `certificate_table_present`, `signature_validated`, and `signature_invalidated_by_edit`. A certificate blob alone is never “verified”.

- [ ] **Step 6: Verify**

Run:

```bash
cargo test -p pe_version_info_core --test inspect_tests --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: read-only inspection works for all committed fixtures and rejects malformed data safely.

- [ ] **Step 7: Commit**

```bash
git add crates/pe_version_info_core fixtures
git commit -F - <<'EOF'
实现跨平台 PE 与 VERSIONINFO 只读检查
EOF
```

## Task 4: Implement deterministic VERSIONINFO merge/build

**Files:**

- Modify: `crates/pe_version_info_core/src/version_info.rs`
- Modify: `crates/pe_version_info_core/src/pe.rs`
- Test: `crates/pe_version_info_core/tests/version_info_tests.rs`

- [ ] **Step 1: Write merge tests**

Cover adding missing VERSIONINFO, updating file/product version, updating known strings, preserving unknown strings, replacing all strings only when requested, normalizing `en-US`/`1200` to `040904B0`, and rejecting oversized UTF-16 values.

- [ ] **Step 2: Implement a pure merge function**

Create a function with this contract:

```rust
pub fn merge_version_info(
    existing: Option<&editpe::VersionInfo>,
    requested: &VersionConfig,
    preserve_unknown_strings: bool,
) -> Result<editpe::VersionInfo, CoreError>
```

The function must not read or write files. It must update `FixedFileInfo.file_version` and `.product_version`, preserve file flags/OS/type fields, update the selected `VersionStringTable`, and set `vars` to the configured translation pair.

- [ ] **Step 3: Implement resource update preparation**

Clone or create the `ResourceDirectory`, call `set_version_info`, and leave the input image bytes untouched until the full output plan validates. If the existing version resource is malformed, return `version_info_malformed` rather than replacing it silently.

- [ ] **Step 4: Verify round-trip behavior**

After building, parse the resulting `VersionInfo` and compare the semantic model. Add a fixture test for the exact string-table key `040904B0` and translation values `0x0409/0x04B0`.

- [ ] **Step 5: Commit**

```bash
git add crates/pe_version_info_core
git commit -F - <<'EOF'
实现 VERSIONINFO 字段合并与语言表规范化
EOF
```

## Task 5: Implement icon decoding and ICO generation without implicit cropping

**Files:**

- Create: `crates/pe_version_info_core/src/icon.rs`
- Modify: `Cargo.toml`
- Modify: `crates/pe_version_info_core/src/error.rs`
- Test: `crates/pe_version_info_core/tests/icon_tests.rs`

- [ ] **Step 1: Write raster/ICO tests**

Cover PNG, JPEG, ICO, transparent contain-fit, explicit background, target sizes, malformed ICO, oversized dimensions, and crop refusal. Assert the output contains the configured sizes and that the source is not cropped by default.

- [ ] **Step 2: Implement bounded raster decode**

Use `image` with disabled unused default formats. Enforce source byte, dimension, and pixel-count limits before allocating large images. Normalize to RGBA8 and use deterministic Lanczos resizing.

- [ ] **Step 3: Implement contain-fit composition**

Render each target size onto a transparent or explicit background square. Center the source using integer coordinates and preserve aspect ratio. Implement `cover` only when `allow_crop` is true, and record `cropped: true` in the report.

- [ ] **Step 4: Implement ICO writing**

Write valid ICO directory entries for `[16, 24, 32, 48, 64, 128, 256]` by default. Validate the resulting ICO by parsing it again before embedding it into a PE resource.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo test -p pe_version_info_core --test icon_tests --locked
cargo test --workspace --locked
```

Then commit:

```bash
git add Cargo.toml Cargo.lock crates/pe_version_info_core
git commit -F - <<'EOF'
实现多格式图标转换并默认保持完整构图
EOF
```

## Task 6: Add SVG and PDF source support behind explicit feature boundaries

**Files:**

- Modify: `Cargo.toml`
- Create: `crates/pe_version_info_core/src/svg.rs`
- Create: `crates/pe_version_info_core/src/pdf.rs`
- Create: `docs/THIRD_PARTY_NOTICES.md`
- Modify: `docs/architecture.md`
- Test: `crates/pe_version_info_core/tests/vector_icon_tests.rs`

- [ ] **Step 1: Implement SVG rendering with `resvg`**

Add `resvg`/`usvg` behind an `svg` feature. Enforce an input-size and node-complexity limit, disallow external resource fetching, render with a deterministic viewport, and feed RGBA pixels into the contain-fit pipeline. Add tests for SVG with intentional whitespace, transparent background, malformed XML, and external-reference rejection.

- [ ] **Step 2: Decide the PDFium distribution mode**

Do not add `pdfium-render` to the default feature set until one target-specific CI job can acquire a pinned PDFium binary. Record the chosen source, release, target triples, SHA-256, license, and update process in `docs/THIRD_PARTY_NOTICES.md`. If that cannot be made reproducible, keep PDF behind an opt-in feature and document the conversion fallback.

- [ ] **Step 3: Implement first-page PDF rendering**

Add a `pdf` feature using `pdfium-render`. Default to page 1, reject page 0/out-of-range values, cap rendered dimensions, and return `pdf_runtime_unavailable` when the runtime cannot be bound. Never render more than the configured page count.

- [ ] **Step 4: Verify**

Run the SVG tests on all hosts. Run PDF tests only where the pinned PDFium runtime is available; record the unexecuted platform combinations explicitly in CI output and release notes.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/pe_version_info_core docs/THIRD_PARTY_NOTICES.md docs/architecture.md
git commit -F - <<'EOF'
增加 SVG 与可审计 PDF 图标输入支持
EOF
```

## Task 7: Replace the main icon and rebuild the PE transactionally

**Files:**

- Modify: `crates/pe_version_info_core/src/pe.rs`
- Create: `crates/pe_version_info_core/src/write.rs`
- Modify: `crates/pe_version_info_core/src/report.rs`
- Test: `crates/pe_version_info_core/tests/apply_tests.rs`

- [ ] **Step 1: Write mutation and safety tests**

Cover separate output, same-path refusal, signed-input refusal, explicit signed-input override, input preservation after decode/write/verify failure, unrelated-resource preservation, missing main icon, and output overwrite refusal.

- [ ] **Step 2: Implement resource mutation**

Parse the input with `editpe`, merge VERSIONINFO, call `set_main_icon` with the generated ICO, and preserve unrelated resource groups. Do not remove all icon groups unless `replace_all_icon_groups` is true.

- [ ] **Step 3: Implement atomic writing**

Write a random sibling temporary path, flush/close it, parse and verify it, then rename it to the requested output. Ensure the original input remains untouched if any stage fails. Handle an existing output only when `overwrite_output` is true.

- [ ] **Step 4: Implement the report**

Include schema version, input/output paths and SHA-256, changed version fields, changed strings, source format, page, target sizes, crop state, signature state, warnings, and stable errors. Redact or omit unrelated absolute paths in normal output.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo test -p pe_version_info_core --test apply_tests --locked
cargo test --workspace --locked
```

Commit:

```bash
git add crates/pe_version_info_core
git commit -F - <<'EOF'
实现 PE 资源事务性写入与签名安全策略
EOF
```

## Task 8: Implement the `pevi` CLI and JSON contract

**Files:**

- Modify: `crates/pevi_cli/src/main.rs`
- Create: `crates/pevi_cli/src/commands.rs`
- Create: `crates/pevi_cli/tests/cli.rs`
- Modify: `schemas/pevi_report_v1.json`
- Create: `examples/pevi.toml`

- [ ] **Step 1: Add CLI contract tests**

Test these commands exactly:

```text
pevi init --output pevi.toml
pevi inspect --input fixture.exe --format json
pevi plan --config pevi.toml --format json
pevi apply --config pevi.toml --format json
pevi verify --config pevi.toml --input output.exe --format json
pevi convert-icon --input logo.svg --output logo.ico
```

Assert exit codes, JSON `schema_version`, stable error codes, and no writes for `inspect`/`plan`.

- [ ] **Step 2: Implement `init` and config loading**

`init` writes a commented template and refuses to overwrite unless `--force` is provided. Config loading resolves all paths and validates before opening the EXE.

- [ ] **Step 3: Implement inspect/plan/apply/verify/convert-icon**

Keep command handlers thin: parse arguments, call Core, render human/JSON output, and map `CoreError` to documented exit codes. `--format json` must emit exactly one JSON object to stdout; logs go to stderr.

- [ ] **Step 4: Implement command safety flags**

Require both `--in-place` and `--confirm-in-place`. Require both `--allow-signed-input` and `--acknowledge-signature-invalidation` for signed inputs. Do not make `--yes` bypass either pair.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p pevi_cli --test cli --locked
cargo run -p pevi_cli -- init --output /tmp/pevi.toml
cargo run -p pevi_cli -- --help
```

- [ ] **Step 6: Commit**

```bash
git add crates/pevi_cli schemas examples
git commit -F - <<'EOF'
提供 pevi CLI 与稳定 JSON 自动化接口
EOF
```

## Task 9: Build the Codex Skill and plugin package

**Files:**

- Create: `plugin/pe-version-info/.codex-plugin/plugin.json`
- Create: `plugin/pe-version-info/skills/pe-version-info/SKILL.md`
- Create: `plugin/pe-version-info/references/configuration.md`
- Create: `plugin/pe-version-info/references/error-codes.md`
- Create: `plugin/pe-version-info/references/platforms.md`
- Create: `plugin/pe-version-info/assets/icon.svg`
- Create: `.agents/plugins/marketplace.json`

- [ ] **Step 1: Generate the plugin scaffold**

Use the official `plugin-creator` skill/script to create a plugin named `pe-version-info`; do not hand-invent unsupported manifest fields. Keep the project repository name `pe_version_info` and plugin identifier `pe-version-info` because the plugin host requires kebab-case identifiers.

- [ ] **Step 2: Write the trigger description**

Trigger on PE `VERSIONINFO`, `VS_VERSION_INFO`, `StringFileInfo`, `VarFileInfo`, `rcedit`, Resource Hacker, `winresource`, executable icons, or Windows EXE/DLL metadata. Do not trigger on generic image conversion or ordinary Rust package metadata.

- [ ] **Step 3: Write the Skill workflow**

The Skill must enforce: locate target; check `pevi --version`; inspect; collect only user-supplied values; write/update config; plan; obtain explicit confirmation for in-place/signature overrides; apply to separate output; verify; report hashes and warnings. It must call the CLI, not reimplement PE parsing in shell/Python.

- [ ] **Step 4: Add references**

Document the configuration schema, error codes, platform/runtime differences, and the distinction between `editpe` (existing PE editor) and `winresource` (build-time resource helper). Keep `SKILL.md` under 500 lines and load detailed material progressively.

- [ ] **Step 5: Add and validate the repo marketplace**

Create `.agents/plugins/marketplace.json` pointing to `./plugin/pe-version-info`, with `policy.installation`, `policy.authentication`, and `category`. Validate the plugin with the official plugin validator and validate the Skill with `quick_validate.py`.

- [ ] **Step 6: Forward-test with a clean agent context**

Ask a fresh agent to use the Skill on a fixture request without revealing the expected implementation. Verify that it inspects before writing, refuses unsafe in-place/signed edits, and runs verification. Remove any test artifacts it creates outside the repository.

- [ ] **Step 7: Commit**

```bash
git add plugin .agents/plugins/marketplace.json
git commit -F - <<'EOF'
提供 PE Version Info Codex Skill 与插件分发结构
EOF
```

## Task 10: Add the optional MCP server and file-selection UI

**Files:**

- Add: `pevi_mcp` to `Cargo.toml`
- Create: `crates/pevi_mcp/src/main.rs`
- Create: `crates/pevi_mcp/src/tools.rs`
- Create: `crates/pevi_mcp/tests/protocol.rs`
- Create: `crates/pevi_mcp/ui/`
- Modify: `plugin/pe-version-info/.mcp.json`
- Modify: `plugin/pe-version-info/.codex-plugin/plugin.json`
- Create: `docs/mcp_ui.md` updates with tested host behavior

- [ ] **Step 1: Define non-UI MCP tools**

Implement `pevi_inspect`, `pevi_plan`, `pevi_apply`, `pevi_verify`, and `pevi_convert_icon`. Use focused schemas, structured results, stable IDs/paths, accurate `readOnlyHint`/`destructiveHint` annotations, and server instructions that put required sequencing in the first 512 characters.

- [ ] **Step 2: Add protocol tests**

Test initialization, tools/list, every valid and invalid schema input, signed-input refusal, output-root enforcement, structured results, and error mapping. Test that the tools remain useful without a UI resource.

- [ ] **Step 3: Build the UI only after tool behavior works**

Use MCP Apps UI resources for a metadata form, icon preview, plan diff, and confirmation. Keep business data in tool results and ephemeral form state in the component. Feature-detect `window.openai.selectFiles`, `uploadFile`, and modal support; provide path/CLI fallback.

- [ ] **Step 4: Enforce local-file privacy**

Do not upload a local EXE merely for inspection when the local MCP process can access it. Display whether the source is a path, selected file ID, or uploaded content. Require action-time confirmation before uploading or writing.

- [ ] **Step 5: Verify with MCP Inspector**

Run the official MCP Inspector against the local endpoint and call every tool with direct, invalid, edge-case, and out-of-scope inputs. Record host limitations in `docs/mcp_ui.md` rather than claiming native OS dialogs are universally available.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/pevi_mcp plugin/pe-version-info docs/mcp_ui.md
git commit -F - <<'EOF'
增加可选 MCP 工具与文件选择确认界面
EOF
```

## Task 11: Add cross-platform CI, release artifacts, and supply-chain checks

**Files:**

- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `.github/dependabot.yml`
- Create: `.github/CODEOWNERS`
- Modify: `docs/security_and_compatibility.md`
- Create: `docs/RELEASING.md`
- Create: `docs/THIRD_PARTY_NOTICES.md`

- [ ] **Step 1: Add quality CI**

Run on Linux, macOS, and Windows where supported:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
npx --yes markdownlint-cli2@0.20.0 '**/*.md'
```

Windows must run fixture mutation and any Explorer/Authenticode verification that cannot be proven by cross-compilation. macOS/Linux jobs must run CLI mutation against PE fixtures.

- [ ] **Step 2: Add release matrix**

Build target-specific binaries for Windows x86_64/ARM64, macOS ARM64/x86_64, and Linux x86_64/ARM64 only after each target is tested. Generate `SHA256SUMS`, SBOM/license notices, and provenance/attestation where available.

- [ ] **Step 3: Add dependency and action pinning**

Pin third-party GitHub Actions to full commit SHA, enable Dependabot, and keep PDFium artifacts pinned by release and checksum. Do not use `curl | sh` or runtime tool downloads.

- [ ] **Step 4: Add release procedure**

Document “build → pevi apply → pevi verify → Authenticode sign → final verify” and explain that signing must be after resource edits. Include rollback and output hash verification.

- [ ] **Step 5: Commit**

```bash
git add .github docs
git commit -F - <<'EOF'
建立跨平台 CI、发布与供应链校验流程
EOF
```

## Task 12: Validate public documentation and perform the Alpha readiness review

**Files:**

- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `CHANGELOG.md`
- Create: `SECURITY.md`
- Create: `SECURITY.zh-CN.md`
- Create: `SUPPORT.md`
- Create: `SUPPORT.zh-CN.md`

- [ ] **Step 1: Update README claims from L0 to the actual maturity**

Do not claim “cross-platform support”, “AI Agent support”, “PDF support”, or “signed binary safety” until the corresponding CI and tests exist. Add exact installation commands, supported target matrix, limitations, privacy statement, and links to the configuration and release docs.

- [ ] **Step 2: Run the full readiness checklist**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
npx --yes markdownlint-cli2@0.20.0 '**/*.md'
git diff --check
```

On Windows, repeat the fixture apply/verify suite and inspect the final file properties. On macOS/Linux, explicitly report that Windows Explorer UI was not exercised.

- [ ] **Step 3: Perform a clean-checkout smoke test**

Clone the repository into a temporary directory, build with `--locked`, run `pevi init`, inspect a fixture, apply to a new output, and verify the output. Do not use artifacts from the developer’s working directory.

- [ ] **Step 4: Review the public contract**

Confirm that CLI JSON, error codes, config schema, MCP tools, Skill trigger text, and README examples agree. Search for `TBD`, `TODO`, placeholder URLs, unsupported claims, and any secrets before publishing.

- [ ] **Step 5: Commit**

```bash
git add README.md README.zh-CN.md CHANGELOG.md SECURITY.md SECURITY.zh-CN.md SUPPORT.md SUPPORT.zh-CN.md
git commit -F - <<'EOF'
完成 PE Version Info Alpha 文档与发布前校验
EOF
```

## Recommended execution order and handoff

Implement Tasks 1–4 first to produce a useful read-only/version-only CLI. Tasks 5–7 add icon and safe mutation behavior. Task 8 turns the core into a CI-ready interface. Task 9 packages the Codex Skill. Task 10 is optional and should not block CLI/Skill release. Tasks 11–12 are required before calling the project Alpha.

The next agent should begin with Task 1, create a local branch or worktree, and commit each completed task. It must not start with the MCP UI: the CLI/core contract is the dependency for every higher-level surface.
