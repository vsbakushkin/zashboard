use super::argument::Argument;

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
}

impl<'a> NfqwsConfig<'a> {
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

        Self {
            queue_number,
            fwmark,
            lua_inits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg<'a>(name: &'a [u8], value: Option<&'a [u8]>) -> Argument<'a> {
        Argument { name, value }
    }

    #[test]
    fn parses_nfqws_config() {
        let args = vec![
            arg(b"qnum", Some(b"200")),
            arg(b"fwmark", Some(b"0x10000000")),
            arg(b"lua-init", Some(b"@first.lua")),
            arg(b"lua-init", Some(b"MYVAR=123")),
            arg(b"lua-init", Some(b"@second.lua")),
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
            }
        );
    }
}
