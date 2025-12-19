use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fs;
use yellowstone_grpc_proto::prelude::CommitmentLevel;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_max_slots")]
    pub max_slots: usize,
    #[serde(default = "default_stop_at_max")]
    pub stop_at_max: bool,
    #[serde(default = "default_commitment")]
    pub commitment: String,
    #[serde(default = "default_warmup_slots")]
    pub warmup_slots: usize,
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

fn default_commitment() -> String {
    "processed".to_string()
}

fn default_warmup_slots() -> usize {
    10
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

        // Validate commitment level
        config.commitment_level()?;

        Ok(config)
    }

    pub fn commitment_level(&self) -> Result<CommitmentLevel> {
        match self.commitment.to_lowercase().as_str() {
            "processed" => Ok(CommitmentLevel::Processed),
            "confirmed" => Ok(CommitmentLevel::Confirmed),
            "finalized" => Ok(CommitmentLevel::Finalized),
            _ => Err(anyhow::anyhow!(
                "Invalid commitment level '{}'. Must be one of: processed, confirmed, finalized",
                self.commitment
            )),
        }
    }
}