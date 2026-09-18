use anyhow::Result;
use tracing_subscriber::EnvFilter;

use crate::storage::paths;

/// Initializes file-backed logging at `%APPDATA%\baby-ssh\logs\baby-ssh.log`.
///
/// The returned guard must be held for the lifetime of `main` — dropping it flushes and closes
/// the non-blocking writer, so an early drop would silently truncate the log.
///
/// Only connection metadata is ever logged here (host, port, username, auth method, lifecycle
/// events). Nothing in this codebase passes a password, passphrase, or key contents to a
/// `tracing` call, so there is no redaction filter to bypass.
pub fn init() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let logs_dir = paths::logs_dir()?;
    std::fs::create_dir_all(&logs_dir)?;
    let file_appender = tracing_appender::rolling::never(&logs_dir, "baby-ssh.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    Ok(guard)
}
