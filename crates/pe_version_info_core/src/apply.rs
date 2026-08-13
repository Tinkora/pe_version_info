use crate::config::{ExecutionAuthorization, IconConfig, Policy, VersionConfig};
use crate::error::CoreError;
use crate::icon::convert_icon;
use crate::verify::verify_requested_resources;
use crate::version_info::prepare_version_resources;
use editpe::{DataDirectoryType, Image};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::Builder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRequest {
    pub input: PathBuf,
    pub output: PathBuf,
    pub version: Option<VersionConfig>,
    pub icon: Option<IconConfig>,
    pub policy: Policy,
    pub authorization: ExecutionAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureReport {
    pub input_certificate_table_present: bool,
    pub output_signature_validated: bool,
    pub signature_invalidated_by_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyReport {
    pub schema_version: u32,
    pub input_path: String,
    pub output_path: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub signature: SignatureReport,
    pub version_changed: bool,
    pub icon_changed: bool,
    pub version: Option<VersionApplyReport>,
    pub icon: Option<IconApplyReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionApplyReport {
    pub file_version: crate::VersionNumber,
    pub product_version: crate::VersionNumber,
    pub language: String,
    pub code_page: u16,
    pub strings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconApplyReport {
    pub source_format: String,
    pub renderer: String,
    pub fit: String,
    pub background: String,
    pub target_sizes: Vec<u16>,
    pub cropped: bool,
}

pub fn apply(request: &ApplyRequest) -> Result<ApplyReport, CoreError> {
    let input_metadata = fs::metadata(&request.input).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => CoreError::PathNotFound(request.input.clone()),
        _ => CoreError::PathNotRegularFile(request.input.clone()),
    })?;
    if !input_metadata.is_file() {
        return Err(CoreError::PathNotRegularFile(request.input.clone()));
    }
    if input_metadata.len() > crate::inspect::MAX_PE_BYTES {
        return Err(CoreError::InputTooLarge);
    }
    let same_file = same_file(&request.input, &request.output)?;
    if same_file && !(request.authorization.in_place && request.authorization.confirm_in_place) {
        return Err(CoreError::InputOutputSame);
    }
    let output = if same_file {
        fs::canonicalize(&request.input).map_err(|_| CoreError::ConfigInvalid)?
    } else {
        request.output.clone()
    };
    if output.exists() && !request.policy.overwrite_output && !same_file {
        return Err(CoreError::OutputExists(output));
    }

    let input_bytes = fs::read(&request.input).map_err(|_| CoreError::InvalidPe)?;
    let input_image = Image::parse(input_bytes.as_slice()).map_err(|_| CoreError::InvalidPe)?;
    let input_certificate_table_present = input_image
        .data_directory(DataDirectoryType::CertificateTable)
        .is_some_and(|directory| directory.virtual_address != 0 && directory.size != 0);
    if input_certificate_table_present && !request.authorization.allow_signed_input {
        return Err(CoreError::SignedInputRejected);
    }
    if input_certificate_table_present && !request.authorization.acknowledge_signature_invalidation
    {
        return Err(CoreError::SignatureInvalidationNotAcknowledged);
    }

    let mut output_image = input_image.clone();
    if let Some(version) = &request.version {
        let resources = prepare_version_resources(
            &output_image,
            version,
            request.policy.preserve_unknown_strings,
        )?;
        output_image
            .set_resource_directory(resources)
            .map_err(|_| CoreError::ResourceMalformed)?;
    }
    let icon_report = if let Some(icon) = &request.icon {
        let artifact = convert_icon(icon)?;
        let report = IconApplyReport {
            source_format: artifact.source_format.clone(),
            renderer: artifact.renderer.clone(),
            fit: match icon.fit {
                crate::config::IconFit::Contain => "contain".to_owned(),
                crate::config::IconFit::Cover => "cover".to_owned(),
            },
            background: icon.background.clone(),
            target_sizes: artifact.target_sizes.clone(),
            cropped: artifact.cropped,
        };
        let mut resources = output_image
            .resource_directory()
            .cloned()
            .unwrap_or_default();
        // Remove the previous main group so repeated updates do not orphan its RT_ICON entries.
        resources
            .remove_main_icon()
            .map_err(|_| CoreError::ResourceMalformed)?;
        resources
            .set_main_icon(artifact.ico)
            .map_err(|_| CoreError::IconInvalid)?;
        output_image
            .set_resource_directory(resources)
            .map_err(|_| CoreError::ResourceMalformed)?;
        Some(report)
    } else {
        None
    };

    let parent = output.parent().ok_or(CoreError::WriteFailed)?;
    if !parent.is_dir() {
        return Err(CoreError::WriteFailed);
    }
    let output_mode_source = if output.exists() {
        output.as_path()
    } else {
        request.input.as_path()
    };
    let mut temporary = Builder::new()
        .prefix(".pevi-")
        .tempfile_in(parent)
        .map_err(|_| CoreError::WriteFailed)?;
    preserve_file_mode(output_mode_source, temporary.as_file())
        .map_err(|_| CoreError::WriteFailed)?;
    output_image
        .write_writer(&mut temporary)
        .map_err(|_| CoreError::WriteFailed)?;
    temporary.flush().map_err(|_| CoreError::WriteFailed)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| CoreError::WriteFailed)?;
    let temporary_bytes = fs::read(temporary.path()).map_err(|_| CoreError::WriteFailed)?;
    let verified =
        Image::parse(temporary_bytes.as_slice()).map_err(|_| CoreError::VerificationFailed)?;
    if verified.data().is_empty() {
        return Err(CoreError::VerificationFailed);
    }
    verify_requested_resources(&verified, request.version.as_ref(), request.icon.as_ref())?;
    if same_file || request.policy.overwrite_output {
        temporary
            .persist(&output)
            .map_err(|_| CoreError::WriteFailed)?;
    } else {
        temporary
            .persist_noclobber(&output)
            .map_err(|error| match error.error.kind() {
                std::io::ErrorKind::AlreadyExists => CoreError::OutputExists(output.clone()),
                _ => CoreError::WriteFailed,
            })?;
    }

    let output_bytes = fs::read(&output).map_err(|_| CoreError::WriteFailed)?;
    Ok(ApplyReport {
        schema_version: crate::SCHEMA_VERSION,
        input_path: request.input.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        input_sha256: sha256(&input_bytes),
        output_sha256: sha256(&output_bytes),
        signature: SignatureReport {
            input_certificate_table_present,
            output_signature_validated: false,
            signature_invalidated_by_edit: input_certificate_table_present,
        },
        version_changed: request.version.is_some(),
        icon_changed: request.icon.is_some(),
        version: request.version.as_ref().map(|version| VersionApplyReport {
            file_version: version.file_version,
            product_version: version.product_version,
            language: version.language.clone(),
            code_page: version.code_page,
            strings: version.strings.clone(),
        }),
        icon: icon_report,
    })
}

fn same_file(input: &Path, output: &Path) -> Result<bool, CoreError> {
    if input == output {
        return Ok(true);
    }
    match fs::metadata(output) {
        Ok(_) => same_file::is_same_file(input, output).map_err(|_| CoreError::ConfigInvalid),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(CoreError::ConfigInvalid),
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(unix)]
fn preserve_file_mode(source: &Path, destination: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(source)?.permissions().mode();
    destination.set_permissions(fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn preserve_file_mode(_source: &Path, _destination: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}
