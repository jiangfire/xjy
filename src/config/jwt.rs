use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry: u64,   // 15 minutes
    pub refresh_token_expiry: u64,   // 7 days
}

impl JwtConfig {
    pub fn from_env() -> Result<Self> {
        let secret = env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET environment variable must be set"))?;

        if secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT_SECRET must be at least 32 characters"
            ));
        }

        let access_token_expiry = env::var("JWT_ACCESS_EXPIRATION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(900); // 15 minutes

        let refresh_token_expiry = env::var("JWT_REFRESH_EXPIRATION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(604800); // 7 days

        Ok(Self {
            secret,
            access_token_expiry,
            refresh_token_expiry,
        })
    }
}