# Security, Signatures, and Compatibility

## 1. Authenticode policy

Changing resources changes the bytes covered by an Authenticode signature. Therefore:

- detect the PE certificate table before mutation;
- reject signed input by default;
- require two explicit flags to proceed with a signed input;
- state that the resulting signature is invalid or absent;
- never print “signed”, “trusted”, or “verified” solely because a certificate blob remains in the file;
- make re-signing a separate release-pipeline step.

The preferred workflow is to modify unsigned build output and sign only after `pevi verify` passes.

## 2. Input/output safety

- Default output is a new sibling file.
- Refuse path traversal outside the configured workspace when invoked by MCP.
- Refuse symlink surprises in `--in-place` mode unless the resolved target is explicitly confirmed.
- Use same-directory temporary output, fsync/close as supported, parse/verify temporary output, then rename.
- Preserve the input on every failure.
- Add `--backup` only as an explicit policy; never silently create backups with unpredictable names.

## 3. Resource safety

Set resource limits before decoding:

- maximum input file size;
- maximum icon source bytes;
- maximum SVG tree complexity;
- maximum PDF pages rendered (one by default);
- maximum raster dimensions and total pixels;
- maximum number and size of VERSIONINFO strings.

Reject malformed/truncated/cyclic resource directories with stable errors and no panic.

## 4. Compatibility matrix

| Dimension | Alpha commitment |
|---|---|
| Host OS | macOS, Linux, Windows |
| Target PE | PE32 and PE32+ EXE/DLL |
| Existing resource | present or absent VERSIONINFO; present or absent main icon |
| Version language | `en-US` / UTF-16LE (`040904B0`) |
| Icon source | PNG, JPEG, ICO, SVG, PDF first page |
| Signed input | reject by default; explicit invalidation override only |
| Output | separate path by default; explicit in-place mode |

Windows-specific visual verification remains required on a Windows runner or VM. Cross-compiling the CLI does not prove Explorer property-dialog or icon-cache behavior.

## 5. Supply chain

- Pin Rust dependencies and release binaries.
- Publish SHA-256 checksums and target triples.
- Record licenses and binary provenance for PDFium, `resvg`, `image`, and `editpe`.
- Do not download tools during a normal `apply` invocation.
- Release builds should use GitHub Actions with least-privilege permissions and artifact provenance where available.

