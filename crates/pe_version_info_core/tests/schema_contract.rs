use pe_version_info_core::schema::{config_schema, report_schema};
use std::fs;
use std::path::Path;

#[test]
fn public_schemas_are_versioned_and_reject_unknown_fields() {
    for schema in [config_schema(), report_schema()] {
        let value = serde_json::to_value(schema).expect("schema should serialize");
        assert_eq!(
            value.pointer("/properties/schema_version/const"),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            value.get("additionalProperties"),
            Some(&serde_json::json!(false))
        );
    }
}

#[test]
fn config_schema_expresses_runtime_constraints() {
    let value = serde_json::to_value(config_schema()).expect("schema should serialize");
    let version = value.pointer("/$defs/VersionConfig/properties").unwrap();
    assert_eq!(version["language"]["const"], "en-US");
    assert_eq!(version["code_page"]["const"], 1200);
    let version_number = value.pointer("/$defs/VersionNumber").unwrap();
    assert_eq!(
        version_number["pattern"],
        "^[0-9]+\\.[0-9]+\\.[0-9]+(\\.[0-9]+)?$"
    );
    let icon_definition = value.pointer("/$defs/IconConfig").unwrap();
    let icon = &icon_definition["properties"];
    assert_eq!(
        icon["background"]["pattern"],
        "^(transparent|#[0-9A-Fa-f]{8})$"
    );
    assert_eq!(icon["target_sizes"]["minItems"], 1);
    assert_eq!(icon["target_sizes"]["maxItems"], 16);
    assert_eq!(icon["target_sizes"]["uniqueItems"], true);
    assert_eq!(
        icon_definition["allOf"][0]["if"]["properties"]["fit"]["const"],
        "cover"
    );
    assert_eq!(icon_definition["allOf"][0]["if"]["required"][0], "fit");
    assert_eq!(
        icon_definition["allOf"][0]["then"]["properties"]["allow_crop"]["const"],
        true
    );
}

#[test]
fn committed_schemas_match_the_rust_contracts() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (path, schema) in [
        ("schemas/pevi_config_v1.json", config_schema()),
        ("schemas/pevi_report_v1.json", report_schema()),
    ] {
        let expected = format!(
            "{}\n",
            serde_json::to_string_pretty(&schema).expect("schema should serialize")
        );
        let actual = fs::read_to_string(repository.join(path))
            .unwrap_or_else(|error| panic!("{path} should be committed: {error}"));
        let actual = actual.replace("\r\n", "\n");
        assert_eq!(actual, expected, "{path} is stale; regenerate schemas");
    }
}

#[test]
fn stable_errors_serialize_safe_details() {
    let report =
        pe_version_info_core::error::CoreError::PathNotFound("input.exe".into()).to_report();
    let value = serde_json::to_value(report).expect("error report should serialize");

    assert_eq!(value["code"], "path_not_found");
    assert_eq!(value["message"], "path does not exist");
    assert_eq!(value["details"]["path"], "input.exe");
}
