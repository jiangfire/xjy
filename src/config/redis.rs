use redis::aio::ConnectionManager;
use tokio::time::{timeout, Duration};

pub async fn get_redis() -> anyhow::Result<ConnectionManager> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let client = redis::Client::open(redis_url)?;
    
    // Add 5 second timeout for Redis connection
    let manager = timeout(Duration::from_secs(5), ConnectionManager::new(client))
        .await
        .map_err(|_| anyhow::anyhow!("Redis connection timeout after 5 seconds"))??;
    
    Ok(manager)
}
