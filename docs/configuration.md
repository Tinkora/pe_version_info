# Configuration and CLI Contract

## 1. Configuration format

Use TOML for human editing and JSON Schema for machine validation. Paths are resolved relative to the configuration file directory, not the process working directory. Absolute paths are accepted but should be avoided in committed project configs.

Example `pevi.toml`:

```toml
schema_version = 1
input = "dist/MyApp.exe"
output = "dist/MyApp-versioned.exe"

[policy]
allow_signed_input = false
overwrite_output = false
backup_input = false
preserve_unknown_strings = true
replace_all_icon_groups = false

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
source = "assets/logo.pdf"
pdf_page = 1
fit = "contain"
background = "transparent"
target_sizes = [16, 24, 32, 48, 64, 128, 256]
```

## 2. Required validation

- `schema_version` must be `1`.
- `input` must identify a regular file with an `.exe` or `.dll` extension, while the PE signature remains authoritative.
- `output` must not equal `input` unless the CLI was invoked with `--in-place` and the confirmation flag.
- `file_version` and `product_version` accept `major.minor.patch[.build]`, with each component in `0..=65535`.
- `language` must be a supported BCP-47/Windows locale mapping. Alpha supports `en-US`; unsupported locales fail instead of silently falling back.
- `code_page` must be a supported VERSIONINFO code page. Alpha supports `1200` (UTF-16LE).
- `target_sizes` must be unique, sorted, and in `16..=256`.
- `fit` is `contain` by default. `cover` is allowed only with an explicit `allow_crop = true`.
- `background` is `transparent` or an explicit `#RRGGBBAA` value.

## 3. CLI commands

### `init`

```text
pevi init [--output <path>] [--force]
```

Writes a commented template. It never reads or modifies an EXE.

### `inspect`

```text
pevi inspect --input <path> [--format human|json] [--include-resource-summary]
```

Returns PE type, architecture, signature presence, VERSIONINFO fields, language tables, icon groups, SHA-256, and warnings. It is read-only.

### `plan`

```text
pevi plan --config <path> [--format human|json]
```

Resolves paths and displays the exact intended changes, including signature consequences and icon conversion details. It never writes.

### `apply`

```text
pevi apply --config <path> [--output <path>] [--in-place --confirm-in-place]
           [--allow-signed-input --acknowledge-signature-invalidation]
           [--format human|json]
```

The config remains authoritative; command-line output/policy flags are explicit overrides recorded in the report. A signed input requires both override flags and produces an unsigned/invalidly signed result warning.

### `verify`

```text
pevi verify --input <path> [--config <path>] [--format human|json]
```

Checks that the file is parseable, requested values match, the icon is valid, and signature state is reported accurately.

### `convert-icon`

```text
pevi convert-icon --input <path> --output <path.ico> [--pdf-page <n>]
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
  "input": { "path": "dist/MyApp.exe", "sha256": "..." },
  "output": { "path": "dist/MyApp-versioned.exe", "sha256": "..." },
  "changes": {
    "version": { "file_version": ["2.1.0.0", "2.2.0.0"] },
    "strings": { "ProductName": [null, "My application"] },
    "icon": { "source_format": "pdf", "page": 1, "cropped": false }
  },
  "signature": {
    "input_signed": false,
    "output_signature_valid": false,
    "warning": null
  },
  "warnings": [],
  "errors": []
}
```

Error objects use stable `code`, a human message, and a safe `details` object. Details must not contain access tokens, full environment dumps, or unrelated directory listings.
