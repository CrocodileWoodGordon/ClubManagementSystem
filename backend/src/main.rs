mod api;
mod config;
mod db;
mod domain;
mod error;
mod services;
mod tasks;
mod utils;

use config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    let app = api::router(pool.clone(), &config.frontend_origin);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    println!("Backend listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
