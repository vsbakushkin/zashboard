use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::zapret::{NfqwsConfig, NfqwsProcess, find_nfqws2_process};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct Status {
    running: bool,
    pid: Option<u32>,
    command: Option<String>,
    config: Option<ConfigStatus>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ConfigStatus {
    queue_number: Option<u16>,
    fwmark: Option<u32>,
}

impl From<&NfqwsConfig<'_>> for ConfigStatus {
    fn from(config: &NfqwsConfig<'_>) -> Self {
        Self {
            queue_number: config.queue_number,
            fwmark: config.fwmark,
        }
    }
}

impl From<Option<NfqwsProcess>> for Status {
    fn from(process: Option<NfqwsProcess>) -> Self {
        match process {
            Some(process) => {
                let command = process
                    .command()
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned());
                let config = ConfigStatus::from(&process.config());
                Status {
                    running: true,
                    pid: Some(process.pid),
                    command,
                    config: Some(config),
                }
            }
            None => Status {
                running: false,
                pid: None,
                command: None,
                config: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_running_status_with_config() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0\
                        --qnum=200\0\
                        --fwmark=0x10000000\0"
                .to_vec(),
        };

        let status = Status::from(Some(process));
        let json = serde_json::to_value(status).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "running": true,
                "pid": 123,
                "command": "/opt/zapret2/nfq2/nfqws2",
                "config": {
                    "queue_number": 200,
                    "fwmark": 268435456
                }
            })
        );
    }

    #[test]
    fn builds_stopped_status_without_config() {
        let status = Status::from(None);
        let json = serde_json::to_value(status).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "running": false,
                "pid": null,
                "command": null,
                "config": null
            })
        );
    }

    #[test]
    fn preserves_missing_optional_config_values() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0".to_vec(),
        };

        let status = Status::from(Some(process));
        let json = serde_json::to_value(status).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "running": true,
                "pid": 123,
                "command": "/opt/zapret2/nfq2/nfqws2",
                "config": {
                    "queue_number": null,
                    "fwmark": null
                }
            })
        );
    }
}
