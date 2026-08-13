use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorReport {
    pub code: String,
    pub message: String,
    pub details: BTreeMap<String, Value>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("configuration is invalid")]
    ConfigInvalid,
    #[error("configuration schema version is unsupported")]
    ConfigVersionUnsupported,
    #[error("path does not exist: {0}")]
    PathNotFound(PathBuf),
    #[error("path is not a regular file: {0}")]
    PathNotRegularFile(PathBuf),
    #[error("input and output resolve to the same path")]
    InputOutputSame,
    #[error("input extension is unsupported")]
    UnsupportedInputExtension,
    #[error("output already exists: {0}")]
    OutputExists(PathBuf),
    #[error("input exceeds the configured size limit")]
    InputTooLarge,
    #[error("input is not a supported PE image")]
    InvalidPe,
    #[error("PE architecture is unsupported")]
    UnsupportedPeArchitecture,
    #[error("signed input is rejected by policy")]
    SignedInputRejected,
    #[error("signature invalidation was not acknowledged")]
    SignatureInvalidationNotAcknowledged,
    #[error("PE resource data is malformed")]
    ResourceMalformed,
    #[error("VERSIONINFO data is malformed")]
    VersionInfoMalformed,
    #[error("icon data is invalid")]
    IconInvalid,
    #[error("icon source exceeds the configured size limit")]
    IconSourceTooLarge,
    #[error("icon cropping requires explicit permission")]
    IconCropNotAllowed,
    #[error("PDF runtime is unavailable")]
    PdfRuntimeUnavailable,
    #[error("output could not be written")]
    WriteFailed,
    #[error("output verification failed")]
    VerificationFailed,
}

impl CoreError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ConfigInvalid => "config_invalid",
            Self::ConfigVersionUnsupported => "config_version_unsupported",
            Self::PathNotFound(_) => "path_not_found",
            Self::PathNotRegularFile(_) => "path_not_regular_file",
            Self::InputOutputSame => "input_output_same",
            Self::UnsupportedInputExtension => "unsupported_input_extension",
            Self::OutputExists(_) => "output_exists",
            Self::InputTooLarge => "input_too_large",
            Self::InvalidPe => "invalid_pe",
            Self::UnsupportedPeArchitecture => "unsupported_pe_architecture",
            Self::SignedInputRejected => "signed_input_rejected",
            Self::SignatureInvalidationNotAcknowledged => "signature_invalidation_not_acknowledged",
            Self::ResourceMalformed => "resource_malformed",
            Self::VersionInfoMalformed => "version_info_malformed",
            Self::IconInvalid => "icon_invalid",
            Self::IconSourceTooLarge => "icon_source_too_large",
            Self::IconCropNotAllowed => "icon_crop_not_allowed",
            Self::PdfRuntimeUnavailable => "pdf_runtime_unavailable",
            Self::WriteFailed => "write_failed",
            Self::VerificationFailed => "verification_failed",
        }
    }

    pub const fn message(&self) -> &'static str {
        match self {
            Self::ConfigInvalid => "configuration is invalid",
            Self::ConfigVersionUnsupported => "configuration schema version is unsupported",
            Self::PathNotFound(_) => "path does not exist",
            Self::PathNotRegularFile(_) => "path is not a regular file",
            Self::InputOutputSame => "input and output resolve to the same file",
            Self::UnsupportedInputExtension => "input extension is unsupported",
            Self::OutputExists(_) => "output already exists",
            Self::InputTooLarge => "input exceeds the configured size limit",
            Self::InvalidPe => "input is not a supported PE image",
            Self::UnsupportedPeArchitecture => "PE architecture is unsupported",
            Self::SignedInputRejected => "signed input is rejected by policy",
            Self::SignatureInvalidationNotAcknowledged => {
                "signature invalidation was not acknowledged"
            }
            Self::ResourceMalformed => "PE resource data is malformed",
            Self::VersionInfoMalformed => "VERSIONINFO data is malformed",
            Self::IconInvalid => "icon data is invalid",
            Self::IconSourceTooLarge => "icon source exceeds the configured size limit",
            Self::IconCropNotAllowed => "icon cropping requires explicit permission",
            Self::PdfRuntimeUnavailable => "PDF runtime is unavailable",
            Self::WriteFailed => "output could not be written",
            Self::VerificationFailed => "output verification failed",
        }
    }

    pub fn to_report(&self) -> ErrorReport {
        let mut details = BTreeMap::new();
        match self {
            Self::PathNotFound(path)
            | Self::PathNotRegularFile(path)
            | Self::OutputExists(path) => {
                details.insert(
                    "path".to_owned(),
                    Value::String(path.to_string_lossy().into_owned()),
                );
            }
            _ => {}
        }
        ErrorReport {
            code: self.code().to_owned(),
            message: self.message().to_owned(),
            details,
        }
    }
}
