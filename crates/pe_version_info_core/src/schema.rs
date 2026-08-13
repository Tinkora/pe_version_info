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
    schema_for!(ConfigFile)
}

pub fn report_schema() -> Schema {
    let _ = schema_for!(PeInspection);
    schema_for!(ReportEnvelope)
}
