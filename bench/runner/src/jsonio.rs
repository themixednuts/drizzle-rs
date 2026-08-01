//! JSON read/write helpers shared by every subcommand.
//!
//! All of them carry the caller's [`Code`] so a failure surfaces with the exit
//! code the step is supposed to report, rather than a generic one.

use crate::code::{Code, Fail};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

pub fn read<T: DeserializeOwned>(path: &Path, code: Code) -> Result<T, Fail> {
    let body = fs::read_to_string(path)
        .map_err(|err| Fail::new(code, format!("failed to read {}: {err}", path.display())))?;
    serde_json::from_str(&body)
        .map_err(|err| Fail::new(code, format!("invalid json {}: {err}", path.display())))
}

/// Write pretty-printed JSON, creating the parent directory if needed.
pub fn write(path: impl AsRef<Path>, value: &impl Serialize, code: Code) -> Result<(), Fail> {
    let path = path.as_ref();
    let body = serde_json::to_string_pretty(value)
        .map_err(|err| Fail::new(code, format!("serialize {} failed: {err}", path.display())))?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            Fail::new(
                code,
                format!("failed to create {}: {err}", parent.display()),
            )
        })?;
    }
    fs::write(path, body)
        .map_err(|err| Fail::new(code, format!("write {} failed: {err}", path.display())))
}
