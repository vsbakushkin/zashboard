use axum::{Json, http::StatusCode};
use serde::Serialize;
use std::io;

use crate::zapret::{
    LuaDesync, LuaDesyncError, LuaDesyncKind, LuaInit, NfqwsConfig, NfqwsProcess,
    find_nfqws2_process,
};

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
    lua_inits: Vec<LuaInitStatus>,
    lua_desyncs: Vec<LuaDesyncStatus>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
enum LuaInitStatus {
    File(String),
    Code(String),
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LuaDesyncStatus {
    Wssize {
        wsize: Option<u32>,
        scale: Option<u32>,
    },
    Multidisorder {
        positions: Vec<String>,
    },
    Unknown {
        function: String,
    },
    Invalid {
        function: String,
    },
}

impl From<&LuaDesync<'_>> for LuaDesyncStatus {
    fn from(desync: &LuaDesync<'_>) -> Self {
        match desync.kind() {
            Ok(LuaDesyncKind::Wssize(wssize)) => LuaDesyncStatus::Wssize {
                wsize: wssize.wsize,
                scale: wssize.scale,
            },
            Ok(LuaDesyncKind::Multidisorder(multidisorder)) => LuaDesyncStatus::Multidisorder {
                positions: multidisorder
                    .positions
                    .iter()
                    .map(|pos| String::from_utf8_lossy(pos).into_owned())
                    .collect(),
            },
            Ok(LuaDesyncKind::Unknown(function)) => LuaDesyncStatus::Unknown {
                function: String::from_utf8_lossy(function).into_owned(),
            },
            Err(LuaDesyncError::InvalidParams) => LuaDesyncStatus::Invalid {
                function: String::from_utf8_lossy(desync.function).into_owned(),
            },
        }
    }
}

impl From<&LuaInit<'_>> for LuaInitStatus {
    fn from(init: &LuaInit<'_>) -> Self {
        match init {
            LuaInit::File(value) => {
                LuaInitStatus::File(String::from_utf8_lossy(value).into_owned())
            }
            LuaInit::Code(value) => {
                LuaInitStatus::Code(String::from_utf8_lossy(value).into_owned())
            }
        }
    }
}

impl From<&NfqwsConfig<'_>> for ConfigStatus {
    fn from(config: &NfqwsConfig<'_>) -> Self {
        Self {
            queue_number: config.queue_number,
            fwmark: config.fwmark,
            lua_inits: config.lua_inits.iter().map(LuaInitStatus::from).collect(),
            lua_desyncs: config
                .lua_desyncs
                .iter()
                .map(LuaDesyncStatus::from)
                .collect(),
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
    status_with(find_nfqws2_process).await
}

async fn status_with(
    find_process: impl FnOnce() -> io::Result<Option<NfqwsProcess>> + Send + 'static,
) -> Result<Json<Status>, StatusCode> {
    let task_result = tokio::task::spawn_blocking(find_process)
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
                        --fwmark=0x10000000\0\
                        --lua-init=@/opt/zapret2/lua/zapret-lib.lua\0\
                        --lua-init=MYVAR=123\0\
                        --lua-desync=wssize:wsize=1:scale=6\0\
                        --lua-desync=multidisorder:pos=1,midsld\0"
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
                    "fwmark": 268435456,
                    "lua_inits": [
                        {
                            "type": "File",
                            "value": "/opt/zapret2/lua/zapret-lib.lua"
                        },
                        {
                            "type": "Code",
                            "value": "MYVAR=123"
                        }
                    ],
                    "lua_desyncs": [
                        {
                            "type": "wssize",
                            "wsize": 1,
                            "scale": 6
                        },
                        {
                            "type": "multidisorder",
                            "positions": ["1", "midsld"]
                        }
                    ]
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
                "config": null,
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
                    "fwmark": null,
                    "lua_inits": [],
                    "lua_desyncs": []
                }
            })
        );
    }

    #[test]
    fn exposes_unknown_lua_desync() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--lua-desync=custom:param=1\0".to_vec(),
        };

        let status = Status::from(Some(process));
        let json = serde_json::to_value(status).unwrap();

        assert_eq!(
            json["config"]["lua_desyncs"],
            serde_json::json!([
                {
                    "type": "unknown",
                    "function": "custom"
                }
            ])
        );
    }

    #[test]
    fn exposes_invalid_lua_desync() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--lua-desync=multidisorder:pos=1,,midsld\0"
                .to_vec(),
        };

        let status = Status::from(Some(process));
        let json = serde_json::to_value(status).unwrap();

        assert_eq!(
            json["config"]["lua_desyncs"],
            serde_json::json!([
                {
                    "type": "invalid",
                    "function": "multidisorder"
                }
            ])
        );
    }

    #[tokio::test]
    async fn returns_running_status() {
        let response = status_with(|| {
            Ok(Some(NfqwsProcess {
                pid: 123,
                cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0".to_vec(),
            }))
        })
        .await
        .expect("status should succeed");

        assert!(response.0.running);
        assert_eq!(response.0.pid, Some(123));
    }

    #[tokio::test]
    async fn returns_stopped_status() {
        let response = status_with(|| Ok(None))
            .await
            .expect("status should succeed");

        assert!(!response.0.running);
        assert_eq!(response.0.pid, None);
    }

    #[tokio::test]
    async fn returns_internal_server_error_when_process_lookup_fails() {
        let result = status_with(|| Err(std::io::Error::other("process lookup failed"))).await;

        assert_eq!(result.err(), Some(StatusCode::INTERNAL_SERVER_ERROR));
    }
}
