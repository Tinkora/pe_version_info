use crate::error::CoreError;
use editpe::types::VersionU32;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionNumber([u16; 4]);

impl VersionNumber {
    pub const fn components(self) -> [u16; 4] {
        self.0
    }

    pub(crate) const fn from_editpe(value: VersionU32) -> Self {
        Self([
            (value.major >> 16) as u16,
            value.major as u16,
            (value.minor >> 16) as u16,
            value.minor as u16,
        ])
    }

    pub(crate) const fn to_editpe(self) -> VersionU32 {
        VersionU32 {
            major: ((self.0[0] as u32) << 16) | self.0[1] as u32,
            minor: ((self.0[2] as u32) << 16) | self.0[3] as u32,
        }
    }
}

impl FromStr for VersionNumber {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let components = value.split('.').collect::<Vec<_>>();
        if !(3..=4).contains(&components.len()) {
            return Err(CoreError::ConfigInvalid);
        }

        let mut parsed = [0u16; 4];
        for (index, component) in components.into_iter().enumerate() {
            if component.is_empty() || !component.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(CoreError::ConfigInvalid);
            }
            parsed[index] = component
                .parse::<u16>()
                .map_err(|_| CoreError::ConfigInvalid)?;
        }
        Ok(Self(parsed))
    }
}

impl fmt::Display for VersionNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}.{}.{}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl Serialize for VersionNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for VersionNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for VersionNumber {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("VersionNumber")
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <String>::json_schema(generator)
    }
}
