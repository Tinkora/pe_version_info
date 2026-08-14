use editpe::constants::{LANGUAGE_ID_EN_US, RT_GROUP_ICON, RT_ICON};
use editpe::{Image, ResourceEntryName};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use pe_version_info_core::config::{
    ExecutionAuthorization, IconConfig, IconFit, Policy, VersionConfig,
};
use pe_version_info_core::error::CoreError;
use pe_version_info_core::inspect::inspect;
use pe_version_info_core::signature::SignatureValidationStatus;
use pe_version_info_core::verify::verify_requested_resources;
use pe_version_info_core::version_info::prepare_version_resources;
use pe_version_info_core::{VersionNumber, apply};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn version() -> VersionConfig {
    VersionConfig {
        file_version: VersionNumber::from_str("9.8.7.6").unwrap(),
        product_version: VersionNumber::from_str("5.4.3.2").unwrap(),
        language: "en-US".to_owned(),
        code_page: 1200,
        strings: [("ProductName".to_owned(), "Applied product".to_owned())]
            .into_iter()
            .collect(),
    }
}

fn write_icon(path: &Path) {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(32, 32, Rgba([255, 0, 0, 255])));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    fs::write(path, bytes.into_inner()).unwrap();
}

fn request(input: PathBuf, output: PathBuf) -> apply::ApplyRequest {
    apply::ApplyRequest {
        input,
        output,
        version: Some(version()),
        icon: None,
        policy: Policy::default(),
        authorization: ExecutionAuthorization::default(),
    }
}

#[test]
fn writes_to_a_separate_output_and_reports_hashes() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    let before = fs::read(&input).unwrap();

    let report =
        apply::apply(&request(input.clone(), output.clone())).expect("apply should succeed");

    assert_eq!(fs::read(&input).unwrap(), before);
    assert!(output.is_file());
    assert_eq!(report.input_sha256.len(), 64);
    assert_eq!(report.output_sha256.len(), 64);
    assert!(!report.signature.input_certificate_table_present);
    assert_eq!(
        report.signature.output_signature_validation,
        SignatureValidationStatus::NotChecked
    );
    let inspection = inspect(&output).unwrap();
    assert_eq!(
        inspection.version_info.unwrap().file_version.components(),
        [9, 8, 7, 6]
    );
}

#[test]
fn rejects_existing_output_without_overwrite_policy() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::write(&output, b"do-not-replace").unwrap();

    let error =
        apply::apply(&request(input, output.clone())).expect_err("existing output should fail");

    assert_eq!(error.code(), "output_exists");
    assert_eq!(fs::read(output).unwrap(), b"do-not-replace");
}

#[cfg(unix)]
#[test]
fn preserves_permissions_for_a_new_output_based_on_input() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o751)).unwrap();

    apply::apply(&request(input, output.clone())).expect("separate output should succeed");

    assert_eq!(
        fs::metadata(output).unwrap().permissions().mode() & 0o777,
        0o751
    );
}

#[test]
fn permits_existing_output_only_with_explicit_overwrite_policy() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::write(&output, b"replace-me").unwrap();
    let mut request = request(input, output.clone());
    request.policy.overwrite_output = true;

    apply::apply(&request).expect("explicit overwrite should succeed");

    assert!(Image::parse(fs::read(output).unwrap()).is_ok());
}

#[test]
fn permits_in_place_only_with_both_explicit_flags() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    let mut request = request(input.clone(), input.clone());

    let error = apply::apply(&request).expect_err("in-place should require confirmation");
    assert_eq!(error.code(), "input_output_same");

    request.authorization.in_place = true;
    let error = apply::apply(&request).expect_err("in-place should require both flags");
    assert_eq!(error.code(), "input_output_same");

    request.authorization.confirm_in_place = true;
    let report = apply::apply(&request).expect("both in-place flags should permit replacement");
    assert_eq!(report.input_sha256.len(), 64);
    assert!(Image::parse(fs::read(input).unwrap()).is_ok());
}

#[cfg(unix)]
#[test]
fn rejects_in_place_symlink_alias_without_confirmation() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let alias = directory.path().join("alias.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    symlink(&input, &alias).unwrap();

    let error = apply::apply(&request(input, alias)).expect_err("symlink alias should be refused");

    assert_eq!(error.code(), "input_output_same");
}

#[cfg(unix)]
#[test]
fn preserves_permissions_for_in_place_edits() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o755)).unwrap();
    let mut request = request(input.clone(), input.clone());
    request.authorization.in_place = true;
    request.authorization.confirm_in_place = true;

    apply::apply(&request).expect("in-place edit should succeed");

    assert_eq!(
        fs::metadata(input).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn rejects_signed_input_by_default_and_requires_both_acknowledgements() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("signed.exe");
    let output = directory.path().join("output.exe");
    let mut bytes = fs::read(fixture("pe32_unsigned.exe")).unwrap();
    add_certificate_table_marker(&mut bytes);
    fs::write(&input, bytes).unwrap();

    let error = apply::apply(&request(input.clone(), output.clone()))
        .expect_err("signed input should fail");
    assert_eq!(error.code(), "signed_input_rejected");

    let mut request = request(input.clone(), output.clone());
    request.authorization.allow_signed_input = true;
    let error =
        apply::apply(&request).expect_err("missing invalidation acknowledgement should fail");
    assert_eq!(error.code(), "signature_invalidation_not_acknowledged");

    request.authorization.acknowledge_signature_invalidation = true;
    let report = apply::apply(&request).expect("both acknowledgements should permit invalidation");
    assert!(report.signature.input_certificate_table_present);
    assert!(report.signature.signature_invalidated_by_edit);
}

#[test]
fn rejects_empty_mutations_before_signature_authorization_or_writing() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("signed.exe");
    let output = directory.path().join("output.exe");
    let mut bytes = fs::read(fixture("pe32_unsigned.exe")).unwrap();
    add_certificate_table_marker(&mut bytes);
    fs::write(&input, bytes).unwrap();

    let error = apply::apply(&apply::ApplyRequest {
        input: input.clone(),
        output: output.clone(),
        version: None,
        icon: None,
        policy: Policy::default(),
        authorization: ExecutionAuthorization::default(),
    })
    .expect_err("an empty mutation must be rejected");

    assert_eq!(error.code(), "no_mutation_requested");
    assert!(!output.exists());
}

#[test]
fn preserves_input_when_icon_decode_or_write_fails() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("bad.png");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::write(&icon, b"not-an-image").unwrap();
    let before = fs::read(&input).unwrap();
    let mut request = request(input.clone(), output.clone());
    request.icon = Some(IconConfig {
        source: icon,
        fit: IconFit::Contain,
        allow_crop: false,
        background: "transparent".to_owned(),
        target_sizes: vec![16, 32],
    });

    let error = apply::apply(&request).expect_err("bad icon should fail before writing");

    assert!(matches!(error, CoreError::IconInvalid));
    assert_eq!(fs::read(input).unwrap(), before);
    assert!(!output.exists());
}

#[test]
fn preserves_existing_output_when_overwrite_pipeline_fails_before_persist() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("bad.png");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::write(&output, b"existing-output").unwrap();
    fs::write(&icon, b"not-an-image").unwrap();
    let mut request = request(input, output.clone());
    request.policy.overwrite_output = true;
    request.icon = Some(IconConfig {
        source: icon,
        fit: IconFit::Contain,
        allow_crop: false,
        background: "transparent".to_owned(),
        target_sizes: vec![16, 32],
    });

    assert!(matches!(
        apply::apply(&request),
        Err(CoreError::IconInvalid)
    ));
    assert_eq!(fs::read(output).unwrap(), b"existing-output");
}

#[test]
fn replaces_main_icon_while_preserving_unrelated_resources() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("icon.png");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    write_icon(&icon);
    let mut request = request(input, output.clone());
    request.version = None;
    request.icon = Some(IconConfig {
        source: icon,
        fit: IconFit::Contain,
        allow_crop: false,
        background: "transparent".to_owned(),
        target_sizes: vec![16, 32, 64],
    });

    apply::apply(&request).expect("icon replacement should succeed");

    let image = Image::parse(fs::read(output).unwrap()).unwrap();
    let resources = image.resource_directory().unwrap();
    assert!(resources.get_main_icon().unwrap().is_some());
    assert!(
        resources
            .root()
            .get(editpe::ResourceEntryName::ID(10))
            .is_some()
    );
}

#[test]
fn rejects_an_output_missing_a_non_primary_icon_frame() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("icon.png");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    write_icon(&icon);
    let mut request = request(input, output.clone());
    request.version = None;
    request.icon = Some(IconConfig {
        source: icon,
        fit: IconFit::Contain,
        allow_crop: false,
        background: "transparent".to_owned(),
        target_sizes: vec![16, 32, 64],
    });

    apply::apply(&request).expect("icon replacement should succeed");

    let output_bytes = fs::read(output).unwrap();
    let mut image = Image::parse(&output_bytes).unwrap();
    let mut resources = image.resource_directory().unwrap().clone();
    let second_icon_id = {
        let group_table = resources
            .root()
            .get(ResourceEntryName::ID(u32::from(RT_GROUP_ICON)))
            .and_then(|entry| entry.as_table())
            .unwrap();
        let main_icon_table = group_table
            .get(ResourceEntryName::from_string("MAINICON"))
            .and_then(|entry| entry.as_table())
            .unwrap();
        let group_data = main_icon_table
            .get(ResourceEntryName::ID(u32::from(LANGUAGE_ID_EN_US)))
            .and_then(|entry| entry.as_data())
            .unwrap()
            .data();

        u16::from_le_bytes(group_data[32..34].try_into().unwrap())
    };
    let icon_table = resources
        .root_mut()
        .get_mut(ResourceEntryName::ID(u32::from(RT_ICON)))
        .and_then(|entry| entry.as_table_mut())
        .unwrap();
    assert!(
        icon_table
            .remove(ResourceEntryName::ID(u32::from(second_icon_id)))
            .is_some()
    );
    image.set_resource_directory(resources).unwrap();
    let corrupted_bytes = image.data().to_vec();
    let corrupted = Image::parse(&corrupted_bytes).unwrap();

    assert!(matches!(
        verify_requested_resources(&corrupted, None, request.icon.as_ref()),
        Err(CoreError::VerificationFailed)
    ));
}

#[test]
fn rejects_an_output_with_a_corrupted_non_primary_icon_frame() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("icon.png");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    write_icon(&icon);
    let mut request = request(input, output.clone());
    request.version = None;
    request.icon = Some(IconConfig {
        source: icon,
        fit: IconFit::Contain,
        allow_crop: false,
        background: "transparent".to_owned(),
        target_sizes: vec![16, 32, 64],
    });

    apply::apply(&request).expect("icon replacement should succeed");

    let output_bytes = fs::read(output).unwrap();
    let mut image = Image::parse(&output_bytes).unwrap();
    let mut resources = image.resource_directory().unwrap().clone();
    let second_icon_id = {
        let group_table = resources
            .root()
            .get(ResourceEntryName::ID(u32::from(RT_GROUP_ICON)))
            .and_then(|entry| entry.as_table())
            .unwrap();
        let main_icon_table = group_table
            .get(ResourceEntryName::from_string("MAINICON"))
            .and_then(|entry| entry.as_table())
            .unwrap();
        let group_data = main_icon_table
            .get(ResourceEntryName::ID(u32::from(LANGUAGE_ID_EN_US)))
            .and_then(|entry| entry.as_data())
            .unwrap()
            .data();

        u16::from_le_bytes(group_data[32..34].try_into().unwrap())
    };
    let payload = resources
        .root_mut()
        .get_mut(ResourceEntryName::ID(u32::from(RT_ICON)))
        .and_then(|entry| entry.as_table_mut())
        .and_then(|table| table.get_mut(ResourceEntryName::ID(u32::from(second_icon_id))))
        .and_then(|entry| entry.as_table_mut())
        .and_then(|table| table.get_mut(ResourceEntryName::ID(u32::from(LANGUAGE_ID_EN_US))))
        .and_then(|entry| entry.as_data_mut())
        .unwrap();
    let mut corrupted_payload = payload.data().to_vec();
    corrupted_payload[0] ^= 0xff;
    payload.set_data(corrupted_payload);
    image.set_resource_directory(resources).unwrap();
    let corrupted_bytes = image.data().to_vec();
    let corrupted = Image::parse(&corrupted_bytes).unwrap();

    assert!(matches!(
        verify_requested_resources(&corrupted, None, request.icon.as_ref()),
        Err(CoreError::VerificationFailed)
    ));
}

#[test]
fn keeps_the_input_unchanged_for_a_failed_resource_preparation() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    let before = fs::read(&input).unwrap();
    let mut image = Image::parse(before.clone()).unwrap();
    let resources = prepare_version_resources(&image, &version(), true).unwrap();
    image.set_resource_directory(resources).unwrap();
    assert!(!output.exists());
    assert_eq!(fs::read(&input).unwrap(), before);
}

fn add_certificate_table_marker(bytes: &mut Vec<u8>) {
    let pe_offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    let optional_header = pe_offset + 4 + 20;
    let magic = u16::from_le_bytes(
        bytes[optional_header..optional_header + 2]
            .try_into()
            .unwrap(),
    );
    let directories = optional_header
        + match magic {
            0x10b => 96,
            0x20b => 112,
            _ => panic!("unexpected PE magic"),
        };
    let certificate_entry = directories + (4 * 8);
    let certificate_offset = bytes.len() as u32;
    bytes.extend_from_slice(&[8, 0, 0, 0, 0, 2, 2, 0]);
    bytes[certificate_entry..certificate_entry + 4]
        .copy_from_slice(&certificate_offset.to_le_bytes());
    bytes[certificate_entry + 4..certificate_entry + 8].copy_from_slice(&8u32.to_le_bytes());
}
