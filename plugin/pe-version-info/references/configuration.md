# Configuration reference

The public config is TOML with schema_version = 1. Paths are resolved from the
directory containing the config file. input must be an existing .exe or .dll;
output is separate unless both in-place authorization flags are supplied.

`pevi plan --config pevi.toml --format json` must run before apply. The
signature flags are per-invocation authorization, not config keys.

Supported version locale is en-US / code page 1200. Supported icon sources are
PNG, JPEG, and ICO. The safe icon default is contain with transparent
letterboxing; cover requires allow_crop = true.

## Icon-only configuration

Omit `[version]` entirely when the user requested only an icon change. Do not
apply example VERSIONINFO values from an unreviewed `pevi init` template.

```toml
schema_version = 1
input = "dist/MyApp.exe"
output = "dist/MyApp-icon.exe"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[icon]
source = "assets/logo.png"
fit = "contain"
allow_crop = false
background = "transparent"
target_sizes = [16, 24, 32, 48, 64, 128, 256]
```

## VERSIONINFO-only configuration

Omit `[icon]` entirely when no icon change was requested.

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
ProductName = "My application"
```

Run the workflow with:

```text
pevi inspect --input dist/MyApp.exe --format json
pevi plan --config pevi.toml --format json
pevi apply --config pevi.toml --format json
pevi verify --input dist/MyApp-versioned.exe --config pevi.toml --format json
```

`apply.data` contains input/output SHA-256, signature state, changed flags,
verified VERSIONINFO requests, and icon conversion details. `verify.data`
contains `matches` plus a fresh output inspection. Successful certificate-table
edits also emit an Authenticode invalidation warning.
