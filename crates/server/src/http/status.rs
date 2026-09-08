use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::zapret::{NfqwsProcess, find_nfqws2_process};

#[derive(Serialize)]
pub(super) struct Status {
    running: bool,
    pid: Option<u32>,
    command: Option<String>,
}

impl From<Option<NfqwsProcess>> for Status {
    fn from(process: Option<NfqwsProcess>) -> Self {
        match process {
            Some(process) => {
                let command = process
                    .command()
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
                Status {
                    running: true,
                    pid: Some(process.pid),
                    command,
                }
            }
            None => Status {
                running: false,
                pid: None,
                command: None,
            },
        }
    }
}

pub(super) async fn status() -> Result<Json<Status>, StatusCode> {
    let task_result = tokio::task::spawn_blocking(find_nfqws2_process)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Status::from(task_result)))
}
