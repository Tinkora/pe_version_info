# Agent Skill and Plugin Design

## 1. Distribution model

The repository will eventually contain a Codex plugin named `pe-version-info` (hyphenated because Codex plugin/Skill identifiers require it). The Rust project and GitHub repository use `pe_version_info` (underscored) according to the Tinkora repository naming convention.

Planned plugin structure:

```text
plugin/pe-version-info/
├── .codex-plugin/plugin.json
├── skills/pe-version-info/SKILL.md
├── references/configuration.md
├── references/error-codes.md
├── references/platforms.md
├── assets/icon.svg
└── .mcp.json                 # only when the MCP server is distributable
```

The plugin should not embed a platform-specific binary in Git. The Skill invokes `pevi` from PATH or a verified release cache. If a plugin-managed binary cache is added later, it must use pinned version + SHA-256 + target triple and never download arbitrary URLs supplied by the model.

## 2. Trigger description

The Skill description should trigger for requests involving:

- Windows EXE/DLL file properties, `VERSIONINFO`, `VS_VERSION_INFO`, `StringFileInfo`, `VarFileInfo`, PE resources, executable icons, `rcedit`, Resource Hacker, or `winresource`;
- setting product/file version, company, copyright, original filename, language, code page, or icon in a built Windows binary;
- inspecting or verifying those fields in CI;
- converting PNG/JPEG/ICO/SVG/PDF assets into a Windows executable icon.

It should not trigger for generic image conversion, Windows UI development, or ordinary Rust application metadata unless a PE resource operation is requested.

## 3. Skill workflow

The Skill body should enforce this sequence:

1. Identify the target EXE/DLL and configuration root.
2. Check that `pevi` is available and print its version. If unavailable, explain the supported installation path; do not substitute an ad-hoc script without user approval.
3. Run `pevi inspect --format json`.
4. Ask for or derive only values the user explicitly supplied. Never invent legal text.
5. Write or update `pevi.toml` using relative paths where possible.
6. Run `pevi plan --format json` and summarize the exact changes.
7. Refuse to overwrite the input unless the user explicitly requests in-place mode and confirms signature consequences.
8. Run `pevi apply` to a separate output by default.
9. Run `pevi verify --config ... --format json`.
10. Report output path, hash, changed fields, icon source/fit/crop state, and signature state.

## 4. AI-friendly requirements

- Stable command names and error codes.
- JSON output as the primary machine interface; human output is a presentation layer.
- No progress bars or prompts when stdout is not a TTY.
- `--yes` is never a blanket bypass for signature or in-place safety flags.
- Configuration is explicit and reviewable in a diff.
- The tool explains “why” a write was refused, including the remediation command.
- The tool reports the selected renderer and target sizes for icon conversion.

## 5. Plugin manifest direction

The future manifest should use the current plugin schema:

```json
{
  "name": "pe-version-info",
  "version": "0.1.0",
  "description": "Inspect and update Windows PE VERSIONINFO and icons with a reproducible CLI.",
  "repository": "https://github.com/Tinkora/pe_version_info",
  "license": "MIT",
  "skills": "./skills/",
  "interface": {
    "displayName": "PE Version Info",
    "shortDescription": "Set and verify Windows EXE metadata",
    "category": "Developer Tools",
    "capabilities": ["Read", "Write"],
    "defaultPrompt": ["Use PE Version Info to inspect and update this Windows executable safely."]
  }
}
```

The actual manifest should be generated/validated by the official `plugin-creator` workflow when implementation starts. Do not publish this example as an installable plugin before the binary and Skill are tested together.

