use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone)]
pub struct CacheService {
    redis: ConnectionManager,
}

impl CacheService {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.redis.clone();
        let result: Option<String> = conn.get(key).await.ok()?;
        result.and_then(|s| serde_json::from_str(&s).ok())
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) {
        let mut conn = self.redis.clone();
        if let Ok(json) = serde_json::to_string(value) {
            let _: Result<(), _> = conn.set_ex(key, json, ttl_secs).await;
        }
    }

    pub async fn invalidate(&self, key: &str) {
        let mut conn = self.redis.clone();
        let _: Result<(), _> = conn.del(key).await;
    }

    #[allow(dead_code)]
    pub async fn invalidate_pattern(&self, pattern: &str) {
        let mut conn = self.redis.clone();
        if let Ok(keys) = redis::cmd("KEYS")
            .arg(pattern)
            .query_async::<Vec<String>>(&mut conn)
            .await
        {
            if !keys.is_empty() {
                let _: Result<(), _> = conn.del(keys).await;
            }
        }
    }
}
