//! API layer that wires together the Axum routers for every bounded context.

mod attendance;
mod classes;
mod enrollments;
mod health;
mod imports;
mod reports;

use axum::Router;

/// Compose every route tree under a single router instance.
pub fn router() -> Router {
    Router::new()
        .merge(health::router())
        .nest("/api/enrollments", enrollments::router())
        .nest("/api/classes", classes::router())
        .nest("/api/attendance", attendance::router())
        .nest("/api/import", imports::router())
        .nest("/api/reports", reports::router())
}
