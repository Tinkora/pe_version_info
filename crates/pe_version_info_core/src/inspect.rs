use crate::error::CoreError;
use crate::signature::SignatureValidationStatus;
use crate::{SCHEMA_VERSION, VersionNumber};
use editpe::constants::{PE_32_MAGIC, PE_64_MAGIC};
use editpe::{DataDirectoryType, Image};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const MAX_PE_BYTES: u64 = 512 * 1024 * 1024;
const IMAGE_FILE_MACHINE_I386: u16 = 0x014c;
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PeKind {
    Pe32,
    Pe32Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PeArchitecture {
    X86,
    X86_64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionStringTableInspection {
    pub key: String,
    pub language_id: Option<u16>,
    pub code_page: Option<u16>,
    pub locale: Option<String>,
    pub strings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionInspection {
    pub file_version: VersionNumber,
    pub product_version: VersionNumber,
    pub string_tables: Vec<VersionStringTableInspection>,
    pub translations: Vec<[u16; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceSummary {
    pub resource_directory_present: bool,
    pub version_info_present: bool,
    pub main_icon_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PeInspection {
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    pub kind: PeKind,
    pub architecture: PeArchitecture,
    pub subsystem: u16,
    pub sha256: String,
    pub resources: ResourceSummary,
    pub certificate_table_present: bool,
    pub signature_validation: SignatureValidationStatus,
    pub signature_invalidated_by_edit: bool,
    pub version_info: Option<VersionInspection>,
}

pub fn inspect(path: &Path) -> Result<PeInspection, CoreError> {
    let metadata = fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => CoreError::PathNotFound(path.to_path_buf()),
        _ => CoreError::PathNotRegularFile(path.to_path_buf()),
    })?;
    if !metadata.is_file() {
        return Err(CoreError::PathNotRegularFile(path.to_path_buf()));
    }
    if metadata.len() > MAX_PE_BYTES {
        return Err(CoreError::InputTooLarge);
    }
    let bytes = fs::read(path).map_err(|_| CoreError::InvalidPe)?;
    let image = Image::parse(bytes.as_slice()).map_err(|_| CoreError::InvalidPe)?;
    let kind = match image.standard_header().magic {
        PE_32_MAGIC => PeKind::Pe32,
        PE_64_MAGIC => PeKind::Pe32Plus,
        _ => return Err(CoreError::UnsupportedPeArchitecture),
    };
    let architecture = match image.coff_header().machine {
        IMAGE_FILE_MACHINE_I386 => PeArchitecture::X86,
        IMAGE_FILE_MACHINE_AMD64 => PeArchitecture::X86_64,
        _ => return Err(CoreError::UnsupportedPeArchitecture),
    };
    let certificate_table_present = image
        .data_directory(DataDirectoryType::CertificateTable)
        .is_some_and(|directory| directory.virtual_address != 0 && directory.size != 0);
    let resources = image.resource_directory();
    let version_info = resources
        .map(|directory| {
            directory
                .get_version_info()
                .map_err(|_| CoreError::VersionInfoMalformed)
        })
        .transpose()?
        .flatten();
    let main_icon_present = resources
        .map(|directory| {
            directory
                .get_main_icon()
                .map(|icon| icon.is_some())
                .map_err(|_| CoreError::ResourceMalformed)
        })
        .transpose()?
        .unwrap_or(false);
    let version_info_present = version_info.is_some();
    let version_info = version_info.map(|version| {
        let string_tables = version
            .strings
            .iter()
            .map(|table| {
                let (language_id, code_page) = parse_string_table_key(&table.key);
                VersionStringTableInspection {
                    key: table.key.clone(),
                    language_id,
                    code_page,
                    locale: (language_id == Some(0x0409)).then(|| "en-US".to_owned()),
                    strings: table
                        .strings
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect(),
                }
            })
            .collect();
        let translations = version
            .vars
            .iter()
            .map(|translation| [translation.major, translation.minor])
            .collect();
        VersionInspection {
            file_version: VersionNumber::from_editpe(version.info.file_version),
            product_version: VersionNumber::from_editpe(version.info.product_version),
            string_tables,
            translations,
        }
    });

    Ok(PeInspection {
        schema_version: SCHEMA_VERSION,
        kind,
        architecture,
        subsystem: image.subsystem(),
        sha256: Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        resources: ResourceSummary {
            resource_directory_present: resources.is_some(),
            version_info_present,
            main_icon_present,
        },
        certificate_table_present,
        signature_validation: SignatureValidationStatus::NotChecked,
        signature_invalidated_by_edit: false,
        version_info,
    })
}

fn parse_string_table_key(key: &str) -> (Option<u16>, Option<u16>) {
    if key.len() != 8 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return (None, None);
    }
    (
        u16::from_str_radix(&key[..4], 16).ok(),
        u16::from_str_radix(&key[4..], 16).ok(),
    )
}
