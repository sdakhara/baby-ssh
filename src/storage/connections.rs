use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Connection;

use super::paths;

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConnectionsFile {
    #[serde(default)]
    connections: Vec<Connection>,
}

/// Owns the on-disk list of connection profiles (metadata only — never credentials).
pub struct ConnectionStore {
    path: PathBuf,
    connections: Vec<Connection>,
}

impl ConnectionStore {
    pub fn load() -> Result<Self> {
        let path = paths::connections_file()?;
        let connections = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            if raw.trim().is_empty() {
                Vec::new()
            } else {
                let file: ConnectionsFile = serde_json::from_str(&raw)
                    .with_context(|| format!("failed to parse {}", path.display()))?;
                file.connections
            }
        } else {
            Vec::new()
        };
        Ok(Self { path, connections })
    }

    fn save(&self) -> Result<()> {
        let file = ConnectionsFile {
            connections: self.connections.clone(),
        };
        let json = serde_json::to_string_pretty(&file)?;
        paths::write_atomic(&self.path, &json)
    }

    pub fn all(&self) -> &[Connection] {
        &self.connections
    }

    pub fn get(&self, id: Uuid) -> Option<&Connection> {
        self.connections.iter().find(|c| c.id == id)
    }

    /// Inserts a new connection or replaces an existing one with the same id, then persists.
    pub fn upsert(&mut self, connection: Connection) -> Result<()> {
        if let Some(existing) = self.connections.iter_mut().find(|c| c.id == connection.id) {
            *existing = connection;
        } else {
            self.connections.push(connection);
        }
        self.save()
    }

    pub fn delete(&mut self, id: Uuid) -> Result<Option<Connection>> {
        let removed = if let Some(idx) = self.connections.iter().position(|c| c.id == id) {
            Some(self.connections.remove(idx))
        } else {
            None
        };
        if removed.is_some() {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn toggle_favorite(&mut self, id: Uuid) -> Result<()> {
        if let Some(c) = self.connections.iter_mut().find(|c| c.id == id) {
            c.favorite = !c.favorite;
            c.updated_at = chrono::Utc::now();
        }
        self.save()
    }

    pub fn touch_last_connected(&mut self, id: Uuid) -> Result<()> {
        if let Some(c) = self.connections.iter_mut().find(|c| c.id == id) {
            c.last_connected_at = Some(chrono::Utc::now());
        }
        self.save()
    }

    /// Connections filtered by `query` (empty matches everything) and sorted with favorites
    /// first, then alphabetically by name.
    pub fn search(&self, query: &str) -> Vec<&Connection> {
        let mut results: Vec<&Connection> = self
            .connections
            .iter()
            .filter(|c| c.matches_search(query))
            .collect();
        results.sort_by(|a, b| {
            b.favorite
                .cmp(&a.favorite)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        results
    }
}
