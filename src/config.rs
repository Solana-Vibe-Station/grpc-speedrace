use std::env;

#[derive(Clone)]
pub struct Config {
    pub endpoint_a: String,
    pub access_token_a: Option<String>,
    pub endpoint_b: String,
    pub access_token_b: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            endpoint_a: env::var("GEYSER_ENDPOINT_A")
                .unwrap_or_else(|_| "https://your-provider-endpoint-a".to_string()),
            access_token_a: env::var("GEYSER_ACCESS_TOKEN_A").ok(),
            endpoint_b: env::var("GEYSER_ENDPOINT_B")
                .unwrap_or_else(|_| "https://your-provider-endpoint-b".to_string()),
            access_token_b: env::var("GEYSER_ACCESS_TOKEN_B").ok(),
        }
    }
    
    pub fn get_config_a(&self) -> SingleConfig {
        SingleConfig {
            endpoint: self.endpoint_a.clone(),
            access_token: self.access_token_a.clone(),
        }
    }
    
    pub fn get_config_b(&self) -> SingleConfig {
        SingleConfig {
            endpoint: self.endpoint_b.clone(),
            access_token: self.access_token_b.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SingleConfig {
    pub endpoint: String,
    pub access_token: Option<String>,
}