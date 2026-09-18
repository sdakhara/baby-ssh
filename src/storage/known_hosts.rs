use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownHostEntry {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
    /// The full OpenSSH-encoded public key, used for exact comparison. The fingerprint alone
    /// is only for display — comparing the full key avoids any (extremely unlikely) fingerprint
    /// collision being treated as a trusted match.
    pub key_openssh: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct KnownHostsFile {
    #[serde(default)]
    hosts: Vec<KnownHostEntry>,
}

pub struct KnownHostsStore {
    path: PathBuf,
    entries: Vec<KnownHostEntry>,
}

impl KnownHostsStore {
    pub fn load() -> Result<Self> {
        let path = paths::known_hosts_file()?;
        let entries = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            if raw.trim().is_empty() {
                Vec::new()
            } else {
                let file: KnownHostsFile = serde_json::from_str(&raw)
                    .with_context(|| format!("failed to parse {}", path.display()))?;
                file.hosts
            }
        } else {
            Vec::new()
        };
        Ok(Self { path, entries })
    }

    fn save(&self) -> Result<()> {
        let file = KnownHostsFile {
            hosts: self.entries.clone(),
        };
        let json = serde_json::to_string_pretty(&file)?;
        paths::write_atomic(&self.path, &json)
    }

    pub fn lookup(&self, host: &str, port: u16) -> Option<&KnownHostEntry> {
        self.entries
            .iter()
            .find(|e| e.host == host && e.port == port)
    }

    /// Records `host` as trusted with the given key, persisting immediately.
    pub fn trust(
        &mut self,
        host: &str,
        port: u16,
        algorithm: &str,
        fingerprint: &str,
        key_openssh: &str,
    ) -> Result<()> {
        let now = Utc::now();
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.host == host && e.port == port)
        {
            entry.algorithm = algorithm.to_string();
            entry.fingerprint = fingerprint.to_string();
            entry.key_openssh = key_openssh.to_string();
            entry.last_seen = now;
        } else {
            self.entries.push(KnownHostEntry {
                host: host.to_string(),
                port,
                algorithm: algorithm.to_string(),
                fingerprint: fingerprint.to_string(),
                key_openssh: key_openssh.to_string(),
                first_seen: now,
                last_seen: now,
            });
        }
        self.save()
    }
}
