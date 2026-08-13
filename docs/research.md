# Research and Source Notes

Research date: 2026-08-13.

## PE and VERSIONINFO

- Microsoft, PE resource section: <https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#the-rsrc-section>
- Microsoft, VERSIONINFO resource: <https://learn.microsoft.com/en-us/windows/win32/menurc/versioninfo-resource>
- Microsoft, StringFileInfo block and `lang-charset`: <https://learn.microsoft.com/en-us/windows/win32/menurc/stringfileinfo-block>
- Microsoft, language identifier constants: <https://learn.microsoft.com/en-us/windows/win32/intl/language-identifier-constants-and-strings>
- Microsoft, `VS_FIXEDFILEINFO`: <https://learn.microsoft.com/en-us/windows/win32/api/verrsrc/ns-verrsrc-vs_fixedfileinfo>

The language identifier page warns that numeric language identifier constants are deprecated in favor of locale names, but the VERSIONINFO resource format still stores language/code-page table identifiers. The tool therefore accepts a user-friendly locale name and writes the resource-table representation required by Windows.

## Rust PE/resource libraries

- `editpe` repository and README: <https://github.com/Systemcluster/editpe>
- `editpe` API documentation: <https://docs.rs/editpe/latest/editpe/>
- `winresource` repository: <https://github.com/BenjaminRi/winresource>
- `winresource` API documentation: <https://docs.rs/winresource/latest/winresource/>

`editpe` documents cross-platform parsing/modification of existing PE resources, including icons and version info. `winresource` documents a build-script workflow that invokes Windows SDK/MinGW resource compilers, so it is a useful comparison and fixture generator rather than the editor core.

## Image and PDF rendering

- `image` crate: <https://github.com/image-rs/image>
- `resvg`: <https://github.com/linebender/resvg>
- `pdfium-render`: <https://github.com/ajrcarey/pdfium-render>
- PDFium binaries: <https://github.com/bblanchon/pdfium-binaries>

`pdfium-render` does not include PDFium; the application must source a compatible dynamic/static/WASM build. This is why PDF support is a feature with explicit binary provenance rather than an invisible dependency.

## Codex/ChatGPT plugin and UI

- Plugins overview: <https://developers.openai.com/codex/plugins>
- Plugin packaging: <https://developers.openai.com/plugins/build/plugins>
- MCP server: <https://developers.openai.com/plugins/build/mcp-server>
- Optional MCP UI: <https://developers.openai.com/plugins/build/chatgpt-ui>

The official documentation describes plugins as bundles of Skills, connectors, MCP servers, and optional UI. Custom UI is an MCP Apps iframe and must retain a non-UI tool path. File-selection/upload methods are host extensions and therefore require feature detection and fallback behavior.
