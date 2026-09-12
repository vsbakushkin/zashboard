use super::argument::Argument;
use super::desync::{self, LuaDesync, LuaDesyncError, LuaDesyncKind};

#[derive(Debug, PartialEq, Eq)]
pub enum LuaInit<'a> {
    File(&'a [u8]),
    Code(&'a [u8]),
}

#[derive(Debug, PartialEq, Eq)]
pub struct NfqwsConfig<'a> {
    pub queue_number: Option<u16>,
    pub fwmark: Option<u32>,
    pub lua_inits: Vec<LuaInit<'a>>,
    pub lua_desyncs: Vec<LuaDesync<'a>>,
}

impl<'a> NfqwsConfig<'a> {
    pub fn desync_kinds(
        &self,
    ) -> impl Iterator<Item = Result<LuaDesyncKind<'a>, LuaDesyncError>> + '_ {
        self.lua_desyncs.iter().map(LuaDesync::kind)
    }

    pub fn parse(args: impl IntoIterator<Item = Argument<'a>>) -> Self {
        let args: Vec<Argument<'a>> = args.into_iter().collect();

        let queue_number = args
            .iter()
            .find(|arg| arg.name == b"qnum")
            .and_then(|arg| arg.value)
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.parse::<u16>().ok());

        let fwmark = args
            .iter()
            .find(|arg| arg.name == b"fwmark")
            .and_then(|arg| arg.value)
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.strip_prefix("0x"))
            .and_then(|value| u32::from_str_radix(value, 16).ok());

        let lua_inits = args
            .iter()
            .filter(|arg| arg.name == b"lua-init")
            .filter_map(|arg| {
                let value = arg.value?;

                if let Some(path) = value.strip_prefix(b"@") {
                    return Some(LuaInit::File(path));
                }

                Some(LuaInit::Code(value))
            })
            .collect();

        let lua_desyncs = args
            .iter()
            .filter(|arg| arg.name == b"lua-desync")
            .filter_map(|arg| arg.value.and_then(desync::parse))
            .collect();

        Self {
            queue_number,
            fwmark,
            lua_inits,
            lua_desyncs,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::zapret::desync::{Multidisorder, Wssize};

    use super::super::desync::LuaDesyncParam;
    use super::*;

    fn arg<'a>(name: &'a [u8], value: Option<&'a [u8]>) -> Argument<'a> {
        Argument { name, value }
    }

    #[test]
    fn returns_typed_desync_kinds() {
        let config = NfqwsConfig {
            queue_number: None,
            fwmark: None,
            lua_inits: vec![],
            lua_desyncs: vec![
                LuaDesync {
                    function: b"wssize",
                    params: vec![
                        LuaDesyncParam {
                            name: b"wsize",
                            value: Some(b"1"),
                        },
                        LuaDesyncParam {
                            name: b"scale",
                            value: Some(b"6"),
                        },
                    ],
                },
                LuaDesync {
                    function: b"multidisorder",
                    params: vec![LuaDesyncParam {
                        name: b"pos",
                        value: Some(b"1,midsld"),
                    }],
                },
            ],
        };

        let kinds: Vec<_> = config.desync_kinds().collect();

        assert_eq!(
            kinds,
            [
                Ok(LuaDesyncKind::Wssize(Wssize {
                    wsize: Some(1),
                    scale: Some(6),
                })),
                Ok(LuaDesyncKind::Multidisorder(Multidisorder {
                    positions: vec![b"1", b"midsld"],
                })),
            ]
        );
    }

    #[test]
    fn preserves_unknown_desync_kind_in_config() {
        let config = NfqwsConfig {
            queue_number: None,
            fwmark: None,
            lua_inits: vec![],
            lua_desyncs: vec![LuaDesync {
                function: b"custom",
                params: vec![],
            }],
        };

        let kinds: Vec<_> = config.desync_kinds().collect();

        assert_eq!(kinds, [Ok(LuaDesyncKind::Unknown(b"custom"))]);
    }

    #[test]
    fn preserves_invalid_desync_error_in_config() {
        let config = NfqwsConfig {
            queue_number: None,
            fwmark: None,
            lua_inits: vec![],
            lua_desyncs: vec![LuaDesync {
                function: b"multidisorder",
                params: vec![LuaDesyncParam {
                    name: b"pos",
                    value: Some(b"1,,midsld"),
                }],
            }],
        };

        let kinds: Vec<_> = config.desync_kinds().collect();

        assert_eq!(kinds, [Err(LuaDesyncError::InvalidParams)]);
    }

    #[test]
    fn parses_nfqws_config() {
        let args = vec![
            arg(b"qnum", Some(b"200")),
            arg(b"fwmark", Some(b"0x10000000")),
            arg(b"lua-init", Some(b"@first.lua")),
            arg(b"lua-init", Some(b"MYVAR=123")),
            arg(b"lua-init", Some(b"@second.lua")),
            arg(b"lua-desync", Some(b"wssize:wsize=1:scale=6")),
            arg(b"lua-desync", Some(b"multidisorder:pos=1,midsld")),
        ];

        assert_eq!(
            NfqwsConfig::parse(args),
            NfqwsConfig {
                queue_number: Some(200),
                fwmark: Some(0x10000000),
                lua_inits: vec![
                    LuaInit::File(b"first.lua"),
                    LuaInit::Code(b"MYVAR=123"),
                    LuaInit::File(b"second.lua"),
                ],
                lua_desyncs: vec![
                    LuaDesync {
                        function: b"wssize",
                        params: vec![
                            LuaDesyncParam {
                                name: b"wsize",
                                value: Some(b"1"),
                            },
                            LuaDesyncParam {
                                name: b"scale",
                                value: Some(b"6"),
                            },
                        ],
                    },
                    LuaDesync {
                        function: b"multidisorder",
                        params: vec![LuaDesyncParam {
                            name: b"pos",
                            value: Some(b"1,midsld"),
                        }],
                    },
                ],
            }
        );
    }

    #[test]
    fn parses_config_when_optional_arguments_are_absent() {
        assert_eq!(
            NfqwsConfig::parse(Vec::<Argument<'_>>::new()),
            NfqwsConfig {
                queue_number: None,
                fwmark: None,
                lua_inits: vec![],
                lua_desyncs: vec![],
            }
        );
    }

    #[test]
    fn ignores_invalid_scalar_values() {
        let args = vec![
            arg(b"qnum", Some(b"invalid")),
            arg(b"fwmark", Some(b"0xhello")),
            arg(b"lua-init", Some(b"@valid.lua")),
        ];

        assert_eq!(
            NfqwsConfig::parse(args),
            NfqwsConfig {
                queue_number: None,
                fwmark: None,
                lua_inits: vec![LuaInit::File(b"valid.lua")],
                lua_desyncs: vec![],
            }
        );
    }

    #[test]
    fn ignores_lua_init_without_value() {
        let args = vec![
            arg(b"lua-init", None),
            arg(b"lua-init", Some(b"@valid.lua")),
        ];

        assert_eq!(
            NfqwsConfig::parse(args),
            NfqwsConfig {
                queue_number: None,
                fwmark: None,
                lua_inits: vec![LuaInit::File(b"valid.lua")],
                lua_desyncs: vec![],
            }
        );
    }
}
