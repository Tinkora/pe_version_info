use crate::config::ConfigFile;
use crate::error::ErrorReport;
use crate::inspect::PeInspection;
use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReportEnvelope {
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    pub ok: bool,
    pub command: String,
    pub data: Value,
    pub warnings: Vec<String>,
    pub errors: Vec<ErrorReport>,
}

pub fn config_schema() -> Schema {
    let mut schema = schema_for!(ConfigFile);
    let root = schema
        .as_object_mut()
        .expect("config schema should be an object");
    let defs = root
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
        .expect("config schema should contain definitions");
    let icon = defs
        .get_mut("IconConfig")
        .and_then(Value::as_object_mut)
        .expect("config schema should define IconConfig");
    icon.insert(
        "allOf".to_owned(),
        serde_json::json!([
            {
                "if": {
                    "required": ["fit"],
                    "properties": {"fit": {"const": "cover"}}
                },
                "then": {"properties": {"allow_crop": {"const": true}}}
            }
        ]),
    );
    schema
}

pub fn report_schema() -> Schema {
    let _ = schema_for!(PeInspection);
    schema_for!(ReportEnvelope)
}
