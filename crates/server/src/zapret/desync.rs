#[derive(Debug, PartialEq, Eq)]
pub struct LuaDesync<'a> {
    pub function: &'a [u8],
    pub params: Vec<LuaDesyncParam<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LuaDesyncParam<'a> {
    pub name: &'a [u8],
    pub value: Option<&'a [u8]>,
}

impl<'a> LuaDesync<'a> {
    pub fn find_param(&self, name: &[u8]) -> Option<&LuaDesyncParam<'a>> {
        self.params.iter().find(|param| param.name == name)
    }
}

impl<'a> LuaDesyncParam<'a> {
    pub fn parse(param: &'a [u8]) -> Option<Self> {
        let mut parts = param.splitn(2, |byte| *byte == b'=');
        let name = parts.next()?;
        if name.is_empty() {
            return None;
        }
        let value = parts.next();
        Some(Self { name, value })
    }

    pub fn parse_u32(&self) -> Option<u32> {
        self.value
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.parse::<u32>().ok())
    }
}

pub fn parse(value: &[u8]) -> Option<LuaDesync<'_>> {
    let mut parts = value.split(|byte| *byte == b':');

    let function = parts.next()?;
    if function.is_empty() {
        return None;
    }

    let params: Vec<_> = parts
        .map(LuaDesyncParam::parse)
        .collect::<Option<Vec<_>>>()?;

    Some(LuaDesync { function, params })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_desync_param_by_name() {
        let desync = LuaDesync {
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
        };

        assert_eq!(
            desync.find_param(b"scale"),
            Some(&LuaDesyncParam {
                name: b"scale",
                value: Some(b"6"),
            })
        );
    }

    #[test]
    fn returns_none_when_desync_param_is_absent() {
        let desync = LuaDesync {
            function: b"wssize",
            params: vec![LuaDesyncParam {
                name: b"wsize",
                value: Some(b"1"),
            }],
        };

        assert_eq!(desync.find_param(b"scale"), None);
    }

    #[test]
    fn returns_first_desync_param_when_name_is_repeated() {
        let desync = LuaDesync {
            function: b"example",
            params: vec![
                LuaDesyncParam {
                    name: b"pos",
                    value: Some(b"1"),
                },
                LuaDesyncParam {
                    name: b"pos",
                    value: Some(b"2"),
                },
            ],
        };

        assert_eq!(
            desync.find_param(b"pos"),
            Some(&LuaDesyncParam {
                name: b"pos",
                value: Some(b"1"),
            })
        );
    }

    #[test]
    fn parses_desync_param_with_value() {
        assert_eq!(
            LuaDesyncParam::parse(b"wsize=1"),
            Some(LuaDesyncParam {
                name: b"wsize",
                value: Some(b"1"),
            })
        );
    }

    #[test]
    fn parses_bare_desync_param() {
        assert_eq!(
            LuaDesyncParam::parse(b"multisplit"),
            Some(LuaDesyncParam {
                name: b"multisplit",
                value: None,
            })
        );
    }

    #[test]
    fn preserves_commas_in_desync_param_value() {
        assert_eq!(
            LuaDesyncParam::parse(b"pos=1,midsld"),
            Some(LuaDesyncParam {
                name: b"pos",
                value: Some(b"1,midsld"),
            })
        );
    }

    #[test]
    fn preserves_empty_desync_param_value() {
        assert_eq!(
            LuaDesyncParam::parse(b"foo="),
            Some(LuaDesyncParam {
                name: b"foo",
                value: Some(b""),
            })
        );
    }

    #[test]
    fn preserves_additional_equals_in_desync_param_value() {
        assert_eq!(
            LuaDesyncParam::parse(b"foo=a=b=c"),
            Some(LuaDesyncParam {
                name: b"foo",
                value: Some(b"a=b=c"),
            })
        );
    }

    #[test]
    fn rejects_empty_desync_param() {
        assert_eq!(LuaDesyncParam::parse(b""), None);
    }

    #[test]
    fn rejects_desync_param_with_empty_name() {
        assert_eq!(LuaDesyncParam::parse(b"=value"), None);
    }

    #[test]
    fn parses_desync_with_multiple_params() {
        assert_eq!(
            parse(b"wssize:wsize=1:scale=6"),
            Some(LuaDesync {
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
            })
        );
    }

    #[test]
    fn preserves_commas_inside_param_value() {
        assert_eq!(
            parse(b"multidisorder:pos=1,midsld"),
            Some(LuaDesync {
                function: b"multidisorder",
                params: vec![LuaDesyncParam {
                    name: b"pos",
                    value: Some(b"1,midsld"),
                }],
            })
        );
    }

    #[test]
    fn parses_desync_without_params() {
        assert_eq!(
            parse(b"syndata"),
            Some(LuaDesync {
                function: b"syndata",
                params: vec![],
            })
        );
    }

    #[test]
    fn preserves_bare_params() {
        assert_eq!(
            parse(b"syndata:multisplit:strategy=23"),
            Some(LuaDesync {
                function: b"syndata",
                params: vec![
                    LuaDesyncParam {
                        name: b"multisplit",
                        value: None,
                    },
                    LuaDesyncParam {
                        name: b"strategy",
                        value: Some(b"23"),
                    },
                ],
            })
        );
    }

    #[test]
    fn rejects_empty_value() {
        assert_eq!(parse(b""), None);
    }

    #[test]
    fn rejects_empty_function() {
        assert_eq!(parse(b":wsize=1"), None);
    }

    #[test]
    fn rejects_empty_param() {
        assert_eq!(parse(b"wssize::scale=6"), None);
    }

    #[test]
    fn parses_desync_param_value_as_u32() {
        let param = LuaDesyncParam {
            name: b"scale",
            value: Some(b"6"),
        };

        assert_eq!(param.parse_u32(), Some(6));
    }

    #[test]
    fn returns_none_when_desync_param_has_no_value() {
        let param = LuaDesyncParam {
            name: b"multisplit",
            value: None,
        };

        assert_eq!(param.parse_u32(), None);
    }

    #[test]
    fn returns_none_when_desync_param_value_is_not_a_number() {
        let param = LuaDesyncParam {
            name: b"scale",
            value: Some(b"abc"),
        };

        assert_eq!(param.parse_u32(), None);
    }
}
