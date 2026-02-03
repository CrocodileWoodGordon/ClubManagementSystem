use dotenvy::var;

/// Centralized strongly-typed configuration derived from env vars.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub frontend_origin: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Self {
            database_url: var("DATABASE_URL")?,
            port: var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            frontend_origin: var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
        })
    }
}
