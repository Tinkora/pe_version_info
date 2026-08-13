# Architecture and Technical Design

## 1. Component graph

```text
                         +--------------------------+
                         | ChatGPT/Codex host       |
                         | Skill + optional MCP UI  |
                         +------------+-------------+
                                      |
                          MCP tools / local command
                                      |
+-------------------+     +----------v-----------+
| pevi CLI           |---->| pe_version_info_core |
| config + reports   |     | PE + VERSIONINFO     |
+---------+---------+     | icon conversion       |
          |               +----------+-----------+
          |                          |
          |              +-----------+------------+
          |              | editpe / image / resvg |
          |              | PDFium optional        |
          |              +------------------------+
          v
     existing EXE/DLL
```

## 2. Workspace layout

```text
pe_version_info/
├── Cargo.toml
├── crates/
│   ├── pe_version_info_core/
│   │   ├── src/pe.rs
│   │   ├── src/version_info.rs
│   │   ├── src/icon.rs
│   │   ├── src/signature.rs
│   │   ├── src/config.rs
│   │   └── src/error.rs
│   ├── pevi_cli/
│   │   └── src/main.rs
│   └── pevi_mcp/
│       └── src/main.rs
├── plugin/
│   └── pe-version-info/
│       ├── .codex-plugin/plugin.json
│       ├── skills/pe-version-info/SKILL.md
│       ├── assets/
│       └── .mcp.json
├── fixtures/
├── tests/
├── docs/
└── .github/workflows/
```

The plugin bundle is distribution metadata and instructions. The Rust crates remain the source of truth for behavior.

## 3. Resource-edit pipeline

1. Read the input bytes and identify PE32/PE32+.
2. Inspect the certificate table and apply the signature policy before doing any mutation.
3. Parse the resource directory with `editpe`.
4. Parse existing `VERSIONINFO` if present; otherwise create a default `VS_VERSION_INFO` structure.
5. Merge only configured values. Preserve unrelated string keys unless `replace_all = true` is explicitly enabled.
6. Normalize the configured language to a Windows language identifier and code page. The initial supported value is `en-US` / UTF-16LE (`0x0409` / `0x04B0`), represented in the string table as `040904B0`.
7. Convert the configured icon source to a square, non-cropped image or a validated ICO. Preserve aspect ratio and use transparent letterboxing unless the user explicitly selects a background color or `cover` fit.
8. Replace the main icon group while preserving unrelated icon groups and resources.
9. Rebuild the resource section transactionally. Do not write directly to the final path.
10. Verify that the output can be parsed again and that all requested values are present.
11. Report signature state, input/output SHA-256, changed fields, icon source, and warnings.

## 4. Why `editpe`, not `winresource`, is the main engine

`winresource` is a build script helper. Its documented flow compiles resources into a Rust crate's Windows build and requires Windows SDK `rc.exe` or MinGW `windres.exe` when cross-compiling. It is not designed as a general editor for an arbitrary existing PE file.

`editpe` explicitly supports cross-platform parsing and modification of existing PE resources, including icons and version info, and rebuilds the resource directory and headers. It therefore matches the primary use case. The implementation must pin and test the selected `editpe` release rather than relying on an unbounded dependency range.

## 5. Image conversion pipeline

| Input | Decoder/renderer | Default behavior | Output |
|---|---|---|---|
| PNG/JPEG | `image` | preserve aspect ratio; transparent letterbox | multi-resolution ICO frames |
| ICO | `image`/ICO parser | preserve existing frames when valid; normalize if requested | validated ICO frames |
| SVG | `resvg`/`usvg` | render at each target size; no crop | multi-resolution ICO frames |
| PDF | PDFium through `pdfium-render` | render selected page, default page 1; no crop | multi-resolution ICO frames |

“任意格式”在产品层应解释为“配置入口可扩展、首版支持明确格式”，不能把未知扩展名强行当作图片。PDFium is a native runtime and must be distributed or discovered per target; its licensing and binary provenance must be documented in `THIRD_PARTY_NOTICES.md` before enabling PDF in a release build.

## 6. Atomic output

`apply` writes to a sibling temporary file in the destination directory, flushes and closes it, verifies the temporary file, then renames it into place. On Windows, rename behavior must account for an existing destination and open handles. `--in-place` is implemented as an explicit output plan, not as a default.

## 7. Determinism

- Stable ordering for JSON reports and VERSIONINFO string keys.
- Explicit renderer version and icon target sizes in the report.
- No network access in the CLI.
- Input paths resolved before execution and recorded as normalized paths only when the user requests verbose diagnostics; reports must not leak unrelated directory contents.

