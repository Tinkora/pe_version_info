use editpe::{Image, VersionStringTable};
use pe_version_info_core::error::CoreError;
use pe_version_info_core::inspect::{PeArchitecture, PeKind, inspect};
use pe_version_info_core::signature::SignatureValidationStatus;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn error_code<T: std::fmt::Debug>(result: Result<T, CoreError>) -> &'static str {
    result.expect_err("operation should fail").code()
}

#[test]
fn inspects_pe32_version_information() {
    let inspection =
        inspect(&fixture("pe32_unsigned.exe")).expect("the committed PE32 fixture should inspect");

    assert_eq!(inspection.schema_version, 1);
    assert_eq!(inspection.kind, PeKind::Pe32);
    assert_eq!(inspection.architecture, PeArchitecture::X86);
    assert_eq!(inspection.sha256.len(), 64);
    assert!(!inspection.certificate_table_present);
    assert_eq!(
        inspection.signature_validation,
        SignatureValidationStatus::NotChecked
    );
    assert!(inspection.resources.version_info_present);
    assert!(!inspection.resources.main_icon_present);
    let version = inspection
        .version_info
        .expect("fixture should contain VERSIONINFO");
    assert_eq!(version.file_version.components(), [1, 2, 3, 4]);
    assert_eq!(version.product_version.components(), [5, 6, 7, 8]);
    assert_eq!(
        version
            .string_tables
            .first()
            .expect("fixture should have a string table")
            .strings
            .get("UnknownFixtureField")
            .map(String::as_str),
        Some("preserve-me")
    );
    assert_eq!(version.string_tables[0].key, "040904B0");
    assert_eq!(version.string_tables[0].language_id, Some(0x0409));
    assert_eq!(version.string_tables[0].code_page, Some(1200));
    assert_eq!(version.string_tables[0].locale.as_deref(), Some("en-US"));
}

#[test]
fn inspects_pe32_plus() {
    let inspection =
        inspect(&fixture("pe64_unsigned.exe")).expect("the committed PE32+ fixture should inspect");

    assert_eq!(inspection.kind, PeKind::Pe32Plus);
    assert_eq!(inspection.architecture, PeArchitecture::X86_64);
}

#[test]
fn preserves_each_version_string_table() {
    let directory = tempdir().expect("temporary directory should be created");
    let path = directory.path().join("multiple-tables.exe");
    let bytes = fs::read(fixture("pe32_unsigned.exe")).expect("base fixture should be readable");
    let mut image = Image::parse(bytes).expect("base fixture should parse");
    let mut resources = image
        .resource_directory()
        .cloned()
        .expect("base fixture should have resources");
    let mut version = resources
        .get_version_info()
        .expect("version resource should be readable")
        .expect("base fixture should have version information");
    let mut extra = VersionStringTable {
        key: "041104B0".to_owned(),
        ..VersionStringTable::default()
    };
    extra
        .strings
        .insert("ProductName".to_owned(), "Japanese table".to_owned());
    version.strings.push(extra);
    resources
        .set_version_info(&version)
        .expect("version information should be updated");
    image
        .set_resource_directory(resources)
        .expect("resource directory should be rebuilt");
    image
        .write_file(&path)
        .expect("derived fixture should be written");

    let inspection = inspect(&path).expect("derived PE should inspect");
    let tables = inspection
        .version_info
        .expect("derived fixture should have version information")
        .string_tables;

    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].key, "040904B0");
    assert_eq!(tables[1].key, "041104B0");
    assert_eq!(tables[1].language_id, Some(0x0411));
    assert_eq!(tables[1].code_page, Some(1200));
    assert_eq!(tables[1].locale, None);
}

#[test]
fn rejects_non_pe_input_without_panicking() {
    let directory = tempdir().expect("temporary directory should be created");
    let path = directory.path().join("invalid.exe");
    fs::write(&path, b"not a PE file").expect("invalid fixture should be written");

    assert_eq!(error_code(inspect(&path)), "invalid_pe");
}

#[test]
fn rejects_inputs_above_the_documented_limit() {
    let directory = tempdir().expect("temporary directory should be created");
    let path = directory.path().join("too-large.exe");
    let file = fs::File::create(&path).expect("large fixture should be created");
    file.set_len(pe_version_info_core::inspect::MAX_PE_BYTES + 1)
        .expect("sparse fixture should be resized");

    assert_eq!(error_code(inspect(&path)), "input_too_large");
}

#[test]
fn reports_certificate_table_presence_without_claiming_validation() {
    let directory = tempdir().expect("temporary directory should be created");
    let path = directory.path().join("certificate-table.exe");
    let mut bytes =
        fs::read(fixture("pe32_unsigned.exe")).expect("base fixture should be readable");
    add_certificate_table_marker(&mut bytes);
    fs::write(&path, bytes).expect("derived fixture should be written");

    let inspection = inspect(&path).expect("derived PE should inspect");

    assert!(inspection.certificate_table_present);
    assert_eq!(
        inspection.signature_validation,
        SignatureValidationStatus::NotChecked
    );
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
            other => panic!("unexpected optional header magic: {other:#x}"),
        };
    let certificate_entry = directories + (4 * 8);
    let certificate_offset = bytes.len() as u32;
    bytes.extend_from_slice(&[8, 0, 0, 0, 0, 2, 2, 0]);
    bytes[certificate_entry..certificate_entry + 4]
        .copy_from_slice(&certificate_offset.to_le_bytes());
    bytes[certificate_entry + 4..certificate_entry + 8].copy_from_slice(&8u32.to_le_bytes());
}
