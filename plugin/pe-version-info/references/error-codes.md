# Stable error codes

| Code | Meaning |
| --- | --- |
| config_invalid | TOML, locale, version, or policy is invalid |
| config_version_unsupported | The schema version is not supported |
| path_not_found | An input or icon path does not exist |
| input_output_same | Input/output resolve to one file, including aliases |
| invalid_pe | The file is not a supported PE image |
| signed_input_rejected | A certificate table is present |
| signature_invalidation_not_acknowledged | Signature invalidation lacks explicit acknowledgement |
| unsupported_input_extension | The extension is outside the candidate |
| icon_invalid | Image or ICO bytes are malformed |
| icon_crop_not_allowed | Cover mode lacks explicit crop permission |
| output_exists | Destination exists without overwrite policy |
| write_failed | Temporary output or final replacement failed |
| verification_failed | Rebuilt output did not parse or match |

Do not retry a failure with weaker safety flags. Explain the required explicit
authorization or ask for a corrected input/config.
