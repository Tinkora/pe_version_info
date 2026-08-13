use crate::config::VersionConfig;
use crate::error::CoreError;
use editpe::types::VersionU16;
use editpe::{Image, ResourceDirectory, VersionInfo, VersionStringTable};

const EN_US_LANGUAGE_ID: u16 = 0x0409;
const EN_US_UTF16_TABLE: &str = "040904B0";

pub fn merge_version_info(
    existing: Option<&VersionInfo>,
    requested: &VersionConfig,
    preserve_unknown_strings: bool,
) -> Result<VersionInfo, CoreError> {
    validate_requested_version(requested)?;

    let mut merged = existing.cloned().unwrap_or_default();
    merged.info.file_version = requested.file_version.to_editpe();
    merged.info.product_version = requested.product_version.to_editpe();

    let table = match merged
        .strings
        .iter_mut()
        .find(|table| table.key.eq_ignore_ascii_case(EN_US_UTF16_TABLE))
    {
        Some(table) => table,
        None => {
            merged.strings.push(VersionStringTable {
                key: EN_US_UTF16_TABLE.to_owned(),
                ..VersionStringTable::default()
            });
            merged
                .strings
                .last_mut()
                .expect("a string table was just inserted")
        }
    };
    table.key = EN_US_UTF16_TABLE.to_owned();
    if !preserve_unknown_strings {
        table.strings.clear();
    }
    table
        .strings
        .insert("FileVersion".to_owned(), requested.file_version.to_string());
    table.strings.insert(
        "ProductVersion".to_owned(),
        requested.product_version.to_string(),
    );
    for (key, value) in &requested.strings {
        table.strings.insert(key.clone(), value.clone());
    }

    let en_us = VersionU16 {
        major: EN_US_LANGUAGE_ID,
        minor: requested.code_page,
    };
    if !merged.vars.contains(&en_us) {
        merged.vars.push(en_us);
    }
    merged.try_build().map_err(|_| CoreError::ConfigInvalid)?;
    Ok(merged)
}

pub fn prepare_version_resources(
    image: &Image<'_>,
    requested: &VersionConfig,
    preserve_unknown_strings: bool,
) -> Result<ResourceDirectory, CoreError> {
    let mut resources = image.resource_directory().cloned().unwrap_or_default();
    let existing = resources
        .get_version_info()
        .map_err(|_| CoreError::VersionInfoMalformed)?;
    let merged = merge_version_info(existing.as_ref(), requested, preserve_unknown_strings)?;
    resources
        .set_version_info(&merged)
        .map_err(|_| CoreError::ResourceMalformed)?;
    Ok(resources)
}

fn validate_requested_version(requested: &VersionConfig) -> Result<(), CoreError> {
    if requested.language != "en-US" || requested.code_page != 1200 {
        return Err(CoreError::ConfigInvalid);
    }
    if requested.strings.iter().any(|(key, value)| {
        key.is_empty()
            || key.encode_utf16().count() >= u16::MAX as usize
            || value.encode_utf16().count() >= u16::MAX as usize
    }) {
        return Err(CoreError::ConfigInvalid);
    }
    Ok(())
}
