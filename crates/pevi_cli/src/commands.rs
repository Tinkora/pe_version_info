use clap::ValueEnum;
use pe_version_info_core::apply::ApplyRequest;
use pe_version_info_core::config::{
    ExecutionAuthorization, IconConfig, IconFit, load_config, load_config_with_output,
};
use pe_version_info_core::error::{CoreError, ErrorReport};
use pe_version_info_core::icon::convert_icon;
use pe_version_info_core::inspect::inspect;
use pe_version_info_core::verify::verify_path;
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::Builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IconFitArg {
    Contain,
    Cover,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthorizationFlags {
    pub in_place: bool,
    pub confirm_in_place: bool,
    pub allow_signed_input: bool,
    pub acknowledge_signature_invalidation: bool,
}

impl AuthorizationFlags {
    fn core(self) -> ExecutionAuthorization {
        ExecutionAuthorization {
            in_place: self.in_place,
            confirm_in_place: self.confirm_in_place,
            allow_signed_input: self.allow_signed_input,
            acknowledge_signature_invalidation: self.acknowledge_signature_invalidation,
        }
    }
}

pub struct CommandContext;

impl CommandContext {
    pub const fn new() -> Self {
        Self
    }

    pub fn init(&self, output: &Path, force: bool) -> Result<String, CoreError> {
        if output.exists() && !force {
            return Err(CoreError::OutputExists(output.to_path_buf()));
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|_| CoreError::WriteFailed)?;
        }
        fs::write(output, TEMPLATE).map_err(|_| CoreError::WriteFailed)?;
        render_success("init", json!({"path": output} ), Format::Human)
    }

    pub fn inspect(&self, input: &Path, format: Format) -> Result<String, CoreError> {
        let data = inspect(input)?;
        render_success(
            "inspect",
            serde_json::to_value(data).map_err(|_| CoreError::WriteFailed)?,
            format,
        )
    }

    pub fn plan(
        &self,
        config_path: &Path,
        format: Format,
        flags: AuthorizationFlags,
    ) -> Result<String, CoreError> {
        let config = load_config(config_path, &flags.core())?;
        let input = inspect(&config.input)?;
        let icon = config
            .icon
            .as_ref()
            .map(|icon| {
                convert_icon(icon).map(|artifact| {
                    json!({
                        "source_format": artifact.source_format,
                        "renderer": artifact.renderer,
                        "target_sizes": artifact.target_sizes,
                        "cropped": artifact.cropped,
                        "fit": match icon.fit {
                            IconFit::Contain => "contain",
                            IconFit::Cover => "cover",
                        },
                        "background": icon.background,
                    })
                })
            })
            .transpose()?;
        let signature_override_authorized =
            flags.allow_signed_input && flags.acknowledge_signature_invalidation;
        let data = json!({
            "input": input,
            "output": config.output,
            "version_requested": config.version.is_some(),
            "icon": icon,
            "signature": {
                "input_certificate_table_present": input.certificate_table_present,
                "edit_invalidates_signature": input.certificate_table_present
                    && (config.version.is_some() || config.icon.is_some()),
                "override_authorized": signature_override_authorized,
            },
            "signature_override_requested": signature_override_authorized,
        });
        render_success("plan", data, format)
    }

    pub fn apply(
        &self,
        config_path: &Path,
        output_override: Option<&Path>,
        format: Format,
        flags: AuthorizationFlags,
    ) -> Result<String, CoreError> {
        let config = load_config_with_output(config_path, &flags.core(), output_override)?;
        let request = ApplyRequest {
            input: config.input,
            output: config.output,
            version: config.version,
            icon: config.icon,
            policy: config.policy,
            authorization: flags.core(),
        };
        let report = pe_version_info_core::apply::apply(&request)?;
        let warnings = report
            .signature
            .signature_invalidated_by_edit
            .then(|| {
                "The resource edit invalidated the input Authenticode signature; verify and sign the output before release."
                    .to_owned()
            })
            .into_iter()
            .collect();
        render_success_with_warnings(
            "apply",
            serde_json::to_value(report).map_err(|_| CoreError::WriteFailed)?,
            format,
            warnings,
        )
    }

    pub fn verify(
        &self,
        input: &Path,
        config_path: Option<&Path>,
        format: Format,
    ) -> Result<String, CoreError> {
        let inspection = inspect(input)?;
        if let Some(config_path) = config_path {
            let config = load_config(
                config_path,
                &ExecutionAuthorization {
                    in_place: true,
                    confirm_in_place: true,
                    ..ExecutionAuthorization::default()
                },
            )?;
            verify_path(input, config.version.as_ref(), config.icon.as_ref())?;
        }
        let data = json!({"input": input, "inspection": inspection, "matches": true});
        render_success("verify", data, format)
    }

    pub fn convert_icon(
        &self,
        input: &Path,
        output: &Path,
        cover: bool,
        allow_crop: bool,
        format: Format,
    ) -> Result<String, CoreError> {
        let config = IconConfig {
            source: input.to_path_buf(),
            fit: if cover {
                IconFit::Cover
            } else {
                IconFit::Contain
            },
            allow_crop,
            background: "transparent".to_owned(),
            target_sizes: vec![16, 24, 32, 48, 64, 128, 256],
        };
        let artifact = convert_icon(&config)?;
        let parent = output.parent().ok_or(CoreError::WriteFailed)?;
        if !parent.is_dir() {
            return Err(CoreError::WriteFailed);
        }
        let mut temporary = Builder::new()
            .prefix(".pevi-icon-")
            .tempfile_in(parent)
            .map_err(|_| CoreError::WriteFailed)?;
        temporary
            .write_all(&artifact.ico)
            .map_err(|_| CoreError::WriteFailed)?;
        temporary.flush().map_err(|_| CoreError::WriteFailed)?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| CoreError::WriteFailed)?;
        temporary
            .persist_noclobber(output)
            .map_err(|error| match error.error.kind() {
                std::io::ErrorKind::AlreadyExists => CoreError::OutputExists(output.to_path_buf()),
                _ => CoreError::WriteFailed,
            })?;
        let data = json!({
            "output": output,
            "source_format": artifact.source_format,
            "renderer": artifact.renderer,
            "target_sizes": artifact.target_sizes,
            "cropped": artifact.cropped,
        });
        render_success("convert-icon", data, format)
    }
}

fn render_success<T: Serialize>(
    command: &str,
    data: T,
    format: Format,
) -> Result<String, CoreError> {
    render_success_with_warnings(command, data, format, Vec::new())
}

fn render_success_with_warnings<T: Serialize>(
    command: &str,
    data: T,
    format: Format,
    warnings: Vec<String>,
) -> Result<String, CoreError> {
    let envelope = json!({
        "schema_version": 1,
        "ok": true,
        "command": command,
        "data": data,
        "warnings": warnings,
        "errors": [],
    });
    if format == Format::Json {
        serde_json::to_string(&envelope).map_err(|_| CoreError::WriteFailed)
    } else {
        Ok(serde_json::to_string_pretty(&envelope).map_err(|_| CoreError::WriteFailed)?)
    }
}

#[allow(dead_code)]
pub fn render_error(command: &str, error: &CoreError, format: Format) -> String {
    let report: ErrorReport = error.to_report();
    let envelope = json!({
        "schema_version": 1,
        "ok": false,
        "command": command,
        "data": Value::Null,
        "warnings": [],
        "errors": [report],
    });
    if format == Format::Json {
        serde_json::to_string(&envelope).unwrap_or_else(|_| "{\"ok\":false}".to_owned())
    } else {
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "operation failed".to_owned())
    }
}

const TEMPLATE: &str = r#"# pevi configuration
schema_version = 1
input = "dist/MyApp.exe"
output = "dist/MyApp-versioned.exe"

[policy]
overwrite_output = false
preserve_unknown_strings = true

# Enable only the resource sections requested by the user.
# [version]
# file_version = "1.0.0.0"
# product_version = "1.0.0.0"
# language = "en-US"
# code_page = 1200
#
# [version.strings]
# ProductName = "My application"
# FileDescription = "My application"
#
# [icon]
# source = "assets/logo.png"
# fit = "contain"
# allow_crop = false
# background = "transparent"
# target_sizes = [16, 24, 32, 48, 64, 128, 256]
"#;
