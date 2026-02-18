use anyhow::Result;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static JWT_CONFIG: OnceLock<crate::config::jwt::JwtConfig> = OnceLock::new();

/// Initialize JWT config from environment. Must be called once at startup.
pub fn init_jwt_config(config: crate::config::jwt::JwtConfig) -> Result<()> {
    JWT_CONFIG
        .set(config)
        .map_err(|_| anyhow::anyhow!("JWT config already initialized"))?;
    Ok(())
}

fn get_config() -> &'static crate::config::jwt::JwtConfig {
    JWT_CONFIG
        .get()
        .expect("JWT config not initialized — call init_jwt_config() at startup")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,  // expiration time
    pub iat: usize,  // issued at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>, // "access" or "refresh"
}

pub fn encode_access_token(user_id: &str) -> Result<String> {
    let config = get_config();
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: now + config.access_token_expiry as usize,
        iat: now,
        token_type: Some("access".to_string()),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| anyhow::anyhow!("Failed to encode access token: {}", e))
}

pub fn encode_refresh_token(user_id: &str) -> Result<String> {
    let config = get_config();
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: now + config.refresh_token_expiry as usize,
        iat: now,
        token_type: Some("refresh".to_string()),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| anyhow::anyhow!("Failed to encode refresh token: {}", e))
}

pub fn decode_jwt(token: &str) -> Result<Claims> {
    let config = get_config();

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| anyhow::anyhow!("Failed to decode JWT: {}", e))
}

#[allow(dead_code)]
pub fn is_refresh_token(claims: &Claims) -> bool {
    matches!(claims.token_type.as_deref(), Some("refresh"))
}

#[allow(dead_code)]
pub fn is_access_token(claims: &Claims) -> bool {
    matches!(claims.token_type.as_deref(), Some("access"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn ensure_config() {
        INIT.call_once(|| {
            std::env::set_var("JWT_SECRET", "a_very_long_secret_key_that_is_at_least_32_chars");
            let config = crate::config::jwt::JwtConfig::from_env().unwrap();
            let _ = init_jwt_config(config);
        });
    }

    #[test]
    fn encode_decode_round_trip() {
        ensure_config();
        let token = encode_access_token("42").unwrap();
        let claims = decode_jwt(&token).unwrap();
        assert_eq!(claims.sub, "42");
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.token_type, Some("access".to_string()));
    }

    #[test]
    fn refresh_token_encode_decode() {
        ensure_config();
        let token = encode_refresh_token("42").unwrap();
        let claims = decode_jwt(&token).unwrap();
        assert_eq!(claims.sub, "42");
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.token_type, Some("refresh".to_string()));
    }

    #[test]
    fn tampered_token_fails() {
        ensure_config();
        let token = encode_access_token("42").unwrap();
        // Flip a character in the middle of the token
        let mut chars: Vec<char> = token.chars().collect();
        let mid = chars.len() / 2;
        chars[mid] = if chars[mid] == 'A' { 'B' } else { 'A' };
        let tampered: String = chars.into_iter().collect();
        assert!(decode_jwt(&tampered).is_err());
    }

    #[test]
    fn expired_token_fails() {
        ensure_config();
        let config = get_config();
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: "42".to_string(),
            exp: now - 3600, // expired 1 hour ago
            iat: now - 7200,
            token_type: Some("access".to_string()),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.secret.as_bytes()),
        )
        .unwrap();
        assert!(decode_jwt(&token).is_err());
    }

    #[test]
    fn empty_token_fails() {
        ensure_config();
        assert!(decode_jwt("").is_err());
    }
}
