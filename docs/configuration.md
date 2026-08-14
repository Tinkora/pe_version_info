# Configuration and CLI Contract

## 1. Configuration format

Use TOML for human editing and JSON Schema for machine validation. Paths are resolved relative to the configuration file directory, not the process working directory. Absolute paths are accepted but should be avoided in committed project configs.

Example `pevi.toml`:

```toml
schema_version = 1
input = "dist/MyApp.exe"
output = "dist/MyApp-versioned.exe"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[version]
file_version = "2.2.0.0"
product_version = "2.2.0.0"
language = "en-US"
code_page = 1200

[version.strings]
FileDescription = "My application"
ProductName = "My application"
InternalName = "my_app"
OriginalFilename = "MyApp.exe"
CompanyName = "Example Company"
LegalCopyright = "Copyright © 2026 Example Company"

[icon]
source = "assets/logo.png"
fit = "contain"
background = "transparent"
target_sizes = [16, 24, 32, 48, 64, 128, 256]
```

Remove an unrequested `[version]` or `[icon]` section entirely. The generated
`pevi init` template keeps both mutation sections commented so that merely
creating a template cannot change VERSIONINFO or icons.

## 2. Required validation

- `schema_version` must be `1`.
- `input` must identify a regular file with an `.exe` or `.dll` extension, while the PE signature remains authoritative.
- `output` must not equal `input` unless the CLI was invoked with `--in-place` and the confirmation flag.
- `file_version` and `product_version` accept `major.minor.patch[.build]`, with each component in `0..=65535`.
- `[version.strings]` must not define `FileVersion` or `ProductVersion` in any casing; the canonical values come from the typed version fields.
- `language` must be a supported BCP-47/Windows locale mapping. Alpha supports `en-US`; unsupported locales fail instead of silently falling back.
- `code_page` must be a supported VERSIONINFO code page. Alpha supports `1200` (UTF-16LE).
- `target_sizes` must be unique, sorted, and in `16..=256`.
- `fit` is `contain` by default. `cover` is allowed only with an explicit `allow_crop = true`.
- `background` is `transparent` or an explicit `#RRGGBBAA` value.
- The first release candidate accepts PNG, JPEG, and ICO icon sources. SVG and
  PDF fail as unsupported until their follow-up feature gates are complete.

## 3. CLI commands

### `init`

```text
pevi init [--output <path>] [--force]
```

Writes a commented template. It never reads or modifies an EXE.

### `inspect`

```text
pevi inspect --input <path> [--format human|json]
```

Returns PE type, architecture, signature presence, VERSIONINFO fields, language tables, icon groups, SHA-256, and warnings. It is read-only.

### `plan`

```text
pevi plan --config <path> [--format human|json]
```

Resolves paths and displays a summary of the requested changes, including signature consequences and icon conversion details. It never writes; the summary is not a stable field-by-field diff.

### `apply`

```text
pevi apply --config <path> [--output <path>] [--in-place --confirm-in-place]
           [--allow-signed-input --acknowledge-signature-invalidation]
           [--format human|json]
```

The config remains authoritative; command-line output/policy flags are explicit overrides recorded in the report. A signed input requires both override flags and produces an unsigned/invalidly signed result warning. `apply` rejects a config that requests neither VERSIONINFO nor icon changes.

### `verify`

```text
pevi verify --input <path> [--config <path>] [--format human|json]
```

Checks that the file is parseable, requested values match, and the icon is valid. PE parsing only reports whether a certificate table is present; it does not validate Authenticode trust or digests. The independent Windows workflow verifies the pre-edit digest and test chain, then proves that the rebuilt output has no signature without changing system trust stores.

### `convert-icon`

```text
pevi convert-icon --input <path> --output <path.ico>
                  [--fit contain|cover --allow-crop]
```

Converts only the icon source. It must not modify an EXE.

## 4. JSON report contract

Every command using `--format json` emits one JSON object with:

```json
{
  "schema_version": 1,
  "ok": true,
  "command": "apply",
  "data": {
    "input_path": "dist/MyApp.exe",
    "output_path": "dist/MyApp-versioned.exe",
    "input_sha256": "...",
    "output_sha256": "...",
    "signature": {
      "input_certificate_table_present": false,
      "output_signature_validation": "not_checked",
      "signature_invalidated_by_edit": false
    },
    "version_changed": true,
    "icon_changed": false,
    "version": {
      "file_version": "2.2.0.0",
      "product_version": "2.2.0.0",
      "language": "en-US",
      "code_page": 1200,
      "strings": {
        "ProductName": "My application"
      }
    },
    "icon": null
  },
  "warnings": [],
  "errors": []
}
```

Error objects use stable `code`, a human message, and a safe `details` object. Details must not contain access tokens, full environment dumps, or unrelated directory listings.

`version` and `icon` are `null` when that resource type was not requested. When
present, they record the values verified in the output. The icon object contains
`source_format`, `renderer`, `target_sizes`, and `cropped`; it never includes the
source image bytes.
