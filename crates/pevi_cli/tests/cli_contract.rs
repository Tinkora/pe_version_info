use assert_cmd::Command;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use pe_version_info_core::config::ConfigFile;
use serde_json::Value;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn write_png(path: &Path) {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(32, 32, Rgba([255, 0, 0, 255])));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    fs::write(path, bytes.into_inner()).unwrap();
}

fn write_colored_png(path: &Path, color: [u8; 4]) {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(32, 32, Rgba(color)));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    fs::write(path, bytes.into_inner()).unwrap();
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn json_command(args: &[&str]) -> (assert_cmd::assert::Assert, Value) {
    let assert = Command::cargo_bin("pevi")
        .expect("pevi binary should build")
        .args(args)
        .assert();
    let output = assert.get_output();
    let value = serde_json::from_slice(&output.stdout).expect("stdout should be one JSON object");
    (assert, value)
}

#[test]
fn init_writes_a_template_and_refuses_overwrite_without_force() {
    let directory = tempdir().unwrap();
    let config = directory.path().join("pevi.toml");

    Command::cargo_bin("pevi")
        .unwrap()
        .args(["init", "--output", config.to_str().unwrap()])
        .assert()
        .success();
    let first = fs::read_to_string(&config).unwrap();
    assert!(first.contains("schema_version = 1"));
    assert!(first.contains("# [version]"));
    assert!(first.contains("# [icon]"));
    let parsed: ConfigFile = toml::from_str(&first).unwrap();
    assert!(parsed.version.is_none());
    assert!(parsed.icon.is_none());

    Command::cargo_bin("pevi")
        .unwrap()
        .args(["init", "--output", config.to_str().unwrap()])
        .assert()
        .failure();
    assert_eq!(fs::read_to_string(config).unwrap(), first);
}

#[test]
fn inspect_json_is_read_only_and_contains_stable_data() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("fixture.exe");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();

    let (assert, value) = json_command(
        [
            "inspect",
            "--input",
            input.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "inspect");
    assert_eq!(value["data"]["kind"], "pe32");
    assert!(value["errors"].as_array().unwrap().is_empty());
    assert!(!directory.path().join("output.exe").exists());
}

#[test]
fn plan_json_never_writes_and_rejects_invalid_config_with_stable_error() {
    let directory = tempdir().unwrap();
    let config = directory.path().join("pevi.toml");
    fs::write(
        &config,
        "schema_version = 1\ninput = \"missing.exe\"\noutput = \"output.exe\"\n",
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "plan",
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "plan");
    assert_eq!(value["errors"][0]["code"], "path_not_found");
    assert!(!directory.path().join("output.exe").exists());
}

#[test]
fn plan_json_reports_icon_policy_and_signature_consequences() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("icon.png");
    let config = directory.path().join("pevi.toml");
    let mut bytes = fs::read(fixture("pe32_unsigned.exe")).unwrap();
    add_certificate_table_marker(&mut bytes);
    fs::write(&input, bytes).unwrap();
    write_png(&icon);
    fs::write(
        &config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[icon]\nsource = \"{}\"\nfit = \"contain\"\nbackground = \"#112233ff\"\ntarget_sizes = [16, 32]\n",
            toml_path(&input),
            toml_path(&output),
            toml_path(&icon)
        ),
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "plan",
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(value["data"]["icon"]["fit"], "contain");
    assert_eq!(value["data"]["icon"]["background"], "#112233ff");
    assert_eq!(
        value["data"]["signature"]["input_certificate_table_present"],
        true
    );
    assert_eq!(
        value["data"]["signature"]["edit_invalidates_signature"],
        true
    );
    assert_eq!(value["data"]["signature"]["override_authorized"], false);
    assert!(!output.exists());
}

#[test]
fn apply_and_verify_json_round_trip_on_a_fixture() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let config = directory.path().join("pevi.toml");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::write(
        &config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[version]\nfile_version = \"9.8.7.6\"\nproduct_version = \"5.4.3.2\"\n",
            toml_path(&input),
            toml_path(&output)
        ),
    )
    .unwrap();

    let (apply_assert, apply_value) = json_command(
        [
            "apply",
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );
    apply_assert.success();
    assert_eq!(apply_value["command"], "apply");
    assert_eq!(apply_value["data"]["version"]["file_version"], "9.8.7.6");
    assert_eq!(apply_value["data"]["version"]["product_version"], "5.4.3.2");
    assert_eq!(apply_value["data"]["version"]["language"], "en-US");
    assert_eq!(apply_value["data"]["version"]["code_page"], 1200);
    assert!(output.exists());

    let (verify_assert, verify_value) = json_command(
        [
            "verify",
            "--input",
            output.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );
    verify_assert.success();
    assert_eq!(verify_value["command"], "verify");
    assert_eq!(verify_value["data"]["matches"], true);
}

#[test]
fn apply_json_reports_icon_conversion_details() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("icon.png");
    let config = directory.path().join("pevi.toml");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    write_png(&icon);
    fs::write(
        &config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[icon]\nsource = \"{}\"\ntarget_sizes = [16, 32, 64]\n",
            toml_path(&input),
            toml_path(&output),
            toml_path(&icon)
        ),
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "apply",
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(value["data"]["icon"]["source_format"], "png");
    assert_eq!(value["data"]["icon"]["renderer"], "image");
    assert_eq!(value["data"]["icon"]["fit"], "contain");
    assert_eq!(value["data"]["icon"]["background"], "transparent");
    assert_eq!(
        value["data"]["icon"]["target_sizes"],
        serde_json::json!([16, 32, 64])
    );
    assert_eq!(value["data"]["icon"]["cropped"], false);
}

#[test]
fn signed_apply_json_warns_that_the_signature_was_invalidated() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let config = directory.path().join("pevi.toml");
    let mut bytes = fs::read(fixture("pe32_unsigned.exe")).unwrap();
    add_certificate_table_marker(&mut bytes);
    fs::write(&input, bytes).unwrap();
    fs::write(
        &config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[version]\nfile_version = \"9.8.7.6\"\nproduct_version = \"5.4.3.2\"\n",
            toml_path(&input),
            toml_path(&output)
        ),
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "apply",
            "--config",
            config.to_str().unwrap(),
            "--allow-signed-input",
            "--acknowledge-signature-invalidation",
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(
        value["data"]["signature"]["signature_invalidated_by_edit"],
        true
    );
    assert_eq!(value["warnings"].as_array().unwrap().len(), 1);
    assert!(
        value["warnings"][0]
            .as_str()
            .unwrap()
            .contains("Authenticode")
    );
}

#[test]
fn verify_rejects_a_config_that_requests_an_icon_when_none_is_present() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let icon = directory.path().join("icon.png");
    let config = directory.path().join("pevi.toml");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    write_png(&icon);
    fs::write(
        &config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[icon]\nsource = \"{}\"\n",
            toml_path(&input),
            toml_path(&output),
            toml_path(&icon)
        ),
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "verify",
            "--input",
            input.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.failure();
    assert_eq!(value["errors"][0]["code"], "verification_failed");
}

#[test]
fn verify_rejects_a_config_when_the_requested_icon_does_not_match() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("output.exe");
    let applied_icon = directory.path().join("applied.png");
    let different_icon = directory.path().join("different.png");
    let applied_config = directory.path().join("applied.toml");
    let verify_config = directory.path().join("verify.toml");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    write_colored_png(&applied_icon, [255, 0, 0, 255]);
    write_colored_png(&different_icon, [0, 0, 255, 255]);
    fs::write(
        &applied_config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[icon]\nsource = \"{}\"\n",
            toml_path(&input),
            toml_path(&output),
            toml_path(&applied_icon)
        ),
    )
    .unwrap();
    fs::write(
        &verify_config,
        format!(
            "schema_version = 1\ninput = \"{}\"\noutput = \"{}\"\n\n[icon]\nsource = \"{}\"\n",
            toml_path(&input),
            toml_path(&output),
            toml_path(&different_icon)
        ),
    )
    .unwrap();

    Command::cargo_bin("pevi")
        .unwrap()
        .args([
            "apply",
            "--config",
            applied_config.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();

    Command::cargo_bin("pevi")
        .unwrap()
        .args([
            "verify",
            "--input",
            output.to_str().unwrap(),
            "--config",
            applied_config.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();

    let (assert, value) = json_command(
        [
            "verify",
            "--input",
            output.to_str().unwrap(),
            "--config",
            verify_config.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.failure();
    assert_eq!(value["errors"][0]["code"], "verification_failed");
}

#[test]
fn apply_output_override_allows_a_config_that_names_the_input() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("override.exe");
    let config = directory.path().join("pevi.toml");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::write(
        &config,
        "schema_version = 1\ninput = \"input.exe\"\noutput = \"input.exe\"\n",
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "apply",
            "--config",
            config.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(value["ok"], true);
    assert!(output.is_file());
}

#[test]
fn apply_relative_output_override_is_resolved_from_config_directory() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.exe");
    let output = directory.path().join("nested/override.exe");
    let config = directory.path().join("pevi.toml");
    fs::copy(fixture("pe32_unsigned.exe"), &input).unwrap();
    fs::create_dir(directory.path().join("nested")).unwrap();
    fs::write(
        &config,
        "schema_version = 1\ninput = \"input.exe\"\noutput = \"input.exe\"\n",
    )
    .unwrap();

    let (assert, value) = json_command(
        [
            "apply",
            "--config",
            config.to_str().unwrap(),
            "--output",
            "nested/override.exe",
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(value["ok"], true);
    assert!(output.is_file());
}

#[test]
fn convert_icon_writes_only_the_requested_ico_output() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("source.png");
    let output = directory.path().join("icon.ico");
    write_png(&source);

    let (assert, value) = json_command(
        [
            "convert-icon",
            "--input",
            source.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            "json",
        ]
        .as_slice(),
    );

    assert.success();
    assert_eq!(value["command"], "convert-icon");
    assert!(output.is_file());
}

#[test]
fn convert_icon_never_replaces_an_existing_output() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("source.png");
    let output = directory.path().join("icon.ico");
    write_png(&source);
    fs::write(&output, b"keep-existing").unwrap();

    Command::cargo_bin("pevi")
        .unwrap()
        .args([
            "convert-icon",
            "--input",
            source.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .failure();

    assert_eq!(fs::read(output).unwrap(), b"keep-existing");
}

#[test]
fn help_lists_the_stable_commands() {
    let output = Command::cargo_bin("pevi")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).unwrap();
    assert!(help.contains("inspect"));
    assert!(help.contains("apply"));
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
