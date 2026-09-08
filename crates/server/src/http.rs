mod health;

use axum::{Router, routing::get};

pub fn app() -> Router {
    Router::new().route("/health", get(health::health))
}
