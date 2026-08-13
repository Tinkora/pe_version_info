use pe_version_info_core::schema::{config_schema, report_schema};
use schemars::Schema;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    write_schema(
        repository.join("schemas/pevi_config_v1.json"),
        config_schema(),
    )?;
    write_schema(
        repository.join("schemas/pevi_report_v1.json"),
        report_schema(),
    )?;
    Ok(())
}

fn write_schema(path: PathBuf, schema: Schema) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = format!("{}\n", serde_json::to_string_pretty(&schema)?);
    fs::write(path, json)?;
    Ok(())
}
