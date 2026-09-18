use anyhow::{Context, Result};

use crate::models::AppConfig;

use super::paths;

pub fn load() -> Result<AppConfig> {
    let path = paths::config_file()?;
    if !path.exists() {
        let config = AppConfig::default();
        save(&config)?;
        return Ok(config);
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(AppConfig::default());
    }
    let config = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &AppConfig) -> Result<()> {
    let path = paths::config_file()?;
    let json = serde_json::to_string_pretty(config)?;
    paths::write_atomic(&path, &json)
}
