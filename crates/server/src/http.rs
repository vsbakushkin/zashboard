mod health;
mod status;

use axum::{Router, routing::get};

pub fn app() -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/status", get(status::status))
}
