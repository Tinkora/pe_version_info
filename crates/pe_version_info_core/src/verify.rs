use crate::config::{IconConfig, VersionConfig};
use crate::error::CoreError;
use crate::icon::convert_icon;
use editpe::constants::{RT_GROUP_ICON, RT_ICON};
use editpe::{Image, ResourceDirectory, ResourceEntryName, ToIcon};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const ICON_DIRECTORY_HEADER_SIZE: usize = 6;
const ICON_DIRECTORY_ENTRY_SIZE: usize = 14;
const ICON_DIRECTORY_METADATA_SIZE: usize = 12;

pub fn verify_path(
    path: &Path,
    version: Option<&VersionConfig>,
    icon: Option<&IconConfig>,
) -> Result<(), CoreError> {
    let bytes = fs::read(path).map_err(|_| CoreError::InvalidPe)?;
    let image = Image::parse(bytes.as_slice()).map_err(|_| CoreError::InvalidPe)?;
    verify_requested_resources(&image, version, icon)
}

pub fn verify_requested_resources(
    image: &Image<'_>,
    version: Option<&VersionConfig>,
    icon: Option<&IconConfig>,
) -> Result<(), CoreError> {
    if version.is_none() && icon.is_none() {
        return Ok(());
    }
    let resources = image
        .resource_directory()
        .ok_or(CoreError::VerificationFailed)?;

    if let Some(requested) = version {
        let found = resources
            .get_version_info()
            .map_err(|_| CoreError::VerificationFailed)?
            .ok_or(CoreError::VerificationFailed)?;
        if found.info.file_version != requested.file_version.to_editpe()
            || found.info.product_version != requested.product_version.to_editpe()
        {
            return Err(CoreError::VerificationFailed);
        }
        let table = found
            .strings
            .iter()
            .find(|table| table.key.eq_ignore_ascii_case("040904B0"))
            .ok_or(CoreError::VerificationFailed)?;
        for (key, value) in &requested.strings {
            if table.strings.get(key) != Some(value) {
                return Err(CoreError::VerificationFailed);
            }
        }
        if !found
            .vars
            .iter()
            .any(|entry| entry.major == 0x0409 && entry.minor == requested.code_page)
        {
            return Err(CoreError::VerificationFailed);
        }
    }

    if let Some(requested) = icon {
        let expected = convert_icon(requested)?;
        let expected_frames = expected
            .ico
            .icons()
            .map_err(|_| CoreError::VerificationFailed)?;
        verify_main_icon_frames(resources, &expected_frames)?;
    }

    Ok(())
}

fn verify_main_icon_frames(
    resources: &ResourceDirectory,
    expected_frames: &[Vec<u8>],
) -> Result<(), CoreError> {
    let root = resources.root();
    let group_table = root
        .get(ResourceEntryName::ID(u32::from(RT_GROUP_ICON)))
        .and_then(|entry| entry.as_table())
        .ok_or(CoreError::VerificationFailed)?;
    let main_icon_name = ResourceEntryName::from_string("MAINICON");
    let main_icon_entry = match group_table.get(&main_icon_name) {
        Some(entry) => entry,
        None => {
            let first_name = group_table
                .entries()
                .into_iter()
                .next()
                .ok_or(CoreError::VerificationFailed)?;
            group_table
                .get(first_name)
                .ok_or(CoreError::VerificationFailed)?
        }
    };
    let main_icon_table = main_icon_entry
        .as_table()
        .ok_or(CoreError::VerificationFailed)?;
    let language = main_icon_table
        .entries()
        .into_iter()
        .next()
        .cloned()
        .ok_or(CoreError::VerificationFailed)?;
    let directory = main_icon_table
        .get(&language)
        .and_then(|entry| entry.as_data())
        .map(|data| data.data())
        .ok_or(CoreError::VerificationFailed)?;

    let header = directory
        .get(..ICON_DIRECTORY_HEADER_SIZE)
        .ok_or(CoreError::VerificationFailed)?;
    if read_u16(header, 0)? != 0 || read_u16(header, 2)? != 1 {
        return Err(CoreError::VerificationFailed);
    }
    let frame_count = usize::from(read_u16(header, 4)?);
    if frame_count != expected_frames.len() {
        return Err(CoreError::VerificationFailed);
    }
    let expected_directory_size = frame_count
        .checked_mul(ICON_DIRECTORY_ENTRY_SIZE)
        .and_then(|size| size.checked_add(ICON_DIRECTORY_HEADER_SIZE))
        .ok_or(CoreError::VerificationFailed)?;
    if directory.len() != expected_directory_size {
        return Err(CoreError::VerificationFailed);
    }

    let icon_table = root
        .get(ResourceEntryName::ID(u32::from(RT_ICON)))
        .and_then(|entry| entry.as_table())
        .ok_or(CoreError::VerificationFailed)?;
    let mut icon_ids = HashSet::with_capacity(frame_count);
    for (index, expected) in expected_frames.iter().enumerate() {
        let expected_metadata = expected
            .get(..ICON_DIRECTORY_METADATA_SIZE)
            .ok_or(CoreError::VerificationFailed)?;
        let expected_payload = expected
            .get(ICON_DIRECTORY_ENTRY_SIZE..)
            .ok_or(CoreError::VerificationFailed)?;
        let entry_offset = index
            .checked_mul(ICON_DIRECTORY_ENTRY_SIZE)
            .and_then(|offset| offset.checked_add(ICON_DIRECTORY_HEADER_SIZE))
            .ok_or(CoreError::VerificationFailed)?;
        let entry = directory
            .get(entry_offset..entry_offset + ICON_DIRECTORY_ENTRY_SIZE)
            .ok_or(CoreError::VerificationFailed)?;
        if entry.get(..ICON_DIRECTORY_METADATA_SIZE) != Some(expected_metadata) {
            return Err(CoreError::VerificationFailed);
        }
        let icon_id = read_u16(entry, ICON_DIRECTORY_METADATA_SIZE)?;
        if !icon_ids.insert(icon_id) {
            return Err(CoreError::VerificationFailed);
        }
        let actual_payload = icon_table
            .get(ResourceEntryName::ID(u32::from(icon_id)))
            .and_then(|entry| entry.as_table())
            .and_then(|table| table.get(&language))
            .and_then(|entry| entry.as_data())
            .map(|data| data.data())
            .ok_or(CoreError::VerificationFailed)?;
        if actual_payload != expected_payload {
            return Err(CoreError::VerificationFailed);
        }
    }

    Ok(())
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, CoreError> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(CoreError::VerificationFailed)?;
    Ok(u16::from_le_bytes(bytes))
}
