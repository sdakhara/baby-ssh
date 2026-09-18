use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub connect_timeout_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 10,
        }
    }
}
