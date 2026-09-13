mod health;
mod status;

use std::io;

use axum::{Router, routing::get};

use crate::zapret::{NfqwsProcess, find_nfqws2_process};

#[derive(Clone, Copy)]
struct AppState {
    find_process: fn() -> io::Result<Option<NfqwsProcess>>,
}

pub fn app() -> Router {
    app_with_state(AppState {
        find_process: find_nfqws2_process,
    })
}

fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/status", get(status::status))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn running_process() -> io::Result<Option<NfqwsProcess>> {
        Ok(Some(NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0".to_vec(),
        }))
    }

    fn failing_process_lookup() -> io::Result<Option<NfqwsProcess>> {
        Err(io::Error::other("process lookup failed"))
    }

    #[tokio::test]
    async fn status_route_returns_running_process() {
        let app = app_with_state(AppState {
            find_process: running_process,
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["running"], true);
        assert_eq!(json["pid"], 123);
        assert_eq!(json["config"]["queue_number"], 200);
    }

    #[tokio::test]
    async fn status_route_returns_internal_server_error() {
        let app = app_with_state(AppState {
            find_process: failing_process_lookup,
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn status_route_rejects_post() {
        let app = app_with_state(AppState {
            find_process: running_process,
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
