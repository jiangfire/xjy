use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::env;
use std::time::Duration;

pub async fn get_database() -> Result<DatabaseConnection, DbErr> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let max_connections: u32 = env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let min_connections: u32 = env::var("DB_MIN_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(max_connections)
        .min_connections(min_connections)
        .connect_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(true);

    Database::connect(opt).await
}
