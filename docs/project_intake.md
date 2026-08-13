# PE Version Info Project Intake

[简体中文](project_intake.zh-CN.md)

Status: Approved for a narrow Draft implementation

Decision date: 2026-08-13

Review date: 2026-11-13

## Reproducible workflow

The target user is a release engineer or coding agent that receives an existing
Windows `.exe` or `.dll` after compilation. During a release, the user needs to
inspect its `VERSIONINFO`, update explicitly supplied product fields and its
main icon, write a separate output, and verify the result before signing.

The input is a local PE file, a versioned configuration file, and an optional
local icon. The output is a new PE file plus a machine-readable report. A failed
edit can ship incorrect Explorer metadata, destroy unrelated resources, or
invalidate an Authenticode signature without the release engineer noticing.

This workflow occurs per Windows release, commonly from a non-Windows CI host.
No network, hosted service, account, or uploaded binary is required.

## Alternatives

| Alternative | Strength | Gap for this workflow |
| --- | --- | --- |
| [`rcedit`](https://github.com/electron/rcedit) | Established CLI for common version strings and icons | Its documented binary is Windows-specific and edits the selected file directly; it does not provide this project's config, plan, transactional-output, or signature-acknowledgement contract. |
| [Resource Hacker](https://www.angusj.com/resourcehacker/) | Mature Windows GUI and command-line resource editor | Windows-focused and difficult to use as a deterministic, versioned, machine-readable CI contract. |
| [`winresource`](https://github.com/BenjaminRi/winresource) | Good Rust build-script integration for resources | Operates during a Rust Windows build and requires `rc.exe` or MinGW tools; it is not a general post-build editor for arbitrary existing PE files. |
| [`editpe`](https://github.com/Systemcluster/editpe) | Cross-platform library for parsing and rebuilding existing PE resources | It is the implementation foundation, not the complete product workflow: callers still own validation, signature policy, atomic output, stable reports, and automation UX. |

## Decision

Proceed as an independent repository because the tool has its own CLI release,
public configuration and report schemas, PE/signature trust boundary, fixture
matrix, and platform-specific release cadence. It does not belong in a browser
toolbox because executables remain local filesystem data and mutation requires
transactional OS file operations.

The differentiation is not a new PE parser. The project wraps the mature
`editpe` engine in one auditable workflow with safe output defaults, explicit
signature consequences, stable errors, deterministic icon conversion, and a
thin interface suitable for both people and agents.

## Initial scope

Always included in the first release candidate:

- PE32 and PE32+ EXE/DLL inspection on macOS, Linux, and Windows;
- `en-US` / UTF-16LE `VERSIONINFO` merge with unknown strings preserved;
- PNG, JPEG, and ICO main-icon input with bounded decoding and no implicit crop;
- separate output by default, atomic replacement, post-write parsing, and hashes;
- signed-input rejection by default and two-part explicit acknowledgement;
- a native `pevi` CLI, versioned JSON contract, and Codex Skill draft.

Explicitly excluded from the first release candidate:

- MCP transport or MCP Apps UI;
- remote file upload, telemetry, analytics, or automatic update checks;
- cryptographic Authenticode trust validation or signing;
- packed executables and arbitrary resource editing;
- default SVG or PDF support until renderer limits, licenses, and reproducible
  runtime distribution are proven on every claimed target.

## Trust and resource boundaries

- Treat every PE, config, and icon as untrusted input.
- Never overwrite the input without explicit in-place and confirmation flags.
- Never describe a certificate table as a valid or trusted signature.
- Reject malformed resources and decoder limit violations without panicking.
- Bound PE bytes, icon bytes, dimensions, pixel count, target frame count, and
  VERSIONINFO string sizes before expensive allocation or mutation.
- Keep normal CLI operation offline and exclude local paths from unrelated
  diagnostics.

## Validation and maturity

The repository remains **Draft** until the exact pushed commit passes hosted
Rust, documentation, and supply-chain checks and a clean consumer can run the
documented inspect, plan, apply, and verify workflow. Alpha additionally requires
a fresh-agent Skill acceptance run, independent Windows resource inspection,
real Authenticode pre/post-edit evidence, and complete candidate release
evidence. Only then may it be called **Alpha** and **Human-usable**.

The Codex Skill is an instruction layer. Without a tested MCP transport and tool
registration, the project must not claim **Agent-callable** or **Dual-use**.

The first candidate succeeds when:

- success, invalid-input, boundary, malformed-input, signature, and write-failure
  outcomes have behavior tests;
- committed fixtures have source, license, architecture, signature state, and
  SHA-256 provenance;
- macOS, Linux, and Windows hosted jobs pass for every claimed native behavior;
- a clean checkout produces the same public schemas and can complete the CLI
  workflow without uncommitted or machine-local state;
- release artifacts, if later authorized, include checksums, SBOM, license
  evidence, and provenance.

## Stop conditions

Review adoption on 2026-11-13. Merge, narrow, or archive the project if the
90-day window has no repeat use, release downloads, actionable feedback, or
documented workflow that the alternatives cannot satisfy. Do not expand formats,
languages, MCP, or UI merely to create activity.
