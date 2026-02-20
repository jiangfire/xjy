use crate::error::{AppError, AppResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowChallenge {
    pub v: u8,
    pub action: String,
    pub target_type: String,
    pub target_id: i32,
    pub user_id: i32,
    pub issued_at: i64,
    pub expires_at: i64,
    pub difficulty: u8,
    pub salt: String,
}

#[derive(Debug, Clone)]
pub struct PowConfig {
    pub secret: Vec<u8>,
    pub ttl_seconds: i64,
    pub difficulty: u8,
    pub version: u8,
}

impl PowConfig {
    pub fn from_env() -> AppResult<Self> {
        // POW_SECRET is optional: fallback to JWT_SECRET to avoid runtime 500s
        // when only the required JWT secret is configured.
        let secret = std::env::var("POW_SECRET")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| std::env::var("JWT_SECRET").ok())
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("POW_SECRET or JWT_SECRET must be set"))
            })?;

        let ttl_seconds: i64 = std::env::var("POW_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120);

        let difficulty: u8 = std::env::var("POW_DIFFICULTY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);

        Ok(Self {
            secret: secret.into_bytes(),
            ttl_seconds,
            difficulty,
            version: 1,
        })
    }
}

pub fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn generate_salt() -> String {
    let mut buf = [0u8; 16];
    // 低成本随机性：优先使用 OS RNG，如果不可用则退化为时间戳哈希（仅用于 salt，不用于密钥）
    if getrandom::getrandom(&mut buf).is_err() {
        let t = now_epoch_seconds().to_le_bytes();
        let mut h = Sha256::new();
        h.update(t);
        buf.copy_from_slice(&h.finalize()[..16]);
    }
    URL_SAFE_NO_PAD.encode(buf)
}

pub fn sign_challenge(secret: &[u8], challenge: &PowChallenge) -> AppResult<String> {
    let payload = serde_json::to_vec(challenge).map_err(|e| AppError::Internal(e.into()))?;
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|e| AppError::Internal(e.into()))?;
    mac.update(&payload);
    let sig = mac.finalize().into_bytes();
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload),
        URL_SAFE_NO_PAD.encode(sig)
    ))
}

pub fn verify_and_decode_challenge(secret: &[u8], token: &str) -> AppResult<PowChallenge> {
    let (payload_b64, sig_b64) = token
        .split_once('.')
        .ok_or_else(|| AppError::Validation("Invalid pow_token".to_string()))?;

    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| AppError::Validation("Invalid pow_token".to_string()))?;
    let sig = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| AppError::Validation("Invalid pow_token".to_string()))?;

    let mut mac = HmacSha256::new_from_slice(secret).map_err(|e| AppError::Internal(e.into()))?;
    mac.update(&payload);
    mac.verify_slice(&sig)
        .map_err(|_| AppError::Validation("Invalid pow_token signature".to_string()))?;

    let challenge: PowChallenge =
        serde_json::from_slice(&payload).map_err(|e| AppError::Internal(e.into()))?;

    let now = now_epoch_seconds();
    if challenge.expires_at < now {
        return Err(AppError::Validation("pow_token expired".to_string()));
    }

    Ok(challenge)
}

pub fn validate_pow_solution(challenge: &PowChallenge, nonce: &str) -> AppResult<()> {
    if nonce.is_empty() || nonce.len() > 128 {
        return Err(AppError::Validation("Invalid pow_nonce".to_string()));
    }

    // PoW: sha256( action|target_type|target_id|user_id|issued_at|expires_at|difficulty|salt|nonce )
    let mut hasher = Sha256::new();
    hasher.update(challenge.action.as_bytes());
    hasher.update(b"|");
    hasher.update(challenge.target_type.as_bytes());
    hasher.update(b"|");
    hasher.update(challenge.target_id.to_le_bytes());
    hasher.update(b"|");
    hasher.update(challenge.user_id.to_le_bytes());
    hasher.update(b"|");
    hasher.update(challenge.issued_at.to_le_bytes());
    hasher.update(b"|");
    hasher.update(challenge.expires_at.to_le_bytes());
    hasher.update(b"|");
    hasher.update([challenge.difficulty]);
    hasher.update(b"|");
    hasher.update(challenge.salt.as_bytes());
    hasher.update(b"|");
    hasher.update(nonce.as_bytes());
    let digest = hasher.finalize();

    if !has_leading_zero_bits(&digest, challenge.difficulty) {
        return Err(AppError::Validation("Invalid pow solution".to_string()));
    }

    Ok(())
}

fn has_leading_zero_bits(bytes: &[u8], bits: u8) -> bool {
    let full_bytes = (bits / 8) as usize;
    let rem_bits = (bits % 8) as usize;

    if bytes.len() < full_bytes + if rem_bits > 0 { 1 } else { 0 } {
        return false;
    }

    if bytes[..full_bytes].iter().any(|&b| b != 0) {
        return false;
    }

    if rem_bits == 0 {
        return true;
    }

    let mask = 0xFFu8 << (8 - rem_bits);
    (bytes[full_bytes] & mask) == 0
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    struct EnvVarRestore {
        key: &'static str,
        value: Option<String>,
    }

    impl EnvVarRestore {
        fn new(key: &'static str) -> Self {
            Self {
                key,
                value: std::env::var(key).ok(),
            }
        }
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            match &self.value {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn leading_zero_bits_works() {
        let a = [0u8; 32];
        assert!(super::has_leading_zero_bits(&a, 0));
        assert!(super::has_leading_zero_bits(&a, 1));
        assert!(super::has_leading_zero_bits(&a, 8));
        assert!(super::has_leading_zero_bits(&a, 9));

        let b = [0x80u8; 32];
        assert!(!super::has_leading_zero_bits(&b, 1));
        assert!(!super::has_leading_zero_bits(&b, 2));

        let c = [0x00u8, 0x7Fu8];
        assert!(super::has_leading_zero_bits(&c, 9));
        assert!(!super::has_leading_zero_bits(&c, 10));
    }

    #[test]
    fn pow_config_falls_back_to_jwt_secret() {
        let _guard = env_lock().lock().unwrap();
        let _pow_restore = EnvVarRestore::new("POW_SECRET");
        let _jwt_restore = EnvVarRestore::new("JWT_SECRET");

        std::env::remove_var("POW_SECRET");
        std::env::set_var("JWT_SECRET", "jwt-secret-for-pow");

        let cfg = super::PowConfig::from_env().unwrap();
        assert_eq!(cfg.secret, b"jwt-secret-for-pow".to_vec());
    }

    #[test]
    fn pow_config_prefers_pow_secret_when_present() {
        let _guard = env_lock().lock().unwrap();
        let _pow_restore = EnvVarRestore::new("POW_SECRET");
        let _jwt_restore = EnvVarRestore::new("JWT_SECRET");

        std::env::set_var("POW_SECRET", "pow-secret");
        std::env::set_var("JWT_SECRET", "jwt-secret");

        let cfg = super::PowConfig::from_env().unwrap();
        assert_eq!(cfg.secret, b"pow-secret".to_vec());
    }
}
