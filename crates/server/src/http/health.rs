use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Health {
    status: &'static str,
}

pub(super) async fn health() -> Json<Health> {
    Json(Health { status: "UP" })
}
