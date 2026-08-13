//! Platform-independent behavior for PE Version Info.

pub mod apply;
pub mod config;
pub mod error;
pub mod icon;
pub mod inspect;
pub mod schema;
pub mod verify;
mod version;
pub mod version_info;

pub use version::VersionNumber;

/// Version of the public configuration and report contracts.
pub const SCHEMA_VERSION: u32 = 1;
