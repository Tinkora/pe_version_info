use crate::error::CoreError;
use crate::{SCHEMA_VERSION, VersionNumber};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const DEFAULT_ICON_SIZES: &[u16] = &[16, 24, 32, 48, 64, 128, 256];
const MAX_ICON_TARGET_SIZES: usize = 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExecutionAuthorization {
    pub in_place: bool,
    pub confirm_in_place: bool,
    pub allow_signed_input: bool,
    pub acknowledge_signature_invalidation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub schema_version: u32,
    pub input: PathBuf,
    pub output: PathBuf,
    pub policy: Policy,
    pub version: Option<VersionConfig>,
    pub icon: Option<IconConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    pub input: PathBuf,
    pub output: PathBuf,
    #[serde(default)]
    pub policy: Policy,
    #[serde(default)]
    pub version: Option<VersionConfig>,
    #[serde(default)]
    pub icon: Option<IconConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub overwrite_output: bool,
    pub preserve_unknown_strings: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            overwrite_output: false,
            preserve_unknown_strings: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionConfig {
    pub file_version: VersionNumber,
    pub product_version: VersionNumber,
    #[serde(default = "default_language")]
    #[schemars(extend("const" = "en-US"))]
    pub language: String,
    #[serde(default = "default_code_page")]
    #[schemars(extend("const" = 1200))]
    pub code_page: u16,
    #[serde(default)]
    pub strings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IconFit {
    Contain,
    Cover,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IconConfig {
    pub source: PathBuf,
    #[serde(default = "default_icon_fit")]
    pub fit: IconFit,
    #[serde(default)]
    pub allow_crop: bool,
    #[serde(default = "default_background")]
    #[schemars(extend("pattern" = r"^(transparent|#[0-9A-Fa-f]{8})$"))]
    pub background: String,
    #[serde(default = "default_target_sizes")]
    #[schemars(
        length(min = 1, max = 16),
        inner(range(min = 16, max = 256)),
        extend("uniqueItems" = true)
    )]
    pub target_sizes: Vec<u16>,
}

pub fn load_config(
    path: &Path,
    authorization: &ExecutionAuthorization,
) -> Result<Config, CoreError> {
    load_config_with_output(path, authorization, None)
}

pub fn load_config_with_output(
    path: &Path,
    authorization: &ExecutionAuthorization,
    output_override: Option<&Path>,
) -> Result<Config, CoreError> {
    let metadata = fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => CoreError::PathNotFound(path.to_path_buf()),
        _ => CoreError::ConfigInvalid,
    })?;
    if !metadata.is_file() {
        return Err(CoreError::PathNotRegularFile(path.to_path_buf()));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(CoreError::ConfigInvalid);
    }

    let bytes = fs::read(path).map_err(|_| CoreError::ConfigInvalid)?;
    let source = std::str::from_utf8(&bytes).map_err(|_| CoreError::ConfigInvalid)?;
    let parsed: ConfigFile = toml::from_str(source).map_err(|_| CoreError::ConfigInvalid)?;
    if parsed.schema_version != SCHEMA_VERSION {
        return Err(CoreError::ConfigVersionUnsupported);
    }

    let base = absolute_parent(path)?;
    let input = resolve_path(&base, &parsed.input);
    validate_regular_file(&input)?;
    validate_pe_extension(&input)?;
    let mut output = output_override
        .map(|path| resolve_path(&base, path))
        .unwrap_or_else(|| resolve_path(&base, &parsed.output));
    if paths_identify_same_file(&input, &output)? {
        if !(authorization.in_place && authorization.confirm_in_place) {
            return Err(CoreError::InputOutputSame);
        }
        output = fs::canonicalize(&input).map_err(|_| CoreError::ConfigInvalid)?;
    }

    if let Some(version) = &parsed.version {
        validate_version(version)?;
    }
    let icon = parsed
        .icon
        .map(|mut icon| {
            icon.source = resolve_path(&base, &icon.source);
            validate_icon(&icon)?;
            Ok(icon)
        })
        .transpose()?;

    Ok(Config {
        schema_version: parsed.schema_version,
        input,
        output,
        policy: parsed.policy,
        version: parsed.version,
        icon,
    })
}

fn absolute_parent(path: &Path) -> Result<PathBuf, CoreError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| CoreError::ConfigInvalid)?
            .join(path)
    };
    absolute
        .parent()
        .map(normalize_path)
        .ok_or(CoreError::ConfigInvalid)
}

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&base.join(path))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn validate_regular_file(path: &Path) -> Result<(), CoreError> {
    let metadata = fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => CoreError::PathNotFound(path.to_path_buf()),
        _ => CoreError::PathNotRegularFile(path.to_path_buf()),
    })?;
    if !metadata.is_file() {
        return Err(CoreError::PathNotRegularFile(path.to_path_buf()));
    }
    Ok(())
}

fn paths_identify_same_file(input: &Path, output: &Path) -> Result<bool, CoreError> {
    if input == output {
        return Ok(true);
    }
    match fs::metadata(output) {
        Ok(_) => same_file::is_same_file(input, output).map_err(|_| CoreError::ConfigInvalid),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(CoreError::ConfigInvalid),
    }
}

fn validate_pe_extension(path: &Path) -> Result<(), CoreError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if matches!(extension.as_deref(), Some("exe" | "dll")) {
        Ok(())
    } else {
        Err(CoreError::UnsupportedInputExtension)
    }
}

fn validate_version(version: &VersionConfig) -> Result<(), CoreError> {
    if version.language != "en-US" || version.code_page != 1200 {
        return Err(CoreError::ConfigInvalid);
    }
    if version.strings.iter().any(|(key, value)| {
        key.is_empty()
            || matches!(
                key.to_ascii_lowercase().as_str(),
                "fileversion" | "productversion"
            )
            || value.encode_utf16().count() >= u16::MAX as usize
    }) {
        return Err(CoreError::ConfigInvalid);
    }
    Ok(())
}

fn validate_icon(icon: &IconConfig) -> Result<(), CoreError> {
    validate_regular_file(&icon.source)?;
    let extension = icon
        .source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("png" | "jpg" | "jpeg" | "ico")) {
        return Err(CoreError::UnsupportedInputExtension);
    }
    if icon.fit == IconFit::Cover && !icon.allow_crop {
        return Err(CoreError::IconCropNotAllowed);
    }
    if icon.target_sizes.is_empty()
        || icon.target_sizes.len() > MAX_ICON_TARGET_SIZES
        || icon.target_sizes.windows(2).any(|pair| pair[0] >= pair[1])
        || icon
            .target_sizes
            .iter()
            .any(|size| !(16..=256).contains(size))
    {
        return Err(CoreError::ConfigInvalid);
    }
    if icon.background != "transparent"
        && !(icon.background.len() == 9
            && icon.background.starts_with('#')
            && icon.background[1..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(CoreError::ConfigInvalid);
    }
    Ok(())
}

fn default_language() -> String {
    "en-US".to_owned()
}

const fn default_code_page() -> u16 {
    1200
}

const fn default_icon_fit() -> IconFit {
    IconFit::Contain
}

fn default_background() -> String {
    "transparent".to_owned()
}

fn default_target_sizes() -> Vec<u16> {
    DEFAULT_ICON_SIZES.to_vec()
}
