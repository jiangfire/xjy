use redis::aio::ConnectionManager;

pub async fn get_redis() -> anyhow::Result<ConnectionManager> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let client = redis::Client::open(redis_url)?;
    let manager = ConnectionManager::new(client).await?;
    Ok(manager)
}
