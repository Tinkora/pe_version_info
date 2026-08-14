# Tinkora PE Version Info Product Specification

Status: Draft
Date: 2026-08-13
Repository: <https://github.com/Tinkora/pe_version_info>

## 1. Problem

Windows Explorer exposes product metadata through the PE `VERSIONINFO` resource. Existing build pipelines usually solve only one narrow case:

- Rust applications can embed resources during their own Windows build with `winresource`.
- `rcedit` can update common fields and icons after a build, but does not cover every VERSIONINFO language/resource-table requirement.
- GUI resource editors are difficult to reproduce in CI and are not naturally usable by an AI agent.

The result is repeated, fragile scripting for a normal release task: set the product name, version, description, copyright, language, and icon; inspect the output; and prove that the final EXE contains exactly those values.

## 2. Target users

1. A developer or release engineer who needs to update an existing `.exe` or `.dll` on Windows, macOS, or Linux.
2. An AI coding agent that needs a deterministic configuration file, dry-run report, machine-readable errors, and no hidden GUI state.
3. A build/CI maintainer who wants one pinned binary and one reproducible command instead of platform-specific Resource Hacker scripts.
4. A ChatGPT/Codex user who wants to upload or select an icon source, edit fields in a structured form, preview the result, and explicitly confirm the write.

## 3. Non-goals

- It is not a general PE disassembler, packer, malware-analysis tool, or executable optimizer.
- It does not make a signed executable remain signed after a resource edit.
- It does not infer arbitrary legal/copyright text or invent product metadata.
- It does not promise support for every possible image or document format. The first release candidate supports PNG, JPEG, and ICO. SVG and PDF remain follow-up formats until their renderer limits, licenses, and runtime distribution are reproducible; unsupported formats must fail clearly.
- It does not silently overwrite an input file.
- It does not upload local EXE or icon contents to a remote service in CLI/Skill mode.

## 4. Product surfaces

### 4.1 Core library

`pe_version_info_core` owns:

- PE type detection (PE32/PE32+), resource parsing, and safe resource rebuild.
- VERSIONINFO parse/build/update operations.
- Main icon replacement and preservation of unrelated resource groups.
- Signature detection and edit policy.
- Stable, serializable diagnostics.

The core must not depend on a desktop UI, an MCP transport, or a network service.

### 4.2 CLI

The `pevi` binary is the normative automation interface:

```text
pevi init --output pevi.toml
pevi inspect --input dist/app.exe --format json
pevi plan --config pevi.toml
pevi apply --config pevi.toml --output dist/app-versioned.exe
pevi verify --input dist/app-versioned.exe --format json
pevi convert-icon --input assets/logo.png --output build/logo.ico
```

The CLI must support Windows, macOS, and Linux. It must return non-zero exit codes for validation, format, signature-policy, and write failures.

### 4.3 Codex Skill

The Skill teaches an AI agent how to:

- locate or create a configuration file;
- resolve relative paths safely;
- call `inspect` before `apply`;
- use `plan`/dry-run output to explain changes;
- require an explicit output path or explicit in-place confirmation;
- run `verify` after writing;
- report unsupported formats and signature consequences without guessing.

The Skill is an instruction layer. It must not reimplement PE parsing in Markdown or shell snippets.

### 4.4 Optional MCP server and UI

The optional `pevi-mcp` server exposes the same operations as structured tools. A UI is only attached to tools that benefit from inspection/edit/confirmation, such as a VERSIONINFO form and icon preview. Every tool remains usable without UI.

ChatGPT/Codex custom UI is an MCP Apps resource rendered in an iframe; local file selection/upload is host-dependent and must feature-detect `window.openai.selectFiles`/`uploadFile`, with a text-path or CLI fallback.

## 5. Success criteria

### Alpha exit criteria

- A clean checkout builds one CLI binary on macOS, Linux, and Windows.
- A fixture EXE can be inspected and rewritten on all three hosts.
- PNG, JPEG, and ICO sources produce a valid multi-resolution ICO/PE main icon without implicit cropping.
- The output VERSIONINFO and main icon are verified by Windows APIs or an inspector independent of `editpe`.
- Signed inputs are rejected by default and the error explains why.
- A real Authenticode-signed fixture is valid before editing and independently reported invalid or unsigned after an explicitly authorized edit.
- `plan` produces a machine-readable summary without writing; it is not a stable field-by-field diff.
- A clean consumer can complete the documented CLI workflow, and a fresh agent can complete the Skill workflow without reading project-specific source code.
- The exact candidate commit passes hosted native, documentation, and supply-chain checks before the repository claims Alpha maturity.
- Candidate artifacts include checksums, an SBOM, license evidence, provenance/attestation where available, and protected `v*` tag governance.

### Stop conditions

Stop expanding the format matrix if a format cannot be supported without a large native runtime, unclear redistribution rights, or non-deterministic rendering. Keep the explicit supported set and provide a conversion hint.

## 6. User-visible compatibility contract

The first stable schema version is `1`. Breaking changes to configuration keys, CLI JSON, MCP tool names, or error codes require schema version `2` or a major release.

While the repository is Draft/Alpha, schema version `1` remains a candidate
contract and may receive breaking corrections before the first stable release.
Such changes must be called out in the changelog and mirrored in the generated
schemas and bilingual documentation. The stable-version rule applies once the
project leaves pre-1.0 maturity.

The tool must preserve all unrelated resources unless the config explicitly requests removal. It must preserve the original file when output is a separate path and must use an atomic temporary-write/rename sequence for replacement.
