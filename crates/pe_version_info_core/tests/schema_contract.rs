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
