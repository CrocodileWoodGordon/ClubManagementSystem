//! API layer that wires together the Axum routers for every bounded context.

mod attendance;
mod admin;
mod classes;
mod enrollments;
mod health;
mod imports;
mod reports;

use axum::Router;

use crate::db::DbPool;

#[derive(Clone)]
pub struct ApiState {
    pub pool: DbPool,
}

/// Compose every route tree under a single router instance.
pub fn router(pool: DbPool) -> Router {
    let state = ApiState { pool };
    Router::<ApiState>::new()
        .merge(health::router())
        .nest("/api/admin", admin::router())
        .nest("/api/enrollments", enrollments::router())
        .nest("/api/classes", classes::router())
        .nest("/api/attendance", attendance::router())
        .nest("/api/import", imports::router())
        .nest("/api/reports", reports::router())
        .with_state(state)
}
