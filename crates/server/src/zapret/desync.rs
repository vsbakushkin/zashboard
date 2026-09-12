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

#[derive(Debug, PartialEq, Eq)]
pub enum LuaDesyncKind<'a> {
    Wssize(Wssize),
    Multidisorder(Multidisorder<'a>),
    Unknown(&'a [u8]),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LuaDesyncError {
    InvalidParams,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Wssize {
    pub wsize: Option<u32>,
    pub scale: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Multidisorder<'a> {
    pub positions: Vec<&'a [u8]>,
}

impl<'a> LuaDesync<'a> {
    pub fn kind(&self) -> Result<LuaDesyncKind<'a>, LuaDesyncError> {
        match self.function {
            b"wssize" => self
                .as_wssize()
                .map(LuaDesyncKind::Wssize)
                .ok_or(LuaDesyncError::InvalidParams),
            b"multidisorder" => self
                .as_multidisorder()
                .map(LuaDesyncKind::Multidisorder)
                .ok_or(LuaDesyncError::InvalidParams),
            function => Ok(LuaDesyncKind::Unknown(function)),
        }
    }

    pub fn find_param(&self, name: &[u8]) -> Option<&LuaDesyncParam<'a>> {
        self.params.iter().find(|param| param.name == name)
    }

    pub fn param_u32(&self, name: &[u8]) -> Option<u32> {
        self.find_param(name).and_then(LuaDesyncParam::parse_u32)
    }

    pub fn as_wssize(&self) -> Option<Wssize> {
        if self.function != b"wssize" {
            return None;
        }

        let wsize = self.param_u32(b"wsize");
        let scale = self.param_u32(b"scale");

        Some(Wssize { wsize, scale })
    }

    pub fn as_multidisorder(&self) -> Option<Multidisorder<'a>> {
        if self.function != b"multidisorder" {
            return None;
        }

        let positions = match self.find_param(b"pos").and_then(|param| param.value) {
            Some(value) => value
                .split(|byte| *byte == b',')
                .map(|pos| (!pos.is_empty()).then_some(pos))
                .collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };

        Some(Multidisorder { positions })
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
    fn identifies_wssize_kind() {
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
            desync.kind(),
            Ok(LuaDesyncKind::Wssize(Wssize {
                wsize: Some(1),
                scale: Some(6),
            }))
        );
    }

    #[test]
    fn identifies_multidisorder_kind() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![LuaDesyncParam {
                name: b"pos",
                value: Some(b"1,midsld"),
            }],
        };

        assert_eq!(
            desync.kind(),
            Ok(LuaDesyncKind::Multidisorder(Multidisorder {
                positions: vec![b"1", b"midsld"],
            }))
        );
    }

    #[test]
    fn preserves_unknown_desync_kind() {
        let desync = LuaDesync {
            function: b"custom",
            params: vec![],
        };

        assert_eq!(desync.kind(), Ok(LuaDesyncKind::Unknown(b"custom")));
    }

    #[test]
    fn rejects_invalid_known_desync_kind() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![LuaDesyncParam {
                name: b"pos",
                value: Some(b"1,,midsld"),
            }],
        };

        assert_eq!(desync.kind(), Err(LuaDesyncError::InvalidParams));
    }

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

    #[test]
    fn returns_numeric_desync_param() {
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

        assert_eq!(desync.param_u32(b"scale"), Some(6));
    }

    #[test]
    fn returns_none_when_numeric_desync_param_is_absent() {
        let desync = LuaDesync {
            function: b"wssize",
            params: vec![LuaDesyncParam {
                name: b"wsize",
                value: Some(b"1"),
            }],
        };

        assert_eq!(desync.param_u32(b"scale"), None);
    }

    #[test]
    fn returns_none_when_desync_param_is_not_numeric() {
        let desync = LuaDesync {
            function: b"wssize",
            params: vec![LuaDesyncParam {
                name: b"scale",
                value: Some(b"abc"),
            }],
        };

        assert_eq!(desync.param_u32(b"scale"), None);
    }

    #[test]
    fn converts_wssize_desync() {
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
            desync.as_wssize(),
            Some(Wssize {
                wsize: Some(1),
                scale: Some(6),
            })
        );
    }

    #[test]
    fn converts_wssize_with_missing_optional_param() {
        let desync = LuaDesync {
            function: b"wssize",
            params: vec![LuaDesyncParam {
                name: b"wsize",
                value: Some(b"1"),
            }],
        };

        assert_eq!(
            desync.as_wssize(),
            Some(Wssize {
                wsize: Some(1),
                scale: None,
            })
        );
    }

    #[test]
    fn returns_none_for_non_wssize_desync() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![LuaDesyncParam {
                name: b"pos",
                value: Some(b"1"),
            }],
        };

        assert_eq!(desync.as_wssize(), None);
    }

    #[test]
    fn ignores_invalid_numeric_wssize_params() {
        let desync = LuaDesync {
            function: b"wssize",
            params: vec![
                LuaDesyncParam {
                    name: b"wsize",
                    value: Some(b"abc"),
                },
                LuaDesyncParam {
                    name: b"scale",
                    value: Some(b"6"),
                },
            ],
        };

        assert_eq!(
            desync.as_wssize(),
            Some(Wssize {
                wsize: None,
                scale: Some(6),
            })
        );
    }

    #[test]
    fn converts_multidisorder_desync() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![LuaDesyncParam {
                name: b"pos",
                value: Some(b"1,midsld"),
            }],
        };

        assert_eq!(
            desync.as_multidisorder(),
            Some(Multidisorder {
                positions: vec![b"1", b"midsld"],
            })
        );
    }

    #[test]
    fn preserves_symbolic_multidisorder_positions() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![LuaDesyncParam {
                name: b"pos",
                value: Some(b"midsld-1,endhost"),
            }],
        };

        assert_eq!(
            desync.as_multidisorder(),
            Some(Multidisorder {
                positions: vec![b"midsld-1", b"endhost"],
            })
        );
    }

    #[test]
    fn converts_multidisorder_without_positions() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![],
        };

        assert_eq!(
            desync.as_multidisorder(),
            Some(Multidisorder { positions: vec![] })
        );
    }

    #[test]
    fn returns_none_for_non_multidisorder_desync() {
        let desync = LuaDesync {
            function: b"wssize",
            params: vec![],
        };

        assert_eq!(desync.as_multidisorder(), None);
    }

    #[test]
    fn rejects_empty_multidisorder_position() {
        let desync = LuaDesync {
            function: b"multidisorder",
            params: vec![LuaDesyncParam {
                name: b"pos",
                value: Some(b"1,,midsld"),
            }],
        };

        assert_eq!(desync.as_multidisorder(), None);
    }
}
