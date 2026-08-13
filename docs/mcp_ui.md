# MCP Server and Native-Feeling UI Plan

## 1. Why MCP is optional

The CLI and Skill solve local build automation without a server. An MCP server is useful when the host needs structured tool discovery, confirmation boundaries, or a visual form. It must not become a remote file-upload service by default.

## 2. Proposed tools

| Tool | Mutating | UI | Purpose |
|---|---:|---:|---|
| `pevi_inspect` | No | Optional | Inspect PE, VERSIONINFO, icon groups, signature state |
| `pevi_plan` | No | Yes | Show a diff-like proposed edit and icon preview metadata |
| `pevi_apply` | Yes | Yes | Apply a previously reviewed plan to an explicit output path |
| `pevi_verify` | No | Optional | Verify output against requested config |
| `pevi_convert_icon` | Writes icon file | Optional | Convert a selected source into ICO |

Every mutating tool must use accurate MCP annotations, return structured content, and require a confirmation step at the host or tool layer. `pevi_apply` must reject an output path outside the allowed workspace roots supplied by the host.

## 3. File interaction

Preferred order:

1. Use host-provided file selection (`window.openai.selectFiles`) when available.
2. Use a host-provided file upload (`window.openai.uploadFile`) only after the user selected the specific icon/EXE and the host confirms the file is available to the tool.
3. Fall back to a local path field and let the local MCP process access the file.
4. If none is possible, direct the user to the CLI.

The UI must display whether a source is local path, selected file ID, or uploaded content. It must not upload an EXE merely to display metadata if local CLI/MCP access is available.

## 4. UI flow

```text
Select EXE/DLL
      │
      v
Inspect -> show current metadata + signature warning
      │
      v
Select icon source / edit fields
      │
      v
Preview plan -> show old/new values + crop/background + output path
      │
      v
User confirmation
      │
      v
Apply -> verify -> show hashes and warnings
```

The UI should keep business data in tool results and ephemeral checkbox/form state in the component. It should remain useful when the component is not rendered by returning the same structured data to the model.

## 5. Host limitations

ChatGPT/Codex plugin UI is not the same as a native Windows/macOS dialog. It is an MCP Apps component in an iframe. It can provide a polished form and file interaction where the host supports the corresponding bridge, but it cannot guarantee access to arbitrary local files or native OS dialogs on every client. The product must preserve a CLI-first path.

