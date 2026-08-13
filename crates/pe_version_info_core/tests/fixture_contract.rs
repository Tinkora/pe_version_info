use editpe::Image;
use editpe::constants::{PE_32_MAGIC, PE_64_MAGIC};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

#[test]
fn parses_committed_pe32_fixture() {
    let image = Image::parse_file(fixture("pe32_unsigned.exe"))
        .expect("the committed PE32 fixture should parse");

    assert_eq!(image.standard_header().magic, PE_32_MAGIC);
}

#[test]
fn parses_committed_pe64_fixture() {
    let image = Image::parse_file(fixture("pe64_unsigned.exe"))
        .expect("the committed PE32+ fixture should parse");

    assert_eq!(image.standard_header().magic, PE_64_MAGIC);
}

#[test]
fn exposes_schema_version_one() {
    assert_eq!(pe_version_info_core::SCHEMA_VERSION, 1);
}
