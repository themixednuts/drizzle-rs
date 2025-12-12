//! Configuration file handling for drizzle.toml

use drizzle_migrations::DrizzleConfig;
use std::path::Path;

/// Load drizzle.toml configuration
pub fn load_config(path: Option<&str>) -> anyhow::Result<Option<DrizzleConfig>> {
    // Try explicit path first
    if let Some(p) = path {
        let config_path = Path::new(p);
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: DrizzleConfig = toml::from_str(&content)?;
            return Ok(Some(config));
        } else {
            anyhow::bail!("Config file not found: {}", p);
        }
    }

    // Try default locations
    for candidate in &["drizzle.toml", "drizzle/drizzle.toml", ".drizzle.toml"] {
        let config_path = Path::new(candidate);
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: DrizzleConfig = toml::from_str(&content)?;
            return Ok(Some(config));
        }
    }

    // No config found is OK - we'll use defaults/CLI args
    Ok(None)
}
