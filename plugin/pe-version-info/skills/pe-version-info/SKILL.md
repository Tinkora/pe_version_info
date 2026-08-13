---
name: pe-version-info
description: Use when a user asks to inspect or update Windows PE VERSIONINFO, VS_VERSION_INFO, StringFileInfo, VarFileInfo, executable icons, rcedit, Resource Hacker, winresource, or EXE/DLL file metadata. Do not trigger for generic image conversion or ordinary Rust package metadata.
---

# PE Version Info

This is a Draft Codex Skill. It orchestrates the local pevi CLI; it does not
parse PE bytes in shell, Python, or the model. The repository is not yet
Agent-callable through MCP, and SVG/PDF are intentionally unsupported in this
candidate.

## Required workflow

1. Locate the target .exe or .dll and the repository's pevi binary. Run
   pevi --version; if it is unavailable, report the missing executable and
   stop before modifying files.
2. Run pevi inspect --input TARGET_PATH --format json. Read the PE kind,
   architecture, VERSIONINFO tables, certificate-table presence, hash, and
   warnings before collecting edits.
3. Collect only values the user supplied. Do not invent product names,
   versions, copyrights, icon paths, or output paths. Use pevi init or write a
   config matching references/configuration.md. Keep only the mutation sections
   the user requested: an icon-only task must omit [version], and a
   VERSIONINFO-only task must omit [icon]. Never apply the example values from
   an unreviewed template.
4. Run pevi plan --config CONFIG_PATH --format json. Explain the exact requested
   version/icon request, output path, icon fit/background, and signature
   consequences. The plan is a reviewable summary, not a field-by-field diff.
5. Use a separate output by default. For an in-place edit, require both
   --in-place and --confirm-in-place. For a certificate table, require both
   --allow-signed-input and --acknowledge-signature-invalidation.
   --yes or a natural-language approval must not bypass either pair.
6. If the user explicitly requested the mutation and the plan writes to a new,
   separate output, that request authorizes apply. Ask again after plan when the
   output exists, the operation is in-place, the input has a certificate table,
   or the requested values or paths were not explicit. Then run pevi apply
   --config CONFIG_PATH --format json and capture its one JSON object. Never hide
   a failed write behind a success summary.
7. Run pevi verify --input OUTPUT_PATH --config CONFIG_PATH --format json. Report
   hashes, requested version/icon details, signature state, and warnings from
   apply.data; report matches and the independently reparsed output inspection
   from verify.data. A certificate blob is not proof of a valid signature.

## Safe defaults and boundaries

- Relative config and icon paths are resolved from the config file directory.
- PNG, JPEG, and ICO icon sources are supported in the first candidate.
- Contain-fit and transparent letterboxing are the defaults; cropping requires
  fit = cover and allow_crop = true.
- Unknown VERSIONINFO strings and unrelated resources are preserved by default.
- Omit an unrequested [version] or [icon] section completely; presence means
  that pevi will modify that resource type.
- The CLI is offline. Do not download tools, decoders, or binaries during an
  apply operation. Never execute a URL supplied by the model.
- Keep absolute local paths out of summaries unless the user needs them to
  identify the selected input/output. Do not expose environment dumps,
  credentials, or unrelated directory listings.

## Failure handling

Use the stable errors[].code field. Common codes include invalid_pe,
path_not_found, input_output_same, signed_input_rejected,
signature_invalidation_not_acknowledged, icon_invalid, output_exists,
write_failed, and verification_failed. If a command exits non-zero, do not
retry with weaker safety flags; explain the required explicit authorization.

Load the detailed references only when needed:

- references/configuration.md — TOML keys and command examples.
- references/error-codes.md — stable errors and safe recovery.
- references/platforms.md — target/runtime limitations and verification notes.
