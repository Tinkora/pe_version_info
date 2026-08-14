use pe_version_info_core::VersionNumber;
use pe_version_info_core::config::{ExecutionAuthorization, IconFit, load_config};
use pe_version_info_core::error::CoreError;
use std::fs;
use std::str::FromStr;
use tempfile::tempdir;

fn write_config(directory: &std::path::Path, body: &str) -> std::path::PathBuf {
    let path = directory.join("pevi.toml");
    fs::write(&path, body).expect("test config should be written");
    path
}

fn error_code<T: std::fmt::Debug>(result: Result<T, CoreError>) -> &'static str {
    result.expect_err("operation should fail").code()
}

#[test]
fn resolves_relative_paths_from_the_config_directory() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("input.exe"), b"fixture")
        .expect("test input should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "input.exe"
output = "output.exe"
"#,
    );

    let config = load_config(&config_path, &ExecutionAuthorization::default())
        .expect("valid paths should resolve");

    assert_eq!(config.input, directory.path().join("input.exe"));
    assert_eq!(config.output, directory.path().join("output.exe"));
}

#[test]
fn rejects_same_input_and_output_without_in_place_confirmation() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "app.exe"
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "input_output_same"
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_alias_of_the_input_without_in_place_confirmation() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().expect("temporary directory should be created");
    let input = directory.path().join("app.exe");
    fs::write(&input, b"fixture").expect("test input should be written");
    symlink(&input, directory.path().join("alias.exe")).expect("test symlink should be created");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "alias.exe"
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "input_output_same"
    );
}

#[cfg(unix)]
#[test]
fn rejects_hard_link_alias_of_the_input_without_in_place_confirmation() {
    let directory = tempdir().expect("temporary directory should be created");
    let input = directory.path().join("app.exe");
    fs::write(&input, b"fixture").expect("test input should be written");
    fs::hard_link(&input, directory.path().join("alias.exe"))
        .expect("test hard link should be created");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "alias.exe"
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "input_output_same"
    );
}

#[test]
fn permits_same_path_only_with_both_in_place_flags() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "app.exe"
"#,
    );
    let authorization = ExecutionAuthorization {
        in_place: true,
        confirm_in_place: true,
        ..ExecutionAuthorization::default()
    };

    load_config(&config_path, &authorization)
        .expect("both explicit in-place flags should authorize the same path");
}

#[test]
fn rejects_version_components_above_u16() {
    assert_eq!(
        error_code(VersionNumber::from_str("1.2.3.65536")),
        "config_invalid"
    );
}

#[test]
fn normalizes_three_component_versions() {
    assert_eq!(
        VersionNumber::from_str("1.2.3")
            .expect("three components should be valid")
            .components(),
        [1, 2, 3, 0]
    );
}

#[test]
fn rejects_unknown_language_in_the_first_candidate() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "output.exe"

[version]
file_version = "1.2.3.4"
product_version = "1.2.3.4"
language = "zh-CN"
code_page = 1200
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "config_invalid"
    );
}

#[test]
fn rejects_cover_fit_without_explicit_crop_permission() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    fs::write(directory.path().join("icon.png"), b"fixture").expect("test icon should be written");
    let config_path = write_config(
        directory.path(),
        r##"
schema_version = 1
input = "app.exe"
output = "output.exe"

[icon]
source = "icon.png"
fit = "cover"
allow_crop = false
background = "#00000000"
target_sizes = [16, 32]
"##,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "icon_crop_not_allowed"
    );
}

#[test]
fn uses_contain_as_the_safe_icon_default() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    fs::write(directory.path().join("icon.png"), b"fixture").expect("test icon should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "output.exe"

[icon]
source = "icon.png"
"#,
    );

    let config = load_config(&config_path, &ExecutionAuthorization::default())
        .expect("safe icon defaults should be valid");

    assert_eq!(
        config.icon.expect("icon config should exist").fit,
        IconFit::Contain
    );
}

#[test]
fn rejects_unknown_configuration_fields() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "output.exe"
invented = true
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "config_invalid"
    );
}

#[test]
fn rejects_unsupported_schema_versions() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 2
input = "app.exe"
output = "output.exe"
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "config_version_unsupported"
    );
}

#[test]
fn rejects_reserved_version_string_keys_case_insensitively() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    for key in [
        "FileVersion",
        "fileversion",
        "ProductVersion",
        "productversion",
    ] {
        let config_path = write_config(
            directory.path(),
            &format!(
                "schema_version = 1\ninput = \"app.exe\"\noutput = \"output.exe\"\n\n[version]\nfile_version = \"1.2.3.4\"\nproduct_version = \"1.2.3.4\"\n[version.strings]\n{key} = \"spoofed\"\n"
            ),
        );
        assert_eq!(
            error_code(load_config(
                &config_path,
                &ExecutionAuthorization::default()
            )),
            "config_invalid",
            "reserved key {key} must be rejected"
        );
    }
}

#[test]
fn rejects_more_than_sixteen_icon_target_sizes() {
    let directory = tempdir().expect("temporary directory should be created");
    fs::write(directory.path().join("app.exe"), b"fixture").expect("test input should be written");
    fs::write(directory.path().join("icon.png"), b"fixture").expect("test icon should be written");
    let config_path = write_config(
        directory.path(),
        r#"
schema_version = 1
input = "app.exe"
output = "output.exe"

[icon]
source = "icon.png"
target_sizes = [16, 24, 32, 40, 48, 56, 64, 72, 80, 96, 112, 128, 144, 160, 192, 224, 256]
"#,
    );

    assert_eq!(
        error_code(load_config(
            &config_path,
            &ExecutionAuthorization::default()
        )),
        "config_invalid"
    );
}
