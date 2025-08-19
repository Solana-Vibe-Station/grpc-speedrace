use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fs;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_max_slots")]
    pub max_slots: usize,
    #[serde(default = "default_stop_at_max")]
    pub stop_at_max: bool,
    pub streams: Vec<StreamConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub name: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

fn default_max_slots() -> usize {
    360
}

fn default_stop_at_max() -> bool {
    false
}

impl Config {
    pub fn from_file() -> Result<Self> {
        let content = fs::read_to_string("config.toml")
            .map_err(|e| anyhow::anyhow!("Failed to read config.toml: {}", e))?;
            
        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config.toml: {}", e))?;
            
        if config.streams.is_empty() {
            return Err(anyhow::anyhow!("No streams configured in config.toml"));
        }
        
        Ok(config)
    }
}