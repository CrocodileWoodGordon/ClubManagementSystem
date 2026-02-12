//! API layer that wires together the Axum routers for every bounded context.

mod admin;
mod attendance;
mod classes;
mod clubs;
mod enrollment_status;
mod enrollments;
mod health;
mod imports;
mod reports;
mod students;

use axum::{
    Router,
    http::{HeaderValue, Method},
};
use tower_http::cors::{Any, CorsLayer};

use crate::db::DbPool;

#[derive(Clone)]
pub struct ApiState {
    pub pool: DbPool,
}

/// Compose every route tree under a single router instance.
pub fn router(pool: DbPool, frontend_origin: &str) -> Router {
    let state = ApiState { pool };
    Router::<ApiState>::new()
        .merge(health::router())
        .nest("/api/admin", admin::router())
        .nest("/api/enrollments", enrollments::router())
        .nest("/api/enrollment-status", enrollment_status::router())
        .nest("/api/classes", classes::router())
        .nest("/api/clubs", clubs::router())
        .nest("/api/attendance", attendance::router())
        .nest("/api/import", imports::router())
        .nest("/api/reports", reports::router())
        .nest("/api/students", students::router())
        .with_state(state)
        .layer(configure_cors(frontend_origin))
}

fn configure_cors(origin: &str) -> CorsLayer {
    if origin.trim() == "*" {
        return CorsLayer::very_permissive();
    }

    let value = HeaderValue::from_str(origin)
        .unwrap_or_else(|_| HeaderValue::from_static("http://localhost:3000"));

    CorsLayer::new()
        .allow_origin(value)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
}
