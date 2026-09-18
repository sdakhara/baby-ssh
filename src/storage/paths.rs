use std::path::PathBuf;

use anyhow::{Context, Result};

/// Root data directory for baby-ssh.
///
/// On Windows this resolves to `%APPDATA%\baby-ssh`. `dirs::config_dir()` resolves to the
/// platform-appropriate equivalent on Linux/macOS, which is how this stays cross-platform
/// without a rewrite when Phase 2 lands.
pub fn app_data_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine the platform config directory")?;
    Ok(base.join("baby-ssh"))
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("config"))
}

pub fn connections_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("connections"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("logs"))
}

pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.json"))
}

pub fn connections_file() -> Result<PathBuf> {
    Ok(connections_dir()?.join("connections.json"))
}

pub fn known_hosts_file() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("known_hosts"))
}

/// Creates every directory baby-ssh writes to. Safe to call on every startup.
pub fn ensure_dirs() -> Result<()> {
    std::fs::create_dir_all(config_dir()?)?;
    std::fs::create_dir_all(connections_dir()?)?;
    std::fs::create_dir_all(logs_dir()?)?;
    Ok(())
}

/// Writes `contents` to `path` atomically by writing to a sibling temp file and renaming it
/// into place, so a crash or power loss mid-write can never corrupt the previous file.
pub fn write_atomic(path: &std::path::Path, contents: &str) -> Result<()> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, contents)
        .with_context(|| format!("failed to write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}
