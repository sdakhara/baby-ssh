use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    Password,
    PrivateKey,
}

impl AuthType {
    pub fn label(self) -> &'static str {
        match self {
            AuthType::Password => "Password",
            AuthType::PrivateKey => "Private Key",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            AuthType::Password => AuthType::PrivateKey,
            AuthType::PrivateKey => AuthType::Password,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthType,
    #[serde(default)]
    pub private_key_path: Option<PathBuf>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub last_connected_at: Option<DateTime<Utc>>,
}

impl Connection {
    /// Returns true if this profile's visible metadata matches the search query.
    /// Credentials are never part of the search surface.
    pub fn matches_search(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query)
            || self.host.to_lowercase().contains(&query)
            || self.username.to_lowercase().contains(&query)
            || self
                .description
                .as_ref()
                .is_some_and(|d| d.to_lowercase().contains(&query))
            || self.tags.iter().any(|t| t.to_lowercase().contains(&query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Connection {
        Connection {
            id: Uuid::new_v4(),
            name: "production-api".to_string(),
            host: "192.168.1.20".to_string(),
            port: 22,
            username: "ubuntu".to_string(),
            auth_type: AuthType::PrivateKey,
            private_key_path: None,
            description: Some("Main API box".to_string()),
            tags: vec!["production".to_string(), "api".to_string()],
            favorite: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_connected_at: None,
        }
    }

    #[test]
    fn empty_query_matches_everything() {
        assert!(sample().matches_search(""));
    }

    #[test]
    fn matches_name_case_insensitively() {
        assert!(sample().matches_search("PRODUCTION"));
    }

    #[test]
    fn matches_host_username_description_and_tags() {
        let c = sample();
        assert!(c.matches_search("192.168"));
        assert!(c.matches_search("ubuntu"));
        assert!(c.matches_search("main api"));
        assert!(c.matches_search("api"));
    }

    #[test]
    fn does_not_match_unrelated_query() {
        assert!(!sample().matches_search("staging"));
    }

    #[test]
    fn auth_type_toggles() {
        assert_eq!(AuthType::Password.toggled(), AuthType::PrivateKey);
        assert_eq!(AuthType::PrivateKey.toggled(), AuthType::Password);
    }
}
