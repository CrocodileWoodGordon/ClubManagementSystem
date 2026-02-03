use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub mod models;

pub type DbPool = PgPool;

/// Initialize a connection pool consumed by repositories + services.
pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
