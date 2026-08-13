use editpe::types::{VersionU16, VersionU32};
use editpe::{Image, ResourceEntryName, VersionInfo, VersionStringTable};
use pe_version_info_core::VersionNumber;
use pe_version_info_core::config::VersionConfig;
use pe_version_info_core::error::CoreError;
use pe_version_info_core::version_info::{merge_version_info, prepare_version_resources};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn requested(strings: &[(&str, &str)]) -> VersionConfig {
    VersionConfig {
        file_version: VersionNumber::from_str("2.3.4.5").unwrap(),
        product_version: VersionNumber::from_str("6.7.8.9").unwrap(),
        language: "en-US".to_owned(),
        code_page: 1200,
        strings: strings
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>(),
    }
}

fn existing_version() -> VersionInfo {
    let mut target = VersionStringTable {
        key: "040904B0".to_owned(),
        ..VersionStringTable::default()
    };
    target
        .strings
        .insert("CompanyName".to_owned(), "Old company".to_owned());
    target
        .strings
        .insert("UnknownField".to_owned(), "preserve me".to_owned());
    let mut other = VersionStringTable {
        key: "041104B0".to_owned(),
        ..VersionStringTable::default()
    };
    other
        .strings
        .insert("ProductName".to_owned(), "Other locale".to_owned());

    VersionInfo {
        info: editpe::types::FixedFileInfo {
            file_version: VersionU32 {
                major: 0x0001_0002,
                minor: 0x0003_0004,
            },
            product_version: VersionU32 {
                major: 0x0005_0006,
                minor: 0x0007_0008,
            },
            file_flags: 0x22,
            file_type: 0x02,
            ..editpe::types::FixedFileInfo::default()
        },
        strings: vec![target, other],
        vars: vec![VersionU16 {
            major: 0x0411,
            minor: 1200,
        }],
    }
}

#[test]
fn creates_canonical_version_information_when_missing() {
    let merged = merge_version_info(None, &requested(&[("ProductName", "New product")]), true)
        .expect("valid requested version information should merge");

    assert_eq!(merged.info.file_version.major, 0x0002_0003);
    assert_eq!(merged.info.file_version.minor, 0x0004_0005);
    assert_eq!(merged.info.product_version.major, 0x0006_0007);
    assert_eq!(merged.info.product_version.minor, 0x0008_0009);
    assert_eq!(merged.strings.len(), 1);
    assert_eq!(merged.strings[0].key, "040904B0");
    assert_eq!(merged.strings[0].strings["FileVersion"], "2.3.4.5");
    assert_eq!(merged.strings[0].strings["ProductVersion"], "6.7.8.9");
    assert_eq!(merged.strings[0].strings["ProductName"], "New product");
    assert_eq!(
        merged.vars,
        vec![VersionU16 {
            major: 0x0409,
            minor: 1200
        }]
    );
}

#[test]
fn updates_requested_fields_and_preserves_unknown_strings_and_fixed_metadata() {
    let existing = existing_version();
    let merged = merge_version_info(
        Some(&existing),
        &requested(&[("CompanyName", "New company")]),
        true,
    )
    .expect("valid requested version information should merge");

    assert_eq!(merged.info.file_flags, 0x22);
    assert_eq!(merged.info.file_type, 0x02);
    assert_eq!(merged.strings[0].strings["CompanyName"], "New company");
    assert_eq!(merged.strings[0].strings["UnknownField"], "preserve me");
    assert_eq!(merged.strings[1], existing.strings[1]);
    assert!(merged.vars.contains(&VersionU16 {
        major: 0x0411,
        minor: 1200,
    }));
    assert!(merged.vars.contains(&VersionU16 {
        major: 0x0409,
        minor: 1200,
    }));
}

#[test]
fn replaces_only_the_selected_string_table_when_preservation_is_disabled() {
    let existing = existing_version();
    let merged = merge_version_info(
        Some(&existing),
        &requested(&[("CompanyName", "New company")]),
        false,
    )
    .expect("valid requested version information should merge");

    assert!(!merged.strings[0].strings.contains_key("UnknownField"));
    assert_eq!(merged.strings[0].strings["CompanyName"], "New company");
    assert_eq!(merged.strings[1], existing.strings[1]);
}

#[test]
fn rejects_version_strings_that_exceed_the_binary_format_limit() {
    let result = merge_version_info(
        None,
        &requested(&[("ProductName", &"x".repeat(u16::MAX as usize))]),
        true,
    );

    assert_eq!(
        result.expect_err("oversized value should fail").code(),
        "config_invalid"
    );
}

#[test]
fn prepares_resources_without_mutating_the_input_image_and_round_trips() {
    let bytes = fs::read(fixture("pe32_unsigned.exe")).expect("fixture should be readable");
    let image = Image::parse(bytes.clone()).expect("fixture should parse");
    let before = image.data().to_vec();

    let resources =
        prepare_version_resources(&image, &requested(&[("ProductName", "Round trip")]), true)
            .expect("resource update should be prepared");

    assert_eq!(image.data(), before);
    let mut output = image.clone();
    output
        .set_resource_directory(resources)
        .expect("prepared resources should rebuild");
    let reparsed = Image::parse(output.data()).expect("rebuilt image should parse");
    let version = reparsed
        .resource_directory()
        .expect("rebuilt image should have resources")
        .get_version_info()
        .expect("rebuilt version resource should parse")
        .expect("rebuilt image should have version information");

    assert_eq!(version.strings[0].key, "040904B0");
    assert_eq!(version.strings[0].strings["ProductName"], "Round trip");
    assert_eq!(
        version.strings[0].strings["UnknownFixtureField"],
        "preserve-me"
    );
    assert_eq!(version.vars[0].major, 0x0409);
    assert_eq!(version.vars[0].minor, 1200);
}

#[test]
fn prepares_resources_when_version_information_is_missing() {
    let bytes = fs::read(fixture("pe32_unsigned.exe")).expect("fixture should be readable");
    let mut image = Image::parse(bytes).expect("fixture should parse");
    let mut resources = image
        .resource_directory()
        .cloned()
        .expect("fixture should have resources");
    resources
        .remove_version_info()
        .expect("fixture version information should be removed");
    image
        .set_resource_directory(resources)
        .expect("fixture without version information should rebuild");

    let resources = prepare_version_resources(&image, &requested(&[]), true)
        .expect("missing version information should be created");

    assert!(
        resources
            .get_version_info()
            .expect("created version information should parse")
            .is_some()
    );
}

#[test]
fn rejects_invalid_locale_at_the_merge_boundary() {
    let mut invalid = requested(&[]);
    invalid.language = "zh-CN".to_owned();

    let result = merge_version_info(None, &invalid, true);

    assert!(matches!(result, Err(CoreError::ConfigInvalid)));
}

#[test]
fn refuses_to_replace_a_malformed_existing_version_resource() {
    let bytes = fs::read(fixture("pe32_unsigned.exe")).expect("fixture should be readable");
    let mut image = Image::parse(bytes).expect("fixture should parse");
    let mut resources = image
        .resource_directory()
        .cloned()
        .expect("fixture should have resources");
    resources
        .root_mut()
        .get_mut(ResourceEntryName::ID(16))
        .and_then(|entry| entry.as_table_mut())
        .and_then(|table| table.get_mut(ResourceEntryName::ID(1)))
        .and_then(|entry| entry.as_table_mut())
        .and_then(|table| table.get_mut(ResourceEntryName::ID(0x0409)))
        .and_then(|entry| entry.as_data_mut())
        .expect("fixture version resource should be reachable")
        .set_data(vec![0]);
    image
        .set_resource_directory(resources)
        .expect("malformed fixture should still be serialized");

    let result = prepare_version_resources(&image, &requested(&[]), true);

    assert!(matches!(result, Err(CoreError::VersionInfoMalformed)));
}
